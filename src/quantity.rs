//! Quantity model

use std::{collections::HashMap, fmt::Display, sync::Arc};

use enum_map::EnumMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::convert::{ConvertError, Converter, PhysicalQuantity, Unit};

/// A quantity used in components
#[derive(Debug, Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[cfg_attr(feature = "ts", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Quantity {
    pub(crate) value: Value,
    pub(crate) unit: Option<String>,
    pub(crate) scalable: bool,
}

impl PartialEq for Quantity {
    fn eq(&self, other: &Self) -> bool {
        // ignore scalable for equality
        self.value == other.value && self.unit == other.unit
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
#[cfg_attr(feature = "ts", derive(Tsify))]
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
    /// Creates a new quantity
    pub fn new(value: Value, unit: Option<String>) -> Self {
        Self {
            value,
            unit,
            scalable: false,
        }
    }

    /// Returns `true` if the quantity is scalable
    ///
    /// Two quantities are equal even if one is scalable and the other not
    pub fn scalable(&self) -> bool {
        self.scalable
    }

    /// Get the unit
    pub fn unit(&self) -> Option<&str> {
        self.unit.as_deref()
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
    pub fn unit_info(&self, converter: &Converter) -> Option<Arc<Unit>> {
        self.unit().and_then(|u| converter.find_unit(u))
    }
}

impl Display for Quantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)?;
        if let Some(unit) = &self.unit {
            f.write_str(" ")?;
            unit.fmt(f)?;
        }
        Ok(())
    }
}

/// Text written so the quantity reader gives it back as it is.
///
/// - `{` and line breaks would start the braces anew or end the line:
///   spaces instead. `}` would end the braces: `\}`.
/// - `\` escapes the next character when read: `\\`.
/// - `--` and `[-` open a comment: a `-` after `-` or `[` is `\-`.
/// - In a value (`in_value`), `%` would start the unit (`\%`) and a leading
///   `=` would be read as a scaling lock (`\=`).
fn escape_text(text: &str, in_value: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut leading = true;
    for c in text.chars() {
        let c = if matches!(c, '{' | '\n' | '\r') {
            ' '
        } else {
            c
        };
        match c {
            '\\' => out.push_str("\\\\"),
            '}' => out.push_str("\\}"),
            '%' if in_value => out.push_str("\\%"),
            '=' if in_value && leading => out.push_str("\\="),
            '-' if matches!(out.chars().next_back(), Some('-' | '[')) => out.push_str("\\-"),
            c => out.push(c),
        }
        if !c.is_whitespace() {
            leading = false;
        }
    }
    out
}

/// A text value written so it is not read back as a number: `2` is `\2`.
fn escape_text_value(text: &str) -> String {
    let escaped = escape_text(text, true);
    let reads_as_number = matches!(
        crate::parser::quantity::parse_quantity_text(&escaped).map(|q| q.value),
        Some(Value::Number(_) | Value::Range { .. })
    );
    if reads_as_number {
        // escape the number's first char, never a (maybe multi-byte) space
        let start = escaped.len() - escaped.trim_start().len();
        format!("{}\\{}", &escaped[..start], &escaped[start..])
    } else {
        escaped
    }
}

impl Number {
    /// The number in Cooklang syntax, exactly: a regular number in its
    /// shortest round-trip form (never rounded like [`Display`]), a fraction
    /// as `num/den` or `whole num/den`, or its exact value when the fraction
    /// carries an error.
    fn to_cooklang(self) -> String {
        match self {
            Number::Regular(n) => format!("{n}"),
            Number::Fraction {
                whole,
                num,
                den,
                err: 0.0,
            } => match (whole, num) {
                (whole, 0) => format!("{whole}"),
                (0, num) => format!("{num}/{den}"),
                (whole, num) => format!("{whole} {num}/{den}"),
            },
            Number::Fraction { .. } => format!("{}", self.value()),
        }
    }
}

impl Value {
    fn to_cooklang(&self) -> String {
        match self {
            Value::Number(n) => escape_text(&n.to_cooklang(), false),
            Value::Range { start, end } => escape_text(
                &format!("{}-{}", start.to_cooklang(), end.to_cooklang()),
                false,
            ),
            Value::Text(t) => escape_text_value(t),
        }
    }
}

impl Quantity {
    /// The quantity in Cooklang brace syntax, without the braces: `2%l`,
    /// `1/2%cup`, `3`, `a pinch`, `1-2%cup`.
    ///
    /// A quantity read from a shopping list, or a text quantity, reads back
    /// as the same value and unit. Text that would be read as syntax is
    /// escaped (`\%`, `\=`, `\-`, `\\`, `\2` for the text `2`). Numbers are
    /// written in full precision, never rounded like [`Display`].
    ///
    /// Not guaranteed to read back the same: `{` and line breaks (written as
    /// spaces), whitespace at either end or repeated inside text and units
    /// (the reader trims and collapses it), an empty unit, an empty text
    /// without a unit, ranges (read as text, as shopping lists have no range
    /// values), and negative, NaN or infinite numbers or fractions with a zero
    /// denominator.
    pub fn to_cooklang(&self) -> String {
        let value = self.value.to_cooklang();
        match &self.unit {
            Some(unit) => format!("{value}%{}", escape_text(unit, false)),
            None => value,
        }
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
                    found: u.clone(),
                    lhs: false,
                });
            }
            (Some(u), None) => {
                return Err(IncompatibleUnits::MissingUnit {
                    found: u.clone(),
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
                                a: a.clone(),
                                b: b.clone(),
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
        let qty = Quantity::new(value, self.unit.clone());

        Ok(qty)
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

impl GroupedQuantity {
    /// Create a new empty group
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add a new quantity to the group
    pub fn add(&mut self, q: &Quantity, converter: &Converter) {
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

    /// Merge the group with another one
    pub fn merge(&mut self, other: &Self, converter: &Converter) {
        for q in other.iter() {
            self.add(q, converter)
        }
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
    /// Tries to create a new approximate number within a margin of error.
    ///
    /// It returns none if:
    /// - The value is an integer
    /// - It can't be represented with the given restrictions as a fraction.
    /// - The number is not positive.
    ///
    /// It will return `Number::Regular` when the number is an integer with less
    /// than a 1e-10 margin of error.
    ///
    /// Otherwise it will return a `Number::Fraction`. `num` can be 0 if the
    /// value is rounded to an integer.
    ///
    /// `accuracy` is a value between 0 and 1 representing the error percent.
    ///
    /// `max_den` is the maximum denominator. The denominator is one a list of
    /// "common" fractions: 2, 3, 4, 5, 8, 10, 16, 32, 64. 64 is the max.
    ///
    /// `max_whole` determines the maximum value of the integer. Setting this to
    /// 0 only allows fractions < 1. Exact values higher than this are also
    /// rejected.
    ///
    /// # Panics
    /// - If `accuracy > 1` or `accuracy < 0`.
    /// - If `max_den > 64`
    pub fn new_approx(value: f64, accuracy: f32, max_den: u8, max_whole: u32) -> Option<Self> {
        assert!((0.0..=1.0).contains(&accuracy));
        assert!(max_den <= 64);
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
    use test_case::test_case;

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

    fn q(value: Value, unit: Option<&str>) -> Quantity {
        Quantity::new(value, unit.map(str::to_string))
    }

    #[test]
    fn to_cooklang_numbers() {
        assert_eq!(
            q(Value::Number(Number::Regular(2.0)), Some("l")).to_cooklang(),
            "2%l"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(1.5)), Some("kg")).to_cooklang(),
            "1.5%kg"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(0.0004)), Some("kg")).to_cooklang(),
            "0.0004%kg"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(1.0 / 3.0)), None).to_cooklang(),
            "0.3333333333333333"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(3.0)), None).to_cooklang(),
            "3"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(1e21)), None).to_cooklang(),
            "1000000000000000000000"
        );
    }

    #[test]
    fn to_cooklang_fractions() {
        let half = Number::Fraction {
            whole: 0,
            num: 1,
            den: 2,
            err: 0.0,
        };
        let one_and_half = Number::Fraction {
            whole: 1,
            num: 1,
            den: 2,
            err: 0.0,
        };
        let lossy = Number::Fraction {
            whole: 0,
            num: 1,
            den: 3,
            err: 0.01,
        };
        assert_eq!(q(Value::Number(half), Some("cup")).to_cooklang(), "1/2%cup");
        assert_eq!(
            q(Value::Number(one_and_half), Some("cup")).to_cooklang(),
            "1 1/2%cup"
        );
        // A fraction carrying an error is written as its exact value.
        assert_eq!(
            q(Value::Number(lossy), None).to_cooklang(),
            format!("{}", 1.0 / 3.0 + 0.01)
        );
    }

    #[test]
    fn to_cooklang_ranges_and_text() {
        let range = Value::Range {
            start: Number::Regular(1.0),
            end: Number::Regular(2.0),
        };
        assert_eq!(q(range, Some("cup")).to_cooklang(), "1-2%cup");
        assert_eq!(
            q(Value::Text("a pinch".into()), None).to_cooklang(),
            "a pinch"
        );
        assert_eq!(
            q(Value::Text("1-2".into()), Some("cup")).to_cooklang(),
            "1-2%cup"
        );
    }

    #[test]
    fn to_cooklang_never_breaks_the_braces_or_the_line() {
        assert_eq!(
            q(Value::Text("a {big}\npinch".into()), Some("c}u\rp")).to_cooklang(),
            "a  big\\} pinch%c\\}u p"
        );
    }

    fn read_back(q: &Quantity) -> Option<Quantity> {
        crate::parser::quantity::parse_quantity_text(&q.to_cooklang())
    }

    #[test]
    fn to_cooklang_escapes_what_the_reader_would_take_as_syntax() {
        let t = |s: &str| q(Value::Text(s.into()), None);
        assert_eq!(t("a--b").to_cooklang(), "a-\\-b");
        assert_eq!(t("[- open").to_cooklang(), "[\\- open");
        assert_eq!(t("a%b").to_cooklang(), "a\\%b");
        assert_eq!(t("=2").to_cooklang(), "\\=2");
        assert_eq!(t("a=b").to_cooklang(), "a=b");
        assert_eq!(t("a\\b").to_cooklang(), "a\\\\b");
        assert_eq!(t("2").to_cooklang(), "\\2");
        assert_eq!(t("1/2").to_cooklang(), "\\1/2");
        assert_eq!(t(".5").to_cooklang(), "\\.5");
        // Text the reader already reads as text needs no escape.
        assert_eq!(t("1-2").to_cooklang(), "1-2");
        assert_eq!(t("1,5").to_cooklang(), "1,5");
        assert_eq!(t("1/0").to_cooklang(), "1/0");
        assert_eq!(t("01").to_cooklang(), "01");
        assert_eq!(
            q(Value::Text("1".into()), Some("a--b")).to_cooklang(),
            "\\1%a-\\-b"
        );
        assert_eq!(
            q(Value::Number(Number::Regular(2.0)), Some("a%b")).to_cooklang(),
            "2%a%b"
        );
        for q in [
            t("a--b"),
            t("[- open"),
            t("a%b"),
            t("=2"),
            t("2"),
            t("a\\b"),
        ] {
            assert_eq!(format!("{:?}", read_back(&q)), format!("{:?}", Some(&q)));
        }
    }

    /// Every text value and unit built from characters the reader treats
    /// specially reads back as the same text and unit.
    #[test]
    fn to_cooklang_text_property() {
        const PIECES: &[&str] = &[
            "\\", "%", "=", "-", "[", "]", "{", "0", "1", "2", "/", ".", " ", "a", "b", "--", "[-",
            "-]", "\\-", "1/2", "1 1/2", "==", "}", "\\}",
        ];
        let mut seed = 0x9e37_79b9_7f4a_7c15_u64;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        // What the reader can give back: no braces, trimmed, single spaces.
        let normal = |s: String| -> String {
            s.replace('{', "")
                .split(' ')
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        };
        let mut checked = 0;
        let piece = |next: &mut dyn FnMut() -> u64| -> String {
            let n = next();
            (0..1 + n % 5)
                .map(|_| PIECES[(next() % PIECES.len() as u64) as usize])
                .collect()
        };
        for _ in 0..20_000 {
            let text = normal(piece(&mut next));
            let unit = (next() % 3 != 0)
                .then(|| normal(piece(&mut next)))
                .filter(|u| !u.is_empty());
            if text.is_empty() && unit.is_none() {
                continue;
            }
            let q = q(Value::Text(text), unit.as_deref());
            assert_eq!(
                format!("{:?}", read_back(&q)),
                format!("{:?}", Some(&q)),
                "written {:?}",
                q.to_cooklang()
            );
            checked += 1;
        }
        assert!(checked > 10_000);
    }
}
