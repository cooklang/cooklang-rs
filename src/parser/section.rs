use crate::{error::label, lexer::T};

use super::{tokens_span, warning, BlockParser, Event};

pub(crate) fn section<'i>(block: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    block.consume(T![=])?;
    block.consume_while(|t| t == T![=]);
    let name_pos = block.current_offset();
    let name_tokens = block.consume_while(|t| t != T![=]);
    let name = block.text(name_pos, name_tokens);
    block.consume_while(|t| t == T![=]);
    block.ws_comments();

    if !block.rest().is_empty() {
        block.warn(
            warning!(
                "A section block is invalid and it will be a step",
                label!(tokens_span(block.rest()), "remove this"),
            )
            .hint("After the ending `=` the line must end for it to be a valid section"),
        );
        return None;
    }

    let name = if name.is_text_empty() {
        None
    } else {
        Some(name)
    };
    Some(Event::Section { name })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{
        parser::{token_stream::TokenStream, BlockParser},
        span::Span,
        Extensions,
    };
    use test_case::test_case;

    macro_rules! text {
        ($s:expr; $offset:expr) => {
            text!($s; $offset, $offset + $s.len())
        };
        ($s:expr; $start:expr, $end:expr) => {
            Some(($s.to_string(), Span::new($start, $end)))
        };
    }

    #[test_case("= section" => text!(" section"; 1); "single char")]
    #[test_case("== section ==" => text!(" section "; 2) ; "fenced")]
    #[test_case("=" => None ; "no name single char")]
    #[test_case("===" => None ; "no name multiple char")]
    #[test_case("= ==" => None ; "no name unbalanced")]
    #[test_case("= = ==" => panics "failed to parse section" ; "more than one split")]
    #[test_case("== section ==    " => text!(" section "; 2) ; "trailing whitespace")]
    #[test_case("== section ==  -- comment  " => text!(" section "; 2) ; "trailing line comment")]
    #[test_case("== section ==  [- comment -]  " => text!(" section "; 2) ; "trailing block comment")]
    #[test_case("== section [- and a comment = -] ==" => text!(" section  "; 2, 33) ; "in between block comment")]
    #[test_case("== section -- and a comment" => text!(" section "; 2) ; "in between line comment")]
    fn test_section(input: &'static str) -> Option<(String, Span)> {
        let tokens = TokenStream::new(input).collect::<Vec<_>>();
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let event = section(&mut bp).expect("failed to parse section");
        bp.finish();
        assert!(events.is_empty());
        let Event::Section { name } = event else {
            panic!()
        };
        name.map(|text| (text.text().into_owned(), text.span()))
    }
}
