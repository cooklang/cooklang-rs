//! Equality in meaning rather than in spelling
//!
//! [`PartialEq`] compares the parsed recipe as it was written. [`SemanticEq`]
//! compares what it means: `500 g` equals `0.5 kg`, `1 h` equals `60 min` and
//! `serves: 4` equals `servings: 4`.
//!
//! This needs a [`Converter`] to resolve units, so it cannot be a [`PartialEq`]
//! implementation.

use crate::Converter;

/// Compares two values for equality in meaning, resolving units with a
/// [`Converter`].
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
pub trait SemanticEq {
    /// Compares `self` with `other`, resolving units with `converter`
    fn equals(&self, other: &Self, converter: &Converter) -> bool;
}

impl<T: SemanticEq + ?Sized> SemanticEq for &T {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        T::equals(self, other, converter)
    }
}

impl<T: SemanticEq> SemanticEq for Option<T> {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => a.equals(b, converter),
            (None, None) => true,
            _ => false,
        }
    }
}

impl<T: SemanticEq> SemanticEq for [T] {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.len() == other.len() && self.iter().zip(other).all(|(a, b)| a.equals(b, converter))
    }
}

impl<A: SemanticEq, B: SemanticEq> SemanticEq for (A, B) {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.0.equals(&other.0, converter) && self.1.equals(&other.1, converter)
    }
}

impl<T: SemanticEq> SemanticEq for Vec<T> {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.as_slice().equals(other.as_slice(), converter)
    }
}

/// Compares two slices as multisets, ignoring the order of their items
///
/// Use this when the order the items are stored in carries no meaning. It pairs
/// each item of `a` with a distinct item of `b`, so duplicates still have to
/// appear the same number of times in both.
///
/// Note that [`SemanticEq`] is not transitive, so which items end up paired can
/// depend on their order. This greedily takes the first match.
pub fn unordered_equals<T: SemanticEq>(a: &[T], b: &[T], converter: &Converter) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut matched = vec![false; b.len()];
    a.iter().all(|item| {
        let pair = (0..b.len()).find(|&i| !matched[i] && item.equals(&b[i], converter));
        match pair {
            Some(i) => {
                matched[i] = true;
                true
            }
            None => false,
        }
    })
}
