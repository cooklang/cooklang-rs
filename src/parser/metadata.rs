use crate::{error::label, lexer::T};

use super::{error, warning, BlockParser, Event};

pub(crate) fn metadata_entry<'i>(block: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    // Parse
    block.consume(T![meta])?;
    let key_pos = block.current_offset();
    let key_tokens = block.until(|t| t == T![:]).or_else(|| {
        block.warn(
            warning!(
                "A metadata block is invalid and it will be a step",
                label!(block.span()),
            )
            .hint("Missing separator `:`"),
        );
        None
    })?;
    let key = block.text(key_pos, key_tokens);
    block.bump(T![:]);
    let value_pos = block.current_offset();
    let value_tokens = block.consume_rest();
    let value = block.text(value_pos, value_tokens);

    // Checks
    if key.is_text_empty() {
        block.error(
            error!(
                "Empty metadata key",
                label!(key.span(), "write the key here"),
            )
            .hint("The key cannot be empty"),
        );
    } else if value.is_text_empty() {
        block.warn(
            warning!(
                format!("Empty metadata value for key: {}", key.text_trimmed()),
                label!(value.span(), "write a value here"),
            )
            .label(label!(key.span())),
        );
    }

    Some(Event::Metadata { key, value })
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{
        parser::{token_stream::tokens, BlockParser},
        span::Span,
        Extensions,
    };

    #[test]
    fn basic_metadata_entry() {
        let input = ">> key: value";
        let tokens = tokens![meta.2, ws.1, word.3, :.1, ws.1, word.5];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let entry = metadata_entry(&mut bp).unwrap();
        bp.finish();
        let Event::Metadata { key, value } = entry else {
            panic!()
        };
        assert_eq!(key.text(), " key");
        assert_eq!(key.span(), Span::new(2, 6));
        assert_eq!(value.text(), " value");
        assert_eq!(value.span(), Span::new(7, 13));
        assert!(events.is_empty());
    }

    #[test]
    fn no_key_metadata_entry() {
        let input = ">>: value";
        let tokens = tokens![meta.2, :.1, ws.1, word.5];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let entry = metadata_entry(&mut bp).unwrap();
        bp.finish();
        let Event::Metadata { key, value } = entry else {
            panic!()
        };
        assert_eq!(key.text(), "");
        assert_eq!(key.span(), Span::pos(2));
        assert_eq!(value.text_trimmed(), "value");
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Error(_)));
    }

    #[test]
    fn no_val_metadata_entry() {
        let input = ">> key:";
        let tokens = tokens![meta.2, ws.1, word.3, :.1];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let entry = metadata_entry(&mut bp).unwrap();
        bp.finish();
        let Event::Metadata { key, value } = entry else {
            panic!()
        };
        assert_eq!(key.text_trimmed(), "key");
        assert_eq!(value.text(), "");
        assert_eq!(value.span(), Span::pos(7));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Warning(_)));

        let input = ">> key:  ";
        let tokens = tokens![meta.2, ws.1, word.3, :.1, ws.2];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let entry = metadata_entry(&mut bp).unwrap();
        bp.finish();
        let Event::Metadata { key, value } = entry else {
            panic!()
        };
        assert_eq!(key.text_trimmed(), "key");
        assert_eq!(value.text(), "  ");
        assert_eq!(value.span(), Span::new(7, 9));
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Warning(_)));
    }

    #[test]
    fn empty_metadata_entry() {
        let input = ">>:";
        let tokens = tokens![meta.2, :.1];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        let entry = metadata_entry(&mut bp).unwrap();
        bp.finish();
        let Event::Metadata { key, value } = entry else {
            panic!()
        };
        assert!(key.text().is_empty());
        assert_eq!(key.span(), Span::pos(2));
        assert!(value.text().is_empty());
        assert_eq!(value.span(), Span::pos(3));
        assert_eq!(events.len(), 1); // no warning if error generated
        assert!(matches!(events[0], Event::Error(_)));

        let input = ">> ";
        let tokens = tokens![meta.2, ws.1];
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        assert!(metadata_entry(&mut bp).is_none());
    }
}
