//! Generate ingredients lists from recipes

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    aisle::AisleConf,
    convert::Converter,
    model::Ingredient,
    quantity::{GroupedQuantity, GroupedValue},
    scale::ScaleOutcome,
    Cookware, ScaledRecipe, Value,
};

/// Ingredient with all quantities from it's references and itself grouped.
///
/// Created from [`ScaledRecipe::group_ingredients`].
#[derive(Debug, Clone, Serialize)]
pub struct GroupedIngredient<'a> {
    /// Index of the ingredient definition in the [`Recipe::ingredients`](crate::model::Recipe::ingredients)
    pub index: usize,
    /// Ingredient definition
    pub ingredient: &'a Ingredient<Value>,
    /// Grouped quantity of itself and all of it references
    pub quantity: GroupedQuantity,
    /// Scale outcome, if scaled to a custom target
    ///
    /// If any scaling outcome was [`ScaleOutcome::Error`], this will be an error.
    /// It will only be one and no particular order is guaranteed.
    ///
    /// If any scaling outcome was [`ScaleOutcome::Fixed`], this will be the fixed.
    pub outcome: Option<ScaleOutcome>,
}

/// Cookware item with all amounts from it's references and itself grouped.
///
/// Created forom [`ScaledRecipe::group_cookware`].
#[derive(Debug, Clone, Serialize)]
pub struct GroupedCookware<'a> {
    /// Index of the item definition in the [`Recipe::cookware`](crate::model::Recipe::cookware)
    pub index: usize,
    /// Cookware definition
    pub cookware: &'a Cookware<Value>,
    /// Grouped amount of itself and all of it references
    pub amount: GroupedValue,
}

impl ScaledRecipe {
    /// List of ingredient **definitions** with quantities of all of it
    /// references grouped.
    ///
    /// Order is the recipe order. This is for a single recipe. If you need to
    /// merge different recipes, see [`IngredientList`].
    ///
    /// ```
    /// # use cooklang::{CooklangParser, Extensions, Converter, Value, Quantity};
    /// let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    /// let recipe = parser.parse("@flour{1000%g} @water @&flour{100%g}")
    ///                 .into_output()
    ///                 .unwrap()
    ///                 .default_scale();
    /// let grouped = recipe.group_ingredients(parser.converter());
    ///
    /// // Only 2 definitions, the second flour is a reference
    /// assert_eq!(grouped.len(), 2);
    ///
    /// let flour = &grouped[0];
    /// assert_eq!(flour.ingredient.name, "flour");
    /// assert_eq!(flour.quantity.to_string(), "1.1 kg");
    ///
    /// // Water is second, because it is after flour in the recipe
    /// let water = &grouped[1];
    /// assert_eq!(water.ingredient.name, "water");
    /// assert!(water.quantity.is_empty());
    /// ```
    pub fn group_ingredients<'a>(&'a self, converter: &Converter) -> Vec<GroupedIngredient<'a>> {
        let mut list = Vec::new();
        let data = self.scaled_data();
        for (index, ingredient) in self.ingredients.iter().enumerate() {
            if !ingredient.relation.is_definition() {
                continue;
            }
            let grouped = ingredient.group_quantities(&self.ingredients, converter);
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

    /// List of cookware **definitions** with amount of all of it
    /// references grouped.
    ///
    /// Order is the recipe order.
    pub fn group_cookware(&self) -> Vec<GroupedCookware> {
        let mut list = Vec::new();
        for (index, cookware) in self.cookware.iter().enumerate() {
            if !cookware.relation.is_definition() {
                continue;
            }
            let amount = cookware.group_amounts(&self.cookware);
            list.push(GroupedCookware {
                index,
                cookware,
                amount,
            })
        }
        list
    }
}

/// List of ingredients with quantities.
///
/// This will only store the ingredient name and quantity. Sorted by name. This
/// is used to combine multiple recipes into a single list. For ingredients of a
/// single recipe, check [`ScaledRecipe::group_ingredients`].
#[derive(Debug, Default)]
pub struct IngredientList(BTreeMap<String, GroupedQuantity>);

impl IngredientList {
    /// Empty list
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingredient list of a recipe
    pub fn from_recipe(recipe: &ScaledRecipe, converter: &Converter, list_references: bool) -> Self {
        let mut list = Self::new();
        list.add_recipe(recipe, converter, list_references);
        list
    }

    /// Add the ingredients from a recipe to the list.
    ///
    /// This is a convenience method instead of manually calling [`IngredientList::add_ingredient`]
    /// for each one.
    ///
    /// Only ingredients for which [`should_be_listed`](crate::Modifiers::should_be_listed)
    /// is true are added.
    ///
    /// Scaling outcomes are ignored, but logged with [tracing] if they are an
    /// error.
    ///
    /// Ingredients are listed based on their [`display_name`](crate::model::Ingredient::display_name).
    pub fn add_recipe(&mut self, recipe: &ScaledRecipe, converter: &Converter, list_references: bool) -> Vec<usize> {
        let mut references = Vec::new();

        for entry in recipe.group_ingredients(converter) {
            let GroupedIngredient {
                ingredient,
                quantity,
                outcome,
                index,
            } = entry;

            if ingredient.reference.is_some() {
                references.push(index);

                if !list_references {
                    continue;
                }
            }

            if !ingredient.modifiers().should_be_listed() {
                continue;
            }

            if let Some(ScaleOutcome::Error(err)) = outcome {
                tracing::error!("Error scaling ingredient: {err}");
            }

            self.add_ingredient(ingredient.display_name().into_owned(), &quantity, converter);
        }

        references
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

    /// Cheks if the list is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Split this list into different categories.
    ///
    /// Ingredients without category will be placed in `"other"`.
    pub fn categorize(self, aisle: &AisleConf) -> CategorizedIngredientList {
        let iifno = aisle.ingredients_info();
        let mut categorized = CategorizedIngredientList::default();
        for (name, quantity) in self.0 {
            if let Some(info) = iifno.get(name.as_str()) {
                categorized
                    .categories
                    .entry(info.category.to_string())
                    .or_default()
                    .0
                    .insert(info.common_name.to_string(), quantity);
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
/// Obtained from [`IngredientList::categorize`].
#[derive(Debug, Default)]
pub struct CategorizedIngredientList {
    /// One ingredient list per category
    ///
    /// Because this is a [`BTreeMap`], the categories are sorted by name
    pub categories: BTreeMap<String, IngredientList>,
    /// Ingredients with no category assigned
    pub other: IngredientList,
}

impl CategorizedIngredientList {
    /// Iterate over all categories sorted by their name. If [`Self::other`] is
    /// not empty, adds an `"other"` category at the end.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &IngredientList)> {
        CategorizedIter {
            categories: self.categories.iter(),
            other: Some(&self.other),
        }
    }
}

/// See [`CategorizedIngredientList::iter`]
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

    /// Iterate over all categories sorted by their name. If [`Self::other`] is
    /// not empty, adds an `"other"` category at the end.
    fn into_iter(self) -> Self::IntoIter {
        CategorizedIntoIter {
            categories: self.categories.into_iter(),
            other: Some(self.other),
        }
    }
}

/// See [`CategorizedIngredientList::into_iter`]
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
