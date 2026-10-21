//! Support for recipe scaling

use crate::{
    convert::Converter,
    float::round_f64,
    metadata::{is_scalable_amount, Servings},
    quantity::Value,
    Quantity, Recipe,
};
use thiserror::Error;

/// Error type for scaling operations
#[derive(Debug, Error, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify))]
pub enum ScaleError {
    /// The recipe's servings, or the target asked for, is not a count
    #[error("Cannot scale recipe: servings is not a positive, finite count")]
    InvalidServings,

    /// The recipe's yield, or the target asked for, is not an amount
    #[error("Cannot scale recipe: yield is missing, or is not a positive, finite amount")]
    InvalidYield,

    /// The target's unit cannot be converted to the yield's
    #[error("Cannot scale recipe: unit mismatch (expected {expected}, got {got})")]
    UnitMismatch { expected: String, got: String },
}

impl Recipe {
    /// Scale a recipe
    ///
    pub fn scale(&mut self, factor: f64, converter: &Converter) {
        let scale_quantity = |q: &mut Quantity| {
            if q.scalable {
                q.value.scale(factor);
                let _ = q.fit(converter);
            }
        };

        // text servings carry no count, so there is nothing to scale
        if let Some(new_servings) = self
            .metadata
            .servings()
            .and_then(|s| s.scaled(factor).as_number())
        {
            if let Some(servings_value) = self.metadata.get_mut(crate::metadata::StdKey::Servings) {
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

        // the yield is an amount the recipe makes, so it cannot stay behind the
        // ingredients it is scaled with
        if let Some(mut qty) = self.metadata.yield_quantity() {
            if let Value::Number(_) = qty.value() {
                qty.value.scale(factor);
                let _ = qty.fit(converter);
                if let Some(entry) = self.metadata.get_mut(crate::metadata::StdKey::Yield) {
                    *entry = yield_metadata_value(&qty);
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
        target: f64,
        converter: &Converter,
    ) -> Result<(), ScaleError> {
        // the target is written back as the recipe's own count, so it has to be
        // a count the recipe could have been written with
        if !is_scalable_amount(target) {
            return Err(ScaleError::InvalidServings);
        }

        let current_servings = self
            .metadata
            .servings()
            .ok_or(ScaleError::InvalidServings)?;

        let base = current_servings
            .as_number()
            .ok_or(ScaleError::InvalidServings)?;

        let factor = target / base;
        self.scale(factor, converter);

        // written to the precision a count is written with, as `scale` does
        let written = round_f64(target, Servings::WRITE_PRECISION);
        if let Some(servings_value) = self.metadata.get_mut(crate::metadata::StdKey::Servings) {
            // Preserve the original type (string or number)
            match servings_value {
                serde_yaml::Value::String(_) => {
                    *servings_value = serde_yaml::Value::String(written.to_string());
                }
                _ => {
                    *servings_value = serde_yaml::Value::Number(serde_yaml::Number::from(written));
                }
            }
        }
        Ok(())
    }

    /// Scale to a target value with optional unit
    ///
    /// This function intelligently chooses the appropriate scaling method:
    /// - If `target_unit` is `Some("servings")`, scales by servings
    /// - If `target_unit` is `Some(other_unit)`, scales by yield with that unit
    /// - If `target_unit` is `None`, applies direct scaling factor
    ///
    /// # Arguments
    /// - `target_value` - The target value (servings count, yield amount, or scaling factor)
    /// - `target_unit` - Optional unit ("servings" for servings-based, other for yield-based, None for direct factor)
    /// - `converter` - Unit converter for fitting quantities
    ///
    /// # Returns
    /// - `Ok(())` on successful scaling
    /// - `Err(ScaleError)` if scaling cannot be performed
    pub fn scale_to_target(
        &mut self,
        target_value: f64,
        target_unit: Option<&str>,
        converter: &Converter,
    ) -> Result<(), ScaleError> {
        match target_unit {
            Some("servings") | Some("serving") => self.scale_to_servings(target_value, converter),
            Some(unit) => {
                // Scale by yield with the specified unit
                self.scale_to_yield(target_value, unit, converter)
            }
            None => {
                // Direct scaling factor
                self.scale(target_value, converter);
                Ok(())
            }
        }
    }

    /// Scale to a specific yield amount with unit
    ///
    /// - `target_value` is the wanted yield amount
    /// - `target_unit` is its unit, which may be any unit the yield's
    ///   converts to: a `500%g` yield scales to `2 kg` by 4, and the yield is
    ///   written back as `2%kg`
    ///
    /// Returns an error if:
    /// - The recipe doesn't have yield metadata
    /// - The yield metadata is not in the correct format
    /// - The target's unit cannot be converted to the yield's
    pub fn scale_to_yield(
        &mut self,
        target_value: f64,
        target_unit: &str,
        converter: &Converter,
    ) -> Result<(), ScaleError> {
        // likewise written back as the recipe's own yield
        if !is_scalable_amount(target_value) {
            return Err(ScaleError::InvalidYield);
        }

        let current_qty = self
            .metadata
            .yield_quantity()
            .ok_or(ScaleError::InvalidYield)?;

        let current_value = match current_qty.value() {
            Value::Number(n) => n.value(),
            _ => return Err(ScaleError::InvalidYield),
        };

        let unit = (!target_unit.is_empty()).then(|| target_unit.to_string());
        let target = Quantity::new(Value::from(target_value), unit);

        // the factor is taken between like amounts, so a target in another
        // unit is converted into the yield's first
        let mismatch = || ScaleError::UnitMismatch {
            expected: target_unit.to_string(),
            got: current_qty.unit().unwrap_or("").to_string(),
        };
        let mut in_yield_unit = target.clone();
        match (current_qty.unit(), target.unit()) {
            (Some(current), Some(wanted)) if current != wanted => in_yield_unit
                .convert(current, converter)
                .map_err(|_| mismatch())?,
            (Some(_), Some(_)) | (None, None) => {}
            _ => return Err(mismatch()),
        }
        let target_in_yield_unit = match in_yield_unit.value() {
            Value::Number(n) => n.value(),
            _ => return Err(ScaleError::InvalidYield),
        };

        let factor = target_in_yield_unit / current_value;
        self.scale(factor, converter);

        // the caller asked for this amount in this unit, so it is written back
        // as asked rather than refitted -- but through the one yield writer, so
        // it cannot disagree with how `scale` renders the same amount
        if let Some(yield_meta) = self.metadata.get_mut(crate::metadata::StdKey::Yield) {
            *yield_meta = yield_metadata_value(&target);
        }

        Ok(())
    }
}

/// Writes a yield quantity back in the `value%unit` form the key is read in
fn yield_metadata_value(qty: &Quantity) -> serde_yaml::Value {
    let value = match qty.value() {
        Value::Number(n) => n.value(),
        // yield only ever parses as a single number
        _ => return serde_yaml::Value::String(qty.value().to_string()),
    };
    match qty.unit() {
        Some(unit) => serde_yaml::Value::String(format!("{value}%{unit}")),
        None => serde_yaml::Value::Number(serde_yaml::Number::from(value)),
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
