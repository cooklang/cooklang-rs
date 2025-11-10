use std::sync::Arc;

use cooklang::aisle::parse_lenient;
use cooklang::metadata::StdKey as OriginalStdKey;

pub mod aisle;
pub mod model;

use aisle::*;
use model::*;

/// Parses a Cooklang recipe from text and applies a scaling factor
///
/// # Arguments
/// * `input` - The raw recipe text in Cooklang format
/// * `scaling_factor` - Factor to scale ingredient quantities (1.0 for no scaling)
///
/// # Returns
/// A parsed recipe object with metadata, sections, ingredients, cookware and timers
#[uniffi::export]
pub fn parse_recipe(input: String, scaling_factor: f64) -> Arc<CooklangRecipe> {
    let parser = cooklang::CooklangParser::default();

    let (mut parsed, _warnings) = parser.parse(&input).into_result().unwrap();

    parsed.scale(scaling_factor, parser.converter());

    Arc::new(into_simple_recipe(&parsed))
}

/// Dereferences a component reference to get the actual component
///
/// # Arguments
/// * `recipe` - The recipe containing the components
/// * `item` - The item reference (IngredientRef, CookwareRef, TimerRef, or Text)
///
/// # Returns
/// The actual component (Ingredient, Cookware, Timer, or Text)
#[uniffi::export]
pub fn deref_component(recipe: &Arc<CooklangRecipe>, item: Item) -> Component {
    match item {
        Item::IngredientRef { index } => {
            Component::IngredientComponent(recipe.ingredients.get(index as usize).unwrap().clone())
        }
        Item::CookwareRef { index } => {
            Component::CookwareComponent(recipe.cookware.get(index as usize).unwrap().clone())
        }
        Item::TimerRef { index } => {
            Component::TimerComponent(recipe.timers.get(index as usize).unwrap().clone())
        }
        Item::Text { value } => Component::TextComponent(value),
    }
}

/// Gets an ingredient by its index
///
/// # Arguments
/// * `recipe` - The recipe containing the ingredients
/// * `index` - The index of the ingredient
///
/// # Returns
/// The ingredient at the specified index
#[uniffi::export]
pub fn deref_ingredient(recipe: &Arc<CooklangRecipe>, index: u32) -> Ingredient {
    recipe.ingredients.get(index as usize).unwrap().clone()
}

/// Gets cookware by its index
///
/// # Arguments
/// * `recipe` - The recipe containing the cookware
/// * `index` - The index of the cookware
///
/// # Returns
/// The cookware at the specified index
#[uniffi::export]
pub fn deref_cookware(recipe: &Arc<CooklangRecipe>, index: u32) -> Cookware {
    recipe.cookware.get(index as usize).unwrap().clone()
}

/// Gets a timer by its index
///
/// # Arguments
/// * `recipe` - The recipe containing the timers
/// * `index` - The index of the timer
///
/// # Returns
/// The timer at the specified index
#[uniffi::export]
pub fn deref_timer(recipe: &Arc<CooklangRecipe>, index: u32) -> Timer {
    recipe.timers.get(index as usize).unwrap().clone()
}

/// Parses an aisle configuration for shopping list organization
///
/// # Arguments
/// * `input` - The aisle configuration text
///
/// # Returns
/// Parsed aisle configuration with categories and ingredient mappings
#[uniffi::export]
pub fn parse_aisle_config(input: String) -> Arc<AisleConf> {
    let mut categories: Vec<AisleCategory> = Vec::new();
    let mut cache: AisleReverseCategory = AisleReverseCategory::default();

    // Use the lenient parser that handles duplicates as warnings
    let result = parse_lenient(&input);
    let parsed = match result.into_result() {
        Ok((parsed, warnings)) => {
            // Log warnings if any
            if warnings.has_warnings() {
                for diag in warnings.iter() {
                    eprintln!("Warning: {}", diag);
                }
            }
            parsed
        }
        Err(report) => {
            // Log errors
            for diag in report.iter() {
                eprintln!("Error: {}", diag);
            }
            // Return empty config on error
            Default::default()
        }
    };

    let _ = &(parsed).categories.iter().for_each(|c| {
        let category = into_category(c);

        // building cache
        category.ingredients.iter().for_each(|i| {
            cache.insert(i.name.clone(), category.name.clone());

            i.aliases.iter().for_each(|a| {
                cache.insert(a.to_string(), category.name.clone());
            });
        });

        categories.push(category);
    });

    let config = AisleConf { categories, cache };

    Arc::new(config)
}

/// Combines a list of ingredients, grouping by name and summing quantities
///
/// # Arguments
/// * `ingredients` - List of ingredients to combine
///
/// # Returns
/// A map of ingredient names to their combined quantities
#[uniffi::export]
pub fn combine_ingredients(ingredients: &[Ingredient]) -> IngredientList {
    let indices = (0..ingredients.len()).map(|i| i as u32).collect();
    combine_ingredients_selected(ingredients, &indices)
}

/// Combines selected ingredients by their indices
///
/// # Arguments
/// * `ingredients` - Full list of ingredients
/// * `indices` - Indices of ingredients to combine
///
/// # Returns
/// A map of ingredient names to their combined quantities
#[uniffi::export]
pub fn combine_ingredients_selected(
    ingredients: &[Ingredient],
    indices: &Vec<u32>,
) -> IngredientList {
    let mut combined: IngredientList = IngredientList::default();

    expand_with_ingredients(ingredients, &mut combined, indices);

    combined
}

// Metadata helper functions
/// Gets the servings from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get servings from
///
/// # Returns
/// Servings as either a number (e.g., 4) or text (e.g., "2-3 portions")
#[uniffi::export]
pub fn metadata_servings(recipe: &Arc<CooklangRecipe>) -> Option<Servings> {
    recipe.metadata.servings().map(|s| s.clone().into())
}

/// Gets the title from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get the title from
///
/// # Returns
/// The recipe title if present
#[uniffi::export]
pub fn metadata_title(recipe: &Arc<CooklangRecipe>) -> Option<String> {
    recipe.metadata.title().map(|s| s.to_string())
}

/// Gets the description from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get the description from
///
/// # Returns
/// The recipe description if present
#[uniffi::export]
pub fn metadata_description(recipe: &Arc<CooklangRecipe>) -> Option<String> {
    recipe.metadata.description().map(|s| s.to_string())
}

/// Gets tags from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get tags from
///
/// # Returns
/// A list of tags if present
#[uniffi::export]
pub fn metadata_tags(recipe: &Arc<CooklangRecipe>) -> Option<Vec<String>> {
    recipe
        .metadata
        .tags()
        .map(|tags| tags.into_iter().map(|t| t.to_string()).collect())
}

/// Gets the author information from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get author from
///
/// # Returns
/// Author name and optional URL
#[uniffi::export]
pub fn metadata_author(recipe: &Arc<CooklangRecipe>) -> Option<NameAndUrl> {
    recipe.metadata.author().map(|a| a.clone().into())
}

/// Gets the source information from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get source from
///
/// # Returns
/// Source name and optional URL
#[uniffi::export]
pub fn metadata_source(recipe: &Arc<CooklangRecipe>) -> Option<NameAndUrl> {
    recipe.metadata.source().map(|s| s.clone().into())
}

/// Gets the time information from recipe metadata
///
/// # Arguments
/// * `recipe` - The recipe to get time from
///
/// # Returns
/// Total time or separate prep/cook times in minutes
#[uniffi::export]
pub fn metadata_time(recipe: &Arc<CooklangRecipe>) -> Option<RecipeTime> {
    let converter = cooklang::Converter::empty();
    recipe.metadata.time(&converter).map(|t| t.clone().into())
}

/// Gets a custom metadata value by key
///
/// # Arguments
/// * `recipe` - The recipe containing metadata
/// * `key` - The metadata key to retrieve
///
/// # Returns
/// The metadata value if present
#[uniffi::export]
pub fn metadata_get(recipe: &Arc<CooklangRecipe>, key: String) -> Option<String> {
    recipe
        .metadata
        .get(&key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Gets a standard metadata value using the StdKey enum
///
/// # Arguments
/// * `recipe` - The recipe containing metadata
/// * `key` - The standard metadata key
///
/// # Returns
/// The metadata value if present
#[uniffi::export]
pub fn metadata_get_std(recipe: &Arc<CooklangRecipe>, key: StdKey) -> Option<String> {
    let original_key = match key {
        StdKey::Title => OriginalStdKey::Title,
        StdKey::Description => OriginalStdKey::Description,
        StdKey::Tags => OriginalStdKey::Tags,
        StdKey::Author => OriginalStdKey::Author,
        StdKey::Source => OriginalStdKey::Source,
        StdKey::Course => OriginalStdKey::Course,
        StdKey::Time => OriginalStdKey::Time,
        StdKey::PrepTime => OriginalStdKey::PrepTime,
        StdKey::CookTime => OriginalStdKey::CookTime,
        StdKey::Servings => OriginalStdKey::Servings,
        StdKey::Difficulty => OriginalStdKey::Difficulty,
        StdKey::Cuisine => OriginalStdKey::Cuisine,
        StdKey::Diet => OriginalStdKey::Diet,
        StdKey::Images => OriginalStdKey::Images,
        StdKey::Locale => OriginalStdKey::Locale,
    };
    recipe
        .metadata
        .get(original_key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Gets all non-standard (custom) metadata keys
///
/// # Arguments
/// * `recipe` - The recipe containing metadata
///
/// # Returns
/// List of custom metadata keys that are not part of the Cooklang standard
#[uniffi::export]
pub fn metadata_custom_keys(recipe: &Arc<CooklangRecipe>) -> Vec<String> {
    use cooklang::metadata::StdKey;
    use std::str::FromStr;

    recipe
        .metadata
        .map
        .keys()
        .filter_map(|key| {
            // Get the string representation of the key
            key.as_str().and_then(|key_str| {
                // Check if it's a standard key by trying to parse it
                if StdKey::from_str(key_str).is_ok() {
                    None // It's a standard key, exclude it
                } else {
                    Some(key_str.to_string()) // It's a custom key, include it
                }
            })
        })
        .collect()
}

/// Formats a Value to a display string with proper fraction handling
///
/// Converts decimals to fractions where appropriate (e.g., 0.666667 -> "2/3")
/// Handles floating point precision issues from scaling (e.g., 0.89999999 -> "0.9")
///
/// # Arguments
/// * `value` - The value to format
///
/// # Returns
/// Formatted string or None for Empty values
#[uniffi::export]
pub fn format_value(value: &Value) -> Option<String> {
    match value {
        Value::Empty => None,
        Value::Number { value } => Some(format_number(*value)),
        Value::Range { start, end } => Some(format!(
            "{} - {}",
            format_number(*start),
            format_number(*end)
        )),
        Value::Text { value } => Some(value.clone()),
    }
}

/// Parses a string into a Value
///
/// Supports fractions (e.g., "1/2" -> 0.5), mixed numbers (e.g., "1 1/2" -> 1.5),
/// ranges (e.g., "1/2 - 3/4" -> Range{0.5, 0.75}), or falls back to text
///
/// # Arguments
/// * `s` - The string to parse
///
/// # Returns
/// Parsed Value (Number, Range, or Text)
#[uniffi::export]
pub fn parse_value(s: String) -> Value {
    // Try to parse as a number first
    if let Ok(num) = s.parse::<f64>() {
        return Value::Number { value: num };
    }

    // Try to parse as a range (e.g., "1 - 2" or "1/2 - 3/4")
    if let Some(dash_pos) = s.find(" - ") {
        let start_str = &s[..dash_pos];
        let end_str = &s[dash_pos + 3..];

        // Try parsing both parts as numbers or fractions
        let start = parse_number_or_fraction(start_str.trim());
        let end = parse_number_or_fraction(end_str.trim());

        if let (Some(start_val), Some(end_val)) = (start, end) {
            return Value::Range {
                start: start_val,
                end: end_val,
            };
        }
    }

    // Try to parse as a fraction
    if let Some(num) = parse_number_or_fraction(&s) {
        return Value::Number { value: num };
    }

    // Otherwise, treat as text
    Value::Text { value: s }
}

/// Formats an Amount to a display string with units
///
/// Combines formatted quantity with units (e.g., "2/3 cups", "1 1/2 tsp")
///
/// # Arguments
/// * `amount` - The amount to format
///
/// # Returns
/// Formatted string with quantity and units
#[uniffi::export]
pub fn format_amount(amount: &Amount) -> String {
    match format_value(&amount.quantity) {
        Some(qty_str) => {
            if let Some(units) = &amount.units {
                format!("{} {}", qty_str, units)
            } else {
                qty_str
            }
        }
        None => amount.units.clone().unwrap_or_default(),
    }
}

/// Formats a number, converting common decimal values to fractions
fn format_number(value: f64) -> String {
    // Round to reasonable precision to handle floating point errors
    // This handles cases like 0.89999999999 -> 0.9
    let rounded = (value * 1000000.0).round() / 1000000.0;

    // Check if it's effectively a whole number
    if (rounded.fract()).abs() < 0.0000001 {
        return format!("{:.0}", rounded);
    }

    // Try to convert to a common fraction
    if let Some(fraction) = decimal_to_fraction(rounded) {
        return fraction;
    }

    // For decimals, determine appropriate precision
    // Round to at most 3 decimal places, but remove trailing zeros
    let rounded_to_3 = (rounded * 1000.0).round() / 1000.0;

    // Format with appropriate precision
    let mut result = if (rounded_to_3 * 100.0).fract().abs() < 0.001 {
        // Has at most 2 decimal places
        format!("{:.2}", rounded_to_3)
    } else {
        // Needs 3 decimal places
        format!("{:.3}", rounded_to_3)
    };

    // Remove trailing zeros and decimal point if not needed
    if result.contains('.') {
        result = result
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
    }

    result
}

/// Converts common decimal values to fraction strings
fn decimal_to_fraction(value: f64) -> Option<String> {
    const EPSILON: f64 = 0.0001;

    // Split into whole and fractional parts
    let whole = value.floor();
    let fract = value - whole;

    // Common fractions and their decimal equivalents
    let common_fractions = [
        (0.125, "1/8"),
        (0.25, "1/4"),
        (0.333333, "1/3"),
        (0.375, "3/8"),
        (0.5, "1/2"),
        (0.625, "5/8"),
        (0.666667, "2/3"),
        (0.75, "3/4"),
        (0.875, "7/8"),
    ];

    // Check if the fractional part matches any common fraction
    for &(decimal, fraction_str) in &common_fractions {
        if (fract - decimal).abs() < EPSILON {
            if whole > 0.0 {
                return Some(format!("{:.0} {}", whole, fraction_str));
            } else {
                return Some(fraction_str.to_string());
            }
        }
    }

    None
}

/// Parses a string that might be a number or a fraction
fn parse_number_or_fraction(s: &str) -> Option<f64> {
    // Try parsing as a plain number first
    if let Ok(num) = s.parse::<f64>() {
        return Some(num);
    }

    // Try parsing as a mixed number (e.g., "1 1/2")
    if let Some(space_pos) = s.find(' ') {
        let whole_str = &s[..space_pos];
        let fract_str = &s[space_pos + 1..];

        if let Ok(whole) = whole_str.parse::<f64>() {
            if let Some(fract) = parse_fraction(fract_str) {
                return Some(whole + fract);
            }
        }
    }

    // Try parsing as a simple fraction (e.g., "1/2")
    parse_fraction(s)
}

/// Parses a simple fraction string (e.g., "1/2") into a decimal
fn parse_fraction(s: &str) -> Option<f64> {
    if let Some(slash_pos) = s.find('/') {
        let numerator_str = &s[..slash_pos];
        let denominator_str = &s[slash_pos + 1..];

        if let (Ok(numerator), Ok(denominator)) =
            (numerator_str.parse::<f64>(), denominator_str.parse::<f64>())
        {
            if denominator != 0.0 {
                return Some(numerator / denominator);
            }
        }
    }

    None
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_recipe() {
        use crate::{
            deref_component, parse_recipe, Amount, Block, Component, Ingredient, Item, Value,
        };

        let recipe = parse_recipe(
            r#"
a test @step @salt{1%mg} more text
"#
            .to_string(),
            1.0,
        );

        assert_eq!(
            deref_component(&recipe, Item::IngredientRef { index: 1 }),
            Component::IngredientComponent(Ingredient {
                name: "salt".to_string(),
                amount: Some(Amount {
                    quantity: Value::Number { value: 1.0 },
                    units: Some("mg".to_string())
                }),
                descriptor: None
            })
        );

        assert_eq!(
            match recipe
                .sections
                .get(0)
                .expect("No blocks found")
                .blocks
                .get(0)
                .expect("No blocks found")
            {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected first block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "a test ".to_string()
                },
                Item::IngredientRef { index: 0 },
                Item::Text {
                    value: " ".to_string()
                },
                Item::IngredientRef { index: 1 },
                Item::Text {
                    value: " more text".to_string()
                }
            ]
        );

        assert_eq!(
            recipe.ingredients,
            vec![
                Ingredient {
                    name: "step".to_string(),
                    amount: None,
                    descriptor: None
                },
                Ingredient {
                    name: "salt".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 1.0 },
                        units: Some("mg".to_string())
                    }),
                    descriptor: None
                },
            ]
        );
    }

    #[test]
    fn test_decimal_range_parsing() {
        use crate::{parse_recipe, Amount, Ingredient, Value};

        let recipe = parse_recipe(
            "For line and @granulated sugar{1.5-2.25%cups}".to_string(),
            1.0,
        );

        assert_eq!(recipe.ingredients.len(), 1);
        assert_eq!(
            recipe.ingredients[0],
            Ingredient {
                name: "granulated sugar".to_string(),
                amount: Some(Amount {
                    quantity: Value::Range {
                        start: 1.5,
                        end: 2.25
                    },
                    units: Some("cups".to_string())
                }),
                descriptor: None
            }
        );
    }

    #[test]
    fn test_decimal_range_scaling() {
        use crate::{parse_recipe, Amount, Ingredient, Value};

        // Test scaling by 2.0
        let recipe = parse_recipe(
            "@granulated sugar{1.5-2.25%cups}".to_string(),
            2.0,
        );

        assert_eq!(recipe.ingredients.len(), 1);
        assert_eq!(
            recipe.ingredients[0],
            Ingredient {
                name: "granulated sugar".to_string(),
                amount: Some(Amount {
                    quantity: Value::Range {
                        start: 3.0,  // 1.5 * 2.0
                        end: 4.5     // 2.25 * 2.0
                    },
                    units: Some("cups".to_string())
                }),
                descriptor: None
            }
        );
    }

    #[test]
    fn test_metadata_helpers() {
        use crate::{
            metadata_servings, metadata_source, metadata_tags, metadata_title, parse_recipe,
            Servings,
        };

        let recipe = parse_recipe(
            r#"---
title: Test Recipe
source: https://google.com
servings: 4
tags: easy, quick, vegetarian
---
a test @step @salt{1%mg} more text
"#
            .to_string(),
            1.0,
        );

        // Test title
        assert_eq!(metadata_title(&recipe), Some("Test Recipe".to_string()));

        // Test source
        let source = metadata_source(&recipe);
        assert!(source.is_some());
        let source = source.unwrap();
        assert_eq!(source.url, Some("https://google.com".to_string()));

        // Test servings
        let servings = metadata_servings(&recipe);
        assert!(servings.is_some());
        match servings.unwrap() {
            Servings::Number { value } => assert_eq!(value, 4),
            _ => panic!("Expected number servings"),
        }

        // Test tags
        let tags = metadata_tags(&recipe);
        assert!(tags.is_some());
        let tags = tags.unwrap();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains(&"easy".to_string()));
        assert!(tags.contains(&"quick".to_string()));
        assert!(tags.contains(&"vegetarian".to_string()));
    }

    #[test]
    fn test_metadata_advanced() {
        use crate::{metadata_author, metadata_servings, parse_recipe, Servings};

        let recipe = parse_recipe(
            r#"---
author: John Doe <https://johndoe.com>
time: 1h 30m
servings: 2-3 portions
---
Cook something delicious
"#
            .to_string(),
            1.0,
        );

        // Test author with URL
        let author = metadata_author(&recipe);
        assert!(author.is_some());
        let author = author.unwrap();
        assert_eq!(author.name, Some("John Doe".to_string()));
        assert_eq!(author.url, Some("https://johndoe.com".to_string()));

        // Note: Time parsing requires units to be loaded in the converter
        // Since we're using an empty converter, time parsing won't work for "1h 30m"
        // We would need to add units configuration for this to work

        // Test text servings
        let servings = metadata_servings(&recipe);
        assert!(servings.is_some());
        match servings.unwrap() {
            Servings::Text { value } => assert_eq!(value, "2-3 portions"),
            _ => panic!("Expected text servings"),
        }
    }

    #[test]
    fn test_metadata_custom_keys() {
        use crate::{metadata_custom_keys, parse_recipe};

        let recipe = parse_recipe(
            r#"---
title: Test Recipe
author: John Doe
custom-field: custom value
nutrition: 500 calories
rating: 5 stars
servings: 4
---
Test recipe content
"#
            .to_string(),
            1.0,
        );

        let custom_keys = metadata_custom_keys(&recipe);

        // Should contain only custom keys, not standard ones
        assert!(custom_keys.contains(&"custom-field".to_string()));
        assert!(custom_keys.contains(&"nutrition".to_string()));
        assert!(custom_keys.contains(&"rating".to_string()));

        // Should not contain standard keys
        assert!(!custom_keys.contains(&"title".to_string()));
        assert!(!custom_keys.contains(&"author".to_string()));
        assert!(!custom_keys.contains(&"servings".to_string()));

        // Check the count
        assert_eq!(custom_keys.len(), 3);
    }

    #[test]
    fn test_parse_aisle_config() {
        use crate::parse_aisle_config;

        let config = parse_aisle_config(
            r#"
[fruit and veg]
apple gala | apples
aubergine
avocado | avocados

[milk and dairy]
butter
egg | eggs
curd cheese
cheddar cheese
feta

[dried herbs and spices]
bay leaves
black pepper
cayenne pepper
dried oregano
"#
            .to_string(),
        );

        assert_eq!(
            config.category_for("bay leaves".to_string()),
            Some("dried herbs and spices".to_string())
        );

        assert_eq!(
            config.category_for("eggs".to_string()),
            Some("milk and dairy".to_string())
        );

        assert_eq!(
            config.category_for("some weird ingredient".to_string()),
            None
        );
    }

    #[test]
    fn test_combine_ingredients() {
        use crate::{
            combine_ingredients, Amount, GroupedQuantityKey, Ingredient, QuantityType, Value,
        };
        use std::collections::HashMap;

        let ingredients = vec![
            Ingredient {
                name: "salt".to_string(),
                amount: Some(Amount {
                    quantity: Value::Number { value: 5.0 },
                    units: Some("g".to_string()),
                }),
                descriptor: None,
            },
            Ingredient {
                name: "pepper".to_string(),
                amount: Some(Amount {
                    quantity: Value::Number { value: 5.0 },
                    units: Some("mg".to_string()),
                }),
                descriptor: None,
            },
            Ingredient {
                name: "salt".to_string(),
                amount: Some(Amount {
                    quantity: Value::Number { value: 0.005 },
                    units: Some("kg".to_string()),
                }),
                descriptor: None,
            },
            Ingredient {
                name: "pepper".to_string(),
                amount: Some(Amount {
                    quantity: Value::Number { value: 1.0 },
                    units: Some("tsp".to_string()),
                }),
                descriptor: None,
            },
        ];

        let combined = combine_ingredients(&ingredients);

        assert_eq!(
            *combined.get("salt").unwrap(),
            HashMap::from([
                (
                    GroupedQuantityKey {
                        name: "kg".to_string(),
                        unit_type: QuantityType::Number
                    },
                    Value::Number { value: 0.005 }
                ),
                (
                    GroupedQuantityKey {
                        name: "g".to_string(),
                        unit_type: QuantityType::Number
                    },
                    Value::Number { value: 5.0 }
                ),
            ])
        );

        assert_eq!(
            *combined.get("pepper").unwrap(),
            HashMap::from([
                (
                    GroupedQuantityKey {
                        name: "mg".to_string(),
                        unit_type: QuantityType::Number
                    },
                    Value::Number { value: 5.0 }
                ),
                (
                    GroupedQuantityKey {
                        name: "tsp".to_string(),
                        unit_type: QuantityType::Number
                    },
                    Value::Number { value: 1.0 }
                ),
            ])
        );
    }

    #[test]
    fn test_parse_recipe_with_note() {
        use crate::{parse_recipe, Block, Item};

        let recipe = parse_recipe(
            r#"
> This dish is even better the next day, after the flavors have melded overnight.

Cook @onions{3%large} until brown
"#
            .to_string(),
            1.0,
        );

        let first_section = recipe.sections.get(0).expect("No sections found");

        assert_eq!(first_section.blocks.len(), 2);

        // Check note block
        let note_block = first_section.blocks.get(0).expect("No blocks found");

        assert_eq!(
            match note_block {
                Block::NoteBlock(note) => note.clone(),
                _ => panic!("Expected first block to be a Note"),
            }
            .text,
            "This dish is even better the next day, after the flavors have melded overnight."
                .to_string()
        );

        // Check step block
        let step_block = first_section.blocks.get(1).expect("No blocks found");

        assert_eq!(
            match step_block {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected second block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "Cook ".to_string()
                },
                Item::IngredientRef { index: 0 },
                Item::Text {
                    value: " until brown".to_string()
                }
            ]
        );
    }

    #[test]
    fn test_parse_recipe_with_multiline_steps() {
        use crate::{parse_recipe, Block, Item};

        let recipe = parse_recipe(
            r#"
add @onions{2} to pan
heat until golden

add @tomatoes{400%g}
simmer for 10 minutes
"#
            .to_string(),
            1.0,
        );
        let first_section = recipe.sections.get(0).expect("No sections found");
        assert_eq!(first_section.blocks.len(), 2);

        // Check first step
        let first_block = first_section.blocks.get(0).expect("No blocks found");
        let second_block = first_section.blocks.get(1).expect("No blocks found");

        assert_eq!(
            match first_block {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected first block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "add ".to_string()
                },
                Item::IngredientRef { index: 0 },
                Item::Text {
                    value: " to pan heat until golden".to_string()
                }
            ]
        );

        // Check second step
        assert_eq!(
            match second_block {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected second block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "add ".to_string()
                },
                Item::IngredientRef { index: 1 },
                Item::Text {
                    value: " simmer for 10 minutes".to_string()
                }
            ]
        );
    }

    #[test]
    fn test_parse_recipe_with_sections() {
        use crate::{parse_recipe, Block, Item};

        let recipe = parse_recipe(
            r#"
= Dough

Mix @flour{200%g} and @water{50%ml} together until smooth.

== Filling ==

Combine @cheese{100%g} and @spinach{50%g}, then season to taste.
"#
            .to_string(),
            1.0,
        );

        let sections = &recipe.sections;

        // Check first section
        let first_section = sections.get(0).expect("No sections found");
        assert_eq!(first_section.title, Some("Dough".to_string()));
        assert_eq!(first_section.blocks.len(), 1);

        let first_block = first_section.blocks.get(0).expect("No blocks found");
        assert_eq!(
            match first_block {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "Mix ".to_string()
                },
                Item::IngredientRef { index: 0 },
                Item::Text {
                    value: " and ".to_string()
                },
                Item::IngredientRef { index: 1 },
                Item::Text {
                    value: " together until smooth.".to_string()
                }
            ]
        );

        // Check second section
        let second_section = sections.get(1).expect("No second section found");
        assert_eq!(second_section.title, Some("Filling".to_string()));
        assert_eq!(second_section.blocks.len(), 1);

        let second_block = second_section.blocks.get(0).expect("No blocks found");
        assert_eq!(
            match second_block {
                Block::StepBlock(step) => step.clone(),
                _ => panic!("Expected block to be a Step"),
            }
            .items,
            vec![
                Item::Text {
                    value: "Combine ".to_string()
                },
                Item::IngredientRef { index: 2 },
                Item::Text {
                    value: " and ".to_string()
                },
                Item::IngredientRef { index: 3 },
                Item::Text {
                    value: ", then season to taste.".to_string()
                }
            ]
        );
    }

    #[test]
    fn test_value_formatting() {
        use crate::{format_amount, format_value, Amount, Value};

        // Test fraction formatting
        let val = Value::Number { value: 0.5 };
        assert_eq!(format_value(&val), Some("1/2".to_string()));

        let val = Value::Number { value: 0.25 };
        assert_eq!(format_value(&val), Some("1/4".to_string()));

        let val = Value::Number { value: 0.75 };
        assert_eq!(format_value(&val), Some("3/4".to_string()));

        let val = Value::Number { value: 0.333333 };
        assert_eq!(format_value(&val), Some("1/3".to_string()));

        let val = Value::Number { value: 0.666667 };
        assert_eq!(format_value(&val), Some("2/3".to_string()));

        // Test mixed numbers
        let val = Value::Number { value: 1.5 };
        assert_eq!(format_value(&val), Some("1 1/2".to_string()));

        let val = Value::Number { value: 2.25 };
        assert_eq!(format_value(&val), Some("2 1/4".to_string()));

        // Test whole numbers
        let val = Value::Number { value: 2.0 };
        assert_eq!(format_value(&val), Some("2".to_string()));

        // Test decimals that don't convert to common fractions
        let val = Value::Number { value: 1.23 };
        assert_eq!(format_value(&val), Some("1.23".to_string()));

        // Test floating point precision issues (like from scaling)
        let val = Value::Number {
            value: 0.89999999999,
        };
        assert_eq!(format_value(&val), Some("0.9".to_string()));

        let val = Value::Number {
            value: 0.30000000001,
        };
        assert_eq!(format_value(&val), Some("0.3".to_string()));

        let val = Value::Number {
            value: 1.9999999999,
        };
        assert_eq!(format_value(&val), Some("2".to_string()));

        // Test that actual 0.899 stays as 0.899
        let val = Value::Number { value: 0.899 };
        assert_eq!(format_value(&val), Some("0.899".to_string()));

        // Test ranges
        let val = Value::Range {
            start: 0.5,
            end: 0.75,
        };
        assert_eq!(format_value(&val), Some("1/2 - 3/4".to_string()));

        // Test text values
        let val = Value::Text {
            value: "pinch".to_string(),
        };
        assert_eq!(format_value(&val), Some("pinch".to_string()));

        // Test empty value
        let val = Value::Empty;
        assert_eq!(format_value(&val), None);

        // Test Amount formatting
        let amount = Amount {
            quantity: Value::Number { value: 0.666667 },
            units: Some("cups".to_string()),
        };
        assert_eq!(format_amount(&amount), "2/3 cups");

        let amount = Amount {
            quantity: Value::Number { value: 1.5 },
            units: Some("tsp".to_string()),
        };
        assert_eq!(format_amount(&amount), "1 1/2 tsp");

        // Test Amount with floating point precision issues
        let amount = Amount {
            quantity: Value::Number {
                value: 0.89999999999,
            },
            units: Some("cups".to_string()),
        };
        assert_eq!(format_amount(&amount), "0.9 cups");
    }

    #[test]
    fn test_value_parsing() {
        use crate::{parse_value, Value};

        // Test parsing fractions
        let val = parse_value("1/2".to_string());
        assert_eq!(val, Value::Number { value: 0.5 });

        let val = parse_value("3/4".to_string());
        assert_eq!(val, Value::Number { value: 0.75 });

        // Test parsing mixed numbers
        let val = parse_value("1 1/2".to_string());
        assert_eq!(val, Value::Number { value: 1.5 });

        let val = parse_value("2 3/4".to_string());
        assert_eq!(val, Value::Number { value: 2.75 });

        // Test parsing regular numbers
        let val = parse_value("5".to_string());
        assert_eq!(val, Value::Number { value: 5.0 });

        let val = parse_value("1.5".to_string());
        assert_eq!(val, Value::Number { value: 1.5 });

        // Test parsing ranges
        let val = parse_value("1/2 - 3/4".to_string());
        assert_eq!(
            val,
            Value::Range {
                start: 0.5,
                end: 0.75
            }
        );

        let val = parse_value("1 - 2".to_string());
        assert_eq!(
            val,
            Value::Range {
                start: 1.0,
                end: 2.0
            }
        );

        // Test parsing text
        let val = parse_value("pinch".to_string());
        assert_eq!(
            val,
            Value::Text {
                value: "pinch".to_string()
            }
        );
    }
}
