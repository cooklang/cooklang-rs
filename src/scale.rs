//! Support for recipe scaling

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    convert::Converter,
    quantity::{ScalableQuantity, ScalableValue, ScaledQuantity, TextValueError, Value},
    Cookware, Ingredient, Quantity, ScalableRecipe, ScaledRecipe, Timer,
};

/// Configures the scaling target
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScaleTarget {
    base: u32,
    target: u32,
    index: Option<usize>,
}

impl ScaleTarget {
    /// Creates a new [`ScaleTarget`].
    /// - `base` is the number of servings the recipe was initially written for.
    ///   Usually this is the first value of `declared_servings` but doesn't
    ///   need to.
    /// - `target` is the wanted number of servings.
    /// - `declared_servigs` is the slice with all the servings of the recipe
    ///   metadata.
    ///
    /// Invalid parameters don't error here, but may do so in the
    /// scaling process.
    fn new(base: u32, target: u32, declared_servings: &[u32]) -> Self {
        ScaleTarget {
            base,
            target,
            index: declared_servings.iter().position(|&s| s == target),
        }
    }

    /// Get the calculated scaling factor
    pub fn factor(&self) -> f64 {
        self.target as f64 / self.base as f64
    }

    /// Get the index into a [`ScalableValue::ByServings`]
    pub fn index(&self) -> Option<usize> {
        self.index
    }

    /// Get the target servings
    pub fn target_servings(&self) -> u32 {
        self.target
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Servings(pub(crate) Option<Vec<u32>>);

/// Possible scaled states of a recipe
#[derive(Debug, Serialize, Deserialize)]
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
    pub fn scale(self, target: u32, converter: &Converter) -> ScaledRecipe {
        let target = if let Servings(Some(servings)) = &self.data {
            let base = servings.first().copied().unwrap_or(1);
            ScaleTarget::new(base, target, servings)
        } else {
            ScaleTarget::new(1, target, &[])
        };

        if target.index() == Some(0) {
            return self.default_scale();
        }

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

    /// Scale the recipe to the default values
    ///
    /// The default values are the ones written in the recipe and the first one
    /// in [`ScalableValue::ByServings`].
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
            Self::ByServings(ref values) => {
                if let Some(index) = target.index {
                    let value = match values.get(index) {
                        Some(v) => v,
                        None => {
                            let value = self.clone();
                            return (
                                self.default_scale(),
                                ScaleOutcome::Error(ScaleError::NotDefined { target, value }),
                            );
                        }
                    };
                    (value.clone(), ScaleOutcome::Scaled)
                } else {
                    let value = self.clone();
                    (
                        self.default_scale(),
                        ScaleOutcome::Error(ScaleError::NotScalable {
                            value,
                            reason:
                                "tried to scale a value linearly when it has the scaling defined",
                        }),
                    )
                }
            }
        }
    }

    fn default_scale(self) -> Self::Output {
        match self {
            Self::Fixed(value) => value,
            Self::Linear(value) => value,
            Self::ByServings(values) => values
                .first()
                .expect("scalable value servings list empty")
                .clone(),
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
    /// [`set_servings`].
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
