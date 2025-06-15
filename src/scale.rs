//! Support for recipe scaling

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::{
    convert::Converter,
    quantity::{ScalableQuantity, ScalableValue, ScaledQuantity, TextValueError, Value},
    Cookware, Ingredient, Quantity, ScalableRecipe, ScaledRecipe, Timer,
};

/// Configures the scaling target
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct ScaleTarget {
    factor: f64,
}

impl ScaleTarget {
    /// Creates a new [`ScaleTarget`].
    ///
    /// - `factor` is the multiplier to scale the recipe by.
    /// Invalid parameters don't error here, but may do so in the
    /// scaling process.
    fn new(factor: f64) -> Self {
        ScaleTarget { factor }
    }

    /// Get the calculated scaling factor
    pub fn factor(&self) -> f64 {
        self.factor
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Servings(pub(crate) Option<Vec<u32>>);

/// Possible scaled states of a recipe
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type")]
pub enum Scaled {
    /// The recipe was scaled to its based servings
    ///
    /// This is the values without scaling or if there are many values
    /// for a component, the first one.
    DefaultScaling,
    /// Scaled to a custom target
    Scaled(ScaledData),
}

/// Data from scaling a recipe
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct ScaledData {
    /// What the target was
    pub target: ScaleTarget,
    /// Outcome of scaling the ingredients. Use the same index as in the recipe.
    pub ingredients: Vec<ScaleOutcome>,
    /// Outcome of scaling the cookware items. Use the same index as in the recipe.
    pub cookware: Vec<ScaleOutcome>,
    /// Outcome of scaling the timers. Use the same index as in the recipe.
    pub timers: Vec<ScaleOutcome>,
}

/// Possible outcomes from scaling a component
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(rename_all = "camelCase")]
pub enum ScaleOutcome {
    /// Success
    Scaled,
    /// Not changed becuse it doesn't have to be changed
    Fixed,
    /// It has no quantity, so it can't be scaled
    NoQuantity,
    /// Error scaling
    Error(#[serde(skip)] ScaleError),
}

/// Possible errors during scaling process
#[derive(Debug, Error, Clone, Default)]
pub enum ScaleError {
    #[error(transparent)]
    TextValueError(#[from] TextValueError),

    #[error("Value not scalable: {reason}")]
    NotScalable {
        value: ScalableValue,
        reason: &'static str,
    },

    #[error("Value scaling not defined for target servings")]
    NotDefined {
        target: ScaleTarget,
        value: ScalableValue,
    },

    /// There has been an error but it can't be determined
    ///
    /// This is used when deserializing, because serializing the [`ScaleOutcome`]
    /// skips the error.
    #[default]
    #[error("Undefined scale error")]
    UndefinedError,
}

impl ScalableRecipe {
    /// Scale a recipe
    ///
    /// Note that this returns a [`ScaledRecipe`] wich doesn't implement this
    /// method. A recipe can only be scaled once.
    pub fn scale(self, factor: f64, converter: &Converter) -> ScaledRecipe {
        let target = ScaleTarget::new(factor);

        let (ingredients, ingredient_outcomes): (Vec<_>, Vec<_>) = self
            .ingredients
            .into_iter()
            .map(|i| i.scale(target))
            .map(|(mut i, o)| {
                if let Some(q) = &mut i.quantity {
                    let _ = q.fit(converter);
                }
                (i, o)
            })
            .unzip();

        let (cookware, cookware_outcomes): (Vec<_>, Vec<_>) =
            self.cookware.into_iter().map(|c| c.scale(target)).unzip();

        let (timers, timer_outcomes): (Vec<_>, Vec<_>) = self
            .timers
            .into_iter()
            .map(|c| c.scale(target))
            .map(|(mut t, o)| {
                if let Some(q) = &mut t.quantity {
                    let _ = q.fit(converter);
                }
                (t, o)
            })
            .unzip();

        let data = ScaledData {
            target,
            ingredients: ingredient_outcomes,
            cookware: cookware_outcomes,
            timers: timer_outcomes,
        };

        ScaledRecipe {
            metadata: self.metadata,
            sections: self.sections,
            ingredients,
            cookware,
            timers,
            inline_quantities: self.inline_quantities,
            data: Scaled::Scaled(data),
        }
    }

    /// - `target` is the wanted number of servings.
    pub fn scale_to_servings(self, target: u32, converter: &Converter) -> ScaledRecipe {
        let base = if let Servings(Some(servings)) = &self.data {
            servings.first().copied().unwrap_or(1)
        } else {
            1
        };

        self.scale(target as f64 / base as f64, converter)
    }

    /// Scale the recipe to the default values
    ///
    /// The default values are the ones written in the recipe.
    pub fn default_scale(self) -> ScaledRecipe {
        let ingredients = self
            .ingredients
            .into_iter()
            .map(Scale::default_scale)
            .collect();
        let cookware = self
            .cookware
            .into_iter()
            .map(Scale::default_scale)
            .collect();
        let timers = self.timers.into_iter().map(Scale::default_scale).collect();

        ScaledRecipe {
            metadata: self.metadata,
            sections: self.sections,
            ingredients,
            cookware,
            timers,
            inline_quantities: self.inline_quantities,
            data: Scaled::DefaultScaling,
        }
    }
}

trait Scale: Sized {
    type Output;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome);
    fn default_scale(self) -> Self::Output;
}

impl Scale for ScalableValue {
    type Output = Value;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome) {
        match self {
            Self::Fixed(value) => (value, ScaleOutcome::Fixed),
            Self::Linear(value) => match linear_scale(value.clone(), target.factor()) {
                Ok(v) => (v, ScaleOutcome::Scaled),
                Err(e) => (value, ScaleOutcome::Error(e)),
            },
        }
    }

    fn default_scale(self) -> Self::Output {
        match self {
            Self::Fixed(value) => value,
            Self::Linear(value) => value,
        }
    }
}

fn linear_scale(value: Value, factor: f64) -> Result<Value, ScaleError> {
    match value {
        Value::Number(n) => Ok(Value::Number((n.value() * factor).into())),
        Value::Range { start, end } => {
            let start = (start.value() * factor).into();
            let end = (end.value() * factor).into();
            Ok(Value::Range { start, end })
        }
        v @ Value::Text(_) => Err(TextValueError(v).into()),
    }
}

impl Scale for ScalableQuantity {
    type Output = ScaledQuantity;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome) {
        let Self { value, unit } = self;
        let (value, outcome) = value.scale(target);
        let scaled = ScaledQuantity { value, unit };
        (scaled, outcome)
    }

    fn default_scale(self) -> Self::Output {
        let Self { value, unit } = self;
        Self::Output {
            value: value.default_scale(),
            unit,
        }
    }
}

impl Scale for Ingredient<ScalableValue> {
    type Output = Ingredient<Value>;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome) {
        let (quantity, outcome) = self.quantity.map(|q| q.scale(target)).unzip();
        let outcome = outcome.unwrap_or(ScaleOutcome::NoQuantity);
        let scaled = Ingredient {
            name: self.name,
            alias: self.alias,
            quantity,
            note: self.note,
            reference: self.reference,
            relation: self.relation,
            modifiers: self.modifiers,
        };
        (scaled, outcome)
    }

    fn default_scale(self) -> Self::Output {
        Ingredient {
            name: self.name,
            alias: self.alias,
            quantity: self.quantity.map(Quantity::default_scale),
            note: self.note,
            reference: self.reference,
            relation: self.relation,
            modifiers: self.modifiers,
        }
    }
}

impl Scale for Cookware<ScalableValue> {
    type Output = Cookware<Value>;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome) {
        let (quantity, outcome) = self.quantity.map(|q| q.scale(target)).unzip();
        let outcome = outcome.unwrap_or(ScaleOutcome::NoQuantity);
        let scaled = Cookware {
            name: self.name,
            alias: self.alias,
            quantity,
            note: self.note,
            relation: self.relation,
            modifiers: self.modifiers,
        };
        (scaled, outcome)
    }

    fn default_scale(self) -> Self::Output {
        Cookware {
            name: self.name,
            alias: self.alias,
            quantity: self.quantity.map(ScalableValue::default_scale),
            note: self.note,
            relation: self.relation,
            modifiers: self.modifiers,
        }
    }
}

impl Scale for Timer<ScalableValue> {
    type Output = Timer<Value>;

    fn scale(self, target: ScaleTarget) -> (Self::Output, ScaleOutcome) {
        let (quantity, outcome) = self.quantity.map(|q| q.scale(target)).unzip();
        let outcome = outcome.unwrap_or(ScaleOutcome::NoQuantity);
        let scaled = Timer {
            name: self.name,
            quantity,
        };
        (scaled, outcome)
    }

    fn default_scale(self) -> Self::Output {
        Timer {
            name: self.name,
            quantity: self.quantity.map(Quantity::default_scale),
        }
    }
}

impl ScalableRecipe {
    /// Get the defined number of servings in the recipe
    ///
    /// This is set automatically from the metadata. To change it manually use
    /// [`set_servings`](ScalableRecipe::set_servings).
    pub fn servings(&self) -> Option<&[u32]> {
        if let Servings(Some(s)) = &self.data {
            Some(s.as_slice())
        } else {
            None
        }
    }

    /// Set the number of servings the recipe is intented for
    ///
    /// This will usually be set automatically from the metadata. But you can
    /// use this method to set it manually or modify it.
    pub fn set_servings(&mut self, servings: Vec<u32>) {
        self.data = Servings(Some(servings))
    }
}

impl ScaledRecipe {
    pub fn scaled(&self) -> &Scaled {
        &self.data
    }

    /// Get the [`ScaledData`] from a recipe after scaling.
    ///
    /// Returns [`None`] if it was [`default scaled`](ScalableRecipe::default_scale).
    pub fn scaled_data(&self) -> Option<&ScaledData> {
        if let Scaled::Scaled(data) = &self.data {
            Some(data)
        } else {
            None
        }
    }

    /// Shorthand to check if [`Self::scaled_data`] is [`Scaled::DefaultScaling`].
    pub fn is_default_scaled(&self) -> bool {
        matches!(self.data, Scaled::DefaultScaling)
    }
}
