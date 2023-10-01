//! Quantity model

use std::{collections::HashMap, fmt::Display, ops::RangeInclusive, sync::Arc};

use enum_map::EnumMap;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ast,
    convert::{ConvertError, Converter, PhysicalQuantity, Unit},
};

/// A quantity used in components
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Quantity<V: QuantityValue = Value> {
    /// Value
    pub value: V,
    pub(crate) unit: Option<QuantityUnit>,
}

pub type ScalableQuantity = Quantity<ScalableValue>;
pub type ScaledQuantity = Quantity<Value>;

/// A value with scaling support
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ScalableValue {
    /// Cannot be scaled
    Fixed(Value),
    /// Scaling is linear to the number of servings
    Linear(Value),
    /// Scaling is in defined steps of the number of servings
    ByServings(Vec<Value>),
}

/// Base value
///
/// The [`Display`] implementation round `f64` to 3 decimal places.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Value {
    /// Numeric
    Number(f64),
    /// Range
    Range(RangeInclusive<f64>),
    /// Text
    ///
    /// It is not possible to operate with this variant.
    Text(String),
}

pub trait QuantityValue: Display + Clone + sealed::Sealed {
    /// Check if the value is or contains text
    fn is_text(&self) -> bool;
}

impl QuantityValue for ScalableValue {
    fn is_text(&self) -> bool {
        match self {
            ScalableValue::Fixed(value) => value.is_text(),
            ScalableValue::Linear(value) => value.is_text(),
            ScalableValue::ByServings(values) => values.iter().any(Value::is_text),
        }
    }
}

impl QuantityValue for Value {
    fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::ScalableValue {}
    impl Sealed for super::Value {}
}

/// Unit text with lazy rich information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct QuantityUnit {
    text: String,
    #[serde(skip)]
    info: OnceCell<UnitInfo>,
}

/// Information about the unit
#[derive(Debug, Clone)]
pub enum UnitInfo {
    /// Unit is known
    Known(Arc<Unit>),
    /// Unknown unit
    Unknown,
}

impl PartialEq for QuantityUnit {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

impl QuantityUnit {
    /// Original text of the unit
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Cached information about the unit.
    ///
    /// If [`None`] is returned it means
    /// the unit has not been parsed yet. Try with [`Self::unit_info_or_parse`].
    pub fn unit_info(&self) -> Option<UnitInfo> {
        self.info.get().cloned()
    }

    /// Information about the unit
    pub fn unit_info_or_parse(&self, converter: &Converter) -> UnitInfo {
        self.info
            .get_or_init(|| UnitInfo::new(&self.text, converter))
            .clone()
    }
}

impl UnitInfo {
    /// Parse the unit with the given converter
    pub fn new(text: &str, converter: &Converter) -> Self {
        match converter.get_unit(&text.into()) {
            Ok(unit) => Self::Known(Arc::clone(unit)),
            Err(_) => Self::Unknown,
        }
    }
}

impl<V: QuantityValue> Quantity<V> {
    /// Creates a new quantity
    pub fn new(value: V, unit: Option<String>) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                text,
                info: OnceCell::new(),
            }),
        }
    }

    /// Creates a new quantity and parse the unit
    pub fn new_and_parse(value: V, unit: Option<String>, converter: &Converter) -> Self {
        Self {
            value,
            unit: unit.map(|text| QuantityUnit {
                info: OnceCell::from(UnitInfo::new(&text, converter)),
                text,
            }),
        }
    }

    /// Createa a new quantity with a known unit
    pub(crate) fn with_known_unit(value: V, unit: Arc<Unit>) -> Self {
        Self {
            value,
            unit: Some(QuantityUnit {
                text: unit.to_string(),
                info: OnceCell::from(UnitInfo::Known(unit)),
            }),
        }
    }

    /// Get the unit
    pub fn unit(&self) -> Option<&QuantityUnit> {
        self.unit.as_ref()
    }

    /// Get the unit text
    ///
    /// This is just a shorthand
    /// ```
    /// # use cooklang::quantity::*;
    /// let q = Quantity::new(
    ///             Value::from(1.0),
    ///             Some("unit".into())
    ///         );
    /// assert_eq!(q.unit_text(), q.unit().map(|u| u.text()));
    /// ```
    pub fn unit_text(&self) -> Option<&str> {
        self.unit.as_ref().map(|u| u.text.as_ref())
    }
}

impl ScalableValue {
    pub(crate) fn from_ast(value: ast::QuantityValue) -> Self {
        match value {
            ast::QuantityValue::Single {
                value,
                auto_scale: None,
                ..
            } => Self::Fixed(value.into_inner()),
            ast::QuantityValue::Single {
                value,
                auto_scale: Some(_),
                ..
            } => Self::Linear(value.into_inner()),
            ast::QuantityValue::Many(v) => Self::ByServings(
                v.into_iter()
                    .map(crate::located::Located::into_inner)
                    .collect(),
            ),
        }
    }
}

impl<V: QuantityValue + Display> Display for Quantity<V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)?;
        if let Some(unit) = &self.unit {
            write!(f, " {}", unit)?;
        }
        Ok(())
    }
}

impl Display for ScalableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(value) => value.fmt(f),
            Self::Linear(value) => write!(f, "{value}*"),
            Self::ByServings(values) => {
                for value in &values[..values.len() - 1] {
                    write!(f, "{}|", value)?;
                }
                write!(f, "{}", values.last().unwrap())
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn float(n: f64) -> f64 {
            (n * 1000.0).round() / 1000.0
        }

        match self {
            Value::Number(n) => write!(f, "{}", float(*n)),
            Value::Range(r) => write!(f, "{}-{}", float(*r.start()), float(*r.end())),
            Value::Text(t) => write!(f, "{}", t),
        }
    }
}

impl Display for QuantityUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<RangeInclusive<f64>> for Value {
    fn from(value: RangeInclusive<f64>) -> Self {
        Self::Range(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

/// Error during adding of quantities
#[derive(Debug, Error)]
pub enum QuantityAddError {
    #[error(transparent)]
    IncompatibleUnits(#[from] IncompatibleUnits),

    #[error(transparent)]
    TextValue(#[from] TextValueError),

    #[error(transparent)]
    Convert(#[from] ConvertError),
}

/// Error that makes quantity units incompatible to be added
#[derive(Debug, Error)]
pub enum IncompatibleUnits {
    #[error("Missing unit: one unit is '{found}' but the other quantity is missing an unit")]
    MissingUnit {
        found: either::Either<QuantityUnit, QuantityUnit>,
    },
    #[error("Different physical quantity: '{a}' '{b}'")]
    DifferentPhysicalQuantities {
        a: PhysicalQuantity,
        b: PhysicalQuantity,
    },
    #[error("Unknown units differ: '{a}' '{b}'")]
    UnknownDifferentUnits { a: String, b: String },
}

impl<V: QuantityValue> Quantity<V> {
    /// Checks if two quantities can be added and return the compatible unit
    /// (if any) or an error if they are not
    pub fn compatible_unit(
        &self,
        rhs: &Self,
        converter: &Converter,
    ) -> Result<Option<Arc<Unit>>, IncompatibleUnits> {
        let base = match (&self.unit, &rhs.unit) {
            // No units = ok
            (None, None) => None,
            // Mixed = error
            (None, Some(u)) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Right(u.to_owned()),
                });
            }
            (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: either::Either::Left(u.to_owned()),
                });
            }
            // Units -> check
            (Some(a), Some(b)) => {
                let a_unit = a.unit_info_or_parse(converter);
                let b_unit = b.unit_info_or_parse(converter);

                match (a_unit, b_unit) {
                    (UnitInfo::Known(a_unit), UnitInfo::Known(b_unit)) => {
                        if a_unit.physical_quantity != b_unit.physical_quantity {
                            return Err(IncompatibleUnits::DifferentPhysicalQuantities {
                                a: a_unit.physical_quantity,
                                b: b_unit.physical_quantity,
                            });
                        }
                        // common unit is first one
                        Some(a_unit)
                    }
                    _ => {
                        // if units are unknown, their text must be equal
                        if a.text != b.text {
                            return Err(IncompatibleUnits::UnknownDifferentUnits {
                                a: a.text.clone(),
                                b: b.text.clone(),
                            });
                        }
                        None
                    }
                }
            }
        };
        Ok(base)
    }
}

impl ScaledQuantity {
    /// Try adding two quantities
    pub fn try_add(&self, rhs: &Self, converter: &Converter) -> Result<Self, QuantityAddError> {
        // 1. Check if the units are compatible and (maybe) get a common unit
        let convert_to = self.compatible_unit(rhs, converter)?;

        // 2. Convert rhs to the unit of the first one if needed
        let mut rhs = rhs.clone();
        if let Some(to) = convert_to {
            rhs.convert(&to, converter)?;
        };

        // 3. Sum values
        let value = self.value.try_add(&rhs.value)?;

        // 4. New quantity
        let qty = Quantity {
            value,
            unit: self.unit.clone(), // unit is mantained
        };

        Ok(qty)
    }

    /// Converts the unit to the best possible match in the same unit system.
    ///
    /// For example, `1000 ml` would be converted to `1 l`.
    pub fn fit(&mut self, converter: &Converter) -> Result<(), ConvertError> {
        use crate::convert::ConvertTo;

        // if the unit is known, convert to the best match in the same system
        if matches!(
            self.unit().map(|u| u.unit_info_or_parse(converter)),
            Some(UnitInfo::Known(_))
        ) {
            self.convert(ConvertTo::SameSystem, converter)?;
        }
        Ok(())
    }
}

pub trait TryAdd: Sized {
    type Err;

    fn try_add(&self, rhs: &Self) -> Result<Self, Self::Err>;
}

/// Error when try to operate on a text value
#[derive(Debug, Error, Clone)]
#[error("Cannot operate on a text value")]
pub struct TextValueError(pub Value);

impl TryAdd for Value {
    type Err = TextValueError;

    fn try_add(&self, rhs: &Self) -> Result<Value, TextValueError> {
        let val = match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
            (Value::Number(n), Value::Range(r)) | (Value::Range(r), Value::Number(n)) => {
                Value::Range(r.start() + n..=r.end() + n)
            }
            (Value::Range(a), Value::Range(b)) => {
                Value::Range(a.start() + b.start()..=a.end() + b.end())
            }
            (t @ Value::Text(_), _) | (_, t @ Value::Text(_)) => {
                return Err(TextValueError(t.to_owned()));
            }
        };

        Ok(val)
    }
}

/// Group of quantities
///
/// This support efficient adding of new quantities, merging other groups and
/// calculating the [`TotalQuantity`].
///
/// This is used to create, and merge ingredients lists.
#[derive(Default, Debug, Clone, Serialize)]
pub struct GroupedQuantity {
    /// known units
    known: EnumMap<PhysicalQuantity, Option<ScaledQuantity>>,
    /// unknown units
    unknown: HashMap<String, ScaledQuantity>,
    /// no units
    no_unit: Option<ScaledQuantity>,
    /// could not operate/add to others
    other: Vec<ScaledQuantity>,
}

impl GroupedQuantity {
    /// Create a new empty group
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a new quantity to the group
    pub fn add(&mut self, q: &ScaledQuantity, converter: &Converter) {
        macro_rules! add {
            ($stored:expr, $quantity:ident, $converter:expr, $other:expr) => {
                match $stored.try_add($quantity, $converter) {
                    Ok(q) => *$stored = q,
                    Err(_) => {
                        $other.push($quantity.clone());
                        return;
                    }
                }
            };
        }

        if q.value.is_text() {
            self.other.push(q.clone());
            return;
        }
        if q.unit.is_none() {
            if let Some(stored) = &mut self.no_unit {
                add!(stored, q, converter, self.other);
            } else {
                self.no_unit = Some(q.clone());
            }
            return;
        }

        let unit = q.unit.as_ref().unwrap();
        let info = unit.unit_info_or_parse(converter);
        match info {
            UnitInfo::Known(unit) => {
                if let Some(stored) = &mut self.known[unit.physical_quantity] {
                    add!(stored, q, converter, self.other);
                } else {
                    self.known[unit.physical_quantity] = Some(q.clone());
                }
            }
            UnitInfo::Unknown => {
                if let Some(stored) = self.unknown.get_mut(unit.text()) {
                    add!(stored, q, converter, self.other);
                } else {
                    self.unknown.insert(unit.text.clone(), q.clone());
                }
            }
        };
    }

    /// Merge the group with another one
    pub fn merge(&mut self, other: &Self, converter: &Converter) {
        for q in other.all_quantities() {
            self.add(q, converter)
        }
    }

    fn all_quantities(&self) -> impl Iterator<Item = &ScaledQuantity> + '_ {
        self.known
            .values()
            .filter_map(|q| q.as_ref())
            .chain(self.unknown.values())
            .chain(self.other.iter())
            .chain(self.no_unit.iter())
    }

    /// Calls [`Quantity::fit`] on all possible underlying units
    ///
    /// This will try to avoid fitting quantities that will produce an error
    /// like, for example, a text value. Other conver errors may
    /// occur, for example, if the converter is [`Converter::empty`].
    ///
    /// However, if this errors, you probably can ignore it and use the unfit
    /// value.
    pub fn fit(&mut self, converter: &Converter) -> Result<(), ConvertError> {
        for q in self.known.values_mut().filter_map(|q| q.as_mut()) {
            q.fit(converter)?;
        }
        Ok(())
    }

    /// Get the [`TotalQuantity`]
    ///
    /// Quantities are already
    pub fn total(&self) -> TotalQuantity {
        let mut all = self.all_quantities().cloned().peekable();

        let Some(first) = all.next() else {
            return TotalQuantity::None;
        };

        if all.peek().is_none() {
            TotalQuantity::Single(first)
        } else {
            let mut many = Vec::with_capacity(1 + all.size_hint().0);
            many.push(first);
            for q in all {
                many.push(q);
            }
            TotalQuantity::Many(many)
        }
    }
}

/// Total quantity from a [`GroupedQuantity`]
///
/// [`TotalQuantity::Many`] is needed to avoid loosing information when not all
/// quantities are compatible. This happens when the total cannot be calculated
/// because 2 or more units can't be added. In this case, the vec contains all
/// the quantities added where possible.
///
/// For example:
/// ```
/// # use cooklang::quantity::*;
/// # use cooklang::convert::Converter;
/// # let converter = Converter::bundled();
/// let a = Quantity::new(Value::from(2.0), Some("l".into()));
/// let b = Quantity::new(Value::from(200.0), Some("ml".into()));
/// let c = Quantity::new(Value::from(1.0), Some("bottle".into()));
///
/// let mut group = GroupedQuantity::empty();
/// group.add(&a, &converter);
/// group.add(&b, &converter);
/// group.add(&c, &converter);
/// let total = group.total();
/// assert_eq!(
///     total,
///     TotalQuantity::Many(vec![
///         Quantity::new(Value::from(2.2), Some("l".into())),
///         Quantity::new(Value::from(1.0), Some("bottle".into()))
///     ])
/// );
/// ```
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum TotalQuantity {
    /// No quantity
    None,
    /// A single quantity
    Single(ScaledQuantity),
    /// Many quantities when they can't be added
    Many(Vec<ScaledQuantity>),
}

impl TotalQuantity {
    /// Get the total quantity as a vec of quantities
    ///
    /// - [`TotalQuantity::None`] is an empty vec.
    /// - [`TotalQuantity::Single`] is a vec with one item.
    /// - [`TotalQuantity::Many`] is just it's inner vec.
    pub fn into_vec(self) -> Vec<ScaledQuantity> {
        match self {
            TotalQuantity::None => vec![],
            TotalQuantity::Single(q) => vec![q],
            TotalQuantity::Many(many) => many,
        }
    }
}

impl From<TotalQuantity> for Vec<ScaledQuantity> {
    fn from(value: TotalQuantity) -> Self {
        value.into_vec()
    }
}
