//! The preference rows against the old converter's `best` lists.

use crate::convert::locales::legacy::legacy_locale;
use crate::convert::locales::preference::{best_unit, Audience, System};
use crate::convert::units::Unit;
use crate::convert_legacy::{ConvertTo, ConvertValue, LegacyConverter, System as Bundled};

const METRIC: Audience = Audience::system(System::Metric);
const US: Audience = Audience::system(System::UsCustomary);

/// The unit bundled picks, as a symbol.
fn bundled_choice(converter: &LegacyConverter, value: f64, from: &str, to: Bundled) -> String {
    let (_, unit) = converter
        .convert(
            ConvertValue::Number(value),
            from.into(),
            ConvertTo::Best(to),
        )
        .expect("a known unit");
    unit.symbol().to_string()
}

/// The legacy metric rows reproduce bundled's metric `best` lists with no exception: that is what they are for
/// (ADR 7). The US rows cannot be checked this way, because bundled fitted a fraction before it applied its
/// thresholds, so their proof is the scaling differential in `recipes.rs` beside this file.
#[test]
fn the_legacy_metric_rows_reproduce_the_bundled_best_lists_exactly() {
    let converter = LegacyConverter::bundled();
    let legacy = legacy_locale();
    let values = [
        0.0, 0.3, 0.5, 1.0, 2.5, 17.25, 100.0, 999.0, 1000.0, 1500.0, 86400.0, -18.0, -1500.0,
    ];
    for unit in Unit::all().filter(|unit| legacy.names(*unit).is_some()) {
        let from = legacy.symbol(unit).expect("filtered to named units");
        for value in values {
            let ours = best_unit(value, unit, Audience::legacy(System::Metric));
            let theirs = bundled_choice(&converter, value, from, Bundled::Metric);
            assert_eq!(
                legacy.written(ours),
                theirs,
                "{value} {from} in legacy metric"
            );
        }
    }
}

/// Our rows reproduce today's `best` lists from units.toml with three deliberate exceptions, recorded in ADR 1
/// and ADR 5: the US rows have a fluid ounce and a cup from a quarter, so where we choose those and bundled
/// disagrees it must have chosen the tablespoon; and the metric rows have the metric spoons, so where we choose
/// one bundled must have chosen millilitres. Metric stands in for bundled's metric, US customary for its
/// "imperial".
#[test]
fn the_rows_reproduce_the_bundled_best_lists_except_the_recorded_divergences() {
    let converter = LegacyConverter::bundled();
    let legacy = legacy_locale();
    let values = [
        0.0, 0.5, 1.0, 2.5, 17.25, 100.0, 999.0, 1000.0, 1500.0, 86400.0, -18.0, -1500.0,
    ];
    // (a unit we choose where bundled did not, the symbol it chose instead)
    let divergences = [
        (Unit::from_id("us fluid ounce").unwrap(), "tbsp"),
        (Unit::from_id("us cup").unwrap(), "tbsp"),
        (Unit::from_id("metric tablespoon").unwrap(), "ml"),
        (Unit::from_id("metric teaspoon").unwrap(), "ml"),
    ];
    let mut seen = [0usize; 4];

    // only the units bundled has; legacy names exactly those
    for unit in Unit::all().filter(|unit| legacy.names(*unit).is_some()) {
        let from = legacy.symbol(unit).expect("filtered to named units");
        for (audience, system) in [(METRIC, Bundled::Metric), (US, Bundled::Imperial)] {
            for value in values {
                let ours = best_unit(value, unit, audience);
                let theirs = bundled_choice(&converter, value, from, system);
                if legacy.written(ours) == theirs {
                    continue;
                }
                match divergences.iter().position(|(u, _)| *u == ours) {
                    Some(i) => {
                        seen[i] += 1;
                        assert_eq!(
                            theirs,
                            divergences[i].1,
                            "{value} {from}: bundled had no {} row",
                            ours.id()
                        );
                    }
                    None => panic!(
                        "{value} {from} in {audience:?}: ours {}, bundled {theirs}",
                        ours.id()
                    ),
                }
            }
        }
    }
    assert!(
        seen.iter().all(|n| *n > 0),
        "a recorded divergence never applied: {seen:?}"
    );
}
