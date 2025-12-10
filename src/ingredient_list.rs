//! Generate ingredients lists from recipes

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    aisle::AisleConf, convert::Converter, model::Ingredient, quantity::GroupedQuantity, Cookware,
    Recipe,
};

/// Ingredient with all quantities from it's references and itself grouped.
///
/// Created from [`ScaledRecipe::group_ingredients`].
#[derive(Debug, Clone, Serialize)]
pub struct GroupedIngredient<'a> {
    /// Index of the ingredient definition in the [`Recipe::ingredients`](crate::model::Recipe::ingredients)
    pub index: usize,
    /// Ingredient definition
    pub ingredient: &'a Ingredient,
    /// Grouped quantity of itself and all of it references
    pub quantity: GroupedQuantity,
}

/// Cookware item with all amounts from it's references and itself grouped.
///
/// Created forom [`ScaledRecipe::group_cookware`].
#[derive(Debug, Clone, Serialize)]
pub struct GroupedCookware<'a> {
    /// Index of the item definition in the [`Recipe::cookware`](crate::model::Recipe::cookware)
    pub index: usize,
    /// Cookware definition
    pub cookware: &'a Cookware,
    /// Grouped amount of itself and all of it references
    pub quantity: GroupedQuantity,
}

impl Recipe {
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
    ///                 .unwrap();
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
        for (index, ingredient) in self.ingredients.iter().enumerate() {
            if !ingredient.relation.is_definition() {
                continue;
            }
            let grouped = ingredient.group_quantities(&self.ingredients, converter);
            list.push(GroupedIngredient {
                index,
                ingredient,
                quantity: grouped,
            });
        }
        list
    }

    /// List of cookware **definitions** with amount of all of it
    /// references grouped.
    ///
    /// Order is the recipe order.
    pub fn group_cookware<'a>(&'a self, converter: &Converter) -> Vec<GroupedCookware<'a>> {
        let mut list = Vec::new();
        for (index, cookware) in self.cookware.iter().enumerate() {
            if !cookware.relation.is_definition() {
                continue;
            }
            let amount = cookware.group_quantities(&self.cookware, converter);
            list.push(GroupedCookware {
                index,
                cookware,
                quantity: amount,
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
    pub fn from_recipe(recipe: &Recipe, converter: &Converter, list_references: bool) -> Self {
        let mut list = Self::new();
        list.add_recipe(recipe, converter, list_references);
        list
    }

    /// Subtract pantry quantities from the ingredient list.
    ///
    /// For each ingredient in the list, if it exists in the pantry with a valid quantity,
    /// subtract that quantity from the required amount. Only subtracts when units match.
    /// Returns a new IngredientList with the remaining quantities needed.
    ///
    /// # Arguments
    ///
    /// * `pantry` - The pantry configuration to subtract from
    /// * `converter` - The unit converter for quantity operations
    ///
    /// # Example
    ///
    /// ```ignore
    /// let shopping_list = ingredient_list.subtract_pantry(&pantry_conf, converter);
    /// ```
    #[cfg(feature = "pantry")]
    pub fn subtract_pantry(
        &self,
        pantry: &crate::pantry::PantryConf,
        converter: &Converter,
    ) -> Self {
        let mut result = Self::new();

        for (ingredient_name, required_quantity) in self.iter() {
            // Check if ingredient exists in pantry
            if let Some((_section, pantry_item)) = pantry.find_ingredient(ingredient_name) {
                // Get pantry quantity if available
                if let Some(pantry_qty_str) = pantry_item.quantity() {
                    // Check for special "unlim" case
                    if pantry_qty_str == "unlim" || pantry_qty_str == "unlimited" {
                        tracing::info!(
                            "Removing '{}' from shopping list (unlimited in pantry)",
                            ingredient_name
                        );
                        continue; // Skip this item entirely
                    }

                    // Try to parse pantry quantity
                    if let Some((pantry_value, pantry_unit)) = pantry_item.parsed_quantity() {
                        // If pantry has 0, keep everything in the shopping list
                        if pantry_value <= 0.0 {
                            result.add_ingredient(
                                ingredient_name.clone(),
                                required_quantity,
                                converter,
                            );
                            continue;
                        }

                        // Try to subtract from each quantity variant
                        let mut remaining_quantities = crate::quantity::GroupedQuantity::empty();
                        let mut any_subtracted = false;
                        let mut unit_mismatch = false;

                        for req_qty in required_quantity.iter() {
                            let req_unit =
                                req_qty.unit().map(|u| u.to_lowercase()).unwrap_or_default();

                            if req_unit == pantry_unit {
                                // Units match, we can subtract
                                if let crate::quantity::Value::Number(req_num) = req_qty.value() {
                                    let req_value: f64 = req_num.value();
                                    let remaining_value = req_value - pantry_value;

                                    if remaining_value > 0.0 {
                                        let remaining_qty = crate::quantity::Quantity::new(
                                            crate::quantity::Value::Number(
                                                crate::quantity::Number::Regular(remaining_value),
                                            ),
                                            req_qty.unit().map(|s| s.to_string()),
                                        );
                                        remaining_quantities.add(&remaining_qty, converter);
                                        let unit_display =
                                            if req_unit.is_empty() { "" } else { &req_unit };
                                        tracing::info!(
                                            "Reduced '{}' from {} {} to {} {} (pantry has {} {})",
                                            ingredient_name,
                                            req_value,
                                            unit_display,
                                            remaining_value,
                                            unit_display,
                                            pantry_value,
                                            unit_display
                                        );
                                    } else {
                                        let unit_display = if pantry_unit.is_empty() {
                                            ""
                                        } else {
                                            &pantry_unit
                                        };
                                        tracing::info!(
                                            "Removing '{}' from shopping list (sufficient in pantry: {} {})",
                                            ingredient_name,
                                            pantry_value,
                                            unit_display
                                        );
                                    }
                                    any_subtracted = true;
                                } else {
                                    remaining_quantities.add(req_qty, converter);
                                }
                            } else {
                                // Units don't match
                                remaining_quantities.add(req_qty, converter);
                                unit_mismatch = true;
                                tracing::warn!(
                                    "Unit mismatch for '{}': recipe needs '{}', pantry has '{}'",
                                    ingredient_name,
                                    req_unit,
                                    pantry_unit
                                );
                            }
                        }

                        if unit_mismatch && !any_subtracted {
                            // Keep full amount due to unit mismatch
                            result.add_ingredient(
                                ingredient_name.clone(),
                                required_quantity,
                                converter,
                            );
                        } else if !remaining_quantities.is_empty() {
                            // Add the remaining quantities
                            result.add_ingredient(
                                ingredient_name.clone(),
                                &remaining_quantities,
                                converter,
                            );
                        }
                        // If remaining_quantities is empty and no unit mismatch, item is fully covered
                    } else {
                        // Can't parse pantry quantity, keep original
                        tracing::warn!(
                            "Cannot parse pantry quantity for '{}': {}",
                            ingredient_name,
                            pantry_qty_str
                        );
                        result.add_ingredient(
                            ingredient_name.clone(),
                            required_quantity,
                            converter,
                        );
                    }
                } else {
                    // No quantity specified in pantry, assume we have it (backward compatibility)
                    tracing::info!(
                        "Removing '{}' from shopping list (found in pantry without quantity)",
                        ingredient_name
                    );
                }
            } else {
                // Not in pantry, keep it in the list
                result.add_ingredient(ingredient_name.clone(), required_quantity, converter);
            }
        }

        result
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
    pub fn add_recipe(
        &mut self,
        recipe: &Recipe,
        converter: &Converter,
        list_references: bool,
    ) -> Vec<usize> {
        let mut references = Vec::new();

        for entry in recipe.group_ingredients(converter) {
            let GroupedIngredient {
                ingredient,
                quantity,
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

    /// Replace names of ingredients with common names given by aisle configuration.
    pub fn use_common_names(self, aisle: &AisleConf, converter: &Converter) -> Self {
        let ingredients_info = aisle.ingredients_info();
        let mut normalized = Self::new();
        for (ingredient_name, quantity) in self.iter(){
            let common_name = ingredients_info
                .get(ingredient_name.as_str())
                .map(|info| info.common_name.to_string())
                .unwrap_or(ingredient_name.to_string());
            normalized.add_ingredient(common_name, quantity, converter);
        }
        normalized
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

#[cfg(all(test, feature = "pantry"))]
mod tests {
    use super::*;
    use crate::{CooklangParser, Extensions};

    #[test]
    fn test_subtract_pantry_unlimited() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        // Create a recipe with some ingredients
        let recipe = parser
            .parse("@salt{1%tsp} @water{1%l} @flour{500%g}")
            .into_output()
            .unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        // Create pantry with unlimited water
        let pantry_toml = r#"
[kitchen]
water = "unlim"
salt = "0.5%tsp"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();

        // Subtract pantry
        let result = list.subtract_pantry(&pantry, &converter);

        // Water should be completely removed (unlimited)
        assert!(!result.iter().any(|(name, _)| name.as_str() == "water"));

        // Salt should have 0.5 tsp remaining (1 - 0.5)
        let salt_qty = result.iter().find(|(name, _)| name.as_str() == "salt");
        assert!(salt_qty.is_some());

        // Flour should remain unchanged (not in pantry)
        let flour_qty = result.iter().find(|(name, _)| name.as_str() == "flour");
        assert!(flour_qty.is_some());
    }

    #[test]
    fn test_subtract_pantry_with_quantities() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        // Create a recipe needing 2 avocados
        let recipe = parser.parse("@avocados{2}").into_output().unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        // Test with 1 avocado in pantry
        let pantry_toml = r#"
[fridge]
avocados = "1"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // Should have 1 avocado remaining
        let avocado_qty = result.iter().find(|(name, _)| name.as_str() == "avocados");
        assert!(avocado_qty.is_some());
        let (_, qty) = avocado_qty.unwrap();
        assert_eq!(qty.to_string(), "1");
    }

    #[test]
    fn test_subtract_pantry_unit_mismatch() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        // Recipe needs grams, pantry has liters
        let recipe = parser.parse("@oil{500%g}").into_output().unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        let pantry_toml = r#"
[pantry]
oil = "1%l"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // Should keep full amount due to unit mismatch
        let oil_qty = result.iter().find(|(name, _)| name.as_str() == "oil");
        assert!(oil_qty.is_some());
        let (_, qty) = oil_qty.unwrap();
        assert_eq!(qty.to_string(), "500 g");
    }

    #[test]
    fn test_subtract_pantry_zero_quantity() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        let recipe = parser.parse("@milk{1%l}").into_output().unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        // Pantry has 0 milk
        let pantry_toml = r#"
[fridge]
milk = "0%l"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // Should keep full amount when pantry has 0
        let milk_qty = result.iter().find(|(name, _)| name.as_str() == "milk");
        assert!(milk_qty.is_some());
        let (_, qty) = milk_qty.unwrap();
        assert_eq!(qty.to_string(), "1 l");
    }

    #[test]
    fn test_subtract_pantry_exact_match() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        let recipe = parser.parse("@rice{2%kg}").into_output().unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        // Pantry has exactly what we need
        let pantry_toml = r#"
[pantry]
rice = "2%kg"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // Rice should be completely removed
        assert!(!result.iter().any(|(name, _)| name.as_str() == "rice"));
    }

    #[test]
    fn test_subtract_pantry_more_than_needed() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        let recipe = parser.parse("@sugar{100%g}").into_output().unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);

        // Pantry has more than we need
        let pantry_toml = r#"
[cupboard]
sugar = "500%g"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // Sugar should be completely removed
        assert!(!result.iter().any(|(name, _)| name.as_str() == "sugar"));
    }

    #[test]
    fn test_use_common_names() {
        let converter = Converter::bundled();
        let parser = CooklangParser::new(Extensions::all(), converter.clone());

        let recipe = parser
            .parse("@unsalted butter{250%g} plus @milk{100%ml} and @unsalted_butter{250%g}")
            .into_output()
            .unwrap();

        // Aisle has some alternative names
        let aisle_toml = r#"
[milk and dairy]
milk
butter | unsalted butter | unsalted_butter
"#;
        let aisle = crate::aisle::parse(aisle_toml).unwrap();

        let mut list = IngredientList::new();
        list.add_recipe(&recipe, &converter, false);
        let list = list.use_common_names(&aisle, &converter);

        // Pantry has some butter
        let pantry_toml = r#"
[fridge]
butter = "200%g"
milk = "500%ml"
"#;
        let pantry = crate::pantry::parse(pantry_toml).unwrap();
        let result = list.subtract_pantry(&pantry, &converter);

        // List should contain 300g (500g - 200g) butter and no milk
        assert!(!result.iter().any(|(name, _)| name.as_str() == "milk"));
        let butter_qty = result.iter().find(|(name, _)| name.as_str() == "butter");
        assert!(butter_qty.is_some());
        let (_, qty) = butter_qty.unwrap();
        assert_eq!(qty.to_string(), "300 g");
    }
}
