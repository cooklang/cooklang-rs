//! Metadata of a recipe

use std::{collections::HashMap, num::ParseFloatError, str::FromStr};

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
    convert::{ConvertError, ConvertTo, ConvertUnit, ConvertValue, PhysicalQuantity, UnknownUnit},
    Converter,
};

/// Metadata of a recipe
///
/// The raw key/value pairs from the recipe are in the `map` field. Many methods
/// on this struct are the parsed values with some special meaning. They return
/// `None` if the key is missing or the value failed to parse.
///
/// Also, most of these values will not have been parsed if the
/// [`SPECIAL_METADATA`](crate::Extensions::SPECIAL_METADATA) extension is not
/// enabled.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Default)]
pub struct Metadata {
    special: HashMap<SpecialKey, SpecialValue>,
    /// All the raw key/value pairs from the recipe
    pub map: IndexMap<String, String>,
}

#[derive(
    Debug,
    Clone,
    Copy,
    strum::Display,
    strum::EnumString,
    strum::AsRefStr,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub(crate) enum SpecialKey {
    Description,
    #[strum(serialize = "tag", to_string = "tags")]
    Tags,
    Emoji,
    Author,
    Source,
    Time,
    #[strum(serialize = "prep_time", to_string = "prep time")]
    PrepTime,
    #[strum(serialize = "cook_time", to_string = "cook time")]
    CookTime,
    Servings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum SpecialValue {
    Tags(Vec<String>),
    NameAndUrl(NameAndUrl),
    Time(RecipeTime),
    Servings(Vec<u32>),
    String(String),
}

macro_rules! unwrap_value {
    ($variant:ident, $value:expr) => {
        if let crate::metadata::SpecialValue::$variant(inner) = $value {
            inner
        } else {
            panic!(
                "Unexpected special value variant. Expected '{}' but got '{:?}'",
                stringify!(SpecialValue::$variant),
                $value
            );
        }
    };
}

impl Metadata {
    /// Description of the recipe
    pub fn description(&self) -> Option<&str> {
        self.map
            .get(SpecialKey::Description.as_ref())
            .map(|s| s.as_str())
    }

    /// Emoji for the recipe
    pub fn emoji(&self) -> Option<&str> {
        self.special
            .get(&SpecialKey::Emoji)
            .map(|v| unwrap_value!(String, v).as_str())
    }

    /// List of tags
    pub fn tags(&self) -> Option<&[String]> {
        self.special
            .get(&SpecialKey::Tags)
            .map(|v| unwrap_value!(Tags, v).as_slice())
    }

    /// Author
    ///
    /// This *who* wrote the recipe.
    pub fn author(&self) -> Option<&NameAndUrl> {
        self.special
            .get(&SpecialKey::Author)
            .map(|v| unwrap_value!(NameAndUrl, v))
    }

    /// Source
    ///
    /// This *where* the recipe was obtained from.
    pub fn source(&self) -> Option<&NameAndUrl> {
        self.special
            .get(&SpecialKey::Source)
            .map(|v| unwrap_value!(NameAndUrl, v))
    }

    /// Time it takes to prepare/cook the recipe
    pub fn time(&self) -> Option<&RecipeTime> {
        self.special
            .get(&SpecialKey::Time)
            .map(|v| unwrap_value!(Time, v))
    }

    /// Servings the recipe is made for
    pub fn servings(&self) -> Option<&[u32]> {
        self.special
            .get(&SpecialKey::Servings)
            .map(|v| unwrap_value!(Servings, v).as_slice())
    }
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
    pub(crate) fn insert_special(
        &mut self,
        key: SpecialKey,
        value: String,
        converter: &Converter,
    ) -> Result<(), MetadataError> {
        match key {
            SpecialKey::Description => {
                self.map.insert(key.as_ref().to_string(), value);
            }
            SpecialKey::Tags => {
                let new_tags = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>();
                if new_tags.iter().any(|t| !is_valid_tag(t)) {
                    return Err(MetadataError::InvalidTag { tag: value });
                }

                let tags_val = self
                    .special
                    .entry(key)
                    .or_insert_with(|| SpecialValue::Tags(Vec::new()));
                unwrap_value!(Tags, tags_val).extend(new_tags);
            }
            SpecialKey::Emoji => {
                let emoji = if value.starts_with(':') && value.ends_with(':') {
                    emojis::get_by_shortcode(&value[1..value.len() - 1])
                } else {
                    emojis::get(&value)
                };
                if let Some(emoji) = emoji {
                    self.special
                        .insert(key, SpecialValue::String(emoji.to_string()));
                } else {
                    return Err(MetadataError::NotEmoji { value });
                }
            }
            SpecialKey::Author | SpecialKey::Source => {
                self.special
                    .insert(key, SpecialValue::NameAndUrl(NameAndUrl::parse(&value)));
            }
            SpecialKey::Time => {
                let time = RecipeTime::Total(parse_time(&value, converter)?);
                self.special.insert(key, SpecialValue::Time(time));
            }
            SpecialKey::PrepTime => {
                let cook_time = self.time().and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { cook_time, .. } => *cook_time,
                });
                let time = RecipeTime::Composed {
                    prep_time: Some(parse_time(&value, converter)?),
                    cook_time,
                };
                self.special
                    .insert(SpecialKey::Time, SpecialValue::Time(time));
            }
            SpecialKey::CookTime => {
                let prep_time = self.time().and_then(|t| match t {
                    RecipeTime::Total(_) => None,
                    RecipeTime::Composed { prep_time, .. } => *prep_time,
                });
                let time = RecipeTime::Composed {
                    prep_time,
                    cook_time: Some(parse_time(&value, converter)?),
                };
                self.special
                    .insert(SpecialKey::Time, SpecialValue::Time(time));
            }
            SpecialKey::Servings => {
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
                self.special
                    .insert(SpecialKey::Servings, SpecialValue::Servings(servings));
            }
        }
        Ok(())
    }

    /// Parse the inner [map](Self::map) updating the special keys
    ///
    /// This can be useful if you edit the inner values of the metadata map and
    /// want the special keys to refresh.
    ///
    /// The error variant of the result contains the key value pairs that had
    /// an error parsing. Even if [`Err`] is returned, some values may have been
    /// updated.
    pub fn parse_special(&mut self, converter: &Converter) -> Result<(), Vec<(String, String)>> {
        let mut new = Self::default();
        let mut errors = Vec::new();
        for (key, val) in &self.map {
            if let Ok(sp_key) = SpecialKey::from_str(key) {
                if new.insert_special(sp_key, val.clone(), converter).is_err() {
                    errors.push((key.clone(), val.clone()));
                }
            }
        }
        self.special = new.special;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Iterates over [`Self::map`] but with all *special* metadata values
    /// skipped
    pub fn map_filtered(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map
            .iter()
            .filter(|(key, _)| SpecialKey::from_str(key).is_err())
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }
}

/// Returns minutes
fn parse_time(s: &str, converter: &Converter) -> Result<u32, ParseTimeError> {
    if s.is_empty() {
        return Err(ParseTimeError::Empty);
    }
    let r = parse_time_with_units(s, converter);
    // if any error, try to fall back to a full float parse
    if r.is_err() {
        let minutes = s.parse::<f64>().map(|m| m.round() as u32);
        if let Ok(minutes) = minutes {
            return Ok(minutes);
        }
    }
    // otherwise return the result whatever it was
    r
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParseTimeError {
    #[error("A value is missing a unit")]
    MissingUnit,
    #[error("Could not find minutes in the configuration")]
    MinutesNotFound,
    #[error(transparent)]
    ConvertError(#[from] ConvertError),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error("An empty value is not valid")]
    Empty,
}

fn dynamic_time_units(
    value: f64,
    unit: &str,
    converter: &Converter,
) -> Result<f64, ParseTimeError> {
    // TODO maybe make this configurable? It will work for 99% of users...
    let minutes = converter
        .find_unit("min")
        .or_else(|| converter.find_unit("minute"))
        .or_else(|| converter.find_unit("minutes"))
        .or_else(|| converter.find_unit("m"))
        .ok_or(ParseTimeError::MinutesNotFound)?;
    if minutes.physical_quantity != PhysicalQuantity::Time {
        return Err(ParseTimeError::MinutesNotFound);
    }
    let (value, _) = converter.convert(
        ConvertValue::Number(value),
        ConvertUnit::Key(unit),
        ConvertTo::from(&minutes),
    )?;
    match value {
        ConvertValue::Number(n) => Ok(n),
        _ => unreachable!(),
    }
}

fn hard_coded_time_units(value: f64, unit: &str) -> Result<f64, ParseTimeError> {
    let minutes = match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => value / 60.0,
        "m" | "min" | "minute" | "minutes" => value,
        "h" | "hour" | "hours" => value * 60.0,
        "d" | "day" | "days" => value * 24.0 * 60.0,
        _ => return Err(ConvertError::UnknownUnit(UnknownUnit(unit.to_string())).into()),
    };
    Ok(minutes)
}

fn parse_time_with_units(s: &str, converter: &Converter) -> Result<u32, ParseTimeError> {
    let to_minutes = |value, unit| {
        if converter.unit_count() == 0 {
            hard_coded_time_units(value, unit)
        } else {
            dynamic_time_units(value, unit, converter)
        }
    };

    let mut total = 0.0;
    let mut parts = s.split_whitespace();
    while let Some(part) = parts.next() {
        let first_non_digit_pos = part
            .char_indices()
            .find_map(|(pos, c)| (!c.is_numeric() && c != '.').then_some(pos));
        let (number, unit) = if let Some(mid) = first_non_digit_pos {
            // if the part contains a non numeric char, split it in two and it will
            // be the unit
            part.split_at(mid)
        } else {
            // otherwise, take the next part as the unit
            let next = parts.next().ok_or(ParseTimeError::MissingUnit)?;
            (part, next)
        };
        let number = number.parse::<f64>()?;
        total += to_minutes(number, unit)?;
    }
    Ok(total.round() as u32)
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
pub(crate) enum MetadataError {
    #[error("Value is not an emoji: {value}")]
    NotEmoji { value: String },
    #[error("Invalid tag: {tag}")]
    InvalidTag { tag: String },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Duplicate servings: {servings:?}")]
    DuplicateServings { servings: Vec<u32> },
    #[error(transparent)]
    ParseTimeError(#[from] ParseTimeError),
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
        let t = |s: &str| parse_time_with_units(s, &converter).ok();
        assert_eq!(t(""), Some(0));
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
        assert_eq!(t("25 secs"), Some(0)); // round down
        assert_eq!(t("1 min 25 secs"), Some(1)); // round down
        assert_eq!(t("   0  hours 90min 59 sec "), Some(91));
    }

    #[test]
    fn special_keys() {
        let t = |s: &str, key: SpecialKey| {
            assert_eq!(SpecialKey::from_str(s).unwrap(), key);
            assert_eq!(key.to_string(), s);
        };
        let t_alias = |s: &str, key: SpecialKey| {
            assert_eq!(SpecialKey::from_str(s).unwrap(), key);
            assert_ne!(key.to_string(), s);
        };

        t("description", SpecialKey::Description);
        t("tags", SpecialKey::Tags);
        t_alias("tag", SpecialKey::Tags);
        t("emoji", SpecialKey::Emoji);
        t("author", SpecialKey::Author);
        t("source", SpecialKey::Source);
        t("time", SpecialKey::Time);
        t("prep time", SpecialKey::PrepTime);
        t_alias("prep_time", SpecialKey::PrepTime);
        t("cook time", SpecialKey::CookTime);
        t_alias("cook_time", SpecialKey::CookTime);
        t("servings", SpecialKey::Servings);
    }

    // To ensure no panics in unwrap_value
    #[test]
    fn special_key_access() {
        let converter = Converter::empty();
        let mut m = Metadata::default();

        let _ = m.insert_special(
            SpecialKey::Description,
            "Description".to_string(),
            &converter,
        );
        assert!(matches!(m.description(), Some(_)));

        let _ = m.insert_special(SpecialKey::Tags, "t1, t2".to_string(), &converter);
        assert!(matches!(m.tags(), Some(_)));

        let _ = m.insert_special(SpecialKey::Emoji, "⛄".to_string(), &converter);
        assert!(matches!(m.emoji(), Some(_)));

        let _ = m.insert_special(SpecialKey::Author, "Rachel".to_string(), &converter);
        assert!(matches!(m.author(), Some(_)));

        let _ = m.insert_special(SpecialKey::Source, "Mom's cookbook".to_string(), &converter);
        assert!(matches!(m.source(), Some(_)));

        let _ = m.insert_special(SpecialKey::PrepTime, "3 min".to_string(), &converter);
        assert!(matches!(m.time(), Some(_)));
        m.special.remove(&SpecialKey::Time);

        let _ = m.insert_special(SpecialKey::CookTime, "3 min".to_string(), &converter);
        assert!(matches!(m.time(), Some(_)));
        m.special.remove(&SpecialKey::Time);

        let _ = m.insert_special(SpecialKey::Time, "3 min".to_string(), &converter);
        assert!(matches!(m.time(), Some(_)));
        m.special.remove(&SpecialKey::Time);

        let _ = m.insert_special(SpecialKey::Servings, "3|4".to_string(), &converter);
        assert!(matches!(m.servings(), Some(_)));
    }

    #[test]
    fn shortcode_emoji() {
        let converter = Converter::empty();

        let mut m = Metadata::default();
        let r = m.insert_special(SpecialKey::Emoji, "taco".to_string(), &converter);
        assert!(r.is_err());

        let mut m = Metadata::default();
        let r = m.insert_special(SpecialKey::Emoji, ":taco:".to_string(), &converter);
        assert!(r.is_ok());
        assert_eq!(m.emoji(), Some("🌮"));

        let mut m = Metadata::default();
        let r = m.insert_special(SpecialKey::Emoji, "🌮".to_string(), &converter);
        assert!(r.is_ok());
        assert_eq!(m.emoji(), Some("🌮"));
    }
}
