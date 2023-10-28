use crate::lexer::T;

use super::{BlockKind, BlockParser, Event};

pub(crate) fn parse_text_block(bp: &mut BlockParser) {
    bp.event(Event::Start(BlockKind::Text));

    while !bp.rest().is_empty() {
        // skip > and leading whitespace
        let _ = bp.consume(T![>]).and_then(|_| bp.consume(T![ws]));
        let start = bp.current_offset();
        let tokens = bp.capture_slice(|bp| {
            bp.consume_while(|t| t != T![newline]);
            let _ = bp.consume(T![newline]);
        });
        let text = bp.text(start, tokens);
        if !text.is_text_empty() {
            bp.event(Event::Text(text));
        }
    }

    bp.event(Event::End(BlockKind::Text));
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        error::SourceReport,
        parser::{mt, token_stream::TokenStream},
        Extensions,
    };

    use super::*;
    use indoc::indoc;
    use test_case::test_case;

    fn t(input: &str) -> (Vec<Event>, SourceReport) {
        let mut tokens = TokenStream::new(input).collect::<Vec<_>>();
        // trim trailing newlines, block splitting should make sure this never
        // reaches the step function
        while let Some(mt![newline]) = tokens.last() {
            tokens.pop();
        }
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        parse_text_block(&mut bp);
        bp.finish();
        let mut other = Vec::new();
        let mut ctx = SourceReport::empty();

        for ev in events {
            match ev {
                Event::Error(err) | Event::Warning(err) => ctx.push(err),
                _ => other.push(ev),
            }
        }
        let [Event::Start(BlockKind::Text), items @ .., Event::End(BlockKind::Text)] =
            other.as_slice()
        else {
            panic!()
        };
        (Vec::from(items), ctx)
    }

    #[test_case(
        indoc! { "
            > a text step
            with 2 lines
        " }
        => "a text step with 2 lines"
        ; "no second line marker"
    )]
    #[test_case(
        indoc! { "
            > a text step
            > with 2 lines
        " }
        => "a text step with 2 lines"
        ; "second line marker"
    )]
    #[test_case(
        indoc! { "
            > with no marker
             <- this ws stays
        " }
        => "with no marker  <- this ws stays"
        ; "no trim if no marker"
    )]
    fn multiline_text_step(input: &str) -> String {
        let (events, ctx) = t(input);
        assert!(ctx.is_empty());
        let mut text = String::new();
        for e in events {
            match e {
                Event::Text(t) => text += &t.text(),
                _ => panic!("not text inside text step"),
            }
        }
        text
    }
}
