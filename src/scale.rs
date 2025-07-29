//! Support for recipe scaling

use crate::{convert::Converter, quantity::Value, Quantity, Recipe};
use thiserror::Error;

#[cfg(feature = "ts")]
use tsify::Tsify;

/// Error type for scaling operations
#[derive(Debug, Error)]
pub enum ScaleError {
    /// The recipe has no valid numeric servings value
    #[error("Cannot scale recipe: servings metadata is not a valid number")]
    InvalidServings,
}

impl Recipe {
    /// Scale a recipe
    ///
    /// Note that this returns a [`ScaledRecipe`] wich doesn't implement this
    /// method. A recipe can only be scaled once.
    pub fn scale(&mut self, factor: f64, converter: &Converter) {
        let scale_quantity = |q: &mut Quantity| {
            if q.scalable {
                q.value.scale(factor);
                let _ = q.fit(converter);
            }
        };

        // Update metadata with new servings (only if numeric)
        if let Some(current_servings) = self.metadata.servings() {
            if let Some(base) = current_servings.as_number() {
                let new_servings = (base as f64 * factor).round() as u32;
                if let Some(servings_value) =
                    self.metadata.get_mut(crate::metadata::StdKey::Servings)
                {
                    // Preserve the original type (string or number)
                    match servings_value {
                        serde_yaml::Value::String(_) => {
                            *servings_value = serde_yaml::Value::String(new_servings.to_string());
                        }
                        _ => {
                            *servings_value =
                                serde_yaml::Value::Number(serde_yaml::Number::from(new_servings));
                        }
                    }
                }
            }
        }

        self.ingredients
            .iter_mut()
            .filter_map(|i| i.quantity.as_mut())
            .for_each(scale_quantity);
        self.cookware
            .iter_mut()
            .filter_map(|i| i.quantity.as_mut())
            .for_each(scale_quantity);
        self.timers
            .iter_mut()
            .filter_map(|i| i.quantity.as_mut())
            .for_each(scale_quantity);
    }

    /// Scale to a specific number of servings
    ///
    /// - `target` is the wanted number of servings.
    ///
    /// Returns an error if the recipe doesn't have a valid numeric servings value.
    pub fn scale_to_servings(
        &mut self,
        target: u32,
        converter: &Converter,
    ) -> Result<(), ScaleError> {
        let current_servings = self
            .metadata
            .servings()
            .ok_or(ScaleError::InvalidServings)?;

        let base = current_servings
            .as_number()
            .ok_or(ScaleError::InvalidServings)?;

        let factor = target as f64 / base as f64;
        self.scale(factor, converter);

        // Update servings metadata to the target value
        if let Some(servings_value) = self.metadata.get_mut(crate::metadata::StdKey::Servings) {
            // Preserve the original type (string or number)
            match servings_value {
                serde_yaml::Value::String(_) => {
                    *servings_value = serde_yaml::Value::String(target.to_string());
                }
                _ => {
                    *servings_value = serde_yaml::Value::Number(serde_yaml::Number::from(target));
                }
            }
        }
        Ok(())
    }
}

impl Value {
    fn scale(&mut self, factor: f64) {
        match self {
            Value::Number(n) => {
                *n = (n.value() * factor).into();
            }
            Value::Range { start, end } => {
                *start = (start.value() * factor).into();
                *end = (end.value() * factor).into();
            }
            Value::Text(_) => {}
        }
    }
}
