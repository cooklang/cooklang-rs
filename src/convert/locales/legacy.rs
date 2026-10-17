//! The locale an untagged recipe reads its units through: the names `units.toml` gives the bundled units, and nothing
//! else. Frozen, so that what an untagged recipe does stays what it always did (ADR 7).
//!
//! Written out rather than generated: `units.toml` derived the prefixed SI units from a base and a prefix list, but
//! what is frozen is the result, and a table shows a change where a loop would hide it. The tests against the old
//! converter in `locales/tests/mod.rs` hold the two to the same names while that converter still exists.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use super::locale::{Locale, UnitNames};
use super::preference::System::{self, Metric, UsCustomary};
use crate::convert::units::Unit;

/// Identifier, the system `units.toml` filed it under, the symbol a conversion writes back, every other spelling.
type Row = (&'static str, System, &'static str, &'static [&'static str]);

#[rustfmt::skip]
const LEGACY_UNITS: &[Row] = &[
    // volume
    ("kiloliter",      Metric,      "kl",    &["kiloliter", "kiloliters", "kilolitre", "kilolitres", "kL"]),
    ("hectoliter",     Metric,      "hl",    &["hectoliter", "hectoliters", "hectolitre", "hectolitres", "hL"]),
    ("decaliter",      Metric,      "dal",   &["decaliter", "decaliters", "decalitre", "decalitres", "daL"]),
    ("liter",          Metric,      "l",     &["liter", "liters", "litre", "litres", "L"]),
    ("deciliter",      Metric,      "dl",    &["deciliter", "deciliters", "decilitre", "decilitres", "dL"]),
    ("centiliter",     Metric,      "cl",    &["centiliter", "centiliters", "centilitre", "centilitres", "cL"]),
    ("milliliter",     Metric,      "ml",    &["milliliter", "milliliters", "millilitre", "millilitres", "mL"]),
    ("us teaspoon",    UsCustomary, "tsp",   &["teaspoon", "teaspoons", "tsp."]),
    ("us tablespoon",  UsCustomary, "tbsp",  &["tablespoon", "tablespoons", "tbsp.", "tbs", "tbs."]),
    ("us fluid ounce", UsCustomary, "fl oz", &["fluid ounce", "fluid ounces", "fl. oz.", "fl. oz", "fl oz."]),
    ("us cup",         UsCustomary, "c",     &["cup", "cups"]),
    ("us pint",        UsCustomary, "pt",    &["pint", "pints"]),
    ("us quart",       UsCustomary, "qt",    &["quart", "quarts"]),
    ("us gallon",      UsCustomary, "gal",   &["gallon", "gallons"]),
    // mass
    ("kilogram",       Metric,      "kg",    &["kilogram", "kilograms"]),
    ("hectogram",      Metric,      "hg",    &["hectogram", "hectograms"]),
    ("decagram",       Metric,      "dag",   &["decagram", "decagrams"]),
    ("gram",           Metric,      "g",     &["gram", "grams"]),
    ("decigram",       Metric,      "dg",    &["decigram", "decigrams"]),
    ("centigram",      Metric,      "cg",    &["centigram", "centigrams"]),
    ("milligram",      Metric,      "mg",    &["milligram", "milligrams"]),
    ("ounce",          UsCustomary, "oz",    &["ounce", "ounces", "oz."]),
    ("pound",          UsCustomary, "lb",    &["pound", "pounds", "lb."]),
    // length
    ("kilometer",      Metric,      "km",    &["kilometer", "kilometers", "kilometre", "kilometres"]),
    ("hectometer",     Metric,      "hm",    &["hectometer", "hectometers", "hectometre", "hectometres"]),
    ("decameter",      Metric,      "dam",   &["decameter", "decameters", "decametre", "decametres"]),
    ("meter",          Metric,      "m",     &["meter", "meters", "metre", "metres"]),
    ("decimeter",      Metric,      "dm",    &["decimeter", "decimeters", "decimetre", "decimetres"]),
    ("centimeter",     Metric,      "cm",    &["centimeter", "centimeters", "centimetre", "centimetres"]),
    ("millimeter",     Metric,      "mm",    &["millimeter", "millimeters", "millimetre", "millimetres"]),
    ("foot",           UsCustomary, "ft",    &["foot", "feet", "'"]),
    ("inch",           UsCustomary, "in",    &["inch", "inches", "\""]),
    // time: one list in `units.toml`, so its system never mattered; metric is where its rows live
    ("second",         Metric,      "s",     &["second", "seconds", "secs", "sec"]),
    ("minute",         Metric,      "min",   &["minute", "minutes", "mins"]),
    ("hour",           Metric,      "h",     &["hour", "hours"]),
    ("day",            Metric,      "d",     &["day", "days"]),
    // temperature: the other spellings `units.toml` gives the degrees are UNIVERSAL_ALIASES
    ("celsius",        Metric,      "°C",    &["celsius"]),
    ("fahrenheit",     UsCustomary, "°F",    &["fahrenheit"]),
];

/// The system `units.toml` filed `unit` under, which is what an untagged recipe is fitted within. A unit it never
/// knew, reached by its identifier, is US customary: `units.toml` filed everything that was not SI, time or Celsius as
/// "imperial", and that rule is frozen with the rest.
pub(crate) fn legacy_system(unit: Unit) -> System {
    LEGACY_UNITS
        .iter()
        .find(|(id, ..)| *id == unit.id())
        .map_or(UsCustomary, |&(_, system, ..)| system)
}

/// Built once: the locale is immutable, and every parse, scale and list of an untagged recipe reads it.
pub(crate) fn legacy_locale() -> Arc<Locale> {
    static LEGACY: LazyLock<Arc<Locale>> = LazyLock::new(|| Arc::new(build_legacy_locale()));
    LEGACY.clone()
}

fn build_legacy_locale() -> Locale {
    let names: HashMap<Unit, UnitNames> = LEGACY_UNITS
        .iter()
        .map(|&(id, _, symbol, aliases)| {
            let unit = Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"));
            let names = UnitNames {
                symbol: Some(symbol.to_string()),
                aliases: aliases.iter().map(|a| (*a).to_string()).collect(),
                long: Vec::new(),
            };
            (unit, names)
        })
        .collect();
    Locale::new(names).expect("the legacy locale is valid")
}
