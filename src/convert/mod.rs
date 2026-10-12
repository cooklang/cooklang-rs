//! The converter. [`crate::convert_legacy`] is the old one, kept until the
//! swap is reviewed: each `_legacy` function on a recipe or quantity is the
//! old form of the one without the suffix, and goes with the old converter
//! (ADR 0009).
//!
//! A [`Converter`] reads and writes units the way one locale names them,
//! and fits them for one audience: the legacy locale and the system a
//! quantity was written in for a recipe with no `locale`, else what the
//! recipe's tag determines.
//!
//! # Differences from the legacy converter
//!
//! - Converting a [`Quantity`] keeps its `scalable` flag. The legacy
//!   converter rebuilds the quantity with [`Quantity::new`], which resets it
//!   to `false`.

pub(crate) mod locales;
pub(crate) mod units;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::metadata::Metadata;
use crate::model::Recipe;
use crate::quantity::{Number, Quantity, Value};
use locales::legacy::{legacy_locale, legacy_system};
use locales::locale::Locale;
use locales::preference::{as_written, best_amount, Audience};
use locales::vendored::reviewed_locale;
use units::{MixedQuantities, Unit};

pub use locales::tag::{InvalidTag, LocaleTag, RegionError};
pub use locales::vendored::Refusal;

#[derive(Debug, thiserror::Error)]
pub enum ConvertError {
    #[error("tried to convert a value with no unit")]
    NoUnit,
    #[error("tried to convert a text value: {0}")]
    TextValue(String),
    #[error("unknown unit: '{0}'")]
    UnknownUnit(String),
    #[error(transparent)]
    MixedQuantities(#[from] MixedQuantities),
}

/// Which locale a recipe reads its units through: the legacy locale when it
/// has no `locale`, else the reviewed locale its tag names (ADR 0009).
///
/// A distinct type rather than a tag, because `legacy` is not a language:
/// a recipe that predates locales is a different kind of thing from `es-MX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Naming {
    Legacy,
    Tagged(LocaleTag),
}

/// Who a fitted quantity is written for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The legacy locale's frozen rows for the system the quantity was
    /// written in: metric stays metric, US stays US (ADR 0009).
    SameSystem,
    /// The audience the recipe's tag determines.
    Audience(Audience),
}

/// Converts quantities written in one locale, for one audience.
///
/// ```
/// use cooklang::{Converter, Quantity, Value};
/// use cooklang::convert::LocaleTag;
///
/// let tag = LocaleTag::parse("sv-SE").unwrap();
/// let converter = Converter::for_tag(&tag).unwrap();
/// let quantity = Quantity::new(Value::from(15.0), Some("ml".to_string()));
/// assert_eq!(converter.fit(&quantity).unwrap().to_string(), "1 msk");
/// ```
#[derive(Debug, Clone)]
pub struct Converter {
    naming: Naming,
    locale: Arc<Locale>,
    target: Target,
}

/// Two converters built from the same naming read and write the same way.
impl PartialEq for Converter {
    fn eq(&self, other: &Self) -> bool {
        self.naming == other.naming
    }
}

impl Converter {
    /// Reads and writes units the way `units.toml` names them, and fits
    /// them within the system they were written in.
    pub fn legacy() -> Self {
        Self {
            naming: Naming::Legacy,
            locale: legacy_locale(),
            target: Target::SameSystem,
        }
    }

    /// Reads and writes units the way the reviewed locale `tag` reaches
    /// names them, and fits them for the audience the tag determines; or
    /// says why there is none.
    pub fn for_tag(tag: &LocaleTag) -> Result<Self, Refusal> {
        let locale = reviewed_locale(tag)?;
        let audience = tag.audience()?;
        Ok(Self {
            naming: Naming::Tagged(tag.clone()),
            locale,
            target: Target::Audience(audience),
        })
    }

    /// The converter `naming` calls for.
    pub fn for_naming(naming: &Naming) -> Result<Self, Refusal> {
        match naming {
            Naming::Legacy => Ok(Self::legacy()),
            Naming::Tagged(tag) => Self::for_tag(tag),
        }
    }

    /// The converter a recipe's own [`naming`](Recipe::naming) calls for.
    pub fn for_recipe(recipe: &Recipe) -> Result<Self, Refusal> {
        Self::for_naming(&recipe.naming())
    }

    /// The naming this converter was built from.
    pub fn naming(&self) -> Naming {
        self.naming.clone()
    }

    /// Who a quantity written in `unit` is fitted for.
    fn audience_for(&self, unit: Unit) -> Audience {
        match self.target {
            Target::SameSystem => Audience::legacy(legacy_system(unit)),
            Target::Audience(audience) => audience,
        }
    }

    /// `quantity`, written in another converter's locale, as this one's
    /// readers spell the same unit: the amount is untouched. A unit `from`
    /// cannot read is left as written, so a list built from recipes in
    /// several locales can add what it recognises and keep the rest.
    pub fn rewrite_from(&self, quantity: &Quantity, from: &Converter) -> Quantity {
        // a text value is never operated on, so its unit stays as written too
        if quantity.value().is_text() {
            return quantity.clone();
        }
        match from.resolve(quantity) {
            Ok(unit) => Quantity {
                value: quantity.value().clone(),
                unit: Some(self.locale.written(unit).to_string()),
                scalable: quantity.scalable,
            },
            Err(_) => quantity.clone(),
        }
    }

    /// `quantity` in the unit its readers would write it in, as scaling
    /// leaves it.
    pub fn fit(&self, quantity: &Quantity) -> Result<Quantity, ConvertError> {
        let from = self.resolve(quantity)?;
        self.convert_for(quantity, self.audience_for(from))
    }

    /// The same amount as `quantity`, expressed in `to` and written with the
    /// locale's symbol for it, or its spaced identifier when the locale has
    /// none. A range converts end by end.
    pub(crate) fn convert(&self, quantity: &Quantity, to: Unit) -> Result<Quantity, ConvertError> {
        let from = self.resolve(quantity)?;
        let convert = |n: &Number| Unit::convert(n.value(), from, to).map(Number::Regular);
        let value = match quantity.value() {
            Value::Number(n) => Value::Number(convert(n)?),
            Value::Range { start, end } => Value::Range {
                start: convert(start)?,
                end: convert(end)?,
            },
            Value::Text(t) => return Err(ConvertError::TextValue(t.clone())),
        };
        Ok(Quantity {
            value,
            unit: Some(self.locale.written(to).to_string()),
            scalable: quantity.scalable,
        })
    }

    /// `quantity` as `audience` would write it: in the unit its rows choose,
    /// as a fraction where the row has one that fits. A range is judged by
    /// its start, as the legacy converter judges it: the smaller unit never
    /// loses information. The end is then written in that same unit.
    pub(crate) fn convert_for(
        &self,
        quantity: &Quantity,
        audience: Audience,
    ) -> Result<Quantity, ConvertError> {
        let from = self.resolve(quantity)?;
        let (value, to) = match quantity.value() {
            Value::Number(n) => {
                let (number, to) = best_amount(n.value(), from, audience);
                (Value::Number(number), to)
            }
            Value::Range { start, end } => {
                let (start, to) = best_amount(start.value(), from, audience);
                let end = Unit::convert(end.value(), from, to)?;
                let end = as_written(end, to, audience);
                (Value::Range { start, end }, to)
            }
            Value::Text(t) => return Err(ConvertError::TextValue(t.clone())),
        };
        Ok(Quantity {
            value,
            unit: Some(self.locale.written(to).to_string()),
            scalable: quantity.scalable,
        })
    }

    /// The unit `quantity` is written in, as this locale reads it.
    /// The locale this converter reads and writes units through.
    pub(crate) fn locale(&self) -> &Locale {
        &self.locale
    }

    /// The unit `text` names in this converter's locale.
    pub(crate) fn resolve_text(&self, text: &str) -> Option<Unit> {
        self.locale.resolve(text)
    }

    fn resolve(&self, quantity: &Quantity) -> Result<Unit, ConvertError> {
        let text = quantity.unit().ok_or(ConvertError::NoUnit)?;
        self.locale
            .resolve(text)
            .ok_or_else(|| ConvertError::UnknownUnit(text.to_string()))
    }
}

impl Metadata {
    /// The locale the `locale` key names, if it has one.
    pub(crate) fn locale_tag(&self) -> Option<LocaleTag> {
        self.locale().and_then(|tag| LocaleTag::parse(tag).ok())
    }

    /// Which locale the recipe reads its units through: the legacy locale
    /// when it has no `locale` (ADR 0009).
    pub fn naming(&self) -> Naming {
        match self.locale_tag() {
            None => Naming::Legacy,
            Some(tag) => Naming::Tagged(tag),
        }
    }

    /// The converter the recipe reads its units through: the one its naming
    /// calls for, or the legacy one when that cannot be served, which is
    /// also what the parser read it through after reporting the refusal.
    pub(crate) fn converter(&self) -> Converter {
        Converter::for_naming(&self.naming()).unwrap_or_else(|_| Converter::legacy())
    }
}

impl Recipe {
    /// The locale the recipe's `locale` key names, if it has one.
    pub(crate) fn locale_tag(&self) -> Option<LocaleTag> {
        self.metadata.locale_tag()
    }

    /// Which locale this recipe reads its units through, from its metadata:
    /// the legacy locale when it has no `locale` (ADR 0009).
    pub fn naming(&self) -> Naming {
        self.metadata.naming()
    }

    /// The converter this recipe reads its units through: the one its
    /// [`naming`](Self::naming) calls for, or the legacy one when that
    /// cannot be served, which is also what the parser read it through
    /// after reporting the refusal. Scaling, grouping, metadata time and
    /// semantic equality all go through it, so a caller never chooses.
    pub fn converter(&self) -> Converter {
        self.metadata.converter()
    }

    /// Writes every quantity as `audience` would write it, in place. A
    /// quantity that cannot be converted is left as it is and its error
    /// returned, as the legacy [`Recipe::convert`] does.
    pub(crate) fn convert_for(
        &mut self,
        audience: Audience,
        converter: &Converter,
    ) -> Vec<ConvertError> {
        let mut errors = Vec::new();
        let mut convert = |quantity: &mut Quantity| match converter.convert_for(quantity, audience)
        {
            Ok(converted) => *quantity = converted,
            Err(error) => errors.push(error),
        };
        for quantity in self
            .ingredients
            .iter_mut()
            .filter_map(|i| i.quantity.as_mut())
        {
            convert(quantity);
        }
        for quantity in self.timers.iter_mut().filter_map(|t| t.quantity.as_mut()) {
            convert(quantity);
        }
        for quantity in &mut self.inline_quantities {
            convert(quantity);
        }
        errors
    }
}
