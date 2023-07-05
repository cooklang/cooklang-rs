//! Metadata of a recipe

use std::ops::RangeInclusive;

pub use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// Utility to create lazy regex
/// from <https://docs.rs/once_cell/latest/once_cell/#lazily-compiled-regex>
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| {
            let _enter = tracing::trace_span!("regex", re = $re).entered();
            regex::Regex::new($re).unwrap()
        })
    }};
}
pub(crate) use regex;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Metadata {
    pub map: IndexMap<String, String>,
}

impl Metadata {
    pub(crate) fn insert(&mut self, key: String, value: String) -> Result<(), MetadataError> {
        self.map.insert(key.clone(), value.clone());

        Ok(())
    }
}


#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("Value is not an emoji: {value}")]
    NotEmoji { value: String },
    #[error("Invalid tag: {tag}")]
    InvalidTag { tag: String },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
}

const TAG_LEN: RangeInclusive<usize> = 1..=32;
fn is_valid_tag(tag: &str) -> bool {
    let re = regex!(r"^\p{Ll}[\p{Ll}\d]*(-[\p{Ll}\d]+)*$");

    TAG_LEN.contains(&tag.chars().count()) && re.is_match(tag)
}

pub fn slugify(text: &str) -> String {
    let text = text
        .trim()
        .replace(|c: char| (c.is_whitespace() || c == '_'), "-")
        .replace(|c: char| !(c.is_alphanumeric() || c == '-'), "")
        .trim_matches('-')
        .to_lowercase();

    let slug = regex!(r"--+").replace_all(&text, "-");

    slug.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_tag() {
        assert!(is_valid_tag("uwu"));
        assert!(is_valid_tag("italian-food"));
        assert!(is_valid_tag("contains-number-1"));
        assert!(is_valid_tag("unicode-ñçá"));
        assert!(!is_valid_tag(""));
        assert!(!is_valid_tag("1ow"));
        assert!(!is_valid_tag("111"));
        assert!(!is_valid_tag("1starts-with-number"));
        assert!(!is_valid_tag("many---hyphens"));
        assert!(!is_valid_tag("other/characters"));
        assert!(!is_valid_tag("other@[]chara€cters"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("text"), "text");
        assert_eq!(slugify("text with spaces"), "text-with-spaces");
        assert_eq!(
            slugify("text with      many\tspaces"),
            "text-with-many-spaces"
        );
        assert_eq!(slugify("text with CAPS"), "text-with-caps");
        assert_eq!(slugify("text with CAPS"), "text-with-caps");
        assert_eq!(slugify("text_with_underscores"), "text-with-underscores");
        assert_eq!(slugify("WhATever_--thiS - - is"), "whatever-this-is");
        assert_eq!(slugify("Sensible recipe name"), "sensible-recipe-name");
    }
}
