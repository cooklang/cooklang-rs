//! Metadata of a recipe

use crate::metadata_value::MetadataValue;
use crate::{
    convert::{ConvertError, ConvertTo, ConvertUnit, ConvertValue, PhysicalQuantity, UnknownUnit},
    Converter,
};
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::{borrow::Cow, num::ParseFloatError, str::FromStr};
use thiserror::Error;
use tsify::Tsify;

/// Metadata of a recipe
///
/// You can use [`Metadata::get`] to get a value. The key can be a `&str`, a
/// [`StdKey`] or any [yaml value](serde_yaml::Value). Once you get a
/// [`serde_yaml::Value`], you can use any of it's methods to get your desired
/// type, or any of the [`CooklangValueExt`] which adds more ways to interpret
/// it.
///
/// Many other methods on this struct are a way to access [`StdKey`] with their
/// _expected_ type. If these methods return `None` it can be because the key
/// was not present or the value was not of the expected type. You can also
/// decide to not use them and extract the metadata you prefer.
#[derive(Debug, PartialEq, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct Metadata {
    /// All the raw key/value pairs from the recipe
    pub map: HashMap<String, MetadataValue>,
}

/// Standard keys from the cooklang spec
///
/// These keys are recommended to be used to maximise the compatibility of a
/// recipe between different cooklang applications. You can read more about it
/// in [the spec](https://cooklang.org/docs/spec/#canonical-metadata).
///
/// To use them, use [`Metadata::get`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StdKey {
    Title,
    Description,
    Tags,
    Author,
    Source,
    Course,
    Time,
    PrepTime,
    CookTime,
    Servings,
    Difficulty,
    Cuisine,
    Diet,
    Images,
    Locale,
}

impl std::fmt::Display for StdKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

#[derive(thiserror::Error, Debug, Clone)]
#[error("Faile to parse '{0}' as a standard key")]
pub struct StdKeyParseError(String);

impl FromStr for StdKey {
    type Err = StdKeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = match s {
            "title" => Self::Title,
            "description" | "introduction" => Self::Description,
            "tags" | "tag" => Self::Tags,
            "author" => Self::Author,
            "source" => Self::Source,
            "servings" | "serves" | "yield" => Self::Servings,
            "course" | "category" => Self::Course,
            "locale" => Self::Locale,
            "time" | "duration" | "time required" => Self::Time,
            "prep time" | "prep_time" => Self::PrepTime,
            "cook time" | "cook_time" => Self::CookTime,
            "difficulty" => Self::Difficulty,
            "cuisine" => Self::Cuisine,
            "diet" => Self::Diet,
            "image" | "images" | "picture" | "pictures" => Self::Images,
            _ => return Err(StdKeyParseError(s.to_string())),
        };
        Ok(k)
    }
}

impl AsRef<str> for StdKey {
    fn as_ref(&self) -> &str {
        match self {
            StdKey::Title => "title",
            StdKey::Description => "description",
            StdKey::Tags => "tags",
            StdKey::Author => "author",
            StdKey::Source => "source",
            StdKey::Servings => "servings",
            StdKey::Course => "course",
            StdKey::Locale => "locale",
            StdKey::Time => "time",
            StdKey::PrepTime => "prep time",
            StdKey::CookTime => "cook time",
            StdKey::Difficulty => "difficulty",
            StdKey::Cuisine => "cuisine",
            StdKey::Diet => "diet",
            StdKey::Images => "image",
        }
    }
}

impl Metadata {
    pub fn get(&self, index: impl MetaIndex) -> Option<&MetadataValue> {
        index.index_into(&self.map)
    }

    pub fn get_mut(&mut self, index: impl MetaIndex) -> Option<&mut MetadataValue> {
        index.index_into_mut(&mut self.map)
    }

    /// Iterates over all entries except the standard keys
    pub fn map_filtered(&self) -> impl Iterator<Item = (&String, &MetadataValue)> {
        self.map
            .iter()
            .filter(|(key, _)| StdKey::from_str(key).is_err())
    }

    /// Title of the recipe
    pub fn title(&self) -> Option<&str> {
        self.get(StdKey::Title).and_then(MetadataValue::as_str)
    }

    /// Description of the recipe
    ///
    /// Just the `description` key as a string.
    pub fn description(&self) -> Option<&str> {
        self.get(StdKey::Description)
            .and_then(MetadataValue::as_str)
    }

    /// List of tags
    ///
    /// The `tags` key [`as_tags`](CooklangValueExt::as_tags)
    pub fn tags(&self) -> Option<Vec<Cow<str>>> {
        self.get(StdKey::Tags).and_then(CooklangValueExt::as_tags)
    }

    /// Author
    ///
    /// This *who* wrote the recipe.
    ///
    /// The `author` key [`as_name_and_url`](CooklangValueExt::as_name_and_url).
    pub fn author(&self) -> Option<NameAndUrl> {
        self.get(StdKey::Author)
            .and_then(CooklangValueExt::as_name_and_url)
    }

    /// Source
    ///
    /// This *where* the recipe was obtained from.
    ///
    /// The `source` key [`as_name_and_url`](CooklangValueExt::as_name_and_url).
    pub fn source(&self) -> Option<NameAndUrl> {
        self.get(StdKey::Source)
            .and_then(CooklangValueExt::as_name_and_url)
    }

    /// Time it takes to prepare/cook the recipe
    ///
    /// The `time` key [`as_time`](CooklangValueExt::as_time). Or, if missing,
    /// the combination of the `prep time` and `cook time` keys
    /// [`as_minutes`](CooklangValueExt::as_minutes).
    pub fn time(&self, converter: &Converter) -> Option<RecipeTime> {
        if let Some(time_val) = self.get(StdKey::Time) {
            time_val.as_time(converter)
        } else {
            let prep_time = self
                .get(StdKey::PrepTime)
                .and_then(|v| v.as_minutes(converter));
            let cook_time = self
                .get(StdKey::CookTime)
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
    pub fn servings(&self) -> Option<Servings> {
        self.get(StdKey::Servings).and_then(|v| v.as_servings())
    }

    /// Recipe locale
    /// See [`CooklangValueExt`] for the expected format.
    pub fn locale(&self) -> Option<(&str, Option<&str>)> {
        self.get(StdKey::Locale)
            .and_then(CooklangValueExt::as_locale)
    }
}

pub trait MetaIndex: private::Sealed {
    fn index_into<'a>(&self, m: &'a HashMap<String, MetadataValue>) -> Option<&'a MetadataValue>;
    fn index_into_mut<'a>(
        &self,
        m: &'a mut HashMap<String, MetadataValue>,
    ) -> Option<&'a mut MetadataValue>;
}

mod private {
    use std::borrow::Borrow;

    pub trait Sealed {}
    impl Sealed for super::StdKey {}
    impl Sealed for super::MetadataValue {}
    impl<T> Sealed for T where T: Borrow<str> {}
}

impl MetaIndex for StdKey {
    #[inline]
    fn index_into<'a>(&self, m: &'a HashMap<String, MetadataValue>) -> Option<&'a MetadataValue> {
        m.get(self.as_ref())
    }

    #[inline]
    fn index_into_mut<'a>(
        &self,
        m: &'a mut HashMap<String, MetadataValue>,
    ) -> Option<&'a mut MetadataValue> {
        m.get_mut(self.as_ref())
    }
}

impl<T> MetaIndex for T
where
    T: Borrow<str>,
{
    #[inline]
    fn index_into<'a>(&self, m: &'a HashMap<String, MetadataValue>) -> Option<&'a MetadataValue> {
        m.get(self.borrow())
    }

    #[inline]
    fn index_into_mut<'a>(
        &self,
        m: &'a mut HashMap<String, MetadataValue>,
    ) -> Option<&'a mut MetadataValue> {
        m.get_mut(self.borrow())
    }
}

/// This trait is implemented for [`MetadataValue`] and adds more ways to
/// transform the value from YAML.
pub trait CooklangValueExt: private::Sealed {
    /// Comma (',') separated string or YAML sequence of strings
    ///
    /// Duplicates and empty entries removed.
    fn as_tags<'a>(&'a self) -> Option<Vec<Cow<'a, str>>>;

    /// String separated by `sep` or YAML sequence of strings and/or numbers
    ///
    /// This only checks types and convert numbers to strings if neccesary.
    fn as_string_list<'a>(&'a self, sep: &str) -> Option<Vec<Cow<'a, str>>>;

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
    /// This can be a single number or string like in
    /// [`as_minutes`](CooklangValueExt::as_minutes) or a mapping of `prep_time`
    /// and `cook_time` where each of them is a number or string.
    fn as_time(&self, converter: &Converter) -> Option<RecipeTime>;

    /// Like [`serde_yaml::Value::as_u64`] but ensuring the value fits in a u32
    fn as_u32(&self) -> Option<u32>;

    /// Locale string
    ///
    /// ISO 639 language code, then optionally an underscore and the ISO 3166
    /// alpha2 "country code" for dialect variants.
    ///
    /// **This only check that the value is a string and has the correct
    /// structure**. Therefore it can return non existent/assigned locales.
    /// Capitalisation is not checked, so, for example, `en_gb` works even
    /// though it _should_ be `en_GB`.
    fn as_locale(&self) -> Option<(&str, Option<&str>)>;

    /// String or number as a string
    fn as_str_like(&self) -> Option<Cow<str>>;

    /// Get servings information
    ///
    /// Can be a number or a string that parses to [`Servings`]
    fn as_servings(&self) -> Option<Servings>;
}

impl CooklangValueExt for MetadataValue {
    fn as_tags(&self) -> Option<Vec<Cow<str>>> {
        value_as_tags(self).ok()
    }

    fn as_string_list<'a>(&'a self, sep: &str) -> Option<Vec<Cow<'a, str>>> {
        if let Some(s) = self.as_str() {
            let v = s.split(sep).map(|e| e.into()).collect();
            Some(v)
        } else if let Some(seq) = self.as_sequence() {
            let mut v = Vec::with_capacity(seq.len());
            for e in seq {
                if let Some(s) = e.as_str_like() {
                    v.push(s);
                } else {
                    return None;
                }
            }
            Some(v)
        } else {
            None
        }
    }

    fn as_name_and_url(&self) -> Option<NameAndUrl> {
        if let Some(s) = self.as_str() {
            Some(NameAndUrl::parse(&s))
        } else if let Some(map) = self.as_mapping() {
            let name = map.get("name").and_then(|v| v.as_str());
            let url = map.get("url").and_then(|v| v.as_str());
            if name.is_none() && url.is_none() {
                return None;
            }
            Some(NameAndUrl::new(name, url))
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

    fn as_u32(&self) -> Option<u32> {
        self.as_f64().map(|f| f as u32)
    }

    fn as_locale(&self) -> Option<(&str, Option<&str>)> {
        value_as_locale(self).ok()
    }

    fn as_str_like(&self) -> Option<Cow<str>> {
        if let Some(s) = self.as_str() {
            Some(Cow::from(s))
        } else if let MetadataValue::Number(num) = self {
            Some(Cow::from(num.to_string()))
        } else {
            None
        }
    }

    fn as_servings(&self) -> Option<Servings> {
        // Try as number first
        if let Some(n) = self.as_u32() {
            return Some(Servings::Number(n));
        }

        // Return as text if it's a string
        if let Some(s) = self.as_str() {
            // Try to parse as number
            if let Ok(n) = s.parse::<u32>() {
                Some(Servings::Number(n))
            } else {
                Some(Servings::Text(s.to_string()))
            }
        } else {
            None
        }
    }
}

fn value_as_tags(val: &MetadataValue) -> Result<Vec<Cow<str>>, MetadataError> {
    let entries: Vec<Cow<str>> = if let Some(s) = val.as_str() {
        s.split(',').map(|e| e.trim().into()).collect()
    } else if let Some(seq) = val.as_sequence() {
        seq.iter()
            .map(|val| val.as_str_like())
            .collect::<Option<_>>()
            .ok_or(MetadataError::BadSequenceType {
                expected: MetaType::String,
                got: seq.first().map(MetaType::from).unwrap_or(MetaType::Unknown),
            })?
    } else {
        return Err(MetadataError::expect_type(MetaType::Sequence, val));
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

fn value_as_minutes(val: &MetadataValue, converter: &Converter) -> Result<u32, MetadataError> {
    if let Some(s) = val.as_str() {
        let t = parse_time(s, converter)?;
        Ok(t)
    } else if let Some(n) = val.as_u32() {
        Ok(n)
    } else {
        Err(MetadataError::expect_type(MetaType::String, val))
    }
}

fn value_as_time(val: &MetadataValue, converter: &Converter) -> Result<RecipeTime, MetadataError> {
    let total_res = value_as_minutes(val, converter);
    match total_res {
        Ok(total) => Ok(RecipeTime::Total(total)),
        Err(MetadataError::BadType { .. }) => {
            let map = val
                .as_mapping()
                .ok_or(MetadataError::expect_type(MetaType::Mapping, val))?;
            let prep_time = map
                .get("prep")
                .map(|v| value_as_minutes(v, converter))
                .transpose()?;
            let cook_time = map
                .get("cook")
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

fn value_as_locale(val: &MetadataValue) -> Result<(&str, Option<&str>), MetadataError> {
    let s = val
        .as_str()
        .ok_or(MetadataError::expect_type(MetaType::String, val))?;

    fn validate(s: &str) -> bool {
        s.len() == 2 && s.chars().all(|c| c.is_ascii_alphabetic())
    }

    if let Some((lang, dial)) = s.split_once("_") {
        if validate(lang) && validate(dial) {
            return Ok((lang, Some(dial)));
        }
    } else if validate(s) {
        return Ok((s, None));
    }
    Err(MetadataError::InvalidLocale(s.to_string()))
}

pub(crate) fn check_std_entry(
    key: StdKey,
    value: &MetadataValue,
    converter: &Converter,
) -> Result<(), MetadataError> {
    match key {
        StdKey::Servings => {
            value
                .as_u32()
                .ok_or(MetadataError::expect_type(MetaType::Number, value))?;
        }
        StdKey::Tags => {
            value_as_tags(value)?;
        }
        StdKey::Time => {
            value_as_time(value, converter)?;
        }
        StdKey::PrepTime | StdKey::CookTime => {
            value_as_minutes(value, converter)?;
        }
        StdKey::Title | StdKey::Description => {
            value
                .as_str()
                .ok_or(MetadataError::expect_type(MetaType::String, value))?;
        }
        StdKey::Locale => {
            value_as_locale(value)?;
        }
        // these have no validation
        StdKey::Author | StdKey::Source => {
            value
                .as_name_and_url()
                .ok_or(MetadataError::expect_type(MetaType::Mapping, value))?;
        }
        StdKey::Course => {}
        StdKey::Difficulty => {}
        StdKey::Cuisine => {}
        StdKey::Diet => {}
        StdKey::Images => {}
    }

    Ok(())
}

/// Servings information that can be numeric or a string
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[serde(untagged)]
pub enum Servings {
    /// Numeric servings count
    Number(u32),
    /// String servings (when it can't be parsed as a number)
    Text(String),
}

impl Servings {
    /// Get the numeric value if available
    pub fn as_number(&self) -> Option<u32> {
        match self {
            Servings::Number(n) => Some(*n),
            Servings::Text(_) => None,
        }
    }

    /// Get as text
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Servings::Number(_) => None,
            Servings::Text(s) => Some(s),
        }
    }
}

impl std::fmt::Display for Servings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Servings::Number(n) => write!(f, "{}", n),
            Servings::Text(s) => write!(f, "{}", s),
        }
    }
}

/// Combination of name and URL.
///
/// At least one of the fields is [`Some`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(deny_unknown_fields)]
pub struct NameAndUrl {
    name: Option<String>,
    url: Option<String>,
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
        if let Some(s) = s.trim_ascii_end().strip_suffix('>') {
            if let Some((name, url)) = s.split_once('<') {
                if !url.trim().is_empty() && !url.contains(['<', '>']) {
                    return Self::new(Some(name), Some(url));
                }
            }
        }

        if is_url(s) {
            return Self::new(None, Some(s));
        }

        Self::new(Some(s), None)
    }

    fn new(name: Option<&str>, url: Option<&str>) -> Self {
        fn filter(s: Option<&str>) -> Option<String> {
            s.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        }
        Self {
            name: filter(name),
            url: filter(url),
        }
    }

    /// Get the name
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the url
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }
}

fn is_url(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once("://") else {
        return false;
    };
    if rest.is_empty() || !scheme.chars().all(|c| c.is_alphabetic()) {
        return false;
    }
    let host = match rest.split_once('/') {
        Some((h, _)) => h,
        None => rest,
    };
    if host.is_empty() || host.contains(char::is_whitespace) {
        return false;
    }
    true
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

    // first try a simpler format. Only "HhMm" allowed, no spaces, no other units
    if let Some(minutes) = parse_common_time_format(s) {
        return Ok(minutes);
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

fn parse_common_time_format(s: &str) -> Option<u32> {
    const H_SEP: char = 'h';
    const M_SEP: char = 'm';

    if s.is_empty() {
        return None;
    }

    let mut it = s.split_inclusive(&[H_SEP, M_SEP]);

    let mut total_minutes: u32 = 0;
    let mut hours_found = false;
    loop {
        match it.next() {
            Some(s) if s.ends_with(H_SEP) && !hours_found => {
                let hours = &s[..s.len() - H_SEP.len_utf8()].parse::<u32>().ok()?;
                total_minutes += hours * 60;
                hours_found = true;
            }
            Some(s) if s.ends_with(M_SEP) => {
                let minutes = &s[..s.len() - M_SEP.len_utf8()].parse::<u32>().ok()?;
                total_minutes += minutes;
                break;
            }
            None => break,
            _ => return None,
        }
    }
    if it.next().is_some() {
        return None;
    }
    Some(total_minutes)
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
        let first_non_digit_pos = part.find(|c: char| !c.is_ascii_digit() && c != '.');
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
    #[error("Expected '{expected}' but got '{got}'")]
    BadType { expected: MetaType, got: MetaType },
    #[error("Expected sequence of '{expected}' but got '{got}'")]
    BadSequenceType { expected: MetaType, got: MetaType },
    #[error("Incorrect mapping fields")]
    BadMapping,
    #[error(transparent)]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error(transparent)]
    ParseTimeError(#[from] ParseTimeError),
    #[error("Invalid locale: {0}")]
    InvalidLocale(String),
}

impl MetadataError {
    fn expect_type(expected: MetaType, val: &MetadataValue) -> Self {
        let got = MetaType::from(val);
        if expected == got && expected == MetaType::Mapping {
            return Self::BadMapping;
        }
        Self::BadType { expected, got }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display, strum::AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum MetaType {
    String,
    Bool,
    Number,
    Sequence,
    Mapping,
    Null,
    Unknown,
}

impl From<&MetadataValue> for MetaType {
    fn from(value: &MetadataValue) -> Self {
        match value {
            MetadataValue::Null => Self::Null,
            MetadataValue::Bool(_) => Self::Bool,
            MetadataValue::Number(_) => Self::Number,
            MetadataValue::String(_) => Self::String,
            MetadataValue::Vector(_) => Self::Sequence,
            MetadataValue::Mapping(_) => Self::Mapping,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata_value;

    #[cfg(feature = "bundled_units")]
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
    fn test_common_time() {
        let f = parse_common_time_format;
        assert_eq!(f(""), None);
        assert_eq!(f("1"), None);
        assert_eq!(f("1m"), Some(1));
        assert_eq!(f("1h"), Some(60));
        assert_eq!(f("1h1m"), Some(61));
        assert_eq!(f("1h90m"), Some(150));
        assert_eq!(f("1d1h1m"), None);
        assert_eq!(f("1d1h1m1s"), None);
        assert_eq!(f("1m1s"), None)
    }

    #[test]
    fn special_keys() {
        let t = |k: StdKey| assert_eq!(k, StdKey::from_str(k.as_ref()).unwrap());
        t(StdKey::Title);
        t(StdKey::Description);
        t(StdKey::Tags);
        t(StdKey::Author);
        t(StdKey::Source);
        t(StdKey::Servings);
        t(StdKey::Course);
        t(StdKey::Locale);
        t(StdKey::Time);
        t(StdKey::PrepTime);
        t(StdKey::CookTime);
        t(StdKey::Difficulty);
        t(StdKey::Cuisine);
        t(StdKey::Diet);
        t(StdKey::Images);
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
            "https://rachel.url",
        );
        t(
            "Rachel R. Peterson <https://rachel.url>",
            "Rachel R. Peterson",
            "https://rachel.url",
        );
        t(
            "Rachel Peterson <https://rachel.url>",
            "Rachel Peterson",
            "https://rachel.url",
        );
        t(
            "Rachel Peter-son <https://rachel.url>",
            "Rachel Peter-son",
            "https://rachel.url",
        );
        t(
            "Rachel`s Cookbook <https://rachel.url>",
            "Rachel`s Cookbook",
            "https://rachel.url",
        );
        t(
            "#rachel <https://rachel.url>",
            "#rachel",
            "https://rachel.url",
        );
        t(
            "Rachel: Best recipes <https://rachel.url>",
            "Rachel: Best recipes",
            "https://rachel.url",
        );
        t(
            "Rachel Peterson: Best recipes <https://rachel.url>",
            "Rachel Peterson: Best recipes",
            "https://rachel.url",
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
        t_no_name("https://rachel.url", "https://rachel.url");
        t_no_name("<https://rachel.url>", "https://rachel.url");
        t_no_name("   <https://rachel.url>", "https://rachel.url");
        t_no_name_no_url("");
        t_no_name_no_url("   ");
    }

    #[test]
    fn tags_from_nums() {
        let v: serde_yaml::Value = serde_yaml::from_str("[2022, baking, summer]").unwrap();
        let v: metadata_value::MetadataValue = v.try_into().unwrap();
        let res = value_as_tags(&v).unwrap();
        assert_eq!(
            res,
            vec![Cow::from("2022"), "baking".into(), "summer".into(),]
        );
    }
}
