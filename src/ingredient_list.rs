use serde::Serialize;

use crate::{convert::Converter, quantity::GroupedQuantity, scale::ScaleOutcome, ScaledRecipe};

pub type IngredientList = Vec<IngredientListEntry>;

#[derive(Debug, Clone, Serialize)]
pub struct IngredientListEntry {
    /// Index into the recipe ingredients (ingredient definition)
    pub index: usize,
    /// Total grouped quantity
    pub quantity: GroupedQuantity,
    /// Scale outcome, if scaled to a custom target
    pub outcome: Option<ScaleOutcome>,
}

impl ScaledRecipe {
    pub fn ingredient_list(&self, converter: &Converter) -> IngredientList {
        let mut list = Vec::new();
        let data = self.scaled_data();
        for (index, ingredient) in self
            .ingredients
            .iter()
            .enumerate()
            .filter(|(_, i)| i.modifiers.should_be_listed())
        {
            let mut grouped = GroupedQuantity::default();
            for q in ingredient.all_quantities(&self.ingredients) {
                grouped.add(q, converter);
            }
            let outcome: Option<ScaleOutcome> = data
                .as_ref()
                .map(|data| {
                    // Color in list depends on outcome of definition and all references
                    let mut outcome = &data.ingredients[index]; // temp value
                    let all_indices = std::iter::once(index)
                        .chain(ingredient.relation.referenced_from().iter().copied());
                    for index in all_indices {
                        match &data.ingredients[index] {
                            e @ ScaleOutcome::Error(_) => return e, // if err, return
                            e @ ScaleOutcome::Fixed => outcome = e, // if fixed, store
                            _ => {}
                        }
                    }
                    outcome
                })
                .cloned();
            list.push(IngredientListEntry {
                index,
                quantity: grouped,
                outcome,
            });
        }
        list
    }
}
