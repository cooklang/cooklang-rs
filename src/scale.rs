//! Support for recipe scaling

use crate::{convert::Converter, quantity::Value, Quantity, Recipe};

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

    /// - `target` is the wanted number of servings.
    pub fn scale_to_servings(&mut self, target: u32, converter: &Converter) {
        let base = self
            .metadata
            .servings()
            .and_then(|s| s.first().copied())
            .unwrap_or(1);
        let factor = target as f64 / base as f64;
        self.scale(factor, converter)
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
