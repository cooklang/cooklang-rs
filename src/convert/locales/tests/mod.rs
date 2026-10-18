mod snapshot;

use super::legacy::*;
use super::locale::*;
use super::vendored::*;
use crate::convert::locales::reference::render_locale;
use crate::convert::locales::tag::LocaleTag;
use crate::convert::units::Unit;
use std::collections::HashMap;
use std::sync::Arc;

fn unit(id: &str) -> Unit {
    Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"))
}

#[test]
fn bad_input_is_an_error_not_a_panic() {
    assert!(render_locale("not a tag")
        .unwrap_err()
        .contains("not a valid locale tag"));
    assert!(render_locale("sw-KE")
        .unwrap_err()
        .contains("no vendored locale"));
}

/// Locales allowed bare words for US units: the two whose official system is US customary.
const BARE_US_WORDS: [&str; 2] = ["en", "es-US"];
/// Locales that name the imperial units: English, and US Spanish with its own Spanish names.
const NAMES_IMPERIAL: [&str; 5] = ["en", "en-001", "en-AU", "en-GB", "es-US"];
const US_UNITS: [&str; 7] = [
    "us cup",
    "us pint",
    "us quart",
    "us gallon",
    "us fluid ounce",
    "us teaspoon",
    "us tablespoon",
];

/// ADR 3: outside US customary locales, an unmarked word for a US unit is refused. CLDR marks only the imperial side of
/// each pair (`brittiska quarts`, `Imp. Gallonen`) and leaves the US side bare, which is CLDR's layout, not evidence
/// that a Swedish or German kitchen means 3.79 l by `gallon`; the readings differ by 20% (4% for the fluid ounce).
#[test]
fn only_marked_names_reach_a_us_unit() {
    for tag in vendored_tags().filter(|t| !BARE_US_WORDS.contains(t)) {
        let locale = vendored_locale(tag).unwrap();
        for id in US_UNITS {
            if let Some(names) = locale.names(unit(id)) {
                for name in names.symbol.iter().chain(&names.aliases) {
                    // en-GB's `US qt`, Finnish `am. gallona`
                    let marked = name.contains("US") || name.starts_with("am.");
                    assert!(marked, "{tag} {id}: '{name}' carries no US marker");
                }
            }
        }
    }
    // and the two US customary locales keep the bare words
    let en = vendored_locale("en").unwrap();
    let es_us = vendored_locale("es-US").unwrap();
    assert_eq!(en.resolve("gal"), Some(unit("us gallon")));
    assert_eq!(en.resolve("qt"), Some(unit("us quart")));
    assert_eq!(es_us.resolve("gal"), Some(unit("us gallon")));
    assert_eq!(es_us.resolve("qt"), Some(unit("us quart")));
}

/// Only English and US Spanish name the imperial units (ADR 3); everywhere else they are written by identifier.
#[test]
fn imperial_names_are_stripped_in_metric_only_locales() {
    let imperial = [
        "imperial pint",
        "imperial fluid ounce",
        "imperial quart",
        "imperial gallon",
    ];
    for tag in vendored_tags().filter(|t| !NAMES_IMPERIAL.contains(t)) {
        let locale = vendored_locale(tag).unwrap();
        for id in imperial {
            assert_eq!(locale.names(unit(id)), None, "{tag} names {id}");
        }
    }
    // English keeps them, bare in Commonwealth English and marked in US English
    let gb = vendored_locale("en-GB").unwrap();
    let en = vendored_locale("en").unwrap();
    assert_eq!(gb.resolve("gal"), Some(unit("imperial gallon")));
    assert_eq!(en.resolve("Imp. gallons"), Some(unit("imperial gallon")));
    // and US Spanish keeps its native Spanish ones
    let us = vendored_locale("es-US").unwrap();
    assert_eq!(us.resolve("galón imperial"), Some(unit("imperial gallon")));
}
