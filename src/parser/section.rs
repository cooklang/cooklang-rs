use crate::{ast, lexer::T};

use super::LineParser;

pub(crate) fn section<'input>(
    line: &mut LineParser<'_, 'input>,
) -> Option<Option<ast::Text<'input>>> {
    line.consume(T![=])?;
    line.consume_while(|t| t == T![=]);
    let name_pos = line.current_offset();
    let name_tokens = line.consume_while(|t| t != T![=]);
    let name = line.text(name_pos, name_tokens);
    line.consume_while(|t| t == T![=]);
    line.ws_comments();

    if !line.rest().is_empty() {
        return None;
    }

    if name.is_text_empty() {
        Some(None)
    } else {
        Some(Some(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parser::{token_stream::TokenStream, LineParser},
        span::Span,
        Extensions,
    };
    use test_case::test_case;

    macro_rules! text {
        ($s:expr; $offset:expr) => {
            text!($s; $offset, $offset + $s.len())
        };
        ($s:expr; $start:expr, $end:expr) => {
            ($s.to_string(), Span::new($start, $end))
        };
    }

    #[test_case("= section" => Some(text!(" section"; 1)); "single char")]
    #[test_case("== section ==" => Some(text!(" section "; 2)) ; "fenced")]
    #[test_case("=" => None ; "no name single char")]
    #[test_case("===" => None ; "no name multiple char")]
    #[test_case("= ==" => None ; "no name unbalanced")]
    #[test_case("= = ==" => panics "failed to parse section" ; "more than one split")]
    #[test_case("== section ==    " => Some(text!(" section "; 2)) ; "trailing whitespace")]
    #[test_case("== section ==  -- comment  " => Some(text!(" section "; 2)) ; "trailing line comment")]
    #[test_case("== section ==  [- comment -]  " => Some(text!(" section "; 2)) ; "trailing block comment")]
    #[test_case("== section [- and a comment = -] ==" => Some(text!(" section  "; 2, 33)) ; "in between block comment")]
    #[test_case("== section -- and a comment" => Some(text!(" section "; 2)) ; "in between line comment")]
    fn test_section(input: &'static str) -> Option<(String, Span)> {
        let tokens = TokenStream::new(input).collect::<Vec<_>>();
        let mut line = LineParser::new(0, &tokens, input, Extensions::all());
        section(&mut line)
            .expect("failed to parse section")
            .map(|text| (text.text().into_owned(), text.span()))
    }
}
