//! Equality in meaning rather than in spelling
//!
//! [`PartialEq`] compares the parsed recipe as it was written. [`LegacySemanticEq`]
//! compares what it means: `500 g` equals `0.5 kg`, `1 h` equals `60 min` and
//! `serves: 4` equals `servings: 4`.
//!
//! This needs a [`LegacyConverter`] to resolve units, so it cannot be a [`PartialEq`]
//! implementation.

use crate::LegacyConverter;

/// Compares two values for equality in meaning, resolving units with a
/// [`LegacyConverter`].
///
/// The types of a parsed recipe implement this, and it composes through
/// [`Option`], slices and [`Vec`], so a whole recipe is compared by comparing
/// its parts.
///
/// # Not an equivalence relation
/// Numbers are compared allowing the error their units tolerate (see the
/// impl for [`Quantity`]), so this relation is reflexive and symmetric but
/// **not transitive**: `a` equalling `b` and `b` equalling `c` does not mean
/// `a` equals `c`. Don't use it to key a map or to dedup by transitivity.
///
/// [`Quantity`]: crate::Quantity
pub trait LegacySemanticEq {
    /// Compares `self` with `other`, resolving units with `converter`
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool;
}

impl<T: LegacySemanticEq + ?Sized> LegacySemanticEq for &T {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        T::equals_legacy(self, other, converter)
    }
}

impl<T: LegacySemanticEq> LegacySemanticEq for Option<T> {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.equals_legacy(b, converter),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: LegacySemanticEq> LegacySemanticEq for [T] {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        self.len() == other.len()
            && self
                .iter()
                .zip(other)
                .all(|(a, b)| a.equals_legacy(b, converter))
    }
}

impl<A: LegacySemanticEq, B: LegacySemanticEq> LegacySemanticEq for (A, B) {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        self.0.equals_legacy(&other.0, converter) && self.1.equals_legacy(&other.1, converter)
    }
}

impl<T: LegacySemanticEq> LegacySemanticEq for Vec<T> {
    fn equals_legacy(&self, other: &Self, converter: &LegacyConverter) -> bool {
        self.as_slice().equals_legacy(other.as_slice(), converter)
    }
}

/// Compares two slices as multisets, ignoring the order of their items
///
/// Use this when the order the items are stored in carries no meaning. It pairs
/// each item of `a` with a distinct item of `b`, so duplicates still have to
/// appear the same number of times in both.
///
/// Note that [`LegacySemanticEq`] is not transitive, so which items end up paired can
/// depend on their order. This greedily takes the first match.
pub fn unordered_equals_legacy<T: LegacySemanticEq>(
    a: &[T],
    b: &[T],
    converter: &LegacyConverter,
) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut matched = vec![false; b.len()];
    a.iter().all(|item| {
        let pair = (0..b.len()).find(|&i| !matched[i] && item.equals_legacy(&b[i], converter));
        match pair {
            Some(i) => {
                matched[i] = true;
                true
            }
            None => false,
        }
    })
}

/// Compares two values for equality in meaning. A quantity carries the unit
/// it was read as (ADR 8), so no converter is needed; [`LegacySemanticEq`]
/// is the same through the old converter and goes with it (ADR 9).
///
/// The types of a parsed recipe implement this, and it composes through
/// [`Option`], slices and [`Vec`], so a whole recipe is compared by comparing
/// its parts. Like the old trait it is reflexive and symmetric but **not
/// transitive**, since numbers are compared within a tolerance.
pub trait SemanticEq {
    /// Compares `self` with `other` by meaning
    fn equals(&self, other: &Self) -> bool;
}

impl<T: SemanticEq + ?Sized> SemanticEq for &T {
    fn equals(&self, other: &Self) -> bool {
        T::equals(self, other)
    }
}

impl<T: SemanticEq> SemanticEq for Option<T> {
    fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.equals(b),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: SemanticEq> SemanticEq for [T] {
    fn equals(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.equals(b))
    }
}

impl<A: SemanticEq, B: SemanticEq> SemanticEq for (A, B) {
    fn equals(&self, other: &Self) -> bool {
        self.0.equals(&other.0) && self.1.equals(&other.1)
    }
}

impl<T: SemanticEq> SemanticEq for Vec<T> {
    fn equals(&self, other: &Self) -> bool {
        self.as_slice().equals(other.as_slice())
    }
}

/// Compares two slices as multisets, ignoring the order of their items; see
/// [`unordered_equals_legacy`], which is the same through the old converter.
pub fn unordered_equals<T: SemanticEq>(a: &[T], b: &[T]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut matched = vec![false; b.len()];
    a.iter().all(|item| {
        let pair = (0..b.len()).find(|&i| !matched[i] && item.equals(&b[i]));
        match pair {
            Some(i) => {
                matched[i] = true;
                true
            }
            None => false,
        }
    })
}
