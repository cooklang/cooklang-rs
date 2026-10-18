use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use super::locale::{Locale, UnitNames};
use super::preference::Row;
use super::preference::System::{self, Metric, UsCustomary};
use crate::convert::units::PhysicalQuantity::{self, Length, Mass, Temperature, Time, Volume};
use crate::convert::units::{LengthUnit, MassUnit, TemperatureUnit, TimeUnit, Unit, VolumeUnit};

/// Identifier, the system `units.toml` filed it under, the symbol a conversion writes back, every other spelling.
type Names = (&'static str, System, &'static str, &'static [&'static str]);


// The units.toml locale frozen here for compatibility.
#[rustfmt::skip]
const LEGACY_UNITS: &[Names] = &[
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

/// The system `units.toml` filed `unit` under, which is what an untagged recipe is fitted within. A unit it never knew,
/// reached by its identifier, is US customary: `units.toml` filed everything that was not SI, time or Celsius as
/// "imperial", and that rule is frozen with the rest.
pub(crate) fn legacy_system(unit: Unit) -> System {
    LEGACY_UNITS
        .iter()
        .find(|(id, ..)| *id == unit.id())
        .map_or(UsCustomary, |&(_, system, ..)| system)
}

const fn row(unit: Unit, at_least: f64, fractions: Option<u8>, or_decimal: bool) -> Row {
    Row {
        unit,
        at_least,
        fractions,
        or_decimal,
    }
}

// The units.toml preferences frozen here for compatibility
#[rustfmt::skip]
pub(crate) static LEGACY_PREFERENCES: &[(System, PhysicalQuantity, &[Row])] = &[
    // `best.metric = ["ml", "l"]`, `fractions.metric = false`
    (Metric, Volume, &[
        row(Unit::Volume(VolumeUnit::Liter),                1.0, None,    false),
        row(Unit::Volume(VolumeUnit::Milliliter),           1.0, None,    false),
    ]),
    // `best.metric = ["mg", "g", "kg"]`
    (Metric, Mass, &[
        row(Unit::Mass(MassUnit::Kilogram),                 1.0, None,    false),
        row(Unit::Mass(MassUnit::Gram),                     1.0, None,    false),
        row(Unit::Mass(MassUnit::Milligram),                1.0, None,    false),
    ]),
    // `best.metric = ["cm", "mm", "m"]`
    (Metric, Length, &[
        row(Unit::Length(LengthUnit::Meter),                1.0, None,    false),
        row(Unit::Length(LengthUnit::Centimeter),           1.0, None,    false),
        row(Unit::Length(LengthUnit::Millimeter),           1.0, None,    false),
    ]),
    // `best.metric = ["C"]`, `fractions.quantity.temperature = false`
    (Metric, Temperature, &[
        row(Unit::Temperature(TemperatureUnit::Celsius),    1.0, None,    false),
    ]),
    // `best = ["s", "h", "min", "d"]`, one list for both systems, `fractions.quantity.time = false`
    (Metric, Time, &[
        row(Unit::Time(TimeUnit::Day),                      1.0, None,    false),
        row(Unit::Time(TimeUnit::Hour),                     1.0, None,    false),
        row(Unit::Time(TimeUnit::Minute),                   1.0, None,    false),
        row(Unit::Time(TimeUnit::Second),                   1.0, None,    false),
    ]),
    // `best.imperial = ["cup", "tsp", "tbsp"]`; `fractions.unit.tsp.max_denominator = 8`, `tbsp = 3`, cup the
    // default 4. A third of a cup and half a tablespoon are the smallest fractions bundled kept in those units: a
    // quarter cup was `4 tbsp` and two thirds of a tablespoon `2 tsp`, whole numbers winning its search.
    (UsCustomary, Volume, &[
        row(Unit::Volume(VolumeUnit::UsCup),                1.0, Some(4), true),
        row(Unit::Volume(VolumeUnit::UsCup),                0.3, Some(4), false),
        row(Unit::Volume(VolumeUnit::UsTablespoon),         2.0, Some(3), true),
        row(Unit::Volume(VolumeUnit::UsTablespoon),         0.5, Some(3), false),
        row(Unit::Volume(VolumeUnit::UsTeaspoon),           0.0, Some(8), false),
    ]),
    // `best.imperial = ["oz", "lb"]`; `fractions.unit.lb.max_denominator = 8`, ounce the default 4. Below a pound
    // bundled kept the pound only for thirds, which the search decided; every other fraction of a pound is a whole
    // number of ounces.
    (UsCustomary, Mass, &[
        row(Unit::Mass(MassUnit::Pound),                    1.0, Some(8), false),
        row(Unit::Mass(MassUnit::Ounce),                    0.0, Some(4), false),
    ]),
    // `best.imperial = ["in", "ft"]`, the default denominator 4
    (UsCustomary, Length, &[
        row(Unit::Length(LengthUnit::Foot),                 1.0, Some(4), false),
        row(Unit::Length(LengthUnit::Inch),                 0.0, Some(4), false),
    ]),
    // `best.imperial = ["F"]`, `fractions.quantity.temperature = false`
    (UsCustomary, Temperature, &[
        row(Unit::Temperature(TemperatureUnit::Fahrenheit), 1.0, None,    false),
    ]),
];

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
