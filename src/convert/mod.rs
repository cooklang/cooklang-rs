//! Support for **configurable** unit conversion
//!
//! This includes:
//! - A layered configuration system
//! - Conversions between systems
//! - Conversions to the best fit possible

use std::{collections::HashMap, ops::RangeInclusive, sync::Arc};

use enum_map::EnumMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    quantity::{Number, Quantity, ScaledQuantity, Value},
    ScaledRecipe,
};

pub use builder::{ConverterBuilder, ConverterBuilderError};
pub use units_file::UnitsFile;

mod builder;
pub mod units_file;

/// Main struct to perform conversions
///
/// This holds information about all the known units and how to convert them.
///
/// To create one use [`Converter::builder`].
///
/// [`Converter::default`] changes with the feature `bundled_units`:
/// - When enabled, [`Converter::bundled`].
/// - When disabled, [`Converter::empty`].
#[derive(Debug, Clone)]
pub struct Converter {
    all_units: Vec<Arc<Unit>>,
    unit_index: UnitIndex,
    quantity_index: UnitQuantityIndex,
    best: EnumMap<PhysicalQuantity, BestConversionsStore>,
    fractions: Fractions,
    default_system: System,
}

impl Converter {
    /// Start to create a new [Converter]
    pub fn builder() -> ConverterBuilder {
        ConverterBuilder::new()
    }

    /// Empty converter
    ///
    /// This is the default when the `bundled_units` feature is disabled.
    ///
    /// The main use case for this is to ignore the units, because an empty
    /// converter will fail to convert everything. Also, if the `ADVANCED_UNITS`
    /// extension is enabled, every timer unit will throw an error, because they
    /// have to be known time units.
    pub fn empty() -> Self {
        Self {
            all_units: Default::default(),
            unit_index: Default::default(),
            quantity_index: Default::default(),
            best: Default::default(),
            default_system: Default::default(),
            fractions: Default::default(),
        }
    }

    /// Converter with the bundled units
    ///
    /// The converter will have the bundled units that doens't need any external
    /// file. These are the basic unit for most of the recipes you will need
    /// (in English).
    ///
    /// This is only available when the `bundled_units` feature is enabled.
    ///
    /// This is the default when the `bundled_units` feature is enabled.
    #[cfg(feature = "bundled_units")]
    pub fn bundled() -> Self {
        ConverterBuilder::new()
            .with_units_file(UnitsFile::bundled())
            .unwrap()
            .finish()
            .unwrap()
    }

    /// Get the default unit [System]
    pub fn default_system(&self) -> System {
        self.default_system
    }

    /// Get the total number of known units.
    ///
    /// This is **not** all the known unit names, just **different units**.
    pub fn unit_count(&self) -> usize {
        self.all_units.len()
    }

    /// Get an iterator of all the known units.
    pub fn all_units(&self) -> impl Iterator<Item = &Unit> {
        self.all_units.iter().map(|u| u.as_ref())
    }

    /// Check if a unit is one of the possible conversions in it's units system.
    ///
    /// When a unit is a *best unit*, the converter can choose it when trying
    /// to get the best match for a value.
    ///
    /// # Panics
    /// If the unit is not known.
    pub fn is_best_unit(&self, unit: &Unit) -> bool {
        let unit_id = self
            .unit_index
            .get_unit_id(unit.symbol())
            .expect("unit not found");
        let Some(system) = unit.system else {
            return false;
        };
        let conversions = self.best[unit.physical_quantity].conversions(system);
        conversions.0.iter().any(|&(_, id)| id == unit_id)
    }

    /// Get the (marked) best units for a quantity and a system.
    ///
    /// If system is None, returns for all the systems.
    pub fn best_units(&self, quantity: PhysicalQuantity, system: Option<System>) -> Vec<Arc<Unit>> {
        match &self.best[quantity] {
            BestConversionsStore::Unified(u) => u.all_units(self).cloned().collect(),
            BestConversionsStore::BySystem { metric, imperial } => match system {
                Some(System::Metric) => metric.all_units(self).cloned().collect(),
                Some(System::Imperial) => imperial.all_units(self).cloned().collect(),
                None => metric
                    .all_units(self)
                    .chain(imperial.all_units(self))
                    .cloned()
                    .collect(),
            },
        }
    }

    /// Find a unit by any of it's names, symbols or aliases
    pub fn find_unit(&self, unit: &str) -> Option<Arc<Unit>> {
        let uid = self.unit_index.get_unit_id(unit).ok()?;
        Some(self.all_units[uid].clone())
    }

    /// Gets the fractions configuration for the given unit
    ///
    /// # Panics
    /// If the unit is not known.
    #[tracing::instrument(level = "trace", skip_all, fields(unit = %unit), ret)]
    pub(crate) fn fractions_config(&self, unit: &Unit) -> FractionsConfig {
        let unit_id = self
            .unit_index
            .get_unit_id(unit.symbol())
            .expect("unit not found");
        self.fractions
            .config(unit.system, unit.physical_quantity, unit_id)
    }

    /// Determines if the unit should be tried to be converted into a fraction
    ///
    /// # Panics
    /// If the unit is not known.
    pub(crate) fn should_fit_fraction(&self, unit: &Unit) -> bool {
        self.fractions_config(unit).enabled
    }
}

#[cfg(not(feature = "bundled_units"))]
impl Default for Converter {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(feature = "bundled_units")]
impl Default for Converter {
    fn default() -> Self {
        Self::bundled()
    }
}

impl PartialEq for Converter {
    fn eq(&self, other: &Self) -> bool {
        self.all_units == other.all_units
            && self.unit_index == other.unit_index
            && self.quantity_index == other.quantity_index
            && self.best == other.best
            && self.default_system == other.default_system
        // temperature_regex ignored, it should be the same if the rest is the
        // the same
    }
}

#[derive(Debug, Clone, Default)]
struct Fractions {
    all: Option<FractionsConfig>,
    metric: Option<FractionsConfig>,
    imperial: Option<FractionsConfig>,
    quantity: HashMap<PhysicalQuantity, FractionsConfig>,
    unit: HashMap<usize, FractionsConfig>,
}

impl Fractions {
    fn config(
        &self,
        system: Option<System>,
        quantity: PhysicalQuantity,
        unit_id: usize,
    ) -> FractionsConfig {
        self.unit
            .get(&unit_id)
            .or_else(|| self.quantity.get(&quantity))
            .or_else(|| {
                system.and_then(|s| match s {
                    System::Metric => self.metric.as_ref(),
                    System::Imperial => self.imperial.as_ref(),
                })
            })
            .or(self.all.as_ref())
            .copied()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct FractionsConfig {
    pub enabled: bool,
    pub accuracy: f32,
    pub max_denominator: u8,
    pub max_whole: u32,
}

impl Default for FractionsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            accuracy: 0.05,
            max_denominator: 4,
            max_whole: u32::MAX,
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct UnitIndex(HashMap<Arc<str>, usize>);

impl UnitIndex {
    fn get_unit_id(&self, key: &str) -> Result<usize, UnknownUnit> {
        self.0
            .get(key)
            .copied()
            .ok_or_else(|| UnknownUnit(key.to_string()))
    }
}

pub(crate) type UnitQuantityIndex = EnumMap<PhysicalQuantity, Vec<usize>>;

/// A unit
///
/// Conversion will be `val * [Self::ratio] + [Self::difference]`
///
/// It implements [Display](std::fmt::Display). It will use [`Self::symbol`] or,
/// if alternate (`#`) is given, it will try the first name.
#[derive(Debug, Clone, Serialize)]
pub struct Unit {
    /// All the names that may be used to format the unit
    pub names: Vec<Arc<str>>,
    /// All the symbols (abbreviations), like `ml` for `millilitres`
    pub symbols: Vec<Arc<str>>,
    /// Custom aliases to parse the unit from a different string
    pub aliases: Vec<Arc<str>>,
    /// Conversion ratio
    pub ratio: f64,
    /// Difference offset to the conversion ratio
    pub difference: f64,
    /// The [`PhysicalQuantity`] this unit belongs to
    pub physical_quantity: PhysicalQuantity,
    /// The unit [System] this unit belongs to, if any
    pub system: Option<System>,
}

impl Unit {
    fn all_keys(&self) -> impl Iterator<Item = &Arc<str>> {
        self.names.iter().chain(&self.symbols).chain(&self.aliases)
    }

    /// Get the symbol that represent this unit. The process is:
    /// - First symbol (if any)
    /// - Or first name (if any)
    /// - Or first alias (if any)
    /// - **panics**
    pub fn symbol(&self) -> &str {
        self.symbols
            .first()
            .or_else(|| self.names.first())
            .or_else(|| self.aliases.first())
            .expect("symbol, name or alias in unit")
    }
}

impl PartialEq for Unit {
    fn eq(&self, other: &Self) -> bool {
        self.names == other.names
            && self.symbols == other.symbols
            && self.aliases == other.aliases
            && self.ratio == other.ratio
            && self.difference == other.difference
            && self.physical_quantity == other.physical_quantity
            && self.system == other.system
        // expand_si and expanded_units ignored
    }
}

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() && !self.names.is_empty() {
            write!(f, "{}", self.names[0])
        } else {
            write!(f, "{}", self.symbol())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum BestConversionsStore {
    Unified(BestConversions),
    BySystem {
        metric: BestConversions,
        imperial: BestConversions,
    },
}

impl BestConversionsStore {
    pub(crate) fn conversions(&self, system: System) -> &BestConversions {
        match self {
            BestConversionsStore::Unified(u) => u,
            BestConversionsStore::BySystem { metric, imperial } => match system {
                System::Metric => metric,
                System::Imperial => imperial,
            },
        }
    }
}

impl Default for BestConversionsStore {
    fn default() -> Self {
        Self::Unified(Default::default())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct BestConversions(Vec<(f64, usize)>);

impl BestConversions {
    fn base(&self) -> Option<usize> {
        self.0.first().map(|c| c.1)
    }

    fn best_unit(
        &self,
        converter: &Converter,
        value: &ConvertValue,
        unit: &Unit,
    ) -> Option<Arc<Unit>> {
        let value = match value {
            ConvertValue::Number(n) => n.abs(),
            ConvertValue::Range(r) => r.start().abs(),
        };
        let base_unit_id = self.base()?;
        let base_unit = &converter.all_units[base_unit_id];
        let norm = converter.convert_f64(value, unit, base_unit);

        let best_id = self
            .0
            .iter()
            .rev()
            .find(|(th, _)| norm >= (th - 0.001))
            .or_else(|| self.0.first())
            .map(|&(_, id)| id)?;
        Some(Arc::clone(&converter.all_units[best_id]))
    }

    fn all_units<'c>(&'c self, converter: &'c Converter) -> impl Iterator<Item = &'c Arc<Unit>> {
        self.0.iter().map(|(_, uid)| &converter.all_units[*uid])
    }
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    Hash,
    strum::Display,
    strum::EnumString,
    enum_map::Enum,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum PhysicalQuantity {
    Volume,
    Mass,
    Length,
    Temperature,
    Time,
}

impl ScaledRecipe {
    /// Convert a [`ScaledRecipe`] to another [`System`] in place.
    ///
    /// When an error occurs, it is stored and the quantity stays the same.
    ///
    /// Returns all the errors while converting. These usually are missing units,
    /// unknown units or text values.
    pub fn convert(&mut self, to: System, converter: &Converter) -> Vec<ConvertError> {
        let mut errors = Vec::new();

        let to = ConvertTo::from(to);

        let mut conv = |q: &mut ScaledQuantity| {
            if let Err(e) = q.convert(to, converter) {
                errors.push(e)
            }
        };

        for igr in &mut self.ingredients {
            if let Some(q) = &mut igr.quantity {
                conv(q);
            }
        }

        // cookware can't have units

        for timer in &mut self.timers {
            if let Some(q) = &mut timer.quantity {
                conv(q);
            }
        }

        for q in &mut self.inline_quantities {
            conv(q);
        }

        errors
    }
}

impl ScaledQuantity {
    pub fn convert<'a>(
        &mut self,
        to: impl Into<ConvertTo<'a>>,
        converter: &Converter,
    ) -> Result<(), ConvertError> {
        self.convert_impl(to.into(), converter)
    }

    #[tracing::instrument(level = "trace", name = "convert", skip_all)]
    fn convert_impl(&mut self, to: ConvertTo, converter: &Converter) -> Result<(), ConvertError> {
        if self.unit().is_none() {
            return Err(ConvertError::NoUnit(self.clone()));
        }

        let unit_info = self.unit_info(converter);
        let original_system;
        let unit = match unit_info {
            Some(ref u) => {
                original_system = u.system;
                ConvertUnit::Unit(u)
            }
            None => {
                return Err(ConvertError::UnknownUnit(UnknownUnit(
                    self.unit().unwrap().to_string(),
                )))
            }
        };
        let value = ConvertValue::try_from(self.value())?;

        let (new_value, new_unit) = converter.convert(value, unit, to)?;
        *self = Quantity::new(new_value.into(), Some(new_unit.symbol().to_string()));
        match to {
            ConvertTo::Unit(_) => {
                self.try_fraction(converter);
            }
            ConvertTo::Best(target_system) => {
                self.fit_fraction(&new_unit, Some(target_system), converter)?;
            }
            ConvertTo::SameSystem => {
                self.fit_fraction(&new_unit, original_system, converter)?;
            }
        }
        Ok(())
    }

    /// Converts the unit to the best possible match in the same unit system.
    ///
    /// For example, `1000 ml` would be converted to `1 l`.
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn fit(&mut self, converter: &Converter) -> Result<(), ConvertError> {
        // only known units can be fitted
        let Some(unit) = self.unit_info(converter) else {
            return Ok(());
        };

        // If configured, try fitting as a fraction
        if converter.should_fit_fraction(&unit)
            && self.fit_fraction(&unit, unit.system, converter)?
        {
            return Ok(());
        }

        // convert to the best in the same system
        self.convert(ConvertTo::SameSystem, converter)?;

        Ok(())
    }

    /// Fits the quantity as an approximation.
    ///
    /// - Finds all the conversions where an approximation is possible
    /// - Get's the best one
    /// - Convert the value(s)
    ///
    /// Returns Ok(true) only if the value could be approximated.
    fn fit_fraction(
        &mut self,
        unit: &Arc<Unit>,
        target_system: Option<System>,
        converter: &Converter,
    ) -> Result<bool, ConvertError> {
        let approx = |val: f64, cfg: FractionsConfig| {
            Number::new_approx(val, cfg.accuracy, cfg.max_denominator, cfg.max_whole)
        };

        let Some(system) = target_system else {
            return Ok(self.try_fraction(converter)); // no system, just keep the same unit
        };

        let value = match self.value() {
            Value::Number(n) => n.value(),
            Value::Range { start, .. } => start.value(),
            Value::Text(ref t) => return Err(ConvertError::TextValue(t.clone())),
        };

        let possible_conversions = converter.best[unit.physical_quantity]
            .conversions(system)
            .0
            .iter()
            .filter_map(|&(_, new_unit_id)| {
                let new_unit = &converter.all_units[new_unit_id];
                let cfg = converter.fractions.config(
                    new_unit.system,
                    new_unit.physical_quantity,
                    new_unit_id,
                );
                if !cfg.enabled {
                    return None;
                }
                let new_value = converter.convert_f64(value, unit, new_unit);
                let new_value = approx(new_value, cfg)?;
                Some((new_value, new_unit))
            });

        let selected = possible_conversions.min_by(|(a, _), (b, _)| {
            let key = |v| match v {
                Number::Fraction {
                    den, err, whole, ..
                } => (den, whole as f64, err.abs()),
                Number::Regular(whole) => (1, whole, 0.0),
            };
            let a = key(*a);
            let b = key(*b);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Less)
        });

        let Some((new_value, new_unit)) = selected else {
            return Ok(false);
        };

        let new_value = match self.value() {
            Value::Number(_) => Value::Number(new_value),
            Value::Range { end, .. } => {
                let end = converter.convert_f64(end.value(), unit, new_unit);
                let end_frac = approx(end, converter.fractions_config(new_unit))
                    .unwrap_or(Number::Regular(end));
                Value::Range {
                    start: new_value,
                    end: end_frac,
                }
            }
            Value::Text(_) => unreachable!(),
        };
        *self = Quantity::new(new_value, Some(new_unit.symbol().to_string()));
        Ok(true)
    }

    /// Tries to convert the value to a fraction, keeping the same unit
    ///
    /// It respects the converter configuration for the unit.
    #[tracing::instrument(level = "trace", skip_all)]
    pub fn try_fraction(&mut self, converter: &Converter) -> bool {
        // only known units can be fitted
        let Some(unit) = self.unit_info(converter) else {
            return false;
        };

        let cfg = converter.fractions_config(&unit);
        if !cfg.enabled {
            return false;
        }

        match self.value_mut() {
            Value::Number(n) => n.try_approx(cfg.accuracy, cfg.max_denominator, cfg.max_whole),
            Value::Range { start, end } => {
                start.try_approx(cfg.accuracy, cfg.max_denominator, cfg.max_whole)
                    || end.try_approx(cfg.accuracy, cfg.max_denominator, cfg.max_whole)
            }
            Value::Text(_) => false,
        }
    }
}

impl Converter {
    /// Perform a conversion
    pub fn convert(
        &self,
        value: ConvertValue,
        unit: ConvertUnit,
        to: ConvertTo,
    ) -> Result<(ConvertValue, Arc<Unit>), ConvertError> {
        let unit = self.get_unit(&unit)?;

        let (value, unit) = match to {
            ConvertTo::Unit(target_unit) => {
                let to = self.get_unit(&target_unit)?;
                let val = self.convert_to_unit(value, unit, to.as_ref())?;
                (val, Arc::clone(to))
            }
            ConvertTo::Best(system) => self.convert_to_best(value, unit, system)?,
            ConvertTo::SameSystem => {
                self.convert_to_best(value, unit, unit.system.unwrap_or(self.default_system))?
            }
        };
        Ok((value, unit))
    }

    fn convert_to_unit(
        &self,
        value: ConvertValue,
        unit: &Unit,
        target_unit: &Unit,
    ) -> Result<ConvertValue, ConvertError> {
        if unit.physical_quantity != target_unit.physical_quantity {
            return Err(ConvertError::MixedQuantities {
                from: unit.physical_quantity,
                to: target_unit.physical_quantity,
            });
        }
        Ok(self.convert_value(value, unit, target_unit))
    }

    fn convert_to_best(
        &self,
        value: ConvertValue,
        unit: &Unit,
        system: System,
    ) -> Result<(ConvertValue, Arc<Unit>), ConvertError> {
        let conversions = self.best[unit.physical_quantity].conversions(system);

        let best_unit = conversions.best_unit(self, &value, unit).ok_or({
            ConvertError::BestUnitNotFound {
                physical_quantity: unit.physical_quantity,
                system: unit.system,
            }
        })?;
        let converted = self.convert_value(value, unit, best_unit.as_ref());

        Ok((converted, best_unit))
    }

    fn convert_value(&self, value: ConvertValue, from: &Unit, to: &Unit) -> ConvertValue {
        match value {
            ConvertValue::Number(n) => ConvertValue::Number(self.convert_f64(n, from, to)),
            ConvertValue::Range(r) => {
                let s = self.convert_f64(*r.start(), from, to);
                let e = self.convert_f64(*r.end(), from, to);
                ConvertValue::Range(s..=e)
            }
        }
    }

    fn convert_f64(&self, value: f64, from: &Unit, to: &Unit) -> f64 {
        if std::ptr::eq(from, to) {
            return value;
        }
        convert_f64(value, from, to)
    }

    pub(crate) fn get_unit<'a>(
        &'a self,
        unit: &'a ConvertUnit,
    ) -> Result<&'a Arc<Unit>, UnknownUnit> {
        let unit = match unit {
            ConvertUnit::Unit(u) => u,
            ConvertUnit::Key(key) => {
                let id = self.unit_index.get_unit_id(key)?;
                &self.all_units[id]
            }
        };
        Ok(unit)
    }
}

pub(crate) fn convert_f64(value: f64, from: &Unit, to: &Unit) -> f64 {
    assert_eq!(from.physical_quantity, to.physical_quantity);

    let norm = (value + from.difference) * from.ratio;
    (norm / to.ratio) - to.difference
}

/// Error when try to convert an unknown unit
#[derive(Debug, Error)]
#[error("Unknown unit: '{0}'")]
pub struct UnknownUnit(pub String);

/// Input value for [`Converter::convert`]
#[derive(PartialEq, Clone, Debug)]
pub enum ConvertValue {
    Number(f64),
    /// It will convert the range as if start and end were 2 calls to convert as
    /// a number
    Range(RangeInclusive<f64>),
}

/// Input unit for [`Converter::convert`]
#[derive(Debug, Clone, Copy)]
pub enum ConvertUnit<'a> {
    /// A unit directly
    ///
    /// This is a small optimization when you already know the unit instance,
    /// but [`ConvertUnit::Key`] will produce the same result with a fast
    /// lookup.
    Unit(&'a Arc<Unit>),
    /// Any name, symbol or alias to a unit
    Key(&'a str),
}

/// Input target for [`Converter::convert`]
#[derive(Debug, Clone, Copy)]
pub enum ConvertTo<'a> {
    SameSystem,
    Best(System),
    Unit(ConvertUnit<'a>),
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Deserialize,
    Serialize,
    Default,
    PartialOrd,
    Ord,
    strum::Display,
    strum::EnumString,
    enum_map::Enum,
)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
pub enum System {
    #[default]
    Metric,
    Imperial,
}

impl<'a> From<&'a str> for ConvertUnit<'a> {
    fn from(value: &'a str) -> Self {
        Self::Key(value)
    }
}

impl<'a> From<&'a Arc<Unit>> for ConvertUnit<'a> {
    fn from(value: &'a Arc<Unit>) -> Self {
        Self::Unit(value)
    }
}

impl<'a> From<&'a str> for ConvertTo<'a> {
    fn from(value: &'a str) -> Self {
        Self::Unit(ConvertUnit::Key(value))
    }
}

impl From<System> for ConvertTo<'_> {
    fn from(value: System) -> Self {
        Self::Best(value)
    }
}

impl<'a> From<&'a Arc<Unit>> for ConvertTo<'a> {
    fn from(value: &'a Arc<Unit>) -> Self {
        Self::Unit(value.into())
    }
}

impl From<ConvertValue> for Value {
    fn from(value: ConvertValue) -> Self {
        match value {
            ConvertValue::Number(n) => Self::Number(n.into()),
            ConvertValue::Range(r) => Self::Range {
                start: (*r.start()).into(),
                end: (*r.end()).into(),
            },
        }
    }
}

impl TryFrom<&Value> for ConvertValue {
    type Error = ConvertError;
    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        let value = match value {
            Value::Number(n) => ConvertValue::Number(n.value()),
            Value::Range { start, end } => ConvertValue::Range(start.value()..=end.value()),
            Value::Text(t) => return Err(ConvertError::TextValue(t.clone())),
        };
        Ok(value)
    }
}

impl From<f64> for ConvertValue {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<RangeInclusive<f64>> for ConvertValue {
    fn from(value: RangeInclusive<f64>) -> Self {
        Self::Range(value)
    }
}

impl PartialOrd<Self> for ConvertValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        fn extract(v: &ConvertValue) -> f64 {
            match v {
                ConvertValue::Number(n) => *n,
                ConvertValue::Range(r) => *r.start(),
            }
        }
        let this = extract(self);
        let other = extract(other);
        this.partial_cmp(&other)
    }
}

/// Errors from converting
#[derive(Debug, Error)]
pub enum ConvertError {
    #[error("Tried to convert a value with no unit")]
    NoUnit(ScaledQuantity),

    #[error("Tried to convert a text value: {0}")]
    TextValue(String),

    #[error("Mixed physical quantities: {from} {to}")]
    MixedQuantities {
        from: PhysicalQuantity,
        to: PhysicalQuantity,
    },

    #[error("Could not find best unit for a {physical_quantity} unit. System: {system:?}")]
    BestUnitNotFound {
        physical_quantity: PhysicalQuantity,
        system: Option<System>,
    },

    #[error(transparent)]
    UnknownUnit(#[from] UnknownUnit),
}
