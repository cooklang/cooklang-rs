//! Utility to add location information to any type

use std::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut, Range},
};

use serde::Serialize;

use crate::{error::Recover, span::Span};

/// Wrapper type that adds location information to another
#[derive(PartialEq, Serialize)]
pub struct Located<T> {
    inner: T,
    span: Span,
}

impl<T> Located<T> {
    /// Creata a new instance of [`Located`]
    pub fn new(inner: T, span: impl Into<Span>) -> Self {
        Self {
            inner,
            span: span.into(),
        }
    }

    /// Map the inner value while keeping the same location
    pub fn map<F, O>(self, f: F) -> Located<O>
    where
        F: FnOnce(T) -> O,
    {
        Located {
            inner: f(self.inner),
            span: self.span,
        }
    }

    /// Discard the location and consume the inner value
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Consume and get the inner value and it's location
    pub fn take_pair(self) -> (T, Span) {
        (self.inner, self.span)
    }

    /// Get a reference to the inner value
    pub fn value(&self) -> &T {
        &self.inner
    }

    /// Get the location
    pub fn span(&self) -> Span {
        self.span
    }
}

impl<T: Clone + Copy> Copy for Located<T> {}

impl<T: Copy> Located<T> {
    /// Get the inner value by copy
    pub fn get(&self) -> T {
        self.inner
    }
}

impl<T> Clone for Located<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            span: self.span,
        }
    }
}

impl<T> Debug for Located<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)?;
        f.write_str(" @ ")?;
        self.span.fmt(f)
    }
}

impl<T> Display for Located<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T> Deref for Located<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Located<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> From<Located<T>> for Range<usize> {
    fn from(value: Located<T>) -> Self {
        value.span.range()
    }
}

impl<T> Recover for Located<T>
where
    T: Recover,
{
    fn recover() -> Self {
        Self {
            inner: T::recover(),
            span: Recover::recover(),
        }
    }
}
