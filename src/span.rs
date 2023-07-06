use std::ops::{Deref, Range};

/// Location in the source code
///
/// The offsets are zero-indexed charactere offsets from the beginning of the source
/// code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
}

impl Span {
    pub(crate) fn to_chars_span<Id>(&self, all_source: &str, source_id: Id) -> CharsSpan<Id> {
        let start = all_source[..self.start].chars().count();
        let len = all_source[self.range()].chars().count();
        CharsSpan {
            span: Span::new(start, start + len),
            source_id,
        }
    }
}

pub(crate) struct CharsSpan<Id> {
    span: Span,
    source_id: Id,
}

impl<Id> Deref for CharsSpan<Id> {
    type Target = Span;

    fn deref(&self) -> &Self::Target {
        &self.span
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

impl crate::context::Recover for Span {
    fn recover() -> Self {
        Self::new(0, 0)
    }
}

impl<Id> ariadne::Span for CharsSpan<Id>
where
    Id: ToOwned + PartialEq,
{
    type SourceId = Id;

    fn source(&self) -> &Self::SourceId {
        &self.source_id
    }

    fn start(&self) -> usize {
        self.span.start
    }

    fn end(&self) -> usize {
        self.span.end
    }
}
