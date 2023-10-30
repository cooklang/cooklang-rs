//! Metadata of a recipe

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

use crate::{
    convert::{ConvertTo, ConvertUnit, ConvertValue, PhysicalQuantity},
    Converter,
};

/// Metadata of a recipe
///
/// The fields on this struct are the parsed values with some special meaning.
/// The raw key/value pairs from the recipe are in the `map` field.
///
/// This struct is non exhaustive because adding a new special metadata value
/// is not a breaking change.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
#[non_exhaustive]
pub struct Metadata {
    /// Description of the recipe
    pub description: Option<String>,
    /// List of tags
    pub tags: Vec<String>,
    /// Emoji for the recipe
    pub emoji: Option<String>,
    /// Author
    pub author: Option<NameAndUrl>,
    /// Source
    ///
    /// This *where* the recipe was obtained from. It's different from author.
    pub source: Option<NameAndUrl>,
    /// Time it takes to prepare/cook the recipe
    pub time: Option<RecipeTime>,
    /// Servings the recipe is made for
    pub servings: Option<Vec<u32>>,
    /// All the raw key/value pairs from the recipe
    pub map: IndexMap<String, String>,
}

/// Combination of name and URL.
///
/// At least one of the fields is [`Some`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameAndUrl {
    name: Option<String>,
    url: Option<Url>,
}

/// Time that takes to prep/cook a recipe
///
/// All values are in minutes.
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
#[serde(untagged, deny_unknown_fields)]
pub enum RecipeTime {
    /// Total time
    Total(u32),
    /// Combination of preparation and cook time
    ///
    /// At least one is [`Some`]
    Composed {
        #[serde(alias = "prep")]
        prep_time: Option<u32>,
        #[serde(alias = "cook")]
        cook_time: Option<u32>,
    },
}

impl Metadata {
    pub(crate) fn insert(
        &mut self,
        key: String,
        value: String,
        converter: &Converter,
    ) -> Result<(), MetadataError> {
        self.map.insert(key.clone(), value.clone());
        match key.as_str() {
            "description" => self.description = Some(value),
            "tag" | "tags" => {
                let new_tags = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>();
                if new_tags.iter().any(|t| !is_valid_tag(t)) {
                    return Err(MetadataError::InvalidTag { tag: value });
                }
                self.tags.extend(new_tags);
            }
            "emoji" => {
                if emojis::get(&value).is_some() {
                    self.emoji = Some(value);
                } else {
                    return Err(MetadataError::NotEmoji { value });
                }
            }
            "author" => self.author = Some(NameAndUrl::parse(&value)),
            "source" => self.source = Some(NameAndUrl::parse(&value)),
            "time" => self.time = Some(RecipeTime::Total(parse_time(&value, converter)?)),
            "prep_time" | "prep time" => {
                let cook_time = self.time.and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { cook_time, .. } => cook_time,
                });
                self.time = Some(RecipeTime::Composed {
                    prep_time: Some(parse_time(&value, converter)?),
                    cook_time,
                });
            }
            "cook_time" | "cook time" => {
                let prep_time = self.time.and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { prep_time, .. } => prep_time,
                });
                self.time = Some(RecipeTime::Composed {
                    prep_time,
                    cook_time: Some(parse_time(&value, converter)?),
                });
            }
            "servings" => {
                let servings = value
                    .split('|')
                    .map(str::trim)
                    .map(str::parse)
                    .collect::<Result<Vec<_>, _>>()?;
                let l = servings.len();
                let dedup_l = {
                    let mut s = servings.clone();
                    s.sort_unstable();
                    s.dedup();
                    s.len()
                };
                if l != dedup_l {
                    return Err(MetadataError::DuplicateServings { servings });
                }
                self.servings = Some(servings)
            }
            _ => {}
        }

        Ok(())
    }

    /// Returns a copy of [`Self::map`] but with all *special* metadata values
    /// removed
    pub fn map_filtered(&self) -> IndexMap<String, String> {
        const ALL_KNOWN_KEYS: &[&str] = &[
            "description",
            "tag",
            "tags",
            "emoji",
            "author",
            "source",
            "time",
            "prep_time",
            "prep time",
            "cook_time",
            "cook time",
            "servings",
        ];
        let mut new_map = self.map.clone();
        new_map.retain(|key, _| !ALL_KNOWN_KEYS.contains(&key.as_ref()));
        new_map
    }
}

/// Returns minutes
fn parse_time(s: &str, converter: &Converter) -> Result<u32, std::num::ParseIntError> {
    match parse_time_with_units(s, converter) {
        Some(time) => Ok(time),
        None => s.parse(),
    }
}

fn parse_time_with_units(s: &str, converter: &Converter) -> Option<u32> {
    let mut total = 0.0;
    let minutes = converter.find_unit("min")?; // TODO maybe make this configurable? It will work for 99% of users...
    if minutes.physical_quantity != PhysicalQuantity::Time {
        return None;
    }
    let to_minutes = ConvertTo::from(&minutes);

    let mut parts = s.split_whitespace();

    while let Some(part) = parts.next() {
        let first_non_digit_pos = part
            .char_indices()
            .find_map(|(pos, c)| (!c.is_numeric()).then_some(pos));
        let (number, unit) = if let Some(mid) = first_non_digit_pos {
            // if the part contains a non numeric char, split it in two and it will
            // be the unit
            part.split_at(mid)
        } else {
            // otherwise, take the next part as the unit
            let next = parts.next()?;
            (part, next)
        };

        let number = number.parse::<u32>().ok()?;
        let (value, _) = converter
            .convert(
                ConvertValue::Number(number as f64),
                ConvertUnit::Key(unit),
                to_minutes,
            )
            .ok()?;
        match value {
            ConvertValue::Number(m) => total += m,
            ConvertValue::Range(_) => unreachable!(),
        }
    }

    let total = total.round() as u32;
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

impl NameAndUrl {
    /// Parse a string into [`NameAndUrl`]
    ///
    /// The string is of the form:
    /// - `Name <Url>`
    /// - `Url`
    /// - `Name`
    ///
    /// The Url validated, so it has to be correct. If no url is found or it's
    /// invalid, everything will be the name.
    pub fn parse(s: &str) -> Self {
        let re = regex!(r"^(\w+(?:\s\w+)*)\s+<([^>]+)>$");
        if let Some(captures) = re.captures(s) {
            let name = &captures[1];
            if let Ok(url) = Url::parse(captures[2].trim()) {
                return NameAndUrl {
                    name: Some(name.to_string()),
                    url: Some(url),
                };
            }
        }

        if let Ok(url) = Url::parse(s) {
            NameAndUrl {
                name: None,
                url: Some(url),
            }
        } else {
            NameAndUrl {
                name: Some(s.to_string()),
                url: None,
            }
        }
    }

    /// Get the name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the url
    pub fn url(&self) -> Option<&Url> {
        self.url.as_ref()
    }
}

impl RecipeTime {
    /// Get the total time prep + cook (minutes)
    pub fn total(self) -> u32 {
        match self {
            RecipeTime::Total(t) => t,
            RecipeTime::Composed {
                prep_time,
                cook_time,
            } => prep_time.iter().chain(cook_time.iter()).sum(),
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MetadataError {
    #[error("Value is not an emoji: {value}")]
    NotEmoji { value: String },
    #[error("Invalid tag: {tag}")]
    InvalidTag { tag: String },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Duplicate servings: {servings:?}")]
    DuplicateServings { servings: Vec<u32> },
}

/// Checks that a tag is valid
///
/// A tag is valid when:
/// - The length is 1 <= len <= 32
/// - lowercase letters, numbers and '-'
/// - starts with a letters
/// - '-' have to be surrounded by letters or numbers, no two '-' can be together
pub fn is_valid_tag(tag: &str) -> bool {
    let tag_len = 1..=32;
    let re = regex!(r"^\p{Ll}[\p{Ll}\d]*(-[\p{Ll}\d]+)*$");

    tag_len.contains(&tag.chars().count()) && re.is_match(tag)
}

/// Transform the input text into a valid tag*
///
/// *Length is not checked
pub fn tagify(text: &str) -> String {
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
    fn test_tagify() {
        assert_eq!(tagify("text"), "text");
        assert_eq!(tagify("text with spaces"), "text-with-spaces");
        assert_eq!(
            tagify("text with      many\tspaces"),
            "text-with-many-spaces"
        );
        assert_eq!(tagify("text with CAPS"), "text-with-caps");
        assert_eq!(tagify("text with CAPS"), "text-with-caps");
        assert_eq!(tagify("text_with_underscores"), "text-with-underscores");
        assert_eq!(tagify("WhATever_--thiS - - is"), "whatever-this-is");
        assert_eq!(tagify("Sensible recipe name"), "sensible-recipe-name");
    }

    #[test]
    fn test_parse_time_with_units() {
        let converter = Converter::bundled();
        let t = |s: &str| parse_time_with_units(s, &converter);
        assert_eq!(t(""), None);
        assert_eq!(t("1"), None);
        assert_eq!(t("1 kilometer"), None);
        assert_eq!(t("1min"), Some(1));
        assert_eq!(t("1 hour"), Some(60));
        assert_eq!(t("1 hour"), Some(60));
        assert_eq!(t("1 hour 30 min"), Some(90));
        assert_eq!(t("1hour 30min"), Some(90));
        assert_eq!(t("1hour30min"), None); // needs space between pairs
        assert_eq!(t("90 minutes"), Some(90));
        assert_eq!(t("30 secs 30 secs"), Some(1)); // sum
        assert_eq!(t("45 secs"), Some(1)); // round up
        assert_eq!(t("25 secs"), None); // round down
        assert_eq!(t("1 min 25 secs"), Some(1)); // round down
        assert_eq!(t("   0  hours 90min 59 sec "), Some(91));
    }
}
