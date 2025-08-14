//! Support for recipe scaling

use crate::{convert::Converter, quantity::Value, Quantity, Recipe};
use thiserror::Error;

/// Error type for scaling operations
#[derive(Debug, Error, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum ScaleError {
    /// The recipe has no valid numeric servings value
    #[error("Cannot scale recipe: servings metadata is not a valid number")]
    InvalidServings,

    /// The recipe has no valid yield metadata
    #[error("Cannot scale recipe: yield metadata is missing or invalid")]
    InvalidYield,

    /// The units don't match between target and current yield
    #[error("Cannot scale recipe: unit mismatch (expected {expected}, got {got})")]
    UnitMismatch { expected: String, got: String },
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

    /// Scale to a specific yield amount with unit
    ///
    /// - `target_value` is the wanted yield amount
    /// - `target_unit` is the unit for the yield
    ///
    /// Returns an error if:
    /// - The recipe doesn't have yield metadata
    /// - The yield metadata is not in the correct format
    /// - The units don't match
    pub fn scale_to_yield(
        &mut self,
        target_value: f64,
        target_unit: &str,
        converter: &Converter,
    ) -> Result<(), ScaleError> {
        // Get current yield from metadata
        let yield_value = self.metadata.get("yield").ok_or(ScaleError::InvalidYield)?;

        let yield_str = yield_value
            .as_str()
            .ok_or(ScaleError::InvalidYield)?
            .to_string(); // Clone to avoid borrowing issues

        // Parse yield value - only support "1000%g" format
        let parts: Vec<&str> = yield_str.split('%').collect();
        if parts.len() != 2 {
            return Err(ScaleError::InvalidYield);
        }
        let current_value = parts[0]
            .parse::<f64>()
            .map_err(|_| ScaleError::InvalidYield)?;
        let current_unit = parts[1].to_string();

        // Check that units match
        if current_unit != target_unit {
            return Err(ScaleError::UnitMismatch {
                expected: target_unit.to_string(),
                got: current_unit.to_string(),
            });
        }

        let factor = target_value / current_value;
        self.scale(factor, converter);

        // Update yield metadata to the target value (always use % format)
        if let Some(yield_meta) = self.metadata.get_mut("yield") {
            *yield_meta = serde_yaml::Value::String(format!("{}%{}", target_value, target_unit));
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
