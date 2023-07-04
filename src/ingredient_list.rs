use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    aisle::AisleConf, convert::Converter, model::Ingredient, quantity::GroupedQuantity,
    scale::ScaleOutcome, ScaledRecipe,
};

/// Ingredient with all quantities from it's references and itself grouped
#[derive(Debug, Clone, Serialize)]
pub struct GroupedIngredient<'a> {
    /// Index of the ingredient definition in the [Recipe::ingredients](crate::model::Recipe::ingredients)
    pub index: usize,
    /// Ingredient definition
    pub ingredient: &'a Ingredient,
    /// Grouped quantity of itself and all of it references
    pub quantity: GroupedQuantity,
    /// Scale outcome, if scaled to a custom target
    ///
    /// If any scaling outcome was [ScaleOutcome::Error], this will be an error.
    /// It will only be one and no particular order is guaranteed.
    ///
    /// If any scaling outcome was [ScaleOutcome::Fixed], this will be the fixed.
    pub outcome: Option<ScaleOutcome>,
}

impl ScaledRecipe {
    /// List of ingredient definitions with quantities of all of it references
    /// combined.
    ///
    /// Order is the recipe order.
    pub fn group_ingredients(&self, converter: &Converter) -> Vec<GroupedIngredient> {
        let mut list = Vec::new();
        let data = self.scaled_data();
        for (index, ingredient) in self.ingredients.iter().enumerate() {
            let mut grouped = GroupedQuantity::default();
            for q in ingredient.all_quantities(&self.ingredients) {
                grouped.add(q, converter);
            }
            let outcome: Option<ScaleOutcome> = data
                .as_ref()
                .map(|data| {
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
            list.push(GroupedIngredient {
                index,
                ingredient,
                quantity: grouped,
                outcome,
            });
        }
        list
    }
}

/// List of ingredients with quantities.
///
/// Sorted by name.
#[derive(Debug, Default)]
pub struct IngredientList(BTreeMap<String, GroupedQuantity>);

impl IngredientList {
    /// Empty list
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingredient list of a recipe
    pub fn from_recipe(recipe: &ScaledRecipe, converter: &Converter) -> Self {
        let mut list = Self::new();
        list.add_recipe(recipe, converter);
        list
    }

    /// Add the ingredients from a recipe to the list.
    ///
    /// This is a convenience method instead of manually calling [IngredientList::add_ingredient]
    /// for each one.
    ///
    /// Only ingredients for which [should_be_listed](crate::ast::Modifiers::should_be_listed)
    /// is true are added.
    ///
    /// Scaling outcomes are ignored, but logged with [tracing] if they are an
    /// error.
    ///
    /// Ingredients are listed based on their [display_name](crate::model::Ingredient::display_name).
    pub fn add_recipe(&mut self, recipe: &ScaledRecipe, converter: &Converter) {
        for entry in recipe.group_ingredients(converter) {
            let GroupedIngredient {
                ingredient,
                quantity,
                outcome,
                ..
            } = entry;

            if !ingredient.modifiers().should_be_listed() {
                continue;
            }

            if let Some(ScaleOutcome::Error(err)) = outcome {
                tracing::error!("Error scaling ingredient: {err}");
            }

            self.add_ingredient(ingredient.display_name().into_owned(), &quantity, converter);
        }
    }

    /// Add an ingredient to the list.
    ///
    /// The quantity will be merged will the ingredients with the same name.
    pub fn add_ingredient(
        &mut self,
        name: String,
        quantity: &GroupedQuantity,
        converter: &Converter,
    ) {
        self.0.entry(name).or_default().merge(quantity, converter)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Split this list into different categories.
    ///
    /// Ingredients without category will be placed in `"other"`.
    pub fn categorize<'a>(self, aisle: &AisleConf) -> CategorizedIngredientList {
        let aisle = aisle.reverse();
        let mut categorized = CategorizedIngredientList::default();
        for (name, quantity) in self.0 {
            if let Some(cat) = aisle.get(name.as_str()) {
                categorized
                    .categories
                    .entry(cat.to_string())
                    .or_default()
                    .0
                    .insert(name, quantity);
            } else {
                categorized.other.0.insert(name, quantity);
            }
        }
        categorized
    }

    /// Iterate over all ingredients sorted by name
    pub fn iter(&self) -> impl Iterator<Item = (&String, &GroupedQuantity)> {
        self.0.iter()
    }
}

impl IntoIterator for IngredientList {
    type Item = (String, GroupedQuantity);

    type IntoIter = std::collections::btree_map::IntoIter<String, GroupedQuantity>;

    /// Iterate over all ingrediends sorted by name
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Ingredient list split into categories.
///
/// Obtained from [IngredientList::categorize].
#[derive(Debug, Default)]
pub struct CategorizedIngredientList {
    /// One ingredient list per category
    ///
    /// Because this is a [BTreeMap], the categories are sorted by name
    pub categories: BTreeMap<String, IngredientList>,
    /// Ingredients with no category assigned
    pub other: IngredientList,
}

impl CategorizedIngredientList {
    /// Iterate over all categories sorted by their name. If [Self::other] is
    /// not empty, adds an `"other"` category at the end.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &IngredientList)> {
        CategorizedIter {
            categories: self.categories.iter(),
            other: Some(&self.other),
        }
    }
}

/// See [CategorizedIngredientList::iter]
pub struct CategorizedIter<'a> {
    categories: std::collections::btree_map::Iter<'a, String, IngredientList>,
    other: Option<&'a IngredientList>,
}

impl<'a> Iterator for CategorizedIter<'a> {
    type Item = (&'a str, &'a IngredientList);

    fn next(&mut self) -> Option<Self::Item> {
        self.categories
            .next()
            .map(|(s, l)| (s.as_str(), l))
            .or_else(|| {
                self.other
                    .take()
                    .filter(|l| !l.is_empty())
                    .map(|l| ("other", l))
            })
    }
}

impl IntoIterator for CategorizedIngredientList {
    type Item = (String, IngredientList);

    type IntoIter = CategorizedIntoIter;

    /// Iterate over all categories sorted by their name. If [Self::other] is
    /// not empty, adds an `"other"` category at the end.
    fn into_iter(self) -> Self::IntoIter {
        CategorizedIntoIter {
            categories: self.categories.into_iter(),
            other: Some(self.other),
        }
    }
}

/// See [CategorizedIngredientList::into_iter]
pub struct CategorizedIntoIter {
    categories: std::collections::btree_map::IntoIter<String, IngredientList>,
    other: Option<IngredientList>,
}

impl Iterator for CategorizedIntoIter {
    type Item = (String, IngredientList);

    fn next(&mut self) -> Option<Self::Item> {
        self.categories.next().or_else(|| {
            self.other
                .take()
                .filter(|l| !l.is_empty())
                .map(|l| ("other".to_string(), l))
        })
    }
}
