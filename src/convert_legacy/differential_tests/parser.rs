//! What only the old pipeline does, and the API only a legacy parser has.

use crate::{CooklangParser, Extensions, LegacyConverter};

/// The new pipeline tells an untagged recipe to add a `locale` (ADR 7); the old one never did.
#[test]
fn the_old_pipeline_does_not_ask_for_a_locale() {
    let old = CooklangParser::new_legacy(Extensions::all(), LegacyConverter::bundled());
    let warnings: Vec<String> = old
        .parse("@flour{1%g}\n")
        .report()
        .warnings()
        .map(|w| w.message.to_string())
        .collect();
    assert!(
        !warnings.iter().any(|m| m.contains("no `locale`")),
        "{warnings:?}"
    );
}

#[test]
fn only_a_legacy_parser_carries_a_converter() {
    assert_eq!(
        CooklangParser::new(Extensions::all()).converter_legacy(),
        None
    );
    let legacy = CooklangParser::new_legacy(Extensions::all(), LegacyConverter::empty());
    assert_eq!(legacy.converter_legacy(), Some(&LegacyConverter::empty()));
}
