//! Which unit to display a value in, for a reader in a region or asking for a measurement system outright.
//!
//! How a locale *names* units is a separate matter (`en-GB` reads `pint` as the imperial pint); this module only
//! decides what a converted value is written in. Which system a region gets is [`official_system`]'s to say.
//!
//! The schema is CLDR's — rows ordered largest first, each with a threshold the value must reach — but the rows are
//! ours, because CLDR has no cooking usage.

use icu_locale_core::subtags::{region, Region};

use crate::quantity::Number;

use crate::convert::units::{
    LengthUnit, MassUnit, PhysicalQuantity, TemperatureUnit, TimeUnit, Unit, VolumeUnit,
};

/// A family of units a reader prefers. A display preference, not a property
/// of a unit: the metric spoons appear in the imperial rows because a British
/// teaspoon is 5 ml.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum System {
    Metric,
    UsCustomary,
    Imperial,
}

/// Who a converted value is written for: where their units come from, and
/// any single unit they asked for outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Audience {
    pub units: Units,
    /// `-u-mu-`: overrides `units` for temperature only, which is the one quantity a reader's system and habit disagree
    /// on. English Canada has metric cups and Fahrenheit ovens.
    pub temperature: Option<TemperatureUnit>,
}

/// Where a reader's units come from: a region, or a system asked for outright with `-u-ms-`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Region(Region),
    System(System),
    /// The frozen rows an untagged recipe is fitted with, for the system the quantity was written in: metric stays
    /// metric, US stays US (ADR 7).
    Legacy(System),
}

impl Audience {
    pub const fn region(region: Region) -> Self {
        Self {
            units: Units::Region(region),
            temperature: None,
        }
    }

    /// Readers who asked for a measurement system outright.
    pub const fn system(system: System) -> Self {
        Self {
            units: Units::System(system),
            temperature: None,
        }
    }

    pub(crate) const fn legacy(system: System) -> Self {
        Self {
            units: Units::Legacy(system),
            temperature: None,
        }
    }

    /// The same readers, with a temperature unit asked for outright.
    pub const fn with_temperature(self, unit: TemperatureUnit) -> Self {
        Self {
            units: self.units,
            temperature: Some(unit),
        }
    }

    fn official(self) -> System {
        match self.units {
            Units::Region(region) => official_system(region),
            Units::System(system) | Units::Legacy(system) => system,
        }
    }
}

/// The measurement system a region's law prescribes, which is what its readers are shown (ADR 5): only the United
/// States is US customary; everyone else, the United Kingdom included, is metric.
///
/// CLDR's own data also lists Liberia as US customary and Myanmar, with the UK, as imperial; all three are officially
/// metric or metricating, and a reader there who wants otherwise can say so with `-u-ms-`.
pub(crate) fn official_system(region: Region) -> System {
    if region == region!("US") {
        System::UsCustomary
    } else {
        System::Metric
    }
}

/// A unit, the amount of it a value must reach to be shown in it, and how
/// it is written when it is: as a fraction with at most this denominator,
/// or as a decimal when there is none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Row {
    pub unit: Unit,
    pub at_least: f64,
    pub fractions: Option<u8>,
    /// When no fraction fits, write a decimal in this unit rather than trying the next row. The legacy rows need it:
    /// bundled fell back to a decimal in the unit its threshold chose (ADR 7).
    pub or_decimal: bool,
}

// The rows themselves are data: `locales/preferences.toml`, one entry per system, legacy system or region and
// quantity, each with its citation. build.rs turns it into `PREFERENCES`, `LEGACY_PREFERENCES` and
// `REGION_PREFERENCES`; `rows_for` says which of them applies.
include!(concat!(env!("OUT_DIR"), "/preferences.rs"));

fn rows(system: System, quantity: PhysicalQuantity) -> Option<&'static [Row]> {
    PREFERENCES
        .iter()
        .find(|(s, q, _)| *s == system && *q == quantity)
        .map(|(_, _, rows)| *rows)
}

fn legacy_rows(system: System, quantity: PhysicalQuantity) -> Option<&'static [Row]> {
    LEGACY_PREFERENCES
        .iter()
        .find(|(s, q, _)| *s == system && *q == quantity)
        .map(|(_, _, rows)| *rows)
}

fn region_rows(region: Region, quantity: PhysicalQuantity) -> Option<&'static [Row]> {
    REGION_PREFERENCES
        .iter()
        .find(|(r, q, _)| *r == region && *q == quantity)
        .map(|(_, _, rows)| *rows)
}

/// The rows that apply to `audience` for `quantity`, most specific first: the `-u-mu-` unit for temperature, a
/// region's own rows, its system's, then metric, which is where time lives. A legacy audience has only its frozen rows
/// (ADR 7).
pub(crate) fn rows_for(audience: Audience, quantity: PhysicalQuantity) -> &'static [Row] {
    if quantity == PhysicalQuantity::Temperature {
        if let Some(unit) = audience.temperature {
            return temperature_row(unit);
        }
    }
    let own = match audience.units {
        Units::Region(region) => region_rows(region, quantity),
        Units::System(_) => None,
        Units::Legacy(system) => {
            return legacy_rows(system, quantity)
                .or_else(|| legacy_rows(System::Metric, quantity))
                .expect("every quantity has legacy metric rows");
        }
    };
    own.or_else(|| rows(audience.official(), quantity))
        .or_else(|| rows(System::Metric, quantity))
        .expect("every quantity has metric rows")
}

fn temperature_row(unit: TemperatureUnit) -> &'static [Row] {
    match unit {
        TemperatureUnit::Celsius => &[Row {
            unit: Unit::Temperature(TemperatureUnit::Celsius),
            at_least: 1.0,
            fractions: None,
            or_decimal: false,
        }],
        TemperatureUnit::Fahrenheit => &[Row {
            unit: Unit::Temperature(TemperatureUnit::Fahrenheit),
            at_least: 1.0,
            fractions: None,
            or_decimal: false,
        }],
    }
}

/// How far a fraction may be from the amount it stands for, as a share of
/// it: the legacy converter's imperial `accuracy`. The precision research's
/// tiers (`docs/research/measurement-precision.md`) are parked.
const FRACTION_TOLERANCE: f32 = 0.05;

/// The rows `value` reaches the threshold of, largest first. The magnitude
/// is what matters, so a negative value — a freezer temperature — is judged
/// by its absolute size.
fn candidates(rows: &[Row], value: f64, from: Unit) -> impl Iterator<Item = &Row> {
    rows.iter().filter(move |row| {
        Unit::convert(value.abs(), from, row.unit).expect("same quantity") >= row.at_least
    })
}

/// The unit `value` should be displayed in for `audience`: the largest one it reaches the threshold of, falling back to
/// the smallest.
///
/// What a conversion writes is [`best_amount`], which also chooses how the amount is written; this is the unit alone,
/// and the snapshot pins it.
#[cfg(test)]
pub(crate) fn best_unit(value: f64, from: Unit, audience: Audience) -> Unit {
    let rows = rows_for(audience, from.physical_quantity());
    candidates(rows, value, from)
        .next()
        .unwrap_or_else(|| rows.last().expect("rows are never empty"))
        .unit
}

/// `value` of `from` as `audience` would write it, with the unit.
///
/// The first row the value reaches the threshold of takes it, unless that
/// row writes fractions and none fits, in which case the next row is tried:
/// `100 ml` is no quarter of a cup, so it is `3 1/2 fl oz`. A fraction is a
/// display form, not a rounding: [`Number::Fraction`] keeps the residual.
pub(crate) fn best_amount(value: f64, from: Unit, audience: Audience) -> (Number, Unit) {
    amount_in(rows_for(audience, from.physical_quantity()), value, from)
}

/// `amount`, already in `unit`, as `audience` writes that unit: a fraction if
/// its row has one that fits, else as it is.
pub(crate) fn as_written(amount: f64, unit: Unit, audience: Audience) -> Number {
    fraction_in(rows_for(audience, unit.physical_quantity()), amount, unit)
}

fn amount_in(rows: &[Row], value: f64, from: Unit) -> (Number, Unit) {
    for row in candidates(rows, value, from) {
        let converted = Unit::convert(value, from, row.unit).expect("same quantity");
        match fraction(converted, row) {
            Some(number) => return (number, row.unit),
            None if row.or_decimal => return (Number::Regular(converted), row.unit),
            None => continue,
        }
    }
    let last = rows.last().expect("rows are never empty").unit;
    let converted = Unit::convert(value, from, last).expect("same quantity");
    (Number::Regular(converted), last)
}

fn fraction_in(rows: &[Row], amount: f64, unit: Unit) -> Number {
    rows.iter()
        .find(|row| row.unit == unit)
        .and_then(|row| fraction(amount, row))
        .unwrap_or(Number::Regular(amount))
}

/// `amount` as `row` writes it, or `None` if the row wants a fraction and
/// none fits. Only a positive amount can be a fraction; zero and negatives
/// are written as they are.
fn fraction(amount: f64, row: &Row) -> Option<Number> {
    match row.fractions {
        Some(den) if amount > 0.0 => Number::new_approx(amount, FRACTION_TOLERANCE, den, u32::MAX),
        _ => Some(Number::Regular(amount)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::units::Unit;

    const METRIC: Audience = Audience::system(System::Metric);
    const US: Audience = Audience::system(System::UsCustomary);
    const IMPERIAL: Audience = Audience::system(System::Imperial);

    fn best(value: f64, from: &str, audience: Audience) -> &'static str {
        best_unit(value, Unit::from_id(from).unwrap(), audience).id()
    }

    /// Bundled wrote a decimal in the threshold's unit when no fraction fitted anywhere, `1.125 c`; a row marked
    /// `or_decimal` does the same instead of trying the next row (ADR 7).
    #[test]
    fn a_legacy_cup_with_no_fraction_stays_a_decimal_cup() {
        let cup = Unit::from_id("us cup").unwrap();
        let (amount, unit) = best_amount(1.125, cup, Audience::legacy(System::UsCustomary));
        assert_eq!(unit, cup);
        assert_eq!(amount, Number::Regular(1.125));
        // below a cup the row is not marked, so the tablespoon row is tried
        let (amount, unit) = best_amount(0.4, cup, Audience::legacy(System::UsCustomary));
        assert_eq!(unit.id(), "us tablespoon");
        assert_eq!(amount.to_string(), "6 1/3");
    }

    #[test]
    fn the_largest_whole_unit_wins() {
        assert_eq!(best(1500.0, "milliliter", METRIC), "liter");
        assert_eq!(best(999.0, "milliliter", METRIC), "milliliter");
        assert_eq!(best(1.0, "liter", METRIC), "liter");
        // below the smallest row, fall back to it rather than failing
        assert_eq!(best(0.1, "milliliter", METRIC), "milliliter");
    }

    #[test]
    fn a_region_gets_its_official_system() {
        assert_eq!(official_system(region!("US")), System::UsCustomary);
        // officially metric, whatever the kitchen still says: the UK, and
        // the two territories CLDR files under the US and UK systems
        for r in [
            region!("GB"),
            region!("LR"),
            region!("MM"),
            region!("DE"),
            region!("IT"),
            region!("ES"),
            region!("001"),
        ] {
            assert_eq!(official_system(r), System::Metric, "{r}");
            assert_eq!(
                best(1500.0, "milliliter", Audience::region(r)),
                "liter",
                "{r}"
            );
            assert_eq!(
                best(1.0, "imperial pint", Audience::region(r)),
                "milliliter",
                "{r}"
            );
        }
        assert_eq!(
            best(500.0, "milliliter", Audience::region(region!("US"))),
            "us cup"
        );
    }

    #[test]
    fn magnitude_is_what_matters_not_sign() {
        // a freezer temperature must not be judged as "smaller than one"
        assert_eq!(best(-18.0, "celsius", US), "fahrenheit");
        assert_eq!(best(-1500.0, "milliliter", METRIC), "liter");
    }

    #[test]
    fn time_is_the_same_in_every_system() {
        for audience in [METRIC, US, IMPERIAL] {
            assert_eq!(best(90.0, "minute", audience), "hour", "{audience:?}");
            assert_eq!(best(30.0, "second", audience), "second", "{audience:?}");
            assert_eq!(best(48.0, "hour", audience), "day", "{audience:?}");
        }
    }

    #[test]
    fn a_region_row_overrides_only_its_own_quantity() {
        let se = Audience::region(region!("SE"));
        let hu = Audience::region(region!("HU"));
        assert_eq!(best(150.0, "milliliter", se), "deciliter");
        assert_eq!(best(500.0, "gram", se), "gram");
        assert_eq!(best(100.0, "celsius", se), "celsius");
        assert_eq!(best(200.0, "milliliter", hu), "deciliter");
        assert_eq!(best(250.0, "gram", hu), "decagram");
        assert_eq!(best(500.0, "gram", hu), "kilogram");
        assert_eq!(best(3.0, "gram", hu), "gram");
        assert_eq!(best(2000.0, "gram", hu), "kilogram");
    }

    #[test]
    fn an_explicit_system_ignores_region_rows() {
        assert_eq!(best(150.0, "milliliter", METRIC), "milliliter");
        assert_eq!(
            best(150.0, "milliliter", Audience::region(region!("SE"))),
            "deciliter"
        );
    }

    fn row(id: &str, at_least: f64, fractions: Option<u8>) -> Row {
        Row {
            unit: Unit::from_id(id).unwrap(),
            at_least,
            fractions,
            or_decimal: false,
        }
    }

    /// The US volume chain as the fraction layer sees it, built here so the
    /// data file cannot mask the rule.
    fn us_rows() -> [Row; 4] {
        [
            row("us cup", 0.25, Some(4)),
            row("us fluid ounce", 1.0, Some(2)),
            row("us tablespoon", 1.0, Some(3)),
            row("us teaspoon", 1.0, Some(8)),
        ]
    }

    fn shown(rows: &[Row], value: f64, from: &str) -> String {
        let (number, unit) = amount_in(rows, value, Unit::from_id(from).unwrap());
        format!("{number} {}", unit.id())
    }

    #[test]
    fn a_clean_fraction_is_written_in_the_largest_unit() {
        let rows = us_rows();
        assert_eq!(shown(&rows, 0.5, "us cup"), "1/2 us cup");
        assert_eq!(shown(&rows, 0.75, "us cup"), "3/4 us cup");
        assert_eq!(shown(&rows, 1.0 / 3.0, "us cup"), "1/3 us cup");
        assert_eq!(shown(&rows, 2.0, "us cup"), "2 us cup");
        // within 5 %: a quarter cup plus a little is a quarter cup
        assert_eq!(shown(&rows, 0.26, "us cup"), "1/4 us cup");
    }

    #[test]
    fn an_odd_amount_falls_to_the_next_unit_with_a_fraction() {
        let rows = us_rows();
        // 100 ml is 0.42 cup, no quarter or third of one; it is 3.38 fl oz
        assert_eq!(shown(&rows, 100.0, "milliliter"), "3 1/2 us fluid ounce");
        // 0.4 cup fits no fraction of a cup or a fluid ounce; 6.4 tbsp fits
        // a third
        assert_eq!(shown(&rows, 0.4, "us cup"), "6 1/3 us tablespoon");
    }

    #[test]
    fn a_decimal_row_takes_anything_that_reaches_it() {
        let rows = [row("liter", 1.0, None), row("milliliter", 0.0, None)];
        assert_eq!(shown(&rows, 1500.0, "milliliter"), "1.5 liter");
        assert_eq!(shown(&rows, 0.3, "milliliter"), "0.3 milliliter");
    }

    #[test]
    fn when_no_fraction_fits_anywhere_the_last_row_is_a_decimal() {
        let rows = [row("us cup", 0.25, Some(4))];
        assert_eq!(shown(&rows, 0.4, "us cup"), "0.4 us cup");
    }

    #[test]
    fn zero_and_negative_amounts_are_never_fractions() {
        let rows = us_rows();
        assert_eq!(shown(&rows, 0.0, "us cup"), "0 us teaspoon");
        assert_eq!(shown(&rows, -0.5, "us cup"), "-0.5 us cup");
    }

    /// The residual is kept: a fraction shows `3 1/2` and still means 3.38.
    #[test]
    fn a_fraction_keeps_the_exact_amount() {
        let rows = us_rows();
        let (number, _) = amount_in(&rows, 100.0, Unit::from_id("milliliter").unwrap());
        assert!(matches!(number, Number::Fraction { .. }));
        assert!(
            (number.value() - 3.381_402).abs() < 1e-6,
            "{}",
            number.value()
        );
    }

    /// The other end of a range is written in the unit the start chose.
    #[test]
    fn an_amount_already_in_a_unit_is_written_as_that_rows_fraction() {
        let rows = us_rows();
        let cup = Unit::from_id("us cup").unwrap();
        assert_eq!(fraction_in(&rows, 8.4535, cup).to_string(), "8 1/2");
        assert_eq!(fraction_in(&rows, 0.4, cup).to_string(), "0.4");
        let liter = Unit::from_id("liter").unwrap();
        assert_eq!(fraction_in(&rows, 0.5, liter).to_string(), "0.5");
    }

    /// `build.rs` refuses a larger denominator; this keeps the two limits the same number.
    #[test]
    fn every_fraction_row_is_within_what_a_number_can_write() {
        let all = PREFERENCES
            .iter()
            .chain(LEGACY_PREFERENCES)
            .map(|(_, _, rows)| *rows)
            .chain(REGION_PREFERENCES.iter().map(|(_, _, rows)| *rows));
        for row in all.flatten() {
            if let Some(den) = row.fractions {
                assert!(
                    (1..=Number::MAX_FRACTION_DENOMINATOR).contains(&den),
                    "{} fractions /{den}",
                    row.unit.id()
                );
                // and the number layer agrees, rather than panicking
                let _ = fraction(1.5, row);
            }
        }
    }

    /// A row's units must all measure the row's quantity, or `convert`
    /// would panic at the first lookup.
    #[test]
    fn every_row_lists_units_of_its_own_quantity() {
        for (_, quantity, rows) in PREFERENCES {
            assert!(!rows.is_empty());
            for row in rows.iter() {
                assert_eq!(row.unit.physical_quantity(), *quantity, "{quantity:?}");
            }
        }
        for (region, quantity, rows) in REGION_PREFERENCES {
            assert!(!rows.is_empty(), "{region}");
            for row in rows.iter() {
                assert_eq!(
                    row.unit.physical_quantity(),
                    *quantity,
                    "{region} {quantity:?}"
                );
            }
        }
    }

    /// Only time is allowed to fall back to the metric rows; a system that
    /// forgot its mass rows would otherwise silently write grams.
    #[test]
    fn every_system_has_its_own_rows_for_everything_but_time() {
        let quantities = [
            PhysicalQuantity::Volume,
            PhysicalQuantity::Mass,
            PhysicalQuantity::Length,
            PhysicalQuantity::Temperature,
        ];
        for system in [System::Metric, System::UsCustomary, System::Imperial] {
            for quantity in quantities {
                assert!(rows(system, quantity).is_some(), "{system:?} {quantity:?}");
            }
            assert_eq!(
                rows(system, PhysicalQuantity::Time).is_some(),
                system == System::Metric,
                "{system:?} time"
            );
        }
    }

    /// `-u-mu-` names a temperature unit outright and touches nothing else:
    /// English Canada keeps its metric cups and gets Fahrenheit ovens.
    #[test]
    fn a_temperature_override_changes_only_temperature() {
        let en_ca = Audience::region(region!("CA")).with_temperature(TemperatureUnit::Fahrenheit);
        assert_eq!(best(180.0, "celsius", en_ca), "fahrenheit");
        assert_eq!(best(-18.0, "celsius", en_ca), "fahrenheit");
        assert_eq!(best(150.0, "milliliter", en_ca), "metric cup");
        assert_eq!(best(500.0, "gram", en_ca), "gram");

        let us_c = US.with_temperature(TemperatureUnit::Celsius);
        assert_eq!(best(350.0, "fahrenheit", us_c), "celsius");
        assert_eq!(best(500.0, "milliliter", us_c), "us cup");
    }

    #[cfg(feature = "bundled_units")]
    mod against_bundled {
        use super::*;
        use crate::convert::locales::legacy::legacy_locale;
        use crate::convert_legacy::{ConvertTo, ConvertValue, LegacyConverter, System as Bundled};

        /// The unit bundled picks, as a symbol.
        fn bundled_choice(
            converter: &LegacyConverter,
            value: f64,
            from: &str,
            to: Bundled,
        ) -> String {
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
        /// thresholds, so their proof is the scaling differential in `tests/differential.rs`.
        #[test]
        fn the_legacy_metric_rows_reproduce_the_bundled_best_lists_exactly() {
            let converter = LegacyConverter::bundled();
            let legacy = legacy_locale();
            let values = [
                0.0, 0.3, 0.5, 1.0, 2.5, 17.25, 100.0, 999.0, 1000.0, 1500.0, 86400.0, -18.0,
                -1500.0,
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
    }
}
