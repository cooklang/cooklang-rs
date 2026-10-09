//! Metadata of a recipe

use crate::float::round_f64;
use std::{borrow::Cow, num::ParseFloatError, str::FromStr};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::{
    convert::{ConvertError, ConvertTo, ConvertUnit, ConvertValue, PhysicalQuantity, UnknownUnit},
    quantity::Value,
    semantic_eq::{unordered_equals, SemanticEq},
    Converter, Quantity,
};

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
    #[cfg_attr(feature = "ts", tsify(type = "Record<any, any>"))]
    pub map: serde_yaml::Mapping,
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
    /// A plain count of servings, e.g. `servings: 4`. See [`Metadata::servings`].
    Servings,
    /// How much the recipe makes as a `value%unit` string, e.g. `yield: 500%g`.
    ///
    /// See [`Metadata::yield_quantity`]. Also matches the `yields` key.
    Yield,
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
#[error("Failed to parse '{0}' as a standard key")]
pub struct StdKeyParseError(String);

impl FromStr for StdKey {
    type Err = StdKeyParseError;

    // TODO: error: alternative names are neither used as aliases nor emitted as custom tags
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let k = match s {
            "title" => Self::Title,
            "description" | "introduction" => Self::Description,
            "tags" | "tag" => Self::Tags,
            "author" => Self::Author,
            "source" => Self::Source,
            "servings" | "serves" => Self::Servings,
            "yield" | "yields" => Self::Yield,
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
            StdKey::Yield => "yield",
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
    pub fn get(&self, index: impl MetaIndex) -> Option<&serde_yaml::Value> {
        index.index_into(&self.map)
    }

    pub fn get_mut(&mut self, index: impl MetaIndex) -> Option<&mut serde_yaml::Value> {
        index.index_into_mut(&mut self.map)
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

    /// Title of the recipe
    pub fn title(&self) -> Option<&str> {
        self.get(StdKey::Title).and_then(serde_yaml::Value::as_str)
    }

    /// Description of the recipe
    ///
    /// Just the `description` key as a string.
    pub fn description(&self) -> Option<&str> {
        self.get(StdKey::Description)
            .and_then(serde_yaml::Value::as_str)
    }

    /// List of tags
    ///
    /// The `tags` key [`as_tags`](CooklangValueExt::as_tags)
    pub fn tags(&self) -> Option<Vec<Cow<'_, str>>> {
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

    /// Servings the recipe is made for (a plain count, e.g. `servings: 4`)
    ///
    /// Recognises the `servings` and `serves` key aliases.
    pub fn servings(&self) -> Option<Servings> {
        first_std_key_value(&self.map, StdKey::Servings).and_then(|v| v.as_servings())
    }

    /// Yield of the recipe as a [`Quantity`]: how much it makes, as opposed
    /// to how many portions it divides into ([`Self::servings`])
    ///
    /// Recognises the `yield` and `yields` keys. The value is a single
    /// positive, finite number with an optional `%unit`, the same separator
    /// quantities use: `500%g`, `500 % g`, `2.5` and `"500"` all parse.
    /// `2.5 dl` (no separator), `1/2%l`, `1-2%l`, `-3%g` and `0` do not.
    ///
    /// Returns `None` if the key is absent or the value does not parse; such a
    /// value stays a regular metadata entry and is reported as a warning.
    pub fn yield_quantity(&self) -> Option<Quantity> {
        first_std_key_value(&self.map, StdKey::Yield).and_then(|v| v.as_yield())
    }

    /// Recipe locale
    /// See [`CooklangValueExt`] for the expected format.
    pub fn locale(&self) -> Option<(&str, Option<&str>)> {
        self.get(StdKey::Locale)
            .and_then(CooklangValueExt::as_locale)
    }
}

/// Compares two metadata structs with semantic equality for standard keys.
///
/// Unlike [`PartialEq`], key aliases (e.g. `serves`/`servings`) resolve to
/// the same key, numeric servings are compared with tolerance, and yield is
/// compared as a unit-aware [`Quantity`].
impl SemanticEq for Metadata {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        // a value neither side can read is still part of what the recipe says,
        // so it falls back to comparing it raw rather than merging every value
        // that fails to parse
        let servings_eq =
            std_key_values_eq(&self.map, &other.map, StdKey::Servings, |a, b| {
                match (a.as_servings(), b.as_servings()) {
                    (Some(x), Some(y)) => x.equals(&y, converter),
                    _ => a == b,
                }
            });

        let yield_eq = std_key_values_eq(&self.map, &other.map, StdKey::Yield, |a, b| {
            match (a.as_yield(), b.as_yield()) {
                (Some(x), Some(y)) => x.equals(&y, converter),
                _ => a == b,
            }
        });

        let simple_std_eq = [
            StdKey::Title,
            StdKey::Description,
            StdKey::Tags,
            StdKey::Author,
            StdKey::Source,
            StdKey::Course,
            StdKey::Difficulty,
            StdKey::Cuisine,
            StdKey::Diet,
            StdKey::Images,
            StdKey::Locale,
        ]
        .iter()
        .all(|k| std_key_values_eq(&self.map, &other.map, *k, |a, b| a == b));

        let time_eq = std_key_values_eq(&self.map, &other.map, StdKey::Time, |a, b| {
            parsed_eq(a, b, |v| value_as_time(v, converter).ok())
        }) && [StdKey::PrepTime, StdKey::CookTime].iter().all(|k| {
            std_key_values_eq(&self.map, &other.map, *k, |a, b| {
                parsed_eq(a, b, |v| value_as_minutes(v, converter).ok())
            })
        });

        let a_custom: Vec<_> = self.map_filtered().collect();
        let b_custom: Vec<_> = other.map_filtered().collect();
        let custom_eq = unordered_equals(&a_custom, &b_custom, converter);

        servings_eq && yield_eq && simple_std_eq && time_eq && custom_eq
    }
}

/// Compares two values by their parsed form, falling back to comparing them raw
/// when either side fails to parse.
fn parsed_eq<T: PartialEq>(
    a: &serde_yaml::Value,
    b: &serde_yaml::Value,
    parse: impl Fn(&serde_yaml::Value) -> Option<T>,
) -> bool {
    match (parse(a), parse(b)) {
        (Some(pa), Some(pb)) => pa == pb,
        _ => a == b,
    }
}

/// Compares the value of every entry written under any alias of `sk`, in any
/// order.
///
/// A recipe can spell the same standard key more than once (`servings` and
/// `serves`), and all of them are part of what it says, so comparing only the
/// first would let the rest differ unnoticed.
fn std_key_values_eq(
    a: &serde_yaml::Mapping,
    b: &serde_yaml::Mapping,
    sk: StdKey,
    mut value_eq: impl FnMut(&serde_yaml::Value, &serde_yaml::Value) -> bool,
) -> bool {
    let a: Vec<_> = std_key_entries(a, sk).map(|(_, v)| v).collect();
    let mut b: Vec<_> = std_key_entries(b, sk).map(|(_, v)| v).collect();
    a.len() == b.len()
        && a.iter()
            .all(|x| match b.iter().position(|y| value_eq(x, y)) {
                Some(i) => {
                    b.swap_remove(i);
                    true
                }
                None => false,
            })
}

/// Look up a value in a mapping by any alias of the given [`StdKey`].
fn first_std_key_value(m: &serde_yaml::Mapping, sk: StdKey) -> Option<&serde_yaml::Value> {
    std_key_entries(m, sk).next().map(|(_, v)| v)
}

/// The key of the first entry written with any alias of the given [`StdKey`].
fn find_std_key(m: &serde_yaml::Mapping, sk: StdKey) -> Option<&serde_yaml::Value> {
    std_key_entries(m, sk).next().map(|(k, _)| k)
}

/// Every entry written with any alias of the given [`StdKey`].
fn std_key_entries(
    m: &serde_yaml::Mapping,
    sk: StdKey,
) -> impl Iterator<Item = (&serde_yaml::Value, &serde_yaml::Value)> {
    m.iter().filter(move |(k, _)| {
        k.as_str()
            .and_then(|s| StdKey::from_str(s).ok())
            .is_some_and(|found| found == sk)
    })
}

/// A servings count and a yield are both amounts a recipe makes, and scaling
/// only ever multiplies them, so nothing at or below zero could become one.
///
/// The warning report is not the only reader of these keys, so this has to hold
/// wherever a value is handed to a caller, not only in [`check_std_entry`].
pub(crate) fn is_scalable_amount(n: f64) -> bool {
    n.is_finite() && n > 0.0
}

/// Split a YAML value stored as a yield into its amount and unit, without
/// judging the amount.
///
/// Accepts `"500%g"` (value + unit) or a plain YAML number/string like `"4"` (no unit).
fn split_yield_value(v: &serde_yaml::Value) -> Option<(f64, Option<String>)> {
    match v {
        serde_yaml::Value::Number(n) => Some((n.as_f64()?, None)),
        serde_yaml::Value::String(s) => {
            if let Some((val_str, unit)) = s.split_once('%') {
                Some((val_str.trim().parse().ok()?, Some(unit.trim().to_string())))
            } else {
                Some((s.trim().parse().ok()?, None))
            }
        }
        _ => None,
    }
}

pub trait MetaIndex: private::Sealed {
    fn index_into<'a>(&self, m: &'a serde_yaml::Mapping) -> Option<&'a serde_yaml::Value>;
    fn index_into_mut<'a>(
        &self,
        m: &'a mut serde_yaml::Mapping,
    ) -> Option<&'a mut serde_yaml::Value>;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::StdKey {}
    impl<T> Sealed for T where T: serde_yaml::mapping::Index {}
}

impl MetaIndex for StdKey {
    #[inline]
    fn index_into<'a>(&self, m: &'a serde_yaml::Mapping) -> Option<&'a serde_yaml::Value> {
        first_std_key_value(m, *self)
    }

    #[inline]
    fn index_into_mut<'a>(
        &self,
        m: &'a mut serde_yaml::Mapping,
    ) -> Option<&'a mut serde_yaml::Value> {
        // the entry is looked up by alias, so it has to be written back through
        // the key as the recipe spelled it
        let key = find_std_key(m, *self)?.clone();
        m.get_mut(&key)
    }
}

impl<T> MetaIndex for T
where
    T: serde_yaml::mapping::Index,
{
    #[inline]
    fn index_into<'a>(&self, m: &'a serde_yaml::Mapping) -> Option<&'a serde_yaml::Value> {
        m.get(self)
    }

    #[inline]
    fn index_into_mut<'a>(
        &self,
        m: &'a mut serde_yaml::Mapping,
    ) -> Option<&'a mut serde_yaml::Value> {
        m.get_mut(self)
    }
}

/// This trait is implemented for [`serde_yaml::Value`] and adds more ways to
/// transform the value from YAML.
pub trait CooklangValueExt: private::Sealed {
    /// Comma (',') separated string or YAML sequence of strings
    ///
    /// Duplicates and empty entries removed.
    fn as_tags(&self) -> Option<Vec<Cow<'_, str>>>;

    /// String separated by `sep` or YAML sequence of strings and/or numbers
    ///
    /// This only checks types and convert numbers to strings if neccesary.
    fn as_string_list<'a>(&'a self, sep: &str) -> Option<Vec<std::borrow::Cow<'a, str>>>;

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
    fn as_str_like(&self) -> Option<Cow<'_, str>>;

    /// Get servings information
    ///
    /// Can be a number or a string that parses to [`Servings`]
    fn as_servings(&self) -> Option<Servings>;

    /// Get the yield as a [`Quantity`]
    ///
    /// Can be a number or a `value%unit` string (unit is optional). `None`
    /// unless the amount is positive and finite.
    fn as_yield(&self) -> Option<Quantity>;
}

impl CooklangValueExt for serde_yaml::Value {
    fn as_tags(&self) -> Option<Vec<Cow<'_, str>>> {
        value_as_tags(self).ok()
    }

    fn as_string_list<'a>(&'a self, sep: &str) -> Option<Vec<Cow<'a, str>>> {
        if let Some(s) = self.as_str() {
            let v = s.split(sep).map(|e| e.into()).collect();
            Some(v)
        } else if let Some(seq) = self.as_sequence() {
            let mut v = Vec::<std::borrow::Cow<'a, str>>::with_capacity(seq.len());
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
        if let Some(s) = self.as_str_like() {
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
        self.as_u64()?.try_into().ok()
    }

    fn as_locale(&self) -> Option<(&str, Option<&str>)> {
        value_as_locale(self).ok()
    }

    fn as_str_like(&self) -> Option<Cow<'_, str>> {
        if let Some(s) = self.as_str() {
            Some(Cow::from(s))
        } else if let serde_yaml::Value::Number(num) = self {
            Some(Cow::from(num.to_string()))
        } else {
            None
        }
    }

    fn as_servings(&self) -> Option<Servings> {
        if let Some(n) = self.as_f64() {
            return is_scalable_amount(n).then_some(Servings::Number(n));
        }

        let s = self.as_str()?;
        match s.parse::<f64>() {
            // a count written as a string is still a count, and still has to be one
            Ok(n) => is_scalable_amount(n).then_some(Servings::Number(n)),
            Err(_) => Some(Servings::Text(s.to_string())),
        }
    }

    fn as_yield(&self) -> Option<Quantity> {
        let (amount, unit) = split_yield_value(self)?;
        // a caller must never scale by an amount `check_std_entry` already
        // reported as invalid
        is_scalable_amount(amount).then(|| Quantity::new(Value::from(amount), unit))
    }
}

fn value_as_tags(val: &serde_yaml::Value) -> Result<Vec<Cow<'_, str>>, MetadataError> {
    let entries = if let Some(s) = val.as_str() {
        s.split(',').map(|e| e.trim().into()).collect()
    } else if let Some(seq) = val.as_sequence() {
        seq.iter()
            .map(|val| val.as_str_like())
            .collect::<Option<Vec<_>>>()
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

fn value_as_minutes(val: &serde_yaml::Value, converter: &Converter) -> Result<u32, MetadataError> {
    if let Some(s) = val.as_str() {
        let t = parse_time(s, converter)?;
        Ok(t)
    } else if let Some(n) = val.as_u32() {
        Ok(n)
    } else {
        Err(MetadataError::expect_type(MetaType::String, val))
    }
}

fn value_as_time(
    val: &serde_yaml::Value,
    converter: &Converter,
) -> Result<RecipeTime, MetadataError> {
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

fn value_as_locale(val: &serde_yaml::Value) -> Result<(&str, Option<&str>), MetadataError> {
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
    value: &serde_yaml::Value,
    converter: &Converter,
) -> Result<(), MetadataError> {
    match key {
        StdKey::Servings => {
            let servings = value
                .as_f64()
                .ok_or(MetadataError::expect_type(MetaType::Number, value))?;
            if !is_scalable_amount(servings) {
                return Err(MetadataError::expect_type(MetaType::Number, value));
            }
        }
        StdKey::Yield => {
            // reported apart from the amount check, so a value that is no
            // quantity at all does not report the wrong reason
            let Some((amount, _)) = split_yield_value(value) else {
                return Err(MetadataError::expect_type(MetaType::String, value));
            };
            if !is_scalable_amount(amount) {
                return Err(MetadataError::expect_type(MetaType::Number, value));
            }
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
///
/// [`PartialEq`] compares the value as written, so `4.0` and `4.00001` differ.
/// Use [`SemanticEq::equals`] to compare them the way a cook would.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
#[serde(untagged)]
pub enum Servings {
    /// Numeric servings count
    Number(f64),
    /// String servings (when it can't be parsed as a number)
    Text(String),
}

/// A custom metadata value carries no unit information, so it means what it
/// says: this compares as written, like [`PartialEq`].
impl SemanticEq for serde_yaml::Value {
    fn equals(&self, other: &Self, _converter: &Converter) -> bool {
        self == other
    }
}

/// Compares two servings counts, allowing the rounding error scaling
/// introduces.
///
/// Unlike [`PartialEq`], numeric servings are compared rounded one digit finer
/// than [`Recipe::scale`](crate::Recipe::scale) writes them, so a count scaled
/// to `2.5` equals one written as `2.50001` while `4.05` and `4.1` stay apart.
/// Text servings are compared as written.
impl SemanticEq for Servings {
    fn equals(&self, other: &Self, _converter: &Converter) -> bool {
        match (self, other) {
            (Servings::Number(a), Servings::Number(b)) => {
                round_f64(*a, Servings::COMPARE_PRECISION)
                    == round_f64(*b, Servings::COMPARE_PRECISION)
            }
            (Servings::Text(a), Servings::Text(b)) => a == b,
            _ => false,
        }
    }
}

impl Servings {
    /// A count is a number of people, so a scaled count is written to one
    /// decimal: more would claim a precision the recipe never had
    pub(crate) const WRITE_PRECISION: u32 = 1;

    /// Comparing one digit finer than the writer keeps every count the writer
    /// could produce distinguishable, while absorbing the float error it leaves
    /// behind
    const COMPARE_PRECISION: u32 = Self::WRITE_PRECISION + 1;

    /// The servings scaled by `factor`, with the count rounded to the precision
    /// a servings count is written with
    ///
    /// Text servings carry no count to scale, so they are returned unchanged.
    pub(crate) fn scaled(&self, factor: f64) -> Self {
        match self {
            Servings::Number(n) => Servings::Number(round_f64(n * factor, Self::WRITE_PRECISION)),
            Servings::Text(_) => self.clone(),
        }
    }

    /// Get the numeric value if available
    pub fn as_number(&self) -> Option<f64> {
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
            Servings::Number(n) => write!(f, "{n}"),
            Servings::Text(s) => write!(f, "{s}"),
        }
    }
}

/// Combination of name and URL.
///
/// At least one of the fields is [`Some`].
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(deny_unknown_fields)]
pub struct NameAndUrl {
    #[cfg_attr(feature = "ts", tsify(optional))]
    name: Option<String>,
    #[cfg_attr(feature = "ts", tsify(optional))]
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
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(untagged, deny_unknown_fields)]
pub enum RecipeTime {
    /// Total time
    Total(u32),
    /// Combination of preparation and cook time
    ///
    /// At least one is [`Some`]
    Composed {
        #[serde(alias = "prep")]
        #[cfg_attr(feature = "ts", tsify(optional))]
        prep_time: Option<u32>,
        #[serde(alias = "cook")]
        #[cfg_attr(feature = "ts", tsify(optional))]
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
    fn expect_type(expected: MetaType, val: &serde_yaml::Value) -> Self {
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

impl From<&serde_yaml::Value> for MetaType {
    fn from(value: &serde_yaml::Value) -> Self {
        match value {
            serde_yaml::Value::Null => Self::Null,
            serde_yaml::Value::Bool(_) => Self::Bool,
            serde_yaml::Value::Number(_) => Self::Number,
            serde_yaml::Value::String(_) => Self::String,
            serde_yaml::Value::Sequence(_) => Self::Sequence,
            serde_yaml::Value::Mapping(_) => Self::Mapping,
            serde_yaml::Value::Tagged(_) => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn servings_must_be_a_positive_finite_number() {
        let converter = Converter::empty();
        let check = |yaml: &str| {
            let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
            check_std_entry(StdKey::Servings, &value, &converter).is_ok()
        };

        assert!(check("4"));
        assert!(check("4.5"));

        // a count of servings has to be something you can divide a recipe into
        assert!(!check("-4"));
        assert!(!check("0"));
        assert!(!check(".nan"));
        assert!(!check(".inf"));
        assert!(!check("-.inf"));
    }

    #[test]
    fn a_yield_must_be_a_positive_finite_amount() {
        let converter = Converter::empty();
        let check = |yaml: &str| {
            let value: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
            check_std_entry(StdKey::Yield, &value, &converter).is_ok()
        };

        assert!(check("500"));
        assert!(check("2.5"));
        assert!(check(r#""500%g""#));
        assert!(check(r#""0.5 % kg""#));

        // a yield is an amount the recipe makes, and scaling only multiplies
        // it, so nothing at or below zero can ever become one
        assert!(!check(r#""-3%g""#));
        assert!(!check("-3"));
        assert!(!check("0"));
        assert!(!check(r#""0%g""#));
        assert!(!check(".nan"));
        assert!(!check(".inf"));
        assert!(!check("-.inf"));

        // and it still has to be a quantity at all
        assert!(!check(r#""2.5 dl""#));
        assert!(!check(r#""a lot""#));
    }

    #[test]
    fn a_value_that_is_not_a_count_is_not_read_as_servings() {
        let servings = |yaml: &str| {
            serde_yaml::from_str::<serde_yaml::Value>(yaml)
                .unwrap()
                .as_servings()
        };

        assert_eq!(servings("4"), Some(Servings::Number(4.0)));
        assert_eq!(servings("4.5"), Some(Servings::Number(4.5)));
        assert_eq!(servings(r#""4""#), Some(Servings::Number(4.0)));
        assert_eq!(
            servings(r#""serves 4 people""#),
            Some(Servings::Text("serves 4 people".into()))
        );

        // the warning report is not the only reader, so a value handed to a
        // caller carries the same rule check_std_entry applies
        assert_eq!(servings("-4"), None);
        assert_eq!(servings("0"), None);
        assert_eq!(servings(".nan"), None);
        assert_eq!(servings(".inf"), None);
        assert_eq!(servings(r#""-4""#), None);
    }

    #[test]
    fn an_amount_that_cannot_be_made_is_not_read_as_a_yield() {
        let qty = |yaml: &str| {
            serde_yaml::from_str::<serde_yaml::Value>(yaml)
                .unwrap()
                .as_yield()
                .map(|q| q.to_string())
        };

        assert_eq!(qty(r#""500%g""#), Some("500 g".to_string()));
        assert_eq!(qty("2.5"), Some("2.5".to_string()));

        assert_eq!(qty(r#""-3%g""#), None);
        assert_eq!(qty("0"), None);
        assert_eq!(qty(".nan"), None);
        assert_eq!(qty(".inf"), None);
    }

    #[test]
    fn metadata_equals_compares_a_value_it_cannot_read_as_written() {
        let converter = Converter::empty();
        let meta = |yaml: &str| Metadata {
            map: serde_yaml::from_str(yaml).unwrap(),
        };

        // two values that are not counts are still two different values, so
        // failing to read them must not merge them
        assert!(!meta("servings: -4").equals(&meta("servings: -5"), &converter));
        assert!(meta("servings: -4").equals(&meta("servings: -4"), &converter));
        assert!(!meta("yield: -3%g").equals(&meta("yield: -5%g"), &converter));
    }

    #[test]
    fn metadata_equals_compares_the_spellings_of_a_key_in_any_order() {
        let converter = Converter::empty();
        let meta = |yaml: &str| Metadata {
            map: serde_yaml::from_str(yaml).unwrap(),
        };

        assert!(meta("servings: 4\nserves: 8").equals(&meta("serves: 8\nservings: 4"), &converter));
        assert!(!meta("servings: 4\nserves: 8").equals(&meta("servings: 4\nserves: 9"), &converter));
    }

    #[test]
    fn servings_partial_eq_is_exact_and_semantic_eq_rounds_to_two_decimals() {
        // a count has no unit, so the converter has nothing to say about it and
        // an empty one gives the same answer as the bundled one
        let converter = Converter::empty();
        let n = Servings::Number;

        // PartialEq compares as written
        assert_ne!(n(4.0), n(4.000_1));
        assert_eq!(n(4.0), n(4.0));

        // equals absorbs the rounding error scaling introduces, because both
        // counts round to the same two decimals
        assert!(n(4.0).equals(&n(4.000_1), &converter));
        assert!(n(4.0).equals(&n(4.004), &converter));
        // a difference a recipe could write down survives the rounding
        assert!(!n(4.0).equals(&n(4.05), &converter));
        assert!(!n(2.5).equals(&n(2.0), &converter));

        // comparing is one digit finer than writing, so two counts the writer
        // could tell apart are never merged: at WRITE_PRECISION both of these
        // round to 4.1
        assert!(!n(4.05).equals(&n(4.1), &converter));
        assert_eq!(Servings::COMPARE_PRECISION, Servings::WRITE_PRECISION + 1);

        // scaling rounds to the precision a count is written with
        assert_eq!(n(4.0).scaled(1.0 / 3.0), n(1.3));
        assert_eq!(
            Servings::Text("two".into()).scaled(2.0),
            Servings::Text("two".into())
        );

        // text servings compare as written either way
        assert!(Servings::Text("two".into()).equals(&Servings::Text("two".into()), &converter));
        assert!(!Servings::Text("two".into()).equals(&n(2.0), &converter));
    }

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
        t(StdKey::Yield);
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
        let v = serde_yaml::from_str("[2022, baking, summer]").unwrap();
        let res = value_as_tags(&v).unwrap();
        assert_eq!(
            res,
            vec![
                Cow::Owned(String::from("2022")),
                Cow::Borrowed("baking"),
                Cow::Borrowed("summer"),
            ]
        );
    }
}
