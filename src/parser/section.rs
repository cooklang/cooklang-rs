use crate::{error::label, lexer::T, Extensions};

use super::{
    spec2::{find_tail_arrow, is_punct, label_len, parse_tail},
    token_stream::Token,
    tokens_span, warning, BlockParser, Event, ProductDecl,
};

pub(crate) fn section<'i>(block: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    block.consume(T![=])?;
    block.consume_while(|t| t == T![=]);
    let name_pos = block.current_offset();
    let mut name_tokens = block.consume_while(|t| t != T![=]);

    // spec 2.0: `= Name ?label ~{anchor} [list] -> product`
    let mut product: Option<ProductDecl<'i>> = None;
    let mut label = None;
    let mut attributes = Vec::new();
    if block.extension(Extensions::PRODUCTS) {
        if let Some(arrow) = find_tail_arrow(block, name_tokens) {
            let tail = &name_tokens[arrow + 2..];
            name_tokens = &name_tokens[..arrow];
            let mut products = parse_tail(block, tail);
            if products.len() > 1 {
                block.warn(warning!(
                    "A section names one product; the others are ignored",
                    label!(tokens_span(tail))
                ));
            }
            product = (!products.is_empty()).then(|| products.remove(0));
        }
    }
    if block.extension(Extensions::ALTERNATIVES) || block.extension(Extensions::PRODUCTS) {
        // the name ends where the first attribute starts
        let starts = |i: usize, toks: &[Token]| -> bool {
            let t = toks[i];
            (t.kind == T![?] && label_len(block, &toks[i..]).is_some()
                && (i == 0 || toks[i - 1].kind == T![ws]))
                || (t.kind == T![~] && toks.get(i + 1).map(|n| n.kind) == Some(T!['{']))
                || is_punct(block, t, "[")
        };
        if let Some(cut) = (0..name_tokens.len()).find(|&i| starts(i, name_tokens)) {
            let mut rest = &name_tokens[cut..];
            name_tokens = &name_tokens[..cut];
            while !rest.is_empty() {
                let t = rest[0];
                if t.kind == T![?] {
                    if let Some(n) = label_len(block, rest) {
                        label = Some(block.text(rest[1].span.start(), &rest[1..n]));
                        rest = &rest[n..];
                        continue;
                    }
                }
                if t.kind == T![~] && rest.get(1).map(|n| n.kind) == Some(T!['{']) {
                    if let Some(end) = rest.iter().position(|t| t.kind == T!['}']) {
                        attributes.push(block.text(t.span.start(), &rest[..=end]));
                        rest = &rest[end + 1..];
                        continue;
                    }
                }
                if is_punct(block, t, "[") {
                    if let Some(end) = rest.iter().position(|x| is_punct(block, *x, "]")) {
                        attributes.push(block.text(rest[1].span.start(), &rest[1..end]));
                        rest = &rest[end + 1..];
                        continue;
                    }
                }
                rest = &rest[1..];
            }
        }
    }

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
    Some(Event::Section {
        name,
        product,
        label,
        attributes,
    })
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
        let Event::Section { name, .. } = event else {
            panic!()
        };
        name.map(|text| (text.text().into_owned(), text.span()))
    }
}
