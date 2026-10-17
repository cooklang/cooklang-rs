//! Quantity model

use std::{collections::HashMap, fmt::Display, sync::Arc};

use crate::convert::locales::locale::Locale;
use crate::convert::{units::Unit as V2Unit, Converter};
use enum_map::EnumMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::{
    convert_legacy::{ConvertError, LegacyConverter, PhysicalQuantity, Unit},
    float::equal_f64,
    semantic_eq::{unordered_equals, unordered_equals_legacy, LegacySemanticEq},
};

/// A quantity used in components
///
/// The unit is kept as written and, once the parser has read it through the recipe's locale, as the [`Unit`](V2Unit) it
/// names (ADR 8). Serialised as `value`, `unit`, `unit_id` (the unit's identifier, absent when unknown) and `scalable`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(into = "QuantityRepr", try_from = "QuantityRepr")]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[cfg_attr(
    feature = "ts",
    tsify(
        into_wasm_abi,
        from_wasm_abi,
        type = "{ value: Value; unit?: string; unit_id?: string; scalable: boolean }"
    )
)]
pub struct Quantity {
    pub(crate) value: Value,
    pub(crate) unit: Option<WrittenUnit>,
    pub(crate) scalable: bool,
}

/// As written: the unit text, whether or not it was resolved
impl PartialEq for Quantity {
    fn eq(&self, other: &Self) -> bool {
        // ignore scalable for equality
        self.value == other.value && self.unit() == other.unit()
    }
}

/// A unit as the recipe wrote it, and what it names when the recipe's locale knows that spelling (ADR 8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrittenUnit {
    text: String,
    unit: Option<V2Unit>,
}

impl WrittenUnit {
    /// Not read through any locale: behaves as an unknown unit.
    pub(crate) fn unresolved(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            unit: None,
        }
    }

    pub(crate) fn known(text: impl Into<String>, unit: V2Unit) -> Self {
        Self {
            text: text.into(),
            unit: Some(unit),
        }
    }

    /// `text` as `locale` reads it.
    pub(crate) fn read(text: impl Into<String>, locale: &Locale) -> Self {
        let text = text.into();
        let unit = locale.resolve(&text);
        Self { text, unit }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn unit(&self) -> Option<V2Unit> {
        self.unit
    }
}

/// The serialised form of a [`Quantity`].
#[derive(Serialize, Deserialize)]
struct QuantityRepr {
    value: Value,
    unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    unit_id: Option<String>,
    scalable: bool,
}

impl From<Quantity> for QuantityRepr {
    fn from(q: Quantity) -> Self {
        Self {
            value: q.value,
            unit_id: q
                .unit
                .as_ref()
                .and_then(|u| u.unit)
                .map(|u| u.id().to_string()),
            unit: q.unit.map(|u| u.text),
            scalable: q.scalable,
        }
    }
}

/// Why a serialised quantity could not be read back.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UnitIdError {
    #[error("unit_id '{0}' is given but there is no unit text")]
    WithoutText(String),
    #[error("unit_id '{0}' names no unit")]
    Unknown(String),
}

impl TryFrom<QuantityRepr> for Quantity {
    type Error = UnitIdError;

    fn try_from(repr: QuantityRepr) -> Result<Self, Self::Error> {
        let unit = match (repr.unit, repr.unit_id) {
            (None, None) => None,
            (None, Some(id)) => return Err(UnitIdError::WithoutText(id)),
            (Some(text), None) => Some(WrittenUnit::unresolved(text)),
            (Some(text), Some(id)) => {
                let unit = V2Unit::from_id(&id).ok_or(UnitIdError::Unknown(id))?;
                Some(WrittenUnit::known(text, unit))
            }
        };
        Ok(Self {
            value: repr.value,
            unit,
            scalable: repr.scalable,
        })
    }
}

/// Base value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Value {
    /// Numeric
    Number(Number),
    /// Range
    Range { start: Number, end: Number },
    /// Text
    ///
    /// It is not possible to operate with this variant.
    Text(String),
}

impl Value {
    pub fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }

    /// The same amount expressed in `to` instead of `from`; a range converts end by end. A text value has no amount to
    /// convert.
    pub(crate) fn in_unit(
        &self,
        from: V2Unit,
        to: V2Unit,
    ) -> Result<Value, crate::convert::ConvertError> {
        let convert = |n: &Number| V2Unit::convert(n.value(), from, to).map(Number::Regular);
        Ok(match self {
            Value::Number(n) => Value::Number(convert(n)?),
            Value::Range { start, end } => Value::Range {
                start: convert(start)?,
                end: convert(end)?,
            },
            Value::Text(t) => return Err(crate::convert::ConvertError::TextValue(t.clone())),
        })
    }
}

/// Wrapper for different kinds of numbers
///
/// This type can represent regular numbers and fractions, which are common in
/// cooking recipes, especially when dealing with imperial units.
///
/// The [`Display`] implementation round `f64` to 3 decimal places.
///
/// ```
/// # use cooklang::quantity::Number;
/// let num = Number::Regular(14.0);
/// assert_eq!(num.to_string(), "14");
/// let num = Number::Regular(14.57893);
/// assert_eq!(num.to_string(), "14.579");
/// let num = Number::Fraction { whole: 0, num: 1, den: 2, err: 0.0 };
/// assert_eq!(num.to_string(), "1/2");
/// assert_eq!(num.value(), 0.5);
/// let num = Number::Fraction { whole: 2, num: 1, den: 2, err: 0.001 };
/// assert_eq!(num.to_string(), "2 1/2");
/// assert_eq!(num.value(), 2.501);
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Number {
    /// A regular number
    Regular(f64),
    /// A fractional number
    ///
    /// This is in the form of `[<whole>] <num>/<den>` and the total value is
    /// `whole + err + num / den`.
    ///
    /// `err` exists to allow lossy conversions between a regular number and a
    /// fraction. Use the alternate (`#`) for the [`Display`] impl to include
    /// the error (if any).
    Fraction {
        whole: u32,
        num: u32,
        den: u32,
        err: f64,
    },
}

impl From<Number> for f64 {
    fn from(n: Number) -> Self {
        n.value()
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Regular(value)
    }
}

impl Number {
    /// Get's the true inner value
    ///
    /// The error is included when it's a fraction.
    pub fn value(self) -> f64 {
        match self {
            Number::Regular(v) => v,
            Number::Fraction {
                whole,
                num,
                den,
                err,
            } => whole as f64 + err + num as f64 / den as f64,
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        self.value().eq(&other.value())
    }
}

impl Quantity {
    /// Creates a new quantity, its unit not read through any locale: it behaves as an unknown unit until
    /// [`read_with`](Self::read_with).
    pub fn new(value: Value, unit: Option<String>) -> Self {
        Self {
            value,
            unit: unit.map(WrittenUnit::unresolved),
            scalable: false,
        }
    }

    /// The quantity with its unit text read through `converter`'s locale.
    pub fn read_with(self, converter: &Converter) -> Self {
        self.resolved(converter.locale())
    }

    pub(crate) fn resolved(mut self, locale: &Locale) -> Self {
        self.reread(locale);
        self
    }

    /// Reads the unit text through `locale`, replacing any earlier reading.
    pub(crate) fn reread(&mut self, locale: &Locale) {
        if let Some(unit) = self.unit.take() {
            self.unit = Some(WrittenUnit::read(unit.text, locale));
        }
    }

    /// Returns `true` if the quantity is scalable
    ///
    /// Two quantities are equal even if one is scalable and the other not
    pub fn scalable(&self) -> bool {
        self.scalable
    }

    /// Get the unit, as written
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_ref().map(WrittenUnit::text)
    }

    /// The unit the text names in the recipe's locale, if it was read and known there (ADR 8).
    pub fn unit_id(&self) -> Option<V2Unit> {
        self.unit.as_ref().and_then(WrittenUnit::unit)
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub(crate) fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    /// Get the corresponding [`Unit`]
    ///
    /// This can return `None` if there is no unit or if it's not in the
    /// `converter`.
    pub fn unit_info(&self, converter: &LegacyConverter) -> Option<Arc<Unit>> {
        self.unit().and_then(|u| converter.find_unit(u))
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)?;
        if let Some(unit) = self.unit() {
            f.write_str(" ")?;
            unit.fmt(f)?;
        }
        Ok(())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => n.fmt(f),
            Value::Range { start, end } => write!(f, "{start}-{end}"),
            Value::Text(t) => t.fmt(f),
        }
    }
}

fn round_float(n: f64) -> f64 {
    (n * 1000.0).round() / 1000.0
}

impl Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Number::Regular(n) => write!(f, "{}", round_float(n)),
            Number::Fraction {
                whole,
                num,
                den,
                err,
            } => {
                if self.value() == 0.0 {
                    return write!(f, "{}", 0.0);
                }

                match (whole, num, den) {
                    (0, 0, _) => write!(f, "{}", 0.0),
                    (0, num, den) => write!(f, "{num}/{den}"),
                    (whole, 0, _) => write!(f, "{whole}"),
                    (whole, num, den) => write!(f, "{whole} {num}/{den}"),
                }?;

                if f.alternate() && err.abs() > 0.001 {
                    write!(f, " ({:+})", round_float(err))?;
                }
                Ok(())
            }
        }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(Number::Regular(value))
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

/// Error during an operation between quantities
#[derive(Debug, Error)]
pub enum QuantityOpError {
    #[error(transparent)]
    IncompatibleUnits(#[from] IncompatibleUnits),

    #[error(transparent)]
    TextValue(#[from] TextValueError),

    #[error(transparent)]
    ConvertLegacy(#[from] ConvertError),

    #[error(transparent)]
    Convert(#[from] crate::convert::ConvertError),
}

#[deprecated = "renamed to `QuantityOpError`"]
pub type QuantityAddError = QuantityOpError;

/// Error during an operation on a [`GroupedQuantity`]
#[derive(Debug, Error)]
pub enum GroupedQuantityOpError {
    #[error("No compatible quantity in the group to subtract from")]
    NoCompatibleQuantity,

    #[error(transparent)]
    Op(#[from] QuantityOpError),
}

/// Error that makes quantity units incompatible to be added
#[derive(Debug, Error)]
pub enum IncompatibleUnits {
    #[error("Missing unit: one unit is '{found}' but the other quantity is missing an unit")]
    MissingUnit {
        /// Found unit
        found: String,
        /// Where it has been found
        ///
        /// - `true` if it has been found in the _left_ _(self)_.
        /// - `false` if it was in the _right_ _(other)_.
        lhs: bool,
    },
    #[error("Different physical quantity: '{a}' '{b}'")]
    DifferentPhysicalQuantities {
        a: PhysicalQuantity,
        b: PhysicalQuantity,
    },
    #[error("Unknown units differ: '{a}' '{b}'")]
    UnknownDifferentUnits { a: String, b: String },
}

impl Quantity {
    /// Checks if two quantities can be added and return the compatible unit
    /// (if any) or an error if they are not
    pub fn compatible_unit_legacy(
        &self,
        rhs: &Self,
        converter: &LegacyConverter,
    ) -> Result<Option<Arc<Unit>>, IncompatibleUnits> {
        let base = match (self.unit(), rhs.unit()) {
            // No units = ok
            (None, None) => None,
            // Mixed = error
            (None, Some(u)) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: u.to_string(),
                    lhs: false,
                });
            }
            (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: u.to_string(),
                    lhs: true,
                });
            }
            // Units -> check
            (Some(a), Some(b)) => {
                let a_unit = converter.find_unit(a);
                let b_unit = converter.find_unit(b);

                match (a_unit, b_unit) {
                    (Some(a_unit), Some(b_unit)) => {
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
                        if a != b {
                            return Err(IncompatibleUnits::UnknownDifferentUnits {
                                a: a.to_string(),
                                b: b.to_string(),
                            });
                        }
                        None
                    }
                }
            }
        };
        Ok(base)
    }

    /// Try adding two quantities
    pub fn try_add_legacy(
        &self,
        rhs: &Self,
        converter: &LegacyConverter,
    ) -> Result<Self, QuantityOpError> {
        self.try_op_legacy(rhs, converter, Value::try_add)
    }

    /// Try subtracting two quantities
    ///
    /// Like [`Quantity::try_add`], the units have to be compatible and the
    /// result keeps the unit of `self`, so `1 l` minus `300 ml` is `0.7 l`.
    ///
    /// The result can be negative, callers that don't want that have to check
    /// it. [`GroupedQuantity::try_saturating_sub`] does.
    pub fn try_sub_legacy(
        &self,
        rhs: &Self,
        converter: &LegacyConverter,
    ) -> Result<Self, QuantityOpError> {
        self.try_op_legacy(rhs, converter, Value::try_sub)
    }

    fn try_op_legacy(
        &self,
        rhs: &Self,
        converter: &LegacyConverter,
        op: impl FnOnce(&Value, &Value) -> Result<Value, TextValueError>,
    ) -> Result<Self, QuantityOpError> {
        let convert_to = self.compatible_unit_legacy(rhs, converter)?;

        let mut rhs = rhs.clone();
        if let Some(to) = convert_to {
            rhs.convert(&to, converter)?;
        };

        let value = op(&self.value, &rhs.value)?;

        // the result keeps the left-hand unit, so `1 l` plus `500 ml` is `1.5 l`
        let qty = Quantity {
            value,
            unit: self.unit.clone(),
            scalable: false,
        };

        Ok(qty)
    }

    /// Adds `rhs`, converted into this quantity's unit; the result keeps this quantity's unit text. Both units must
    /// have been read (ADR 8): an unread or unknown unit adds only to the same text.
    pub fn try_add(&self, rhs: &Self) -> Result<Self, QuantityOpError> {
        self.try_op(rhs, Value::try_add)
    }

    /// Subtracts `rhs`, converted into this quantity's unit, as [`try_add`](Self::try_add) adds it.
    ///
    /// The result can be negative, callers that don't want that have to check it.
    /// [`GroupedQuantity::try_saturating_sub`] does.
    pub fn try_sub(&self, rhs: &Self) -> Result<Self, QuantityOpError> {
        self.try_op(rhs, Value::try_sub)
    }

    fn try_op(
        &self,
        rhs: &Self,
        op: impl FnOnce(&Value, &Value) -> Result<Value, TextValueError>,
    ) -> Result<Self, QuantityOpError> {
        let rhs_value = match self.compatible_unit(rhs)? {
            Some(to) => {
                let from = rhs.unit_id().expect("compatible_unit read both units");
                rhs.value.in_unit(from, to)?
            }
            None => rhs.value.clone(),
        };
        let value = op(&self.value, &rhs_value)?;
        Ok(Quantity {
            value,
            unit: self.unit.clone(),
            scalable: false,
        })
    }

    /// The unit `rhs` must be converted into to be added to `self`, if any, or why the two cannot be added.
    pub(crate) fn compatible_unit(&self, rhs: &Self) -> Result<Option<V2Unit>, IncompatibleUnits> {
        match (self.unit(), rhs.unit()) {
            (None, None) => Ok(None),
            (None, Some(u)) => Err(IncompatibleUnits::MissingUnit {
                found: u.to_string(),
                lhs: false,
            }),
            (Some(u), None) => Err(IncompatibleUnits::MissingUnit {
                found: u.to_string(),
                lhs: true,
            }),
            (Some(a), Some(b)) => match (self.unit_id(), rhs.unit_id()) {
                (Some(a_unit), Some(b_unit)) => {
                    if a_unit.physical_quantity() != b_unit.physical_quantity() {
                        return Err(IncompatibleUnits::DifferentPhysicalQuantities {
                            a: a_unit.physical_quantity().into(),
                            b: b_unit.physical_quantity().into(),
                        });
                    }
                    Ok(Some(a_unit))
                }
                // if units are unknown, their text must be equal
                _ if a != b => Err(IncompatibleUnits::UnknownDifferentUnits {
                    a: a.to_string(),
                    b: b.to_string(),
                }),
                _ => Ok(None),
            },
        }
    }

    /// Where [`GroupedQuantity::add`] puts this quantity.
    fn slot(&self) -> Slot<'_> {
        match (self.unit(), self.unit_id()) {
            (Some(_), Some(unit)) => Slot::Known(unit.physical_quantity().into()),
            (Some(text), None) => Slot::Unknown(text),
            (None, _) => Slot::NoUnit,
        }
    }
}

/// Compares two quantities that may be written with different units
///
/// Unlike [`PartialEq`], which compares the quantities as written, both are
/// converted to the metric base unit of their family, so `3 dl` equals
/// `300 ml` and, across systems, `1 cup` equals `236.59 ml`.
///
/// The values are then compared with a small absolute tolerance, so the
/// error the conversion itself leaves behind does not make two equal
/// amounts differ.
///
/// Quantities that can't be converted, like text values, quantities without
/// a unit or with a unit unknown to the `converter`, are compared as
/// written.
impl LegacySemanticEq for Quantity {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        // both go to the metric base, otherwise each one would go to the base
        // unit of its own system and the two would never be comparable
        let base = |q: &Quantity| {
            let mut q = q.clone();
            q.convert_to_base_unit(converter).ok()?;
            Some(q)
        };

        match (base(self), base(other)) {
            (Some(a), Some(b)) => a.unit() == b.unit() && value_equals(&a.value, &b.value),
            // at least one of them can't be converted, compare them as written
            _ => self.unit() == other.unit() && value_equals(&self.value, &other.value),
        }
    }
}

/// Both quantities go to the smallest metric unit of their family and are compared there; what cannot be converted, a
/// text value or a unit that was not read or not known (ADR 8), is compared as written.
impl crate::semantic_eq::SemanticEq for Quantity {
    fn equals(&self, other: &Self) -> bool {
        let base = |q: &Quantity| {
            let from = q.unit_id()?;
            let to = V2Unit::metric_base(from.physical_quantity());
            Some((to, q.value.in_unit(from, to).ok()?))
        };
        match (base(self), base(other)) {
            (Some((ua, a)), Some((ub, b))) => ua == ub && value_equals(&a, &b),
            _ => self.unit() == other.unit() && value_equals(&self.value, &other.value),
        }
    }
}

impl Quantity {
    /// [`SemanticEq::equals`], for the grouped operations.
    pub(crate) fn equals(&self, other: &Self) -> bool {
        crate::semantic_eq::SemanticEq::equals(self, other)
    }
}

fn value_equals(a: &Value, b: &Value) -> bool {
    let eq = |a: Number, b: Number| equal_f64(a.value(), b.value());
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => eq(*a, *b),
        (Value::Range { start: sa, end: ea }, Value::Range { start: sb, end: eb }) => {
            eq(*sa, *sb) && eq(*ea, *eb)
        }
        (Value::Text(a), Value::Text(b)) => a == b,
        _ => false,
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
            (Value::Number(a), Value::Number(b)) => Value::Number((a.value() + b.value()).into()),
            (Value::Number(n), Value::Range { start, end })
            | (Value::Range { start, end }, Value::Number(n)) => Value::Range {
                start: (start.value() + n.value()).into(),
                end: (end.value() + n.value()).into(),
            },
            (Value::Range { start: s1, end: e1 }, Value::Range { start: s2, end: e2 }) => {
                Value::Range {
                    start: (s1.value() + s2.value()).into(),
                    end: (e1.value() + e2.value()).into(),
                }
            }
            (t @ Value::Text(_), _) | (_, t @ Value::Text(_)) => {
                return Err(TextValueError(t.to_owned()));
            }
        };

        Ok(val)
    }
}

pub trait TrySub: Sized {
    type Err;

    fn try_sub(&self, rhs: &Self) -> Result<Self, Self::Err>;
}

impl TrySub for Value {
    type Err = TextValueError;

    /// Subtracts the ranges as intervals, so the smallest possible result is
    /// the start and the largest the end
    fn try_sub(&self, rhs: &Self) -> Result<Value, TextValueError> {
        let val = match (self, rhs) {
            (Value::Number(a), Value::Number(b)) => Value::Number((a.value() - b.value()).into()),
            (Value::Range { start, end }, Value::Number(n)) => Value::Range {
                start: (start.value() - n.value()).into(),
                end: (end.value() - n.value()).into(),
            },
            (Value::Number(n), Value::Range { start, end }) => Value::Range {
                start: (n.value() - end.value()).into(),
                end: (n.value() - start.value()).into(),
            },
            (Value::Range { start: s1, end: e1 }, Value::Range { start: s2, end: e2 }) => {
                Value::Range {
                    start: (s1.value() - e2.value()).into(),
                    end: (e1.value() - s2.value()).into(),
                }
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
/// This support efficient adding of new quantities, merging other groups..
///
/// This is used to create, and merge ingredients lists.
///
/// This can return many quantities to avoid loosing information when not all
/// quantities are compatible. If a single total can be calculated, it will be
/// single quantity. If the total cannot be calculated because 2 or more units
/// can't be added, it contains all the quantities added where possible.
///
/// The display impl is a comma separated list of all the quantities.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[cfg_attr(feature = "ts", tsify(into_wasm_abi, from_wasm_abi))]
pub struct GroupedQuantity {
    /// known units
    known: EnumMap<PhysicalQuantity, Option<Quantity>>,
    /// unknown units
    unknown: HashMap<String, Quantity>,
    /// no units
    no_unit: Option<Quantity>,
    /// could not operate/add to others
    other: Vec<Quantity>,
}

/// Where a quantity is stored inside a [`GroupedQuantity`]
#[derive(Clone, Copy)]
enum Slot<'a> {
    Known(PhysicalQuantity),
    Unknown(&'a str),
    NoUnit,
}

/// Clamps a range's negative start back to zero, returning `true` if nothing
/// is left of the value
///
/// Only a value that landed at or below zero is caught here. A subtraction that should
/// have reached zero but stopped just short of it is caught by comparing the
/// amounts before subtracting.
fn clamp_remaining(value: &mut Value) -> bool {
    match value {
        Value::Number(n) => n.value() <= 0.0,
        Value::Range { start, end } => {
            if end.value() <= 0.0 {
                return true;
            }
            if start.value() < 0.0 {
                *start = 0.0.into();
            }
            false
        }
        Value::Text(_) => false,
    }
}

impl GroupedQuantity {
    /// Create a new empty group
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a new quantity to the group
    pub fn add_legacy(&mut self, q: &Quantity, converter: &LegacyConverter) {
        macro_rules! add {
            ($stored:expr, $quantity:ident, $converter:expr, $other:expr) => {
                match $stored.try_add_legacy($quantity, $converter) {
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

        let unit_text = q.unit().unwrap();
        let info = q.unit_info(converter);
        match info {
            Some(unit) => {
                if let Some(stored) = &mut self.known[unit.physical_quantity] {
                    add!(stored, q, converter, self.other);
                } else {
                    self.known[unit.physical_quantity] = Some(q.clone());
                }
            }
            None => {
                if let Some(stored) = self.unknown.get_mut(unit_text) {
                    add!(stored, q, converter, self.other);
                } else {
                    self.unknown.insert(unit_text.to_string(), q.clone());
                }
            }
        };
    }

    /// Adds a quantity to the group: to the one of its physical quantity when its unit was read (ADR 8), to the one
    /// with the same text when it was not, to the unitless one when it has none; a text value, or one that cannot be
    /// added, is kept apart.
    pub fn add(&mut self, q: &Quantity) {
        if q.value.is_text() {
            self.other.push(q.clone());
            return;
        }
        let slot = q.slot();
        let stored = match slot {
            Slot::Known(physical_quantity) => &mut self.known[physical_quantity],
            Slot::Unknown(unit_text) => {
                if !self.unknown.contains_key(unit_text) {
                    self.unknown.insert(unit_text.to_string(), q.clone());
                    return;
                }
                match self.unknown.get(unit_text).unwrap().try_add(q) {
                    Ok(sum) => {
                        self.unknown.insert(unit_text.to_string(), sum);
                    }
                    Err(_) => self.other.push(q.clone()),
                }
                return;
            }
            Slot::NoUnit => &mut self.no_unit,
        };
        match stored {
            None => *stored = Some(q.clone()),
            Some(existing) => match existing.try_add(q) {
                Ok(sum) => *existing = sum,
                Err(_) => self.other.push(q.clone()),
            },
        }
    }

    /// Subtract a quantity from the group, saturating at zero
    ///
    /// The quantity is subtracted from the one it would have been added to by [`add`](Self::add); if nothing is there
    /// the group is unchanged and the error says so. Subtracting an amount that [equals](crate::SemanticEq::equals)
    /// what is stored leaves nothing, so `9 tsp` empties a group of `3 tbsp` instead of leaving the residue of
    /// converting between them.
    pub fn try_saturating_sub(&mut self, q: &Quantity) -> Result<(), GroupedQuantityOpError> {
        if q.value.is_text() {
            let at = self
                .other
                .iter()
                .position(|stored| stored == q)
                .ok_or(GroupedQuantityOpError::NoCompatibleQuantity)?;
            self.other.remove(at);
            return Ok(());
        }

        let slot = q.slot();
        let stored = match slot {
            Slot::Known(physical_quantity) => self.known[physical_quantity].as_ref(),
            Slot::Unknown(unit_text) => self.unknown.get(unit_text),
            Slot::NoUnit => self.no_unit.as_ref(),
        };
        let stored = stored.ok_or(GroupedQuantityOpError::NoCompatibleQuantity)?;

        // the same amount written another way must leave a clean zero, not
        // the residue of converting between units, so it is compared first
        let took_all = stored.equals(q);
        let mut remaining = stored.try_sub(q)?;
        let empty = took_all || clamp_remaining(remaining.value_mut());

        match slot {
            Slot::Known(physical_quantity) => {
                self.known[physical_quantity] = (!empty).then_some(remaining)
            }
            Slot::Unknown(unit_text) => {
                if empty {
                    self.unknown.remove(unit_text);
                } else {
                    self.unknown.insert(unit_text.to_string(), remaining);
                }
            }
            Slot::NoUnit => self.no_unit = (!empty).then_some(remaining),
        }
        Ok(())
    }

    /// Merge the group with another one
    pub fn merge(&mut self, other: &Self) {
        for q in other.iter() {
            self.add(q)
        }
    }

    /// Subtracts every quantity of `other` that could be added to this group, or none of them: the group is unchanged
    /// when any one fails. The quantities of `other` that could not be added to any other one are skipped, since they
    /// are stored apart from the unit they failed to merge into and subtracting them would subtract that amount twice.
    pub fn try_saturating_sub_group(&mut self, other: &Self) -> Result<(), GroupedQuantityOpError> {
        let mut taken = self.clone();
        for q in other.mergeable() {
            taken.try_saturating_sub(q)?;
        }
        *self = taken;
        Ok(())
    }

    /// Writes every quantity with a known unit as `converter`'s readers would write it. A quantity that cannot be
    /// fitted is left as it is.
    pub fn fit(&mut self, converter: &Converter) {
        for q in self.known.values_mut().filter_map(|q| q.as_mut()) {
            if let Ok(fitted) = converter.fit(q) {
                *q = fitted;
            }
        }
    }

    /// Subtract a quantity from the group, saturating at zero
    ///
    /// The quantity is subtracted from the one it would have been added to by
    /// [`GroupedQuantity::add`], converting units when needed, so subtracting
    /// `300 ml` from a group with `1 l` leaves `0.7 l`.
    ///
    /// A group holds an amount needed, so nothing can be taken twice: unlike
    /// [`Quantity::try_sub`], the result saturates at zero and the quantity is
    /// removed from the group instead of going negative. A range is only
    /// removed when its end reaches zero, a negative start is clamped.
    ///
    /// A text value has no amount to subtract, so subtracting one removes the
    /// equal text quantity from the group, if it holds one.
    ///
    /// Returns an error, leaving the group untouched, when the group has no
    /// quantity to subtract from and when the units are incompatible. Numeric
    /// quantities that could not be added to any other are never subtracted
    /// from.
    ///
    /// Subtracting an amount that [equals](LegacySemanticEq::equals_legacy) what is stored leaves
    /// nothing, so `9 tsp` empties a group of `3 tbsp` instead of leaving the
    /// residue of converting between them.
    pub fn try_saturating_sub_legacy(
        &mut self,
        q: &Quantity,
        converter: &LegacyConverter,
    ) -> Result<(), GroupedQuantityOpError> {
        // a text value has no amount to subtract, so the only thing it can
        // take out of the group is itself, which `add` keeps in `other`
        if q.value.is_text() {
            let at = self
                .other
                .iter()
                .position(|stored| stored == q)
                .ok_or(GroupedQuantityOpError::NoCompatibleQuantity)?;
            self.other.remove(at);
            return Ok(());
        }

        // where `add` would have put it
        let slot = match q.unit() {
            Some(unit_text) => match q.unit_info(converter) {
                Some(unit) => Slot::Known(unit.physical_quantity),
                None => Slot::Unknown(unit_text),
            },
            None => Slot::NoUnit,
        };

        let stored = match slot {
            Slot::Known(physical_quantity) => self.known[physical_quantity].as_ref(),
            Slot::Unknown(unit_text) => self.unknown.get(unit_text),
            Slot::NoUnit => self.no_unit.as_ref(),
        };
        let stored = stored.ok_or(GroupedQuantityOpError::NoCompatibleQuantity)?;

        // converting between units goes through f64 ratios, so taking the same
        // amount written another way leaves residue rather than a clean zero
        let took_all = stored.equals_legacy(q, converter);
        let mut remaining = stored.try_sub_legacy(q, converter)?;
        let empty = took_all || clamp_remaining(remaining.value_mut());

        match slot {
            Slot::Known(physical_quantity) => {
                self.known[physical_quantity] = (!empty).then_some(remaining)
            }
            Slot::Unknown(unit_text) => {
                if empty {
                    self.unknown.remove(unit_text);
                } else {
                    self.unknown.insert(unit_text.to_string(), remaining);
                }
            }
            Slot::NoUnit => self.no_unit = (!empty).then_some(remaining),
        }

        Ok(())
    }

    /// Merge the group with another one
    pub fn merge_legacy(&mut self, other: &Self, converter: &LegacyConverter) {
        for q in other.iter() {
            self.add_legacy(q, converter)
        }
    }

    /// Subtract another group from this one, saturating at zero
    ///
    /// Every quantity of `other` is subtracted with
    /// [`GroupedQuantity::try_saturating_sub`], all of them or none: the first
    /// one that can't be subtracted returns its error, leaving the group
    /// untouched.
    ///
    /// The quantities of `other` that could not be added to any other one are
    /// skipped. They are stored apart from the unit they failed to merge into,
    /// so subtracting them would subtract that amount twice.
    pub fn try_saturating_sub_group_legacy(
        &mut self,
        other: &Self,
        converter: &LegacyConverter,
    ) -> Result<(), GroupedQuantityOpError> {
        let mut taken = self.clone();
        for q in other.mergeable() {
            taken.try_saturating_sub_legacy(q, converter)?;
        }
        *self = taken;
        Ok(())
    }

    /// The quantities that were merged into a unit slot, in contrast to the
    /// ones kept apart because they could not be added to any other
    fn mergeable(&self) -> impl Iterator<Item = &Quantity> {
        self.known
            .values()
            .filter_map(|q| q.as_ref())
            .chain(self.unknown.values())
            .chain(self.no_unit.iter())
    }

    /// Calls [`Quantity::fit_legacy`] on all possible underlying units
    ///
    /// This will try to avoid fitting quantities that will produce an error
    /// like, for example, a text value. Other conver errors may
    /// occur, for example, if the converter is [`LegacyConverter::empty`].
    ///
    /// However, if this errors, you probably can ignore it and use the unfit
    /// value.
    pub fn fit_legacy(&mut self, converter: &LegacyConverter) -> Result<(), ConvertError> {
        for q in self.known.values_mut().filter_map(|q| q.as_mut()) {
            q.fit_legacy(converter)?;
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Quantity> {
        self.known
            .values()
            .filter_map(|q| q.as_ref())
            .chain(self.unknown.values())
            .chain(self.other.iter())
            .chain(self.no_unit.iter())
    }

    pub fn len(&self) -> usize {
        self.known.values().filter(|q| q.is_some()).count()
            + self.unknown.len()
            + self.other.len()
            + (self.no_unit.is_some() as usize)
    }

    /// Turn the group into a single vec
    pub fn into_vec(self) -> Vec<Quantity> {
        let len = self.len();
        let mut v = Vec::with_capacity(len);
        for q in self
            .known
            .into_values()
            .flatten()
            .chain(self.unknown.into_values())
            .chain(self.other.into_iter())
            .chain(self.no_unit.into_iter())
        {
            v.push(q)
        }
        debug_assert_eq!(len, v.len(), "misscalculated groupedquantity len");
        v
    }
}

/// Compares two groups quantity by quantity with the [`LegacySemanticEq`] impl of
/// [`Quantity`]
///
/// So groups with the same quantities written with different units, like
/// `3 dl` and `300 ml`, are equal.
///
/// Note that the groups are compared as they are, not as they would be
/// after adding all their quantities together: a group with `1 l` and
/// `1 bunch` is not equal to one with `500 ml`, `500 ml` and `1 bunch`,
/// because the two volumes are stored separately in the second one only if
/// they could not be added.
impl LegacySemanticEq for GroupedQuantity {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        // `known` is an EnumMap, both always hold the same slots
        self.no_unit.equals_legacy(&other.no_unit, converter)
            && self
                .known
                .values()
                .zip(other.known.values())
                .all(|(a, b)| a.equals_legacy(b, converter))
            && self.unknown.len() == other.unknown.len()
            && self.unknown.iter().all(|(unit, a)| {
                other
                    .unknown
                    .get(unit)
                    .is_some_and(|b| a.equals_legacy(b, converter))
            })
            // the order of the quantities that could not be added carries no
            // meaning, so they are compared as a set
            && unordered_equals_legacy(&self.other, &other.other, converter)
    }
}

impl crate::semantic_eq::SemanticEq for GroupedQuantity {
    fn equals(&self, other: &Self) -> bool {
        crate::semantic_eq::SemanticEq::equals(&self.no_unit, &other.no_unit)
            && self
                .known
                .values()
                .zip(other.known.values())
                .all(|(a, b)| crate::semantic_eq::SemanticEq::equals(a, b))
            && self.unknown.len() == other.unknown.len()
            && self.unknown.iter().all(|(unit, a)| {
                other
                    .unknown
                    .get(unit)
                    .is_some_and(|b| crate::semantic_eq::SemanticEq::equals(a, b))
            })
            && unordered_equals(&self.other, &other.other)
    }
}

impl Display for GroupedQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        display_comma_separated(f, self.iter())
    }
}

fn display_comma_separated<T>(
    f: &mut impl std::fmt::Write,
    mut iter: impl Iterator<Item = T>,
) -> std::fmt::Result
where
    T: Display,
{
    match iter.next() {
        Some(first) => write!(f, "{first}")?,
        None => return Ok(()),
    }
    for q in iter {
        write!(f, ", {q}")?;
    }
    Ok(())
}

// All the fractions stuff

static TABLE: std::sync::LazyLock<FractionLookupTable> =
    std::sync::LazyLock::new(FractionLookupTable::new);

#[derive(Debug)]
struct FractionLookupTable(Vec<(i16, (u8, u8))>);

impl FractionLookupTable {
    const FIX_RATIO: f64 = 1e4;
    const DENOMS: &'static [u8] = &[2, 3, 4, 8, 10, 16];

    pub fn new() -> Self {
        #[allow(clippy::const_is_empty)]
        {
            // I really want to be sure clippy
            debug_assert!(!Self::DENOMS.is_empty());
        }
        debug_assert!(Self::DENOMS.windows(2).all(|w| w[0] < w[1]));
        let mut table = Vec::new();

        for &den in Self::DENOMS {
            for num in 1..den {
                // not include 1
                let val = num as f64 / den as f64;

                // convert to fixed decimal
                let fixed = (val * Self::FIX_RATIO) as i16;

                // only insert if not already in
                //
                // Because we are iterating from low to high denom, then the value
                // will only be present with the smallest possible denom.
                if let Err(pos) = table.binary_search_by_key(&fixed, |&(x, _)| x) {
                    table.insert(pos, (fixed, (num, den)));
                }
            }
        }

        table.shrink_to_fit();

        Self(table)
    }

    pub fn lookup(&self, val: f64, max_den: u8) -> Option<(u8, u8)> {
        let fixed = (val * Self::FIX_RATIO) as i16;
        let t = self.0.as_slice();
        let pos = t.binary_search_by_key(&fixed, |&(x, _)| x);

        let found = pos.is_ok_and(|i| {
            let (x, (_, d)) = t[i];
            x == fixed && d <= max_den
        });
        if found {
            return Some(t[pos.unwrap()].1);
        }

        let pos = pos.unwrap_or_else(|i| i);

        let high = t[pos..].iter().find(|(_, (_, d))| *d <= max_den).copied();
        let low = t[..pos].iter().rfind(|(_, (_, d))| *d <= max_den).copied();

        match (low, high) {
            (None, Some((_, f))) | (Some((_, f)), None) => Some(f),
            (Some((a_val, a)), Some((b_val, b))) => {
                let a_err = (a_val - fixed).abs();
                let b_err = (b_val - fixed).abs();
                if a_err.cmp(&b_err).then(a.1.cmp(&b.1)).is_le() {
                    Some(a)
                } else {
                    Some(b)
                }
            }
            (None, None) => None,
        }
    }
}

impl Number {
    /// The largest denominator [`new_approx`](Self::new_approx) writes. A preference row's `fractions` must not exceed
    /// it; `build.rs` checks.
    pub const MAX_FRACTION_DENOMINATOR: u8 = 64;

    /// Tries to create a new approximate number within a margin of error.
    ///
    /// It returns none if:
    /// - The value is an integer
    /// - It can't be represented with the given restrictions as a fraction.
    /// - The number is not positive.
    ///
    /// It will return `Number::Regular` when the number is an integer with less than a 1e-10 margin of error.
    ///
    /// Otherwise it will return a `Number::Fraction`. `num` can be 0 if the value is rounded to an integer.
    ///
    /// `accuracy` is a value between 0 and 1 representing the error percent.
    ///
    /// `max_den` is the maximum denominator. The denominator is one a list of "common" fractions: 2, 3, 4, 5, 8, 10,
    /// 16, 32, 64. [`MAX_FRACTION_DENOMINATOR`](Self::MAX_FRACTION_DENOMINATOR) is the max.
    ///
    /// `max_whole` determines the maximum value of the integer. Setting this to 0 only allows fractions < 1. Exact
    /// values higher than this are also rejected.
    ///
    /// # Panics
    /// - If `accuracy > 1` or `accuracy < 0`.
    /// - If `max_den > MAX_FRACTION_DENOMINATOR`
    pub fn new_approx(value: f64, accuracy: f32, max_den: u8, max_whole: u32) -> Option<Self> {
        assert!((0.0..=1.0).contains(&accuracy));
        assert!(max_den <= Self::MAX_FRACTION_DENOMINATOR);
        if value <= 0.0 || !value.is_finite() {
            return None;
        }

        let max_err = accuracy as f64 * value;

        let whole = value.trunc() as u32;
        let decimal = value.fract();

        if whole > max_whole || whole == u32::MAX {
            return None;
        }

        if decimal < 1e-10 {
            return Some(Self::Regular(value));
        }

        let rounded = value.round() as u32;
        let round_err = value - value.round();
        if round_err.abs() < max_err && rounded > 0 && rounded <= max_whole {
            return Some(Self::Fraction {
                whole: rounded,
                num: 0,
                den: 1,
                err: round_err,
            });
        }

        let (num, den) = TABLE.lookup(decimal, max_den)?;
        let approx_value = whole as f64 + num as f64 / den as f64;
        let err = value - approx_value;
        if err.abs() > max_err {
            return None;
        }
        Some(Self::Fraction {
            whole,
            num: num as u32,
            den: den as u32,
            err,
        })
    }

    /// Tries to approximate the number to a fraction if possible and not an
    /// integer
    pub fn try_approx(&mut self, accuracy: f32, max_den: u8, max_whole: u32) -> bool {
        match Self::new_approx(self.value(), accuracy, max_den, max_whole) {
            Some(f) => {
                *self = f;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::LazyLock;
    use test_case::test_case;

    static CONVERTER: LazyLock<LegacyConverter> = LazyLock::new(LegacyConverter::bundled);

    macro_rules! frac {
        ($whole:expr) => {
            frac!($whole, 0, 1)
        };
        ($num:expr, $den:expr) => {
            frac!(0, $num, $den)
        };
        ($whole:expr, $num:expr, $den:expr) => {
            Some(Number::Fraction {
                whole: $whole,
                num: $num,
                den: $den,
                ..
            })
        };
    }

    #[test_case(1.0 => matches Some(Number::Regular(v)) if v == 1.0 ; "exact")]
    #[test_case(1.00000000001 => matches Some(Number::Regular(v)) if 1.0 - v < 1e-10 && v > 1.0 ; "exactish")]
    #[test_case(0.01 => None ; "no approx 0")]
    #[test_case(1.9999 => matches frac!(2) ; "round up")]
    #[test_case(1.0001 => matches frac!(1) ; "round down")]
    #[test_case(400.0001 => matches frac!(400) ; "not wrong round up")]
    #[test_case(399.9999 => matches frac!(400) ; "not wrong round down")]
    #[test_case(1.5 => matches frac!(1, 1, 2) ; "trivial frac")]
    #[test_case(0.2501 => matches frac!(1, 4) ; "frac with err")]
    fn fractions(value: f64) -> Option<Number> {
        let num = Number::new_approx(value, 0.05, 4, u32::MAX);
        if let Some(num) = num {
            assert!((num.value() - value).abs() < 10e-9);
        }
        num
    }

    fn qty(q: &str) -> Quantity {
        let (value, unit) = match q.split_once("%") {
            Some((value, unit)) => (value, Some(unit.to_string())),
            None => (q, None),
        };

        let number = |s: &str| s.parse::<f64>().ok().map(Number::from);
        let range = || {
            let (start, end) = value.split_once("-")?;
            Some(Value::Range {
                start: number(start)?,
                end: number(end)?,
            })
        };

        let value = range()
            .or_else(|| number(value).map(Value::Number))
            .unwrap_or_else(|| Value::Text(value.to_string()));

        Quantity::new(value, unit)
    }
    fn group(quantities: &[&str]) -> GroupedQuantity {
        let mut g = GroupedQuantity::empty();
        for q in quantities {
            g.add_legacy(&qty(q), &CONVERTER);
        }
        g
    }

    /// The operations through the legacy locale do what the `_legacy` ones do (ADR 9). Results are compared with the
    /// old converter, which only reads them.
    mod v2 {
        use super::*;
        use crate::Converter;

        /// Read through the legacy locale, as the parser would read it.
        fn qty(q: &str) -> Quantity {
            super::qty(q).read_with(&Converter::legacy())
        }

        fn group(quantities: &[&str]) -> GroupedQuantity {
            let mut g = GroupedQuantity::empty();
            for q in quantities {
                g.add(&qty(q));
            }
            g
        }

        #[test]
        fn add_quantities() {
            let eq = |a: Quantity, b: &str| a.equals_legacy(&qty(b), &CONVERTER);
            let add = |a: &str, b: &str| qty(a).try_add(&qty(b)).unwrap();
            let add_err = |a: &str, b: &str| qty(a).try_add(&qty(b)).is_err();

            // rhs is converted, the unit of lhs is kept
            assert!(eq(add("1%l", "500%ml"), "1.5%l"));
            assert_eq!(add("1%l", "500%ml").unit(), Some("l"));
            assert!(eq(add("1%cup", "2%tbsp"), "1.125%cup"));
            assert!(eq(add("2%bunch", "1%bunch"), "3%bunch"));
            assert!(eq(add("2", "3"), "5"));
            assert!(eq(add("1-2%l", "500%ml"), "1.5-2.5%l"));

            assert!(add_err("1%l", "1%kg"));
            assert!(add_err("1%l", "1%bunch"));
            assert!(add_err("1%l", "1"));
            assert!(add_err("some", "1"));
        }

        #[test]
        fn subtract_quantities() {
            let eq = |a: Quantity, b: &str| a.equals_legacy(&qty(b), &CONVERTER);
            let sub = |a: &str, b: &str| qty(a).try_sub(&qty(b)).unwrap();
            let sub_err = |a: &str, b: &str| qty(a).try_sub(&qty(b)).is_err();

            assert!(eq(sub("1%l", "300%ml"), "0.7%l"));
            assert!(eq(sub("1%kg", "500%g"), "500%g"));
            assert!(eq(sub("2%bunch", "1%bunch"), "1%bunch"));
            assert!(eq(sub("2", "3"), "-1"));
            assert!(eq(sub("100%g", "1%kg"), "-900%g"));
            assert!(eq(sub("1-2%l", "500%ml"), "0.5-1.5%l"));
            assert!(eq(sub("2%l", "0.5-1%l"), "1-1.5%l"));

            assert!(sub_err("1%l", "1%kg"));
            assert!(sub_err("1%l", "1%bunch"));
            assert!(sub_err("1%l", "1"));
            assert!(sub_err("some", "1"));
            assert!(sub_err("1", "some"));
        }

        #[test]
        fn grouping_slots_by_what_a_unit_measures() {
            let g = group(&[
                "1%l", "500%ml", "1%kg", "2%bunch", "1%bunch", "3", "some", "1%cup",
            ]);
            let mut shown: Vec<String> = g.iter().map(|q| q.to_string()).collect();
            shown.sort();
            assert_eq!(shown, ["1 kg", "1.737 l", "3", "3 bunch", "some"]);
        }

        #[test]
        fn subtracting_the_same_amount_written_differently_empties_the_group() {
            let mut g = group(&["3%tbsp"]);
            g.try_saturating_sub(&qty("9%tsp")).unwrap();
            assert!(
                g.is_empty(),
                "left over: {:?}",
                g.iter().collect::<Vec<_>>()
            );

            let mut g = group(&["3%tbsp"]);
            g.try_saturating_sub(&qty("3%tsp")).unwrap();
            assert!(!g.is_empty());
        }

        #[test]
        fn saturating_sub_from_grouped_quantities() {
            let sub = |mut a: GroupedQuantity, b: Quantity| {
                a.try_saturating_sub(&b).unwrap();
                a
            };
            let sub_err = |mut a: GroupedQuantity, b: Quantity| a.try_saturating_sub(&b).is_err();
            let eq = |a: GroupedQuantity, b: GroupedQuantity| a.equals_legacy(&b, &CONVERTER);

            assert!(eq(
                sub(group(&["1%l", "1%kg", "2%bunch", "3"]), qty("500%ml")),
                group(&["0.5%l", "1%kg", "2%bunch", "3"])
            ));
            assert!(sub(group(&["1%l"]), qty("1000%ml")).is_empty());
            assert!(sub(group(&["1%l"]), qty("2%l")).is_empty());
            assert!(eq(
                sub(group(&["1-2%l"]), qty("1.5%l")),
                group(&["0-0.5%l"])
            ));
            assert!(sub_err(group(&["1%l"]), qty("1%kg")));
            assert!(sub_err(group(&["1%l"]), qty("1")));

            // a group subtracted skips its unmergeable quantities
            let mut other = group(&["2%tbsp"]);
            other.add(&qty("some"));
            let mut g = group(&["3%tbsp"]);
            g.try_saturating_sub_group(&other).unwrap();
            assert_eq!(
                g.iter().map(|q| q.to_string()).collect::<Vec<_>>(),
                ["1 tbsp"]
            );
        }

        #[test]
        fn fitting_a_group_fits_each_known_quantity() {
            let v2 = Converter::legacy();
            let mut g = group(&["1000%ml", "1500%g", "2%bunch"]);
            g.fit(&v2);
            let mut shown: Vec<String> = g.iter().map(|q| q.to_string()).collect();
            shown.sort();
            assert_eq!(shown, ["1 l", "1.5 kg", "2 bunch"]);
        }
    }

    /// ADR 8: the unit as written and the unit it names travel together.
    mod written_unit {
        use super::*;
        use crate::convert::LocaleTag;
        use crate::Converter;

        fn swedish() -> Converter {
            Converter::for_tag(&LocaleTag::parse("sv-SE").unwrap()).unwrap()
        }

        fn msk() -> Quantity {
            Quantity::new(Value::from(2.0), Some("msk".into()))
        }

        #[test]
        fn reading_resolves_the_text_through_the_locale() {
            assert_eq!(msk().unit_id(), None);
            let read = msk().read_with(&swedish());
            assert_eq!(read.unit(), Some("msk"));
            assert_eq!(read.unit_id(), V2Unit::from_id("metric tablespoon"));
            // a spelling the locale does not know stays unknown
            let unknown =
                Quantity::new(Value::from(1.0), Some("tbsp".into())).read_with(&swedish());
            assert_eq!(unknown.unit_id(), None);
        }

        #[test]
        fn a_read_quantity_serialises_its_unit_id_and_reads_it_back() {
            let q = msk().read_with(&swedish());
            let json = serde_json::to_value(&q).unwrap();
            assert_eq!(json["unit"], "msk");
            assert_eq!(json["unit_id"], "metric tablespoon");
            let back: Quantity = serde_json::from_value(json).unwrap();
            assert_eq!(back.unit_id(), q.unit_id());
            assert_eq!(back, q);
        }

        #[test]
        fn an_unread_unit_has_no_unit_id_on_the_wire() {
            let json = serde_json::to_value(msk()).unwrap();
            assert_eq!(json["unit"], "msk");
            assert!(json.get("unit_id").is_none(), "{json}");
            let none = serde_json::to_value(Quantity::new(Value::from(1.0), None)).unwrap();
            assert_eq!(none["unit"], serde_json::Value::Null);
        }

        #[test]
        fn a_document_without_unit_id_reads_unresolved() {
            let q: Quantity = serde_json::from_str(
                r#"{"value":{"type":"number","value":{"type":"regular","value":2.0}},"unit":"msk","scalable":true}"#,
            )
            .unwrap();
            assert_eq!(q.unit(), Some("msk"));
            assert_eq!(q.unit_id(), None);
            assert!(q.scalable());
        }

        #[test]
        fn a_unit_id_that_names_nothing_or_has_no_text_is_an_error() {
            let value = r#"{"type":"number","value":{"type":"regular","value":2.0}}"#;
            let bad = serde_json::from_str::<Quantity>(&format!(
                r#"{{"value":{value},"unit":"msk","unit_id":"bunch","scalable":true}}"#
            ))
            .unwrap_err();
            assert!(bad.to_string().contains("names no unit"), "{bad}");
            let no_text = serde_json::from_str::<Quantity>(&format!(
                r#"{{"value":{value},"unit":null,"unit_id":"liter","scalable":true}}"#
            ))
            .unwrap_err();
            assert!(no_text.to_string().contains("no unit text"), "{no_text}");
        }

        #[test]
        fn as_written_equality_ignores_resolution() {
            assert_eq!(msk(), msk().read_with(&swedish()));
            assert_ne!(msk(), Quantity::new(Value::from(2.0), Some("tbsp".into())));
        }
    }

    #[test]
    fn subtracting_the_same_amount_written_differently_empties_the_group() {
        // 3 tbsp and 9 tsp are the same amount; the conversion goes through
        // f64 ratios, so the subtraction must not leave residue behind
        let mut g = group(&["3%tbsp"]);
        g.try_saturating_sub_legacy(&qty("9%tsp"), &CONVERTER)
            .unwrap();
        assert!(
            g.is_empty(),
            "left over: {:?}",
            g.iter().collect::<Vec<_>>()
        );

        // a partial subtraction still leaves what remains
        let mut g = group(&["3%tbsp"]);
        g.try_saturating_sub_legacy(&qty("3%tsp"), &CONVERTER)
            .unwrap();
        assert!(!g.is_empty());
    }

    #[test]
    fn subtracting_a_group_skips_its_unmergeable_quantities() {
        // a text quantity cannot be added to any other, so it sits in `other`;
        // subtracting the group should still subtract the quantities that can be
        let mut other = group(&["2%tbsp"]);
        other.add_legacy(&qty("some"), &CONVERTER);

        let mut g = group(&["3%tbsp"]);
        g.try_saturating_sub_group_legacy(&other, &CONVERTER)
            .expect("the unmergeable quantity is not subtracted");
        assert_eq!(g.iter().count(), 1);
        assert_eq!(g.iter().next().unwrap().to_string(), "1 tbsp");
    }

    #[test]
    fn compare_quantities() {
        let eq = |a: &str, b: &str| qty(a).equals_legacy(&qty(b), &CONVERTER);

        // same unit family
        assert!(eq("3%dl", "300%ml"));
        assert!(eq("3%dl", "0.3%l"));
        assert!(eq("1%kg", "1000%g"));
        assert!(!eq("3%dl", "2%dl"));
        assert!(!eq("1%kg", "999%g"));

        // across systems
        assert!(eq("236.588236%ml", "1%cup"));
        assert!(eq("453.59237%g", "1%lb"));
        assert!(!eq("1%l", "1%cup"));
        // a unit's fractions `accuracy` no longer loosens the comparison, so a
        // cup is only the volume it converts to
        assert!(!eq("240%ml", "1%cup"));
        assert!(!eq("260%ml", "1%cup"));

        // different physical quantity
        assert!(!eq("1%l", "1%kg"));
        assert!(!eq("1%l", "1%h"));

        // unknown units are compared as written
        assert!(eq("1%bunch", "1%bunch"));
        assert!(!eq("1%bunch", "1%clove"));
        assert!(!eq("1%bunch", "1000%bunch"));

        // no unit
        assert!(eq("2", "2"));
        assert!(eq("2.0", "2"));
        assert!(!eq("2", "3"));
        // the tolerance absorbs representation error, not a digit the recipe
        // actually wrote down
        assert!(!eq("2.3", "2.301"));
        assert!(!eq("100", "100.5"));
        assert!(!eq("2", "2%ml"));

        // nor does a unit with fractions enabled absorb one
        assert!(!eq("2%tsp", "2.05%tsp"));
        assert!(!eq("2%tsp", "2.5%tsp"));

        assert!(eq("some", "some"));
        assert!(!eq("some", "a lot"));

        // ranges
        assert!(eq("1-2%l", "1000-2000%ml"));
        assert!(!eq("1-2%l", "1000-3000%ml"));
        assert!(!eq("1-2%l", "1%l"));
    }

    #[test]
    fn compare_grouped_quantities() {
        let eq = |a: &[&str], b: &[&str]| group(a).equals_legacy(&group(b), &CONVERTER);

        assert!(eq(&[], &[]));
        assert!(!eq(&[], &["1%l"]));

        // added together first, then compared with different units
        assert!(eq(&["3%dl", "2%dl"], &["500%ml"]));
        assert!(!eq(&["3%dl", "2%dl"], &["400%ml"]));

        // every kind of quantity has to match
        assert!(eq(
            &["1%kg", "1%l", "2%bunch", "3"],
            &["1000%g", "1000%ml", "2%bunch", "3"]
        ));
        assert!(!eq(&["1%kg", "1%l"], &["1%kg"]));
        assert!(!eq(&["2%bunch"], &["2%clove"]));
        assert!(!eq(&["2%bunch"], &["3%bunch"]));
        assert!(!eq(&["3"], &["4"]));
        // same value, but one has a unit and the other doesn't
        assert!(!eq(&["2"], &["2%l"]));

        assert!(eq(&["some", "a lot"], &["a lot", "some"]));
        assert!(!eq(&["some", "a lot"], &["a lot", "some", "some"]));
    }

    #[test]
    fn subtract_quantities() {
        let eq = |a: Quantity, b: &str| a.equals_legacy(&qty(b), &CONVERTER);
        let sub = |a: &str, b: &str| qty(a).try_sub_legacy(&qty(b), &CONVERTER).unwrap();
        let sub_err = |a: &str, b: &str| qty(a).try_sub_legacy(&qty(b), &CONVERTER).is_err();

        // rhs is converted, the unit of lhs is kept
        assert!(eq(sub("1%l", "300%ml"), "0.7%l"));
        assert!(eq(sub("1%kg", "500%g"), "500%g"));
        assert!(eq(sub("2%bunch", "1%bunch"), "1%bunch"));

        // the result can be negative
        assert!(eq(sub("2", "3"), "-1"));
        assert!(eq(sub("100%g", "1%kg"), "-900%g"));

        // ranges are subtracted as intervals
        assert!(eq(sub("1-2%l", "500%ml"), "0.5-1.5%l"));
        assert!(eq(sub("1-2%l", "0.5-1%l"), "0-1.5%l"));
        assert!(eq(sub("2%l", "0.5-1%l"), "1-1.5%l"));

        // incompatible like when adding
        assert!(sub_err("1%l", "1%kg"));
        assert!(sub_err("1%l", "1%bunch"));
        assert!(sub_err("1%l", "1"));
        assert!(sub_err("some", "1"));
        assert!(sub_err("1", "some"));
    }

    #[test]
    fn saturating_sub_from_grouped_quantities() {
        let sub = |mut a: GroupedQuantity, b: Quantity| {
            a.try_saturating_sub_legacy(&b, &CONVERTER).unwrap();
            a
        };
        let sub_err = |mut a: GroupedQuantity, b: Quantity| {
            a.try_saturating_sub_legacy(&b, &CONVERTER).is_err()
        };
        let eq = |a: GroupedQuantity, b: GroupedQuantity| a.equals_legacy(&b, &CONVERTER);

        // only the compatible quantity of the group changes
        assert!(eq(
            sub(group(&["1%l", "1%kg", "2%bunch", "3"]), qty("500%ml")),
            group(&["0.5%l", "1%kg", "2%bunch", "3"])
        ));
        assert!(eq(
            sub(group(&["2%bunch"]), qty("1%bunch")),
            group(&["1%bunch"])
        ));
        assert!(eq(sub(group(&["3"]), qty("1")), group(&["2"])));

        // saturates at zero, removing the quantity from the group
        assert!(sub(group(&["1%l"]), qty("1000%ml")).is_empty());
        assert!(sub(group(&["1%l"]), qty("2%l")).is_empty());
        assert!(eq(
            sub(group(&["1%l", "1%kg"]), qty("2%l")),
            group(&["1%kg"])
        ));
        // a range is only removed when its end reaches zero
        assert!(eq(
            sub(group(&["1-2%l"]), qty("1.5%l")),
            group(&["0-0.5%l"])
        ));
        assert!(sub(group(&["1-2%l"]), qty("2%l")).is_empty());

        // nothing to subtract from
        assert!(sub_err(group(&[]), qty("1%l")));
        assert!(sub_err(group(&["1%l"]), qty("1%kg")));
        assert!(sub_err(group(&["1%l"]), qty("1%bunch")));
        assert!(sub_err(group(&["1%l"]), qty("1")));
        // a text value has no amount, so it takes its equal out of the group
        assert!(eq(
            sub(group(&["1%l", "some%g"]), qty("some%g")),
            group(&["1%l"])
        ));
        assert!(sub(group(&["some"]), qty("some")).is_empty());
        // and nothing else, which is the same error as an empty slot
        assert!(sub_err(group(&["a lot"]), qty("some")));
        assert!(matches!(
            group(&["1%l"]).try_saturating_sub_legacy(&qty("some"), &CONVERTER),
            Err(GroupedQuantityOpError::NoCompatibleQuantity)
        ));

        {
            // the group is untouched when it errors
            let mut g = group(&["1%l"]);
            assert!(g
                .try_saturating_sub_legacy(&qty("1%kg"), &CONVERTER)
                .is_err());
            assert!(g.equals_legacy(&group(&["1%l"]), &CONVERTER));
        }

        {
            // a whole group is subtracted at once
            let mut g = group(&["1%l", "2%bunch", "3"]);
            assert!(g
                .try_saturating_sub_group_legacy(&group(&["500%ml", "1%bunch"]), &CONVERTER)
                .is_ok());
            assert!(eq(g, group(&["0.5%l", "1%bunch", "3"])));
        }

        {
            // or not at all, even when some of its quantities could be
            let mut g = group(&["1%l", "2%bunch"]);
            assert!(g
                .try_saturating_sub_group_legacy(&group(&["500%ml", "1%clove"]), &CONVERTER)
                .is_err());
            assert!(eq(g, group(&["1%l", "2%bunch"])));
        }
    }

    #[test]
    fn convert_to_base_unit_goes_to_the_smallest_metric_unit_without_refitting() {
        let base = |value: f64, unit: &str| {
            let mut q = Quantity::new(Value::from(value), Some(unit.to_string()));
            q.convert_to_base_unit(&CONVERTER).map(|_| q.to_string())
        };

        assert_eq!(base(1.0, "kg").unwrap(), "1000000 mg");
        // no fraction fitting afterwards, or this would climb back to 1 kg
        assert_eq!(base(1000000.0, "mg").unwrap(), "1000000 mg");
        // always metric, so an imperial amount lands in millilitres
        let cup = base(1.0, "cup").unwrap();
        assert!(cup.ends_with(" ml"), "{cup}");

        let unitless = Quantity::new(Value::from(1.0), None).convert_to_base_unit(&CONVERTER);
        assert!(matches!(unitless, Err(ConvertError::NoUnit(_))));
        let unknown = Quantity::new(Value::from(1.0), Some("smidgen".to_string()))
            .convert_to_base_unit(&CONVERTER);
        assert!(matches!(unknown, Err(ConvertError::UnknownUnit(_))));
    }
}
