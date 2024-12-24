//! Configuration data structures used in [`ConverterBuilder`](super::ConverterBuilder)

use enum_map::EnumMap;
use serde::Deserialize;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

use super::{FractionsConfig, PhysicalQuantity, System};

/// Configuration struct for units used in [`ConverterBuilder`](super::ConverterBuilder)
///
/// This structure is designed for deserializing [TOML](https://toml.io/en/),
/// but you can try other formats supported by serde.
#[derive(Debug, Deserialize, Clone, PartialEq)]
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
    /// Automatic conversion to fractions
    ///
    /// If enabled, a decimal value will be converted to a fraction if possible.
    pub fractions: Option<Fractions>,
    /// Extend and/or edit units from other layers before
    pub extend: Option<Extend>,
    /// Declare new units
    #[serde(default)]
    pub quantity: Vec<QuantityGroup>,
}

/// [SI] configuration used in [`UnitsFile`]
///
/// [SI]: https://en.wikipedia.org/wiki/International_System_of_Units
#[derive(Debug, Deserialize, Default, Clone, PartialEq)]
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
#[derive(
    Debug, Deserialize, Clone, Copy, strum::Display, strum::AsRefStr, enum_map::Enum, PartialEq,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
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

/// Configuration for fractions
///
/// A unit can have more than one layer, which are applied in the order:
/// - `all`
/// - `metric` / `imperial`
/// - `quantity`
/// - `unit`
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Fractions {
    /// The base configuration
    pub all: Option<FractionsConfigWrapper>,
    /// For metric units
    pub metric: Option<FractionsConfigWrapper>,
    /// For imperial units
    pub imperial: Option<FractionsConfigWrapper>,
    /// For each [`PhysicalQuantity`]
    pub quantity: HashMap<PhysicalQuantity, FractionsConfigWrapper>,
    /// For specific units. The keys are any unit name, symbol, or alias.
    pub unit: HashMap<String, FractionsConfigWrapper>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FractionsConfigWrapper {
    Toggle(bool),
    Custom(FractionsConfigHelper),
}

impl FractionsConfigWrapper {
    pub fn get(self) -> FractionsConfigHelper {
        match self {
            FractionsConfigWrapper::Toggle(enabled) => FractionsConfigHelper {
                enabled: Some(enabled),
                ..Default::default()
            },
            FractionsConfigWrapper::Custom(cfg) => cfg,
        }
    }
}

/// Fractions configuration layer
#[derive(Debug, Clone, Copy, Deserialize, Default, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct FractionsConfigHelper {
    /// If fractions are enabled. Defaults to `false`
    pub enabled: Option<bool>,
    /// Max percent of error allowed (0 to 1). Defaults to `0.05` (5%).
    pub accuracy: Option<f32>,
    /// Max denominator allowed (1 to 16). Defaults to 4.
    pub max_denominator: Option<u8>,
    /// Max whole number allowed. Defaults to [`u32::MAX`].
    pub max_whole: Option<u32>,
}

impl FractionsConfigHelper {
    /// Merges this layer with another
    ///
    /// It keeps the values defined in `self` and falls back to `other`.
    pub(crate) fn merge(self, other: FractionsConfigHelper) -> Self {
        Self {
            enabled: self.enabled.or(other.enabled),
            accuracy: self.accuracy.or(other.accuracy),
            max_denominator: self.max_denominator.or(other.max_denominator),
            max_whole: self.max_whole.or(other.max_whole),
        }
    }

    /// Defines the configuration to a [`FractionsConfig`]
    ///
    /// Non set values will take [`FractionsConfig::default`].
    pub(crate) fn define(self) -> FractionsConfig {
        let d = FractionsConfig::default();
        FractionsConfig {
            enabled: self.enabled.unwrap_or(d.enabled),
            accuracy: self.accuracy.unwrap_or(d.accuracy).clamp(0.0, 1.0),
            max_denominator: self
                .max_denominator
                .unwrap_or(d.max_denominator)
                .clamp(1, 16),
            max_whole: self.max_whole.unwrap_or(d.max_whole),
        }
    }
}

/// Extend units from other layers config used in [`UnitsFile`]
///
/// The maps's keys are any name, symbol or alias of the unit you want to extend.
#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Extend {
    /// Precedence when joining to other layers
    pub precedence: Precedence,
    /// Map for units to edit
    pub units: HashMap<String, ExtendUnitEntry>,
}

/// Precedence when joining a list to other layers
///
/// This is important in, for example, the case of symbols. The first symbol
/// is the one that will be used for formatting.
#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Precedence {
    /// The list will be added before the current ones (*higher priority*)
    #[default]
    Before,
    /// The list will be added after the current ones (*lower priority*)
    After,
    /// The list will replace the current ones
    Override,
}

/// Editable unit
///
/// See [`Unit`](super::Unit). If the unit is automatially generated (expanded) from another
/// one, only aliases can be set.
#[derive(Debug, Deserialize, Clone, Default, PartialEq)]
#[serde(default)]
pub struct ExtendUnitEntry {
    pub ratio: Option<f64>,
    pub difference: Option<f64>,
    #[serde(alias = "name")]
    pub names: Option<Vec<Arc<str>>>,
    #[serde(alias = "symbol")]
    pub symbols: Option<Vec<Arc<str>>>,
    #[serde(alias = "alias")]
    pub aliases: Option<Vec<Arc<str>>>,
}

/// Configuration of a group of units belonging to a [physical quantity]
///
/// [physical quantity]: https://en.wikipedia.org/wiki/Physical_quantity
#[derive(Debug, Deserialize, Clone, PartialEq)]
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
    #[serde(default)]
    pub units: Option<Units>,
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
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
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
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(untagged, deny_unknown_fields)]
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
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UnitEntry {
    /// Names. For example: `grams`
    ///
    /// This will expand with [`SI`] configuration.
    #[serde(alias = "name")]
    pub names: Vec<Arc<str>>,
    /// Symbols. For example: `g`
    ///
    /// This will expand with [`SI`] configuration.
    #[serde(alias = "symbol")]
    pub symbols: Vec<Arc<str>>,
    /// Whatever other way you want to call the unit.
    ///
    /// This **WILL NOT** expand with [`SI`] configuration.
    #[serde(default, alias = "alias")]
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

include!(concat!(env!("OUT_DIR"), "/bundled_units.rs"));

#[cfg(feature = "bundled_units")]
impl UnitsFile {
    /// Get the bundled units file
    ///
    /// This is only available with the `bundled_units` feature.
    pub fn bundled() -> Self {
        __bundled_units::get_bundled()
    }
}

#[cfg(all(test, feature = "bundled_units"))]
mod tests {
    use super::*;

    #[test]
    fn generated_bundled() {
        // this makes sure the build.rs script is working correctly
        let text = std::fs::read_to_string("units.toml").unwrap();
        let expected: UnitsFile = toml::from_str(&text).unwrap();
        assert_eq!(expected, UnitsFile::bundled());
    }
}
