//! A [`Converter`] reads and writes units the way one locale names them, and fits them for one audience: the legacy
//! locale and the system a quantity was written in for a recipe with no `locale`, else what the recipe's tag
//! determines.

pub(crate) mod locales;
pub mod units;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::metadata::Metadata;
use crate::model::Recipe;
use crate::quantity::{Number, Quantity, Value, WrittenUnit};
use locales::legacy::{legacy_locale, legacy_system};
use locales::locale::Locale;
use locales::preference::{as_written, best_amount};
use locales::vendored::reviewed_locale;
use units::MixedQuantities;

pub use locales::preference::{Audience, System, Units};
pub use locales::tag::{InvalidTag, LocaleTag, RegionError};
pub use locales::vendored::Refusal;
pub use units::{PhysicalQuantity, Unit};

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

/// Which locale a recipe reads its units through: the legacy locale when it has no `locale`, else the reviewed locale
/// its tag names (ADR 7).
///
/// A distinct type rather than a tag, because `legacy` is not a language: a recipe that predates locales is a different
/// kind of thing from `es-MX`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Naming {
    Legacy,
    Tagged(LocaleTag),
}

/// Who a fitted quantity is written for.
///
/// One audience serves a tagged recipe, whatever its units; an untagged one needs the unit first, since each is fitted
/// within the system it was written in, so it becomes an [`Audience`] only in [`audience_for`].
///
/// [`audience_for`]: Converter::audience_for
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    SameSystem,
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
/// // a quantity built by hand is read through the converter's locale first;
/// // one the parser built already carries its unit (ADR 8)
/// let quantity = Quantity::new(Value::from(15.0), Some("ml".to_string())).read_with(&converter);
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

    pub fn for_naming(naming: &Naming) -> Result<Self, Refusal> {
        match naming {
            Naming::Legacy => Ok(Self::legacy()),
            Naming::Tagged(tag) => Self::for_tag(tag),
        }
    }

    pub fn for_recipe(recipe: &Recipe) -> Result<Self, Refusal> {
        Self::for_naming(&recipe.naming())
    }

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

    /// `quantity` as this converter's readers spell its unit: the amount is untouched. A unit that was not read, or not
    /// known where it was read, is left as written, so a list built from recipes in several locales adds what it
    /// recognises and keeps the rest (ADR 8).
    pub fn rewrite(&self, quantity: &Quantity) -> Quantity {
        // a text value is never operated on, so its unit stays as written too
        match quantity.unit_id() {
            Some(unit) if !quantity.value().is_text() => Quantity {
                value: quantity.value().clone(),
                unit: Some(WrittenUnit::known(self.locale.written(unit), unit)),
                scalable: quantity.scalable,
            },
            _ => quantity.clone(),
        }
    }

    /// `quantity` in the unit its readers would write it in, as scaling leaves it. Its unit must have been read (ADR
    /// 8); one that was not, or is not known, is an [`UnknownUnit`](ConvertError::UnknownUnit).
    pub fn fit(&self, quantity: &Quantity) -> Result<Quantity, ConvertError> {
        let from = self.resolve(quantity)?;
        self.convert_for(quantity, self.audience_for(from))
    }

    /// The same amount as `quantity`, expressed in `to` and written as this locale writes it. A range converts end by
    /// end.
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
            unit: Some(WrittenUnit::known(self.locale.written(to), to)),
            scalable: quantity.scalable,
        })
    }

    /// `quantity` as `audience` would write it: in the unit its rows choose, as a fraction where the row has one that
    /// fits. A range is judged by its start, so that the smaller unit never loses information, and its end is written
    /// in whatever unit that chose.
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
            unit: Some(WrittenUnit::known(self.locale.written(to), to)),
            scalable: quantity.scalable,
        })
    }

    pub(crate) fn locale(&self) -> &Locale {
        &self.locale
    }

    pub(crate) fn resolve_text(&self, text: &str) -> Option<Unit> {
        self.locale.resolve(text)
    }

    /// The unit `quantity` was read as (ADR 8).
    fn resolve(&self, quantity: &Quantity) -> Result<Unit, ConvertError> {
        let text = quantity.unit().ok_or(ConvertError::NoUnit)?;
        quantity
            .unit_id()
            .ok_or_else(|| ConvertError::UnknownUnit(text.to_string()))
    }
}

impl Metadata {
    /// The locale the `locale` key names, if it has one.
    pub(crate) fn locale_tag(&self) -> Option<LocaleTag> {
        self.locale().and_then(|tag| LocaleTag::parse(tag).ok())
    }

    /// The [`Naming`] the `locale` key gives the recipe.
    pub fn naming(&self) -> Naming {
        match self.locale_tag() {
            None => Naming::Legacy,
            Some(tag) => Naming::Tagged(tag),
        }
    }

    /// The converter this [`naming`](Self::naming) calls for, or the legacy one when it cannot be served, which is what
    /// the parser read the recipe through after reporting that refusal.
    pub(crate) fn converter(&self) -> Converter {
        Converter::for_naming(&self.naming()).unwrap_or_else(|_| Converter::legacy())
    }
}

impl Recipe {
    /// See [`Metadata::locale_tag`].
    pub fn locale_tag(&self) -> Option<LocaleTag> {
        self.metadata.locale_tag()
    }

    /// See [`Metadata::naming`].
    pub fn naming(&self) -> Naming {
        self.metadata.naming()
    }

    /// The converter this recipe reads its units through, from its [`naming`](Self::naming). Scaling, grouping,
    /// metadata time and equality all go through it, so a caller never chooses.
    pub fn converter(&self) -> Converter {
        self.metadata.converter()
    }

    /// Reads every quantity's unit through the recipe's own [`converter`](Self::converter), as the parser does. For a
    /// recipe deserialised from a document written before units were recorded (ADR 8); a parsed recipe needs no call.
    pub fn read_units(&mut self) {
        let converter = self.converter();
        let locale = converter.locale();
        let quantities = self
            .ingredients
            .iter_mut()
            .filter_map(|i| i.quantity.as_mut())
            .chain(self.cookware.iter_mut().filter_map(|c| c.quantity.as_mut()))
            .chain(self.timers.iter_mut().filter_map(|t| t.quantity.as_mut()))
            .chain(self.inline_quantities.iter_mut());
        for quantity in quantities {
            quantity.reread(locale);
        }
    }

    /// Writes every quantity as `audience` would write it, in place, while
    /// keeping the recipe's own unit names: a `sv-SE` recipe written for an
    /// American reader is in US units spelled the Swedish way, and the US
    /// units Swedish declines to name are written by their identifier
    /// (ADR 4).
    ///
    /// Units are read through the recipe's own
    /// [`converter`](Self::converter). A quantity that cannot be converted
    /// is left as it is and its error returned.
    ///
    /// ```
    /// # use cooklang::{CooklangParser, Extensions};
    /// # use cooklang::convert::{Audience, System};
    /// let parser = CooklangParser::new(Extensions::all());
    /// let mut recipe = parser
    ///     .parse("---\nlocale: sv-SE\n---\nSmält @smör{2%msk} i @mjölk{5%dl}.\n")
    ///     .into_output()
    ///     .unwrap();
    /// let errors = recipe.convert_for(Audience::system(System::UsCustomary));
    /// assert!(errors.is_empty());
    /// let written: Vec<String> = recipe
    ///     .ingredients
    ///     .iter()
    ///     .map(|i| i.quantity.as_ref().unwrap().to_string())
    ///     .collect();
    /// assert_eq!(written, ["1 us fluid ounce", "17 us fluid ounce"]);
    /// ```
    pub fn convert_for(&mut self, audience: Audience) -> Vec<ConvertError> {
        let converter = self.converter();
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
