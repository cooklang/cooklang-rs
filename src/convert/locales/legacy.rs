//! The names `units.toml` gives the bundled units, as a [`Locale`], so the
//! two converters can be compared on the same recipe text. Frozen: it names
//! exactly those units.

use std::collections::HashMap;

use super::locale::{Locale, UnitNames};
use super::preference::System;
use crate::convert::units::{PhysicalQuantity, Unit};

/// A unit's names in one locale: its id, its symbols and its other spellings.
/// The first symbol is the one a conversion writes back.
#[rustfmt::skip]
type Names = (&'static str, &'static [&'static str], &'static [&'static str]);

/// The metric bases, which `units.toml` also names with every SI prefix.
#[rustfmt::skip]
const LEGACY_SI_UNITS: &[Names] = &[
    ("liter", &["l", "L"], &["liter", "liters", "litre", "litres"]),
    ("meter", &["m"],      &["meter", "meters", "metre", "metres"]),
    ("gram",  &["g"],      &["gram", "grams"]),
];

/// Every other unit `units.toml` defines.
#[rustfmt::skip]
const LEGACY_UNITS: &[Names] = &[
    ("us teaspoon",    &["tsp", "tsp."], &["teaspoon", "teaspoons"]),
    ("us tablespoon",  &["tbsp", "tbsp.", "tbs", "tbs."], &["tablespoon", "tablespoons"]),
    ("us fluid ounce", &["fl oz", "fl. oz.", "fl. oz", "fl oz."], &["fluid ounce", "fluid ounces"]),
    ("us cup",         &["c"], &["cup", "cups"]),
    ("us pint",        &["pt"], &["pint", "pints"]),
    ("us quart",       &["qt"], &["quart", "quarts"]),
    ("us gallon",      &["gal"], &["gallon", "gallons"]),
    ("foot",           &["ft", "'"], &["foot", "feet"]),
    ("inch",           &["in", "\""], &["inch", "inches"]),
    ("ounce",          &["oz", "oz."], &["ounce", "ounces"]),
    ("pound",          &["lb", "lb."], &["pound", "pounds"]),
    ("second",         &["s", "sec"], &["second", "seconds", "secs"]),
    ("minute",         &["min"], &["minute", "minutes", "mins"]),
    ("hour",           &["h"], &["hour", "hours"]),
    ("day",            &["d"], &["day", "days"]),
    // the other spellings `units.toml` gives the degrees are UNIVERSAL_ALIASES
    ("celsius",        &["°C"], &["celsius"]),
    ("fahrenheit",     &["°F"], &["fahrenheit"]),
];

/// Prefix word and symbol, in `units.toml` order.
#[rustfmt::skip]
const SI_PREFIXES: [(&str, &str); 6] = [
    ("kilo", "k"), ("hecto", "h"), ("deca", "da"),
    ("deci", "d"), ("centi", "c"), ("milli", "m"),
];

/// The system `units.toml` filed `unit` under, which is what an untagged
/// recipe is fitted within: the SI families and Celsius were metric,
/// everything else "imperial", which is US customary. Time was one list, so
/// its system does not matter; it is metric, where its rows live.
pub(crate) fn legacy_system(unit: Unit) -> System {
    let id = unit.id();
    let si = LEGACY_SI_UNITS
        .iter()
        .any(|(base, _, _)| id == *base || id.ends_with(base));
    if si || unit.physical_quantity() == PhysicalQuantity::Time || id == "celsius" {
        System::Metric
    } else {
        System::UsCustomary
    }
}

pub(crate) fn legacy_locale() -> Locale {
    let mut names = HashMap::new();
    let mut add = |id: &str, unit_names: UnitNames| {
        let unit = Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"));
        names.insert(unit, unit_names);
    };

    for &(id, symbols, aliases) in LEGACY_SI_UNITS {
        add(id, unit_names(symbols, aliases));
        for (word, symbol) in SI_PREFIXES {
            add(
                &format!("{word}{id}"),
                unit_names(&with_prefix(symbols, symbol), &with_prefix(aliases, word)),
            );
        }
    }
    for &(id, symbols, aliases) in LEGACY_UNITS {
        add(id, unit_names(symbols, aliases));
    }

    Locale::new(names).expect("the legacy locale is valid")
}

fn unit_names<S: AsRef<str>>(symbols: &[S], aliases: &[S]) -> UnitNames {
    let (symbol, other_symbols) = symbols.split_first().expect("a symbol");
    UnitNames {
        symbol: Some(symbol.as_ref().to_string()),
        aliases: aliases
            .iter()
            .chain(other_symbols)
            .map(|s| s.as_ref().to_string())
            .collect(),
        long: Vec::new(),
    }
}

fn with_prefix(strings: &[&str], prefix: &str) -> Vec<String> {
    strings.iter().map(|s| format!("{prefix}{s}")).collect()
}
