mod snapshot;

use super::legacy::*;
use super::locale::*;
use super::vendored::*;
use crate::convert::units::Unit;
use std::collections::HashMap;
use std::sync::Arc;

fn names(symbol: &str, aliases: &[&str]) -> UnitNames {
    UnitNames {
        symbol: Some(symbol.to_string()),
        aliases: aliases.iter().map(|a| a.to_string()).collect(),
        long: Vec::new(),
    }
}

fn unit(id: &str) -> Unit {
    Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"))
}

/// A locale naming only the given units.
fn locale_with(entries: &[(&str, UnitNames)]) -> Result<Locale, LocaleError> {
    let map: HashMap<Unit, UnitNames> = entries
        .iter()
        .map(|(id, unit_names)| (unit(id), unit_names.clone()))
        .collect();
    Locale::new(map)
}

#[test]
fn resolve_matches_symbols_and_aliases_exactly() {
    let locale = locale_with(&[("us cup", names("c", &["cup", "cups"]))]).unwrap();
    let cup = unit("us cup");

    assert_eq!(locale.resolve("c"), Some(cup));
    assert_eq!(locale.resolve("cup"), Some(cup));
    assert_eq!(locale.resolve("cups"), Some(cup));

    // exact and case-sensitive, like the legacy converter
    assert_eq!(locale.resolve("Cup"), None);
    assert_eq!(locale.resolve(" cup"), None);
    assert_eq!(locale.resolve(""), None);
    // `C` is the universal Celsius spelling, not this locale's `c`
    assert_eq!(locale.resolve("C"), Some(unit("celsius")));
}

#[test]
fn the_symbol_is_written_back_when_there_is_one() {
    let locale = locale_with(&[("us cup", names("c", &["cup"]))]).unwrap();
    assert_eq!(locale.symbol(unit("us cup")), Some("c"));
    assert_eq!(locale.written(unit("us cup")), "c");
}

#[test]
fn an_unnamed_unit_is_written_as_its_identifier() {
    let locale = locale_with(&[]).unwrap();
    assert_eq!(locale.symbol(unit("us cup")), None);
    assert_eq!(locale.names(unit("us cup")), None);
    assert_eq!(locale.written(unit("us cup")), "us cup");
    assert_eq!(locale.written(unit("liter")), "liter");
}

#[test]
fn the_identifier_resolves_in_every_locale() {
    let empty = locale_with(&[]).unwrap();
    let named = locale_with(&[("us cup", names("c", &["cup"]))]).unwrap();
    let legacy = legacy_locale();
    for locale in [&empty, &named, &*legacy] {
        assert_eq!(locale.resolve("us cup"), Some(unit("us cup")));
        assert_eq!(
            locale.resolve("us fluid ounce"),
            Some(unit("us fluid ounce"))
        );
        assert_eq!(locale.resolve("liter"), Some(unit("liter")));
    }
    // what a locale writes, it can read back, named or not
    for unit in Unit::all() {
        assert_eq!(empty.resolve(empty.written(unit)), Some(unit), "{unit:?}");
    }
}

#[test]
fn a_string_may_name_only_one_unit() {
    let cup = unit("us cup");
    let celsius = unit("celsius");

    // alias clashing with another unit's alias
    let err = locale_with(&[
        ("us cup", names("c", &["cup"])),
        ("celsius", names("°C", &["cup"])),
    ])
    .unwrap_err();
    assert!(matches!(
        err,
        LocaleError::DuplicateAlias { ref alias, a, b }
            if alias == "cup" && [a, b].contains(&cup) && [a, b].contains(&celsius)
    ));

    // alias clashing with another unit's symbol
    let err = locale_with(&[
        ("us cup", names("c", &[])),
        ("celsius", names("°C", &["c"])),
    ])
    .unwrap_err();
    assert!(matches!(err, LocaleError::DuplicateAlias { ref alias, .. } if alias == "c"));

    // the same string twice on one unit is not a clash
    assert!(locale_with(&[("us cup", names("c", &["c", "cup", "cup"]))]).is_ok());
}

/// Which unit the build reaches first comes out of a `HashMap`, so a clash over an identifier has to name the borrower
/// and the owner the same way whichever order it is found in.
#[test]
fn a_clash_over_an_identifier_is_reported_the_same_way_either_way_round() {
    let err = locale_with(&[
        ("us cup", names("c", &["us cup"])),
        ("celsius", names("\u{b0}C", &["us cup"])),
    ])
    .unwrap_err();
    assert_eq!(
        err,
        LocaleError::AliasIsAnIdentifier {
            alias: "us cup".to_string(),
            unit: unit("celsius"),
            of: unit("us cup"),
        }
    );
}

#[test]
fn an_alias_may_not_be_another_units_spaced_identifier() {
    // `us cup` must always mean the US cup, so no locale may hand it to
    // another unit; a unit's own spaced identifier is fine as its alias
    let err = locale_with(&[("celsius", names("°C", &["us cup"]))]).unwrap_err();
    assert!(matches!(
        err,
        LocaleError::AliasIsAnIdentifier { ref alias, unit: u, of }
            if alias == "us cup" && u == unit("celsius") && of == unit("us cup")
    ));
    assert!(locale_with(&[("us cup", names("c", &["us cup"]))]).is_ok());
}

#[cfg(feature = "bundled_units")]
mod vendored {
    use super::*;

    #[test]
    fn every_vendored_locale_builds_and_reads_back_what_it_writes() {
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
}

#[cfg(feature = "bundled_units")]
mod fallback {
    use super::*;
    use crate::convert::locales::tag::LocaleTag;

    fn locale(tag: &str) -> Option<Arc<Locale>> {
        for_tag(&LocaleTag::parse(tag).unwrap())
    }

    #[test]
    fn extensions_do_not_affect_which_names_are_used() {
        let pint = unit("us pint");
        for tag in ["en-GB-u-ms-metric", "en-GB-u-rg-uszzzz"] {
            assert_eq!(locale(tag).unwrap().symbol(pint), Some("US pt"), "{tag}");
        }
    }

    #[test]
    fn case_is_normalised_before_lookup() {
        let pint = unit("us pint");
        assert_eq!(locale("EN-gb").unwrap().symbol(pint), Some("US pt"));
    }
}

#[cfg(feature = "bundled_units")]
mod locale_files {
    use super::*;

    /// As much of `locales/<tag>.toml` as these tests check. `build.rs` reads
    /// the same shape.
    #[derive(serde::Deserialize)]
    struct LocaleFile {
        #[serde(default)]
        unit: Vec<Entry>,
    }

    #[derive(serde::Deserialize)]
    struct Entry {
        id: String,
        symbol: Option<String>,
        #[serde(default)]
        aliases: Vec<String>,
    }

    fn locale_files() -> Vec<(String, LocaleFile)> {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/locales");
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).expect("locales/ exists") {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "toml") {
                continue;
            }
            let tag = path.file_stem().unwrap().to_string_lossy().to_string();
            // the display rows and the parent chain are not locales
            if matches!(tag.as_str(), "preferences" | "parent-locales") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let parsed: LocaleFile = toml::from_str(&text).unwrap_or_else(|e| panic!("{tag}: {e}"));
            out.push((tag, parsed));
        }
        assert!(out.len() >= 6, "only {} locale files found", out.len());
        out
    }

    /// Every file is a vendored locale, names only real units, and says what
    /// the locale actually does.
    ///
    /// The last clause is the one with teeth: it is what stops the file and
    /// the build disagreeing about a name.
    #[test]
    fn every_file_is_a_locale_and_says_what_it_does() {
        for (tag, file) in locale_files() {
            assert!(vendored_tags().any(|t| t == tag), "{tag} is not vendored");
            let locale = vendored_locale(&tag).unwrap();
            for entry in &file.unit {
                let unit = Unit::from_id(&entry.id)
                    .unwrap_or_else(|| panic!("{tag}: '{}' is not a unit", entry.id));
                assert!(
                    entry.symbol.is_some() || !entry.aliases.is_empty(),
                    "{tag} {}: names nothing",
                    entry.id
                );
                assert_eq!(
                    locale.symbol(unit),
                    entry.symbol.as_deref(),
                    "{tag} {} symbol",
                    entry.id
                );
                for alias in &entry.aliases {
                    assert_eq!(locale.resolve(alias), Some(unit), "{tag} '{alias}'");
                }
            }
        }
    }
}

/// Long forms: the unit spelled out with the plural the number calls for.
#[cfg(feature = "bundled_units")]
mod long_forms {
    use super::*;

    #[test]
    fn without_a_long_form_the_number_precedes_the_written_form() {
        let de = vendored_locale("de").unwrap();
        // declined: no symbol, no long form, so the spaced identifier
        assert_eq!(de.written_long(2.0, unit("us cup")), "2 us cup");
        // a hand-named unit with no long patterns: the symbol
        assert_eq!(de.written_long(2.0, unit("decagram")), "2 dag");
        // legacy has no long forms at all
        assert_eq!(legacy_locale().written_long(2.0, unit("us cup")), "2 c");
    }
}

mod markdown {
    use crate::convert::locales::reference::render_locale;

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(render_locale("not a tag")
            .unwrap_err()
            .contains("not a valid locale tag"));
        assert!(render_locale("sw-KE")
            .unwrap_err()
            .contains("no vendored locale"));
    }
}

/// ADR 3: outside US customary locales, an unmarked word for a US unit is refused. CLDR marks only the imperial side of
/// each pair (`brittiska quarts`, `Imp. Gallonen`) and leaves the US side bare, which is CLDR's layout, not evidence
/// that a Swedish or German kitchen means 3.79 l by `gallon`; the readings differ by 20% (4% for the fluid ounce).
#[cfg(feature = "bundled_units")]
mod unmarked_us_words {
    use super::*;

    /// Fully adjudicated locales whose official system is not US customary.
    const NON_US: [&str; 13] = [
        "da", "de", "en-001", "en-AU", "en-GB", "es", "es-419", "es-CO", "es-MX", "fi", "ja", "no",
        "sv",
    ];
    const US_UNITS: [&str; 7] = [
        "us cup",
        "us pint",
        "us quart",
        "us gallon",
        "us fluid ounce",
        "us teaspoon",
        "us tablespoon",
    ];

    #[test]
    fn only_marked_names_reach_a_us_unit() {
        for tag in NON_US {
            let locale = vendored_locale(tag).unwrap();
            for id in US_UNITS {
                let unit = unit(id);
                if let Some(names) = locale.names(unit) {
                    for name in names.symbol.iter().chain(&names.aliases) {
                        // en-GB's `US qt`, Finnish `am. gallona`
                        let marked = name.contains("US") || name.starts_with("am.");
                        assert!(marked, "{tag} {id}: '{name}' carries no US marker");
                    }
                }
            }
        }
        // and the two US customary locales keep the bare words
        for tag in ["en", "es-US"] {
            let locale = vendored_locale(tag).unwrap();
            assert_eq!(locale.resolve("gal"), Some(unit("us gallon")), "{tag}");
            assert_eq!(locale.resolve("qt"), Some(unit("us quart")), "{tag}");
        }
    }

    /// Every non-English locale here strips the imperial names (ADR 3). Locales whose region has a US/UK-system past --
    /// hi, ms, he, ar, zh-Hant -- keep theirs, and are not in `NON_US`.
    #[test]
    fn imperial_names_are_stripped_in_metric_only_locales() {
        let imperial = [
            "imperial pint",
            "imperial fluid ounce",
            "imperial quart",
            "imperial gallon",
        ];
        for tag in NON_US {
            if tag.starts_with("en") {
                continue;
            }
            let locale = vendored_locale(tag).unwrap();
            for id in imperial {
                let unit = unit(id);
                assert_eq!(locale.names(unit), None, "{tag} names {id}");
                assert_eq!(locale.written(unit), unit.id(), "{tag} {id}");
                assert_eq!(locale.resolve(unit.id()), Some(unit), "{tag} {id}");
            }
            assert_eq!(
                locale.resolve("us gallon"),
                Some(unit("us gallon")),
                "{tag}"
            );
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
}

/// `UNIVERSAL_ALIASES` resolve in every locale, whatever it names.
mod universal_aliases {
    use super::*;

    #[test]
    fn degree_variants_resolve_in_every_locale() {
        let celsius = unit("celsius");
        let fahrenheit = unit("fahrenheit");
        let empty = locale_with(&[]).unwrap();
        let mut locales: Vec<(String, Arc<Locale>)> = vec![
            ("empty".to_string(), Arc::new(empty)),
            ("legacy".to_string(), legacy_locale()),
        ];
        for tag in vendored_tags() {
            locales.push((tag.to_string(), vendored_locale(tag).unwrap()));
        }
        for (tag, locale) in &locales {
            for s in ["℃", "ºC"] {
                assert_eq!(locale.resolve(s), Some(celsius), "{tag} '{s}'");
            }
            for s in ["℉", "ºF"] {
                assert_eq!(locale.resolve(s), Some(fahrenheit), "{tag} '{s}'");
            }
            assert_eq!(locale.resolve("C"), Some(celsius), "{tag}");
            assert_eq!(locale.resolve("F"), Some(fahrenheit), "{tag}");
        }
    }

    #[test]
    fn a_locale_may_not_give_a_universal_alias_to_another_unit() {
        let err = locale_with(&[("us cup", names("c", &["℃"]))]).unwrap_err();
        assert!(matches!(err, LocaleError::DuplicateAlias { ref alias, .. } if alias == "℃"));
        // its own unit is fine
        assert!(locale_with(&[("celsius", names("°C", &["℃"]))]).is_ok());
    }
}

/// The review gate: a `reviewed` key in a locale's own file decides whether it may be served to a reader; an
/// unreviewed one is refused, not degraded.
mod reviewed {
    use super::*;
    use crate::convert::locales::tag::LocaleTag;

    fn served(tag: &str) -> Result<Arc<Locale>, Refusal> {
        reviewed_locale(&LocaleTag::parse(tag).unwrap())
    }

    /// Deliberately a list rather than a count, so that reviewing the next locale is a conscious act with a test to
    /// update, not something that slips in.
    #[test]
    fn only_da_fi_pl_and_sv_are_reviewed() {
        let reviewed: Vec<&str> = vendored_tags().filter(|t| names_reviewed(t)).collect();
        assert_eq!(reviewed, ["da", "fi", "pl", "sv"]);
        assert!(
            !names_reviewed("no"),
            "Norwegian is adjudicated, not reviewed"
        );
    }

    #[test]
    fn a_reviewed_locale_is_served() {
        let sv = served("sv-SE").unwrap();
        assert_eq!(sv.written(unit("metric tablespoon")), "msk");
    }

    /// Adjudication does not imply review: a locale with a verdict for every ambiguous string is still refused until a
    /// speaker confirms it. The refusal names the locale reached, not the tag asked for, since that is the one needing
    /// an entry: `en-NZ` reaches `en-001`.
    #[test]
    fn an_unreviewed_locale_is_refused_by_the_name_it_reached() {
        if names_reviewed("en-001") {
            return;
        }
        assert_eq!(
            served("en-NZ").unwrap_err(),
            Refusal::Unreviewed("en-001".to_string())
        );
        let message = served("en-NZ").unwrap_err().to_string();
        assert!(message.contains("show-locale-as-md -- en-001"), "{message}");
    }

    /// Unknown is not unreviewed, so the two failures stay distinguishable.
    #[test]
    fn an_unknown_tag_is_refused_as_unknown() {
        assert_eq!(
            served("xh").unwrap_err(),
            Refusal::Unknown("xh".to_string())
        );
    }

    /// Any vendored locale with no entry, found rather than named: hardcoding one would make the test fail the day
    /// that locale is reviewed, which is the one day it should still pass.
    fn an_unreviewed_tag() -> &'static str {
        vendored_tags()
            .find(|t| !names_reviewed(t))
            .expect("every vendored locale is reviewed, so this test is moot")
    }

    /// `show-locale-as-md` must keep working on an unreviewed locale, or the refusal would point at a command that
    /// cannot run.
    #[test]
    fn the_command_the_refusal_names_still_works_on_an_unreviewed_locale() {
        let tag = an_unreviewed_tag();
        let report = crate::locale_markdown(tag).expect("renders");
        assert!(report.starts_with(&format!("# Locale `{tag}`")), "{tag}");
    }
}
