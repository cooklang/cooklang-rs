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

/// The review gate: a `reviewed` key in a locale's own file decides whether it may be served to a reader; an
/// unreviewed one is refused, not degraded.
fn served(tag: &str) -> Result<Arc<Locale>, Refusal> {
    reviewed_locale(&LocaleTag::parse(tag).unwrap())
}

/// Any vendored locale with no entry, found rather than named: hardcoding one would make the test fail the day
/// that locale is reviewed, which is the one day it should still pass.
fn an_unreviewed_tag() -> &'static str {
    vendored_tags()
        .find(|t| !names_reviewed(t))
        .expect("every vendored locale is reviewed, so this test is moot")
}

#[track_caller]
fn assert_reaches(tag: &str, vendored: Option<&str>) {
    assert_eq!(tag_for(&LocaleTag::parse(tag).unwrap()), vendored, "{tag}");
}

#[test]
fn every_vendored_locale_reads_back_what_it_writes() {
    let mut count = 0;
    for tag in vendored_tags() {
        let locale = vendored_locale(tag).unwrap_or_else(|| panic!("{tag} missing"));
        for unit in Unit::all() {
            let written = locale.written(unit);
            assert!(!written.is_empty(), "{tag} {}", unit.id());
            assert_eq!(locale.resolve(written), Some(unit), "{tag} '{written}'");
        }
        count += 1;
    }
    assert!(count >= 34, "only {count} locales");
    assert!(vendored_locale("sw").is_none(), "sw is not vendored");
}

/// Extensions choose units, not names; lookup follows CLDR's parent chain before it truncates.
#[test]
fn tag_for_follows_cldr_parents_and_ignores_extensions() {
    assert_reaches("en-GB-u-ms-metric", Some("en-GB"));
    assert_reaches("EN-gb", Some("en-GB"));
    // the parent chain, where truncation would reach US English
    assert_reaches("en-NZ", Some("en-001"));
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

/// Deliberately a list rather than a count, so that reviewing the next locale is a conscious act with a test to
/// update, not something that slips in.
#[test]
fn only_da_fi_pl_and_sv_are_reviewed() {
    let reviewed: Vec<&str> = vendored_tags().filter(|t| names_reviewed(t)).collect();
    assert_eq!(reviewed, ["da", "fi", "pl", "sv"]);
}

/// Adjudication does not imply review: a locale with a verdict for every ambiguous string is still refused until a
/// speaker confirms it, by name; an unknown tag is refused as unknown, so the two stay distinguishable.
#[test]
fn a_locale_is_refused_as_unreviewed_or_as_unknown() {
    let tag = an_unreviewed_tag();
    assert_eq!(
        served(tag).unwrap_err(),
        Refusal::Unreviewed(tag.to_string())
    );
    let message = served(tag).unwrap_err().to_string();
    assert!(
        message.contains(&format!("show-locale-as-md -- {tag}")),
        "{message}"
    );
    assert_eq!(
        served("xh").unwrap_err(),
        Refusal::Unknown("xh".to_string())
    );
}
