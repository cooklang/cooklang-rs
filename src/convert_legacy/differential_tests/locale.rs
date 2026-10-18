//! The legacy locale names exactly what the old converter's bundled `units.toml` named.

use std::sync::Arc;

use crate::convert::locales::legacy::legacy_locale;
use crate::convert::units::Unit;
use crate::convert_legacy::LegacyConverter;

#[test]
fn legacy_locale_resolves_every_bundled_key() {
    let converter = LegacyConverter::bundled();
    let locale = legacy_locale();
    for bundled in converter.all_units() {
        for key in bundled
            .names
            .iter()
            .chain(&bundled.symbols)
            .chain(&bundled.aliases)
        {
            let unit = locale
                .resolve(key)
                .unwrap_or_else(|| panic!("legacy locale does not know '{key}'"));
            let symbol = locale
                .symbol(unit)
                .expect("legacy names every bundled unit");
            let same = converter.find_unit(symbol).unwrap();
            assert_eq!(
                same.symbol(),
                bundled.symbol(),
                "'{key}' resolves to '{}'",
                unit.id()
            );
        }
    }
}

#[test]
fn bundled_resolves_every_legacy_alias() {
    let converter = LegacyConverter::bundled();
    let locale = legacy_locale();
    // legacy names exactly the bundled units
    let named: Vec<Unit> = Unit::all()
        .filter(|unit| locale.names(*unit).is_some())
        .collect();
    assert_eq!(named.len(), converter.unit_count());
    for unit in named {
        let names = locale.names(unit).expect("filtered to named units");
        let symbol = names.symbol.as_deref().expect("with a symbol");
        let bundled = converter
            .find_unit(symbol)
            .unwrap_or_else(|| panic!("bundled does not know '{symbol}'"));
        assert_eq!(bundled.symbol(), symbol, "symbol of '{}'", unit.id());
        for alias in &names.aliases {
            let other = converter
                .find_unit(alias)
                .unwrap_or_else(|| panic!("bundled does not know '{alias}'"));
            assert!(
                Arc::ptr_eq(&bundled, &other),
                "'{alias}' is not '{}' in bundled",
                unit.id()
            );
        }
    }
}

#[test]
fn both_reject_the_same_unknown_spellings() {
    let converter = LegacyConverter::bundled();
    let locale = legacy_locale();
    for text in ["Cup", "ML", "", "bunch", " g", "kilos"] {
        assert_eq!(locale.resolve(text), None, "'{text}'");
        assert!(converter.find_unit(text).is_none(), "'{text}'");
    }
}
