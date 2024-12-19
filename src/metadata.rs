//! Metadata of a recipe

use std::{num::ParseFloatError, str::FromStr};

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
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
pub struct Metadata {
    /// All the raw key/value pairs from the recipe
    pub map: serde_yaml::Mapping,
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
pub(crate) enum StdKey {
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

impl Metadata {
    pub fn get(&self, index: impl serde_yaml::mapping::Index) -> Option<&serde_yaml::Value> {
        self.map.get(index)
    }

    pub fn get_mut(
        &mut self,
        index: impl serde_yaml::mapping::Index,
    ) -> Option<&mut serde_yaml::Value> {
        self.map.get_mut(index)
    }

    /// Iterates over all entries except the standard keys
    pub fn map_filtered(&self) -> impl Iterator<Item = (&serde_yaml::Value, &serde_yaml::Value)> {
        self.map.iter().filter(|(key, _)| {
            if let Some(key_t) = key.as_str() {
                StdKey::from_str(key_t).is_err()
            } else {
                true
            }
        })
    }

    /// Description of the recipe
    ///
    /// Just the `description` key as a string.
    pub fn description(&self) -> Option<&str> {
        self.get(StdKey::Description.as_ref())
            .and_then(|v| v.as_str())
    }

    /// Emoji for the recipe
    ///
    /// The `emoji` key [`as_emoji`](CooklangValueExt::as_emoji)
    pub fn emoji(&self) -> Option<&str> {
        self.get(StdKey::Emoji.as_ref()).and_then(|v| v.as_emoji())
    }

    /// List of tags
    ///
    /// The `tags` key [`as_tags`](CooklangValueExt::as_tags)
    pub fn tags(&self) -> Option<Vec<&str>> {
        self.get(StdKey::Tags.as_ref()).and_then(|v| v.as_tags())
    }

    /// Author
    ///
    /// This *who* wrote the recipe.
    ///
    /// The `author` key [`as_name_and_url`](CooklangValueExt::as_value_and_url).
    pub fn author(&self) -> Option<NameAndUrl> {
        self.get(StdKey::Author.as_ref())
            .and_then(|v| v.as_name_and_url())
    }

    /// Source
    ///
    /// This *where* the recipe was obtained from.
    ///
    /// The `source` key [`as_name_and_url`](CooklangValueExt::as_value_and_url).
    pub fn source(&self) -> Option<NameAndUrl> {
        self.get(StdKey::Source.as_ref())
            .and_then(|v| v.as_name_and_url())
    }

    /// Time it takes to prepare/cook the recipe
    ///
    /// The `time` key [`as_time`](CooklangValueExt::as_time). Or, if missing,
    /// the combination of the `prep_time` and `cook_time` keys
    /// [`as_minutes`](CooklangValueExt::as_minutes).
    pub fn time(&self, converter: &Converter) -> Option<RecipeTime> {
        if let Some(time_val) = self.get(StdKey::Time.as_ref()) {
            time_val.as_time(converter)
        } else {
            let prep_time = self
                .get(StdKey::PrepTime.as_ref())
                .and_then(|v| v.as_minutes(converter));
            let cook_time = self
                .get(StdKey::CookTime.as_ref())
                .and_then(|v| v.as_minutes(converter));
            if prep_time.is_some() || cook_time.is_some() {
                Some(RecipeTime::Composed {
                    prep_time,
                    cook_time,
                })
            } else {
                None
            }
        }
    }

    /// Servings the recipe is made for
    pub fn servings(&self) -> Option<Vec<u32>> {
        self.get(StdKey::Servings.as_ref())
            .and_then(|v| v.as_servings())
    }
}

pub trait CooklangValueExt: private::Sealed {
    /// Returns `Some` only if the value is a string and it's an emoji.
    ///
    /// It can be a literal emoji (`🦀`) or a shortcode like `:crab:`
    fn as_emoji(&self) -> Option<&str>;

    /// Comma (',') separated string or YAML sequence of string
    ///
    /// Duplicates and empty entries removed.
    fn as_tags(&self) -> Option<Vec<&str>>;

    /// Pipe ('|') separated string or YAML sequence of numbers
    ///
    /// Duplicates not allowed.
    fn as_servings(&self) -> Option<Vec<u32>>;

    /// Get a [`NameAndUrl`]
    ///
    /// This can be a single string or a YAML mapping with `name` and `url` fields.
    /// For the string formats see [`NameAndUrl::parse`].
    fn as_name_and_url(&self) -> Option<NameAndUrl>;

    /// Gets a value as minutes
    ///
    /// It can be a natural (positive) number or a string. The string can have
    /// units and multiple parts. If the units are missing, minutes is assumed.
    ///
    /// Examples:
    /// - `30` 30 minutes
    /// - `1h` 60 minutes
    /// - `1h 30min` 90 minutes
    fn as_minutes(&self, converter: &Converter) -> Option<u32>;

    /// Get a [`RecipeTime`]
    ///
    /// This can be a single number or string like in [`as_minutes`] or a mapping
    /// of `prep_time` and `cook_time` where each of them is a number or string.
    fn as_time(&self, converter: &Converter) -> Option<RecipeTime>;

    /// Like [`serde_yaml::Value::as_u64`] but ensuring the value fits in a u32
    fn as_u32(&self) -> Option<u32>;
}

mod private {
    pub trait Sealed {}
    impl Sealed for serde_yaml::Value {}
}

impl CooklangValueExt for serde_yaml::Value {
    fn as_emoji(&self) -> Option<&str> {
        value_as_emoji(self).ok()
    }

    fn as_tags(&self) -> Option<Vec<&str>> {
        value_as_tags(self).ok()
    }

    fn as_servings(&self) -> Option<Vec<u32>> {
        value_as_servings(self).ok()
    }

    fn as_name_and_url(&self) -> Option<NameAndUrl> {
        if let Some(s) = self.as_str() {
            Some(NameAndUrl::parse(s))
        } else if let Some(map) = self.as_mapping() {
            let name = map.get("name")?.as_str()?;
            let url_str = map.get("url")?.as_str()?;
            let url = Url::parse(url_str).ok()?;
            Some(NameAndUrl::new(Some(name), Some(url)))
        } else {
            None
        }
    }

    fn as_minutes(&self, converter: &Converter) -> Option<u32> {
        value_as_minutes(self, converter).ok()
    }

    fn as_time(&self, converter: &Converter) -> Option<RecipeTime> {
        value_as_time(self, converter).ok()
    }

    #[inline]
    fn as_u32(&self) -> Option<u32> {
        self.as_u64()?.try_into().ok()
    }
}

fn value_as_emoji(val: &serde_yaml::Value) -> Result<&str, MetadataError> {
    let s = val.as_str().ok_or(MetadataError::UnexpectedType)?;
    let emoji = if s.starts_with(':') && s.ends_with(':') {
        emojis::get_by_shortcode(&s[1..s.len() - 1])
    } else {
        emojis::get(s)
    };
    emoji
        .map(|e| e.as_str())
        .ok_or_else(|| MetadataError::NotEmoji {
            value: s.to_string(),
        })
}

fn value_as_tags(val: &serde_yaml::Value) -> Result<Vec<&str>, MetadataError> {
    let entries = if let Some(s) = val.as_str() {
        s.split(',').map(|e| e.trim()).collect()
    } else if let Some(seq) = val.as_sequence() {
        seq.iter()
            .map(|val| val.as_str())
            .collect::<Option<Vec<&str>>>()
            .ok_or(MetadataError::UnexpectedType)?
    } else {
        return Err(MetadataError::UnexpectedType);
    };
    let mut tags = Vec::with_capacity(entries.len());
    for tag in entries {
        if tag.is_empty() || tags.contains(&tag) {
            continue;
        }
        tags.push(tag);
    }
    Ok(tags)
}

fn value_as_servings(val: &serde_yaml::Value) -> Result<Vec<u32>, MetadataError> {
    let servings: Vec<u32> = if let Some(s) = val.as_str() {
        s.split('|')
            .map(str::trim)
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?
    } else if let Some(seq) = val.as_sequence() {
        let mut v = Vec::with_capacity(seq.len());
        for e in seq {
            let n = e.as_u32().ok_or(MetadataError::UnexpectedType)?;
            v.push(n);
        }
        v
    } else {
        return Err(MetadataError::UnexpectedType);
    };

    let l = servings.len();
    let dedup_l = {
        let mut temp = servings.clone();
        temp.sort_unstable();
        temp.dedup();
        temp.len()
    };
    if l != dedup_l {
        return Err(MetadataError::DuplicateServings { servings });
    }
    Ok(servings)
}

fn value_as_minutes(val: &serde_yaml::Value, converter: &Converter) -> Result<u32, MetadataError> {
    if let Some(s) = val.as_str() {
        let t = parse_time(s, converter)?;
        Ok(t)
    } else if let Some(n) = val.as_u32() {
        Ok(n)
    } else {
        Err(MetadataError::UnexpectedType)
    }
}

fn value_as_time(
    val: &serde_yaml::Value,
    converter: &Converter,
) -> Result<RecipeTime, MetadataError> {
    let total_res = value_as_minutes(val, converter);
    match total_res {
        Ok(total) => Ok(RecipeTime::Total(total)),
        Err(MetadataError::UnexpectedType) => {
            let map = val.as_mapping().ok_or(MetadataError::UnexpectedType)?;
            let prep_time = map
                .get(StdKey::PrepTime.as_ref())
                .map(|v| value_as_minutes(v, converter))
                .transpose()?;
            let cook_time = map
                .get(StdKey::CookTime.as_ref())
                .map(|v| value_as_minutes(v, converter))
                .transpose()?;
            Ok(RecipeTime::Composed {
                prep_time,
                cook_time,
            })
        }
        Err(other) => Err(other),
    }
}

pub(crate) fn check_std_entry(
    key: StdKey,
    value: &serde_yaml::Value,
    converter: &Converter,
) -> Result<Option<crate::scale::Servings>, MetadataError> {
    match key {
        StdKey::Tags => value_as_tags(value).map(|_| None),
        StdKey::Emoji => value_as_emoji(value).map(|_| None),
        StdKey::Time => value_as_time(value, converter).map(|_| None),
        StdKey::PrepTime | StdKey::CookTime => value_as_minutes(value, converter).map(|_| None),
        StdKey::Servings => value_as_servings(value).map(|s| Some(crate::scale::Servings(Some(s)))),
        _ => Ok(None),
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
    /// - `<Url>`
    /// - `Name`
    ///
    /// The Url validated, so it has to be correct. If no url is found or it's
    /// invalid, everything will be the name.
    pub fn parse(s: &str) -> Self {
        let regex_encapsulated_url = regex!(r"^([^<]*)<([^<>]+)>$");
        if let Some(captures) = regex_encapsulated_url.captures(s) {
            if let Ok(url) = Url::parse(captures[2].trim()) {
                // if the user has written the URL inside '<..>', keep it even
                // if it has no host
                return Self::new(Some(&captures[1]), Some(url));
            }
        }

        if let Ok(url) = Url::parse(s.trim()) {
            // Safety check so a URL like `Rachel: best recipes`, where "Rachel"
            // is the protocol, doesn't get detected.
            if url.has_host() {
                return Self::new(None, Some(url));
            }
        }

        Self::new(Some(s), None)
    }

    fn new(name: Option<&str>, url: Option<Url>) -> Self {
        let name = name
            .map(|n| n.trim())
            .filter(|n| !n.is_empty())
            .map(String::from);
        Self { name, url }
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
    #[error("Unexpected value data type")]
    UnexpectedType,
    #[error("Value is not an emoji: {value}")]
    NotEmoji { value: String },
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Duplicate servings: {servings:?}")]
    DuplicateServings { servings: Vec<u32> },
    #[error(transparent)]
    ParseTimeError(#[from] ParseTimeError),
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let t = |s: &str, key: StdKey| {
            assert_eq!(StdKey::from_str(s).unwrap(), key);
            assert_eq!(key.to_string(), s);
        };
        let t_alias = |s: &str, key: StdKey| {
            assert_eq!(StdKey::from_str(s).unwrap(), key);
            assert_ne!(key.to_string(), s);
        };

        t("description", StdKey::Description);
        t("tags", StdKey::Tags);
        t_alias("tag", StdKey::Tags);
        t("emoji", StdKey::Emoji);
        t("author", StdKey::Author);
        t("source", StdKey::Source);
        t("time", StdKey::Time);
        t("prep time", StdKey::PrepTime);
        t_alias("prep_time", StdKey::PrepTime);
        t("cook time", StdKey::CookTime);
        t_alias("cook_time", StdKey::CookTime);
        t("servings", StdKey::Servings);
    }

    #[test]
    fn parse_name_and_url() {
        let t = |s: &str, name: &str, url: &str| {
            let name_and_url = NameAndUrl::parse(s);
            assert_eq!(name_and_url.url.as_ref().unwrap().as_str(), url);
            assert_eq!(name_and_url.name.as_ref().unwrap().as_str(), name);
        };

        let t_no_url = |s: &str, name: &str| {
            let name_and_url = NameAndUrl::parse(s);
            assert_eq!(name_and_url.name.as_ref().unwrap().as_str(), name);
            assert_eq!(name_and_url.url, None);
        };

        let t_no_name = |s: &str, url: &str| {
            let name_and_url = NameAndUrl::parse(s);
            assert_eq!(name_and_url.name, None);
            assert_eq!(name_and_url.url.as_ref().unwrap().as_str(), url);
        };

        let t_no_name_no_url = |s: &str| {
            let name_and_url = NameAndUrl::parse(s);
            assert_eq!(name_and_url.name, None);
            assert_eq!(name_and_url.url, None);
        };

        t(
            "Rachel <https://rachel.url>",
            "Rachel",
            "https://rachel.url/",
        );
        t(
            "Rachel R. Peterson <https://rachel.url>",
            "Rachel R. Peterson",
            "https://rachel.url/",
        );
        t(
            "Rachel Peterson <https://rachel.url>",
            "Rachel Peterson",
            "https://rachel.url/",
        );
        t(
            "Rachel Peter-son <https://rachel.url>",
            "Rachel Peter-son",
            "https://rachel.url/",
        );
        t(
            "Rachel`s Cookbook <https://rachel.url>",
            "Rachel`s Cookbook",
            "https://rachel.url/",
        );
        t(
            "#rachel <https://rachel.url>",
            "#rachel",
            "https://rachel.url/",
        );
        t(
            "Rachel: Best recipes <https://rachel.url>",
            "Rachel: Best recipes",
            "https://rachel.url/",
        );
        t(
            "Rachel Peterson: Best recipes <https://rachel.url>",
            "Rachel Peterson: Best recipes",
            "https://rachel.url/",
        );
        t(
            "Rachel Peterson: Best recipes <smb://rachel.url>",
            "Rachel Peterson: Best recipes",
            "smb://rachel.url",
        );
        t_no_url("Rachel", "Rachel");
        t_no_url("Rachel Peterson", "Rachel Peterson");
        t_no_url("Rachel R. Peterson", "Rachel R. Peterson");
        t_no_url("Rachel Peter-son", "Rachel Peter-son");
        t_no_url("Rachel`s Cookbook", "Rachel`s Cookbook");
        t_no_url("Rachel's Cookbook", "Rachel's Cookbook");
        t_no_url("#rachel", "#rachel");
        t_no_url("<#rach>el", "<#rach>el");
        t_no_url("<>", "<>");
        t_no_url("< >", "< >");
        t_no_url("Rachel:// Peterson", "Rachel:// Peterson");
        t_no_url("Rachel: Best recipes", "Rachel: Best recipes");
        t_no_url(
            "Rachel <https://two.rachel.url> <https://rachel.url>",
            "Rachel <https://two.rachel.url> <https://rachel.url>",
        );
        t_no_url(
            "Rachel <<https://bad.rachel.url>",
            "Rachel <<https://bad.rachel.url>",
        );
        t_no_name("https://rachel.url", "https://rachel.url/");
        t_no_name("<https://rachel.url>", "https://rachel.url/");
        t_no_name("   <https://rachel.url>", "https://rachel.url/");
        t_no_name_no_url("");
        t_no_name_no_url("   ");
    }

    #[test]
    fn shortcode_emoji() {
        let v = serde_yaml::Value::from("taco");
        let r = value_as_emoji(&v);
        assert!(r.is_err());

        let v = serde_yaml::Value::from(":taco:");
        let r = value_as_emoji(&v);
        assert_eq!(r.unwrap(), "🌮");

        let v = serde_yaml::Value::from("🌮");
        let r = value_as_emoji(&v);
        assert_eq!(r.unwrap(), "🌮");
    }
}
