//! Configuration data structures used in [`ConverterBuilder`](super::ConverterBuilder)

use enum_map::EnumMap;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

use super::{PhysicalQuantity, System};

/// Configuration struct for units used in [`ConverterBuilder`](super::ConverterBuilder)
///
/// This structure is designed for deserializing [TOML](https://toml.io/en/),
/// but you can try other formats supported by serde.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnitsFile {
    /// Set the default system
    ///
    /// This is used in cases where the converter is asked to infer a system, but
    /// the unit doesn't belong to one, so the default is used.
    pub default_system: Option<System>,
    /// [SI] configuration
    ///
    /// This is optional, but at least one layer has to have it when
    /// [`UnitEntry::expand_si`] is used.
    ///
    /// [SI]: https://en.wikipedia.org/wiki/International_System_of_Units
    pub si: Option<SI>,
    /// Extend units from other layers before
    pub extend: Option<Extend>,
    /// Declare new units
    #[serde(default)]
    pub quantity: Vec<QuantityGroup>,
}

/// [SI] configuration used in [`UnitsFile`]
///
/// [SI]: https://en.wikipedia.org/wiki/International_System_of_Units
#[derive(Debug, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct SI {
    /// Prefixes for the names of the units when expanding
    ///
    /// This is optional, but at least one layer has to have it when
    /// [`UnitEntry::expand_si`] is used.
    pub prefixes: Option<EnumMap<SIPrefix, Vec<String>>>,
    /// Prefixes for the symbols of the units when expanding
    ///
    /// This is optional, but at least one layer has to have it when
    /// [`UnitEntry::expand_si`] is used.
    pub symbol_prefixes: Option<EnumMap<SIPrefix, Vec<String>>>,
    /// Precedence when joining to other layers
    #[serde(default)]
    pub precedence: Precedence,
}

/// [SI] supported prefixes
///
/// [SI]: https://en.wikipedia.org/wiki/International_System_of_Units
#[derive(Debug, Deserialize, Clone, Copy, strum::Display, strum::AsRefStr, enum_map::Enum)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum SIPrefix {
    Kilo,
    Hecto,
    Deca,
    Deci,
    Centi,
    Milli,
}

impl SIPrefix {
    /// Get the ratio of the prefix
    ///
    /// ```
    /// # use cooklang::convert::units_file::SIPrefix;
    /// assert_eq!(SIPrefix::Kilo.ratio(), 1000.0);
    /// ```
    pub fn ratio(&self) -> f64 {
        match self {
            SIPrefix::Kilo => 1e3,
            SIPrefix::Hecto => 1e2,
            SIPrefix::Deca => 1e1,
            SIPrefix::Deci => 1e-1,
            SIPrefix::Centi => 1e-2,
            SIPrefix::Milli => 1e-3,
        }
    }
}

/// Extend units from other layers config used in [`UnitsFile`]
///
/// The maps's keys are any name, symbol or alias of the unit you want to extend.
#[derive(Debug, Default, Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Extend {
    /// Precedence when joining to other layers
    pub precedence: Precedence,
    /// Map for new names
    pub names: HashMap<String, Vec<Arc<str>>>,
    /// Map for new symbols
    pub symbols: HashMap<String, Vec<Arc<str>>>,
    /// Map for new aliases
    pub aliases: HashMap<String, Vec<Arc<str>>>,
}

/// Precedence when joining a list to other layers
///
/// This is important in, for example, the case of symbols. The first symbol
/// is the one that will be used for formatting.
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename = "snake_case")]
pub enum Precedence {
    /// The list will be added before the current ones (*higher priority*)
    #[default]
    Before,
    /// The list will be added after the current ones (*lower priority*)
    After,
    /// The list will replace the current ones
    Override,
}

/// Configuration of a group of units belonging to a [physical quantity]
///
/// [physical quantity]: https://en.wikipedia.org/wiki/Physical_quantity
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct QuantityGroup {
    /// Quantity of the group
    ///
    /// All units in this struct will be belong to this quantity
    pub quantity: PhysicalQuantity,
    /// List of best units
    ///
    /// This is optional by at least one [`QuantityGroup`] of the quantity in
    /// any [`UnitsFile`] in a converter has to define it and not be empty.
    ///
    /// **This will always replace the configuration from [`UnitsFile`] before**
    #[serde(default)]
    pub best: Option<BestUnits>,
    /// Definition of units
    pub units: Units,
}

/// List of best units
///
/// The *best* units are the one elegible for automatic unit convertion to a
/// system or to fit a value to it's best possible unit.
///
/// The difference between the 2 variants of this enum is that one has information
/// about the system and the other doesn't. It's the same in [`Units`]. You can
/// set a unit's system in either, this enum, in [`Units`] or in both (but it
/// has to match).
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged, rename = "snake_case", deny_unknown_fields)]
pub enum BestUnits {
    /// List without system information
    Unified(Vec<String>),
    /// A list per system
    BySystem {
        metric: Vec<String>,
        imperial: Vec<String>,
    },
}

/// New units
///
/// The difference between the 2 variants of this enum is that one has information
/// about the system and the other doesn't. It's the same in [`BestUnits`]. You can
/// set a unit's system in either, this enum, in [`BestUnits`] or in both (but it
/// has to match).
#[derive(Debug, Deserialize, Clone)]
#[serde(untagged, rename = "snake_case", deny_unknown_fields)]
pub enum Units {
    /// List without [`System`] information
    Unified(Vec<UnitEntry>),
    /// A list per [`System`] or uspecified
    BySystem {
        #[serde(default)]
        metric: Vec<UnitEntry>,
        #[serde(default)]
        imperial: Vec<UnitEntry>,
        #[serde(default)]
        unspecified: Vec<UnitEntry>,
    },
}

/// A new unit
///
/// This does not carry the [`System`] information, see [`Units`] and/or
/// [`BestUnits`].
///
/// Conversions will be `val * [Self::ratio] + [Self::difference]`.
#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct UnitEntry {
    /// Names. For example: `grams`
    ///
    /// This will expand with [`SI`] configuration.
    pub names: Vec<Arc<str>>,
    /// Symbols. For example: `g`
    ///
    /// This will expand with [`SI`] configuration.
    pub symbols: Vec<Arc<str>>,
    /// Whatever other way you want to call the unit.
    ///
    /// This **WILL NOT** expand with [`SI`] configuration.
    #[serde(default)]
    pub aliases: Vec<Arc<str>>,
    /// Conversion ratio.
    ///
    /// All units of a [`PhysicalQuantity`] have to be configured carefuly so
    /// ratios match. The easiest way is setting one unit to have ratio of `1`
    /// and set all other ratios to match.
    ///
    /// For example, if `gram` has a ratio of `1`, `kilogram` will have a
    /// ratio of `1000`.
    pub ratio: f64,
    /// Difference correction
    ///
    /// Some units cannot be linearly converted to others just with a `ratio`.
    /// (namely celsius to fahrenheit).
    #[serde(default)]
    pub difference: f64,
    /// Mark this unit to expand with [`SI`] configuration.
    ///
    /// For example, if this unit is `gram` and is marked with `expand_si`, it
    /// will generate `kilogram`, `hectogram`, `decagram`, `decigram`,
    /// `centigram` and `milligram` automatically so you don't have to.
    #[serde(default)]
    pub expand_si: bool,
}

#[cfg(feature = "bundled_units")]
impl UnitsFile {
    /// Get the bundled units file
    ///
    /// This is only available with the `bundled_units` feature.
    pub fn bundled() -> Self {
        const TEXT: &str = include_str!("../../units.toml");
        static FILE: Lazy<UnitsFile> = Lazy::new(|| toml::from_str(TEXT).unwrap());
        FILE.clone()
    }
}
