use crate::{error::label, lexer::T};

use super::{BlockParser, Event, ParserError, ParserWarning};

pub(crate) fn metadata_entry<'i>(line: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    // Parse
    line.consume(T![meta])?;
    let key_pos = line.current_offset();
    let key_tokens = line.until(|t| t == T![:])?;
    let key = line.text(key_pos, key_tokens);
    line.bump(T![:]);
    let value_pos = line.current_offset();
    let value_tokens = line.consume_rest();
    let value = line.text(value_pos, value_tokens);

    // Checks
    if key.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: "metadata entry",
            what: "key",
            reason: "is empty",
            labels: vec![label!(key.span(), "this cannot be empty")],
            help: None,
        });
    } else if value.is_text_empty() {
        line.warn(ParserWarning::EmptyMetadataValue {
            key: key.located_string_trimmed(),
        });
    }

    Some(Event::Metadata { key, value })
}

#[cfg(test)]
mod tests {
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
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let (_events, context) = line.finish();
        let Event::Metadata { key, value } = entry else { panic!() };
        assert_eq!(key.text(), " key");
        assert_eq!(key.span(), Span::new(2, 6));
        assert_eq!(value.text(), " value");
        assert_eq!(value.span(), Span::new(7, 13));
        assert!(context.errors.is_empty());
        assert!(context.warnings.is_empty());
    }

    #[test]
    fn no_key_metadata_entry() {
        let input = ">>: value";
        let tokens = tokens![meta.2, :.1, ws.1, word.5];
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let (_events, context) = line.finish();
        let Event::Metadata { key, value } = entry else { panic!() };
        assert_eq!(key.text(), "");
        assert_eq!(key.span(), Span::pos(2));
        assert_eq!(value.text_trimmed(), "value");
        assert_eq!(context.errors.len(), 1);
        assert!(context.warnings.is_empty());
    }

    #[test]
    fn no_val_metadata_entry() {
        let input = ">> key:";
        let tokens = tokens![meta.2, ws.1, word.3, :.1];
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let (_events, context) = line.finish();
        let Event::Metadata { key, value } = entry else { panic!() };
        assert_eq!(key.text_trimmed(), "key");
        assert_eq!(value.text(), "");
        assert_eq!(value.span(), Span::pos(7));
        assert!(context.errors.is_empty());
        assert_eq!(context.warnings.len(), 1);

        let input = ">> key:  ";
        let tokens = tokens![meta.2, ws.1, word.3, :.1, ws.2];
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let (_events, context) = line.finish();
        let Event::Metadata { key, value } = entry else { panic!() };
        assert_eq!(key.text_trimmed(), "key");
        assert_eq!(value.text(), "  ");
        assert_eq!(value.span(), Span::new(7, 9));
        assert!(context.errors.is_empty());
        assert_eq!(context.warnings.len(), 1);
    }

    #[test]
    fn empty_metadata_entry() {
        let input = ">>:";
        let tokens = tokens![meta.2, :.1];
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        let entry = metadata_entry(&mut line).unwrap();
        let (_events, context) = line.finish();
        let Event::Metadata { key, value } = entry else { panic!() };
        assert!(key.text().is_empty());
        assert_eq!(key.span(), Span::pos(2));
        assert!(value.text().is_empty());
        assert_eq!(value.span(), Span::pos(3));
        assert_eq!(context.errors.len(), 1);
        assert!(context.warnings.is_empty()); // no warning if error generated

        let input = ">> ";
        let tokens = tokens![meta.2, ws.1];
        let mut line = BlockParser::new(0, &tokens, input, Extensions::all());
        assert!(metadata_entry(&mut line).is_none())
    }
}
