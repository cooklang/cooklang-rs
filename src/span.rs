//! Utility to represent a location in the source code

use std::ops::Range;

/// Location in the source code
///
/// The offsets are zero-indexed charactere offsets from the beginning of the source
/// code.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, PartialOrd, Ord)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub(crate) fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub(crate) fn pos(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }

    /// Start offset of the span
    pub fn start(&self) -> usize {
        self.start
    }

    /// End (exclusive) offset of the span
    pub fn end(&self) -> usize {
        self.end
    }

    /// Get the span as a range
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Len of the span in bytes
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the span is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl std::fmt::Debug for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

impl From<Range<usize>> for Span {
    fn from(value: Range<usize>) -> Self {
        Self::new(value.start, value.end)
    }
}

impl From<Span> for Range<usize> {
    fn from(value: Span) -> Self {
        value.start..value.end
    }
}

impl<T> From<crate::located::Located<T>> for Span {
    fn from(value: crate::located::Located<T>) -> Self {
        value.span()
    }
}

impl crate::error::Recover for Span {
    fn recover() -> Self {
        Self::new(0, 0)
    }
}
