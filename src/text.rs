use std::{borrow::Cow, fmt::Debug};

use serde::Serialize;

use crate::{located::Located, span::Span};

/// Borrowed text with location information and the ability to skip fragments
///
/// Comments are skipped, and an [`&str`] can't represent that by itself.
///
/// This implemets [`PartialEq`] and it will return true if the text matches, it
/// ignores the location.
#[derive(Clone, Serialize)]
pub struct Text<'a> {
    data: TextData<'a>,
}

#[derive(Clone, Serialize)]
enum TextData<'a> {
    Empty { offset: usize },
    Single { fragment: TextFragment<'a> },
    Fragmented { fragments: Vec<TextFragment<'a>> },
}

impl<'a> TextData<'a> {
    fn push(&mut self, fragment: TextFragment<'a>) {
        match self {
            TextData::Empty { .. } => *self = Self::Single { fragment },
            TextData::Single { fragment: current } => {
                *self = Self::Fragmented {
                    fragments: vec![*current, fragment],
                }
            }
            TextData::Fragmented { fragments } => fragments.push(fragment),
        }
    }

    fn as_slice(&self) -> &[TextFragment<'a>] {
        match self {
            TextData::Empty { .. } => &[],
            TextData::Single { fragment } => std::slice::from_ref(fragment),
            TextData::Fragmented { fragments } => fragments.as_slice(),
        }
    }

    fn span(&self) -> Span {
        match self {
            TextData::Empty { offset } => Span::pos(*offset),
            TextData::Single { fragment } => fragment.span(),
            TextData::Fragmented { fragments } => {
                let start = fragments.first().unwrap().span().start();
                let end = fragments.last().unwrap().span().end();
                Span::new(start, end)
            }
        }
    }
}

impl<'a> Text<'a> {
    pub(crate) fn empty(offset: usize) -> Self {
        Self {
            data: TextData::Empty { offset },
        }
    }

    pub(crate) fn from_str(s: &'a str, offset: usize) -> Self {
        let mut t = Self::empty(offset);
        t.append_fragment(TextFragment::new(s, offset));
        t
    }

    pub(crate) fn append_fragment(&mut self, fragment: TextFragment<'a>) {
        assert!(self.span().end() <= fragment.offset);
        if fragment.text.is_empty() {
            return;
        }
        self.data.push(fragment);
    }

    pub(crate) fn append_str(&mut self, s: &'a str, offset: usize) {
        self.append_fragment(TextFragment::new(s, offset))
    }

    /// Get the span of the original input of the text
    ///
    /// If there are skipped fragments in between, these fragments will be included
    /// as [`Span`] is only a start an end. To be able to get multiple spans, see
    /// [`Self::fragments`].
    pub fn span(&self) -> Span {
        self.data.span()
    }

    /// Get the text of all the fragments concatenated
    ///
    /// A soft break is always rendered as a ascii whitespace.
    pub fn text(&self) -> Cow<'a, str> {
        // Contiguous text fragments may be joined together without a copy.
        // but most Text instances will only be one fragment anyways

        let mut s = Cow::default();
        for f in self.fragments() {
            let text = match f.kind {
                TextFragmentKind::Text => f.text,
                TextFragmentKind::SoftBreak => " ",
            };
            s += text;
        }
        s
    }

    /// Get the text trimmed (start and end)
    pub fn text_outer_trimmed(&self) -> Cow<'a, str> {
        match self.text() {
            Cow::Borrowed(s) => Cow::Borrowed(s.trim()),
            Cow::Owned(s) => Cow::Owned(s.trim().to_owned()),
        }
    }

    /// Get the text trimmed from whitespaces before, after and in between words
    pub fn text_trimmed(&self) -> Cow<'a, str> {
        let t = self.text_outer_trimmed();

        if !t.contains("  ") {
            return t;
        }

        let mut t = t.into_owned();
        let mut prev = ' ';
        t.retain(|c| {
            let r = c != ' ' || prev != ' ';
            prev = c;
            r
        });
        Cow::from(t)
    }

    /// Checks that the text is not empty or blank, i.e. whitespace does not count
    pub fn is_text_empty(&self) -> bool {
        self.fragments().iter().all(|f| f.text.trim().is_empty())
    }

    /// Get all the [`TextFragment`]s that compose the text
    pub fn fragments(&self) -> &[TextFragment<'a>] {
        self.data.as_slice()
    }

    /// Convenience method to the the text in [`Located`]
    pub fn located_text_trimmed(&self) -> Located<Cow<str>> {
        Located::new(self.text_trimmed(), self.span())
    }

    /// Convenience method to the the text in [`Located`] and owned
    pub fn located_string_trimmed(&self) -> Located<String> {
        self.located_text_trimmed().map(Cow::into_owned)
    }
}

impl std::fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text())
    }
}

impl std::fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fragments = self.fragments();
        match fragments.len() {
            0 => write!(f, "<empty> @ {:?}", self.span()),
            1 => fragments[0].fmt(f),
            _ => f.debug_list().entries(fragments).finish(),
        }
    }
}

impl PartialEq for Text<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.fragments() == other.fragments()
    }
}

impl From<Text<'_>> for Span {
    fn from(value: Text<'_>) -> Self {
        value.span()
    }
}

/// Fragment that compose a [`Text`]
///
/// This implemets [`PartialEq`] and it will return true if the text matches, it
/// ignores the location.
#[derive(Clone, Copy, Serialize)]
pub struct TextFragment<'a> {
    text: &'a str,
    offset: usize,
    kind: TextFragmentKind,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub enum TextFragmentKind {
    Text,
    SoftBreak,
}

impl<'a> TextFragment<'a> {
    pub(crate) fn new(text: &'a str, offset: usize) -> Self {
        Self {
            text,
            offset,
            kind: TextFragmentKind::Text,
        }
    }

    pub(crate) fn soft_break(text: &'a str, offset: usize) -> Self {
        Self {
            text,
            offset,
            kind: TextFragmentKind::SoftBreak,
        }
    }

    /// Get the inner text
    pub fn text(&self) -> &str {
        self.text
    }

    /// Get the span of the original input of the fragment
    pub fn span(&self) -> Span {
        Span::new(self.start(), self.end())
    }

    /// Start offset of the fragment
    pub fn start(&self) -> usize {
        self.offset
    }

    /// End offset (not included) of the fragment
    pub fn end(&self) -> usize {
        self.offset + self.text.len()
    }
}

impl Debug for TextFragment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            TextFragmentKind::Text => write!(f, "{:?}", self.text),
            TextFragmentKind::SoftBreak => write!(f, "SoftBreak({:?})", self.text),
        }?;
        write!(f, " @ {:?}", self.span())
    }
}

impl PartialEq for TextFragment<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
    }
}

#[cfg(test)]
mod tests {
    use super::Text;
    use test_case::test_case;

    #[test_case("a b c" => "a b c"; "no trim")]
    #[test_case("  a b c  " => "a b c"; "outer trim")]
    #[test_case("  a    b      c  " => "a b c"; "inner trim")]
    fn trim_whitespace(t: &str) -> String {
        let t = Text::from_str(t, 0);
        t.text_trimmed().into_owned()
    }
}
