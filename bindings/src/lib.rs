use std::sync::Arc;

use cooklang::aisle::parse as parse_aisle_config_original;
use cooklang::analysis::parse_events;
use cooklang::parser::PullParser;
use cooklang::{Converter, Extensions};

pub mod aisle;
pub mod model;

use aisle::*;
use model::*;

#[uniffi::export]
pub fn parse_recipe(input: String) -> CooklangRecipe {
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    let mut parser = PullParser::new(&input, extensions);
    let parsed = parse_events(
        &mut parser,
        &input,
        extensions,
        &converter,
        Default::default(),
    )
    .unwrap_output();

    into_simple_recipe(&parsed)
}

#[uniffi::export]
pub fn parse_metadata(input: String) -> CooklangMetadata {
    let mut metadata = CooklangMetadata::new();
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    let parser = PullParser::new(&input, extensions);

    let parsed = parse_events(
        parser.into_meta_iter(),
        &input,
        extensions,
        &converter,
        Default::default(),
    )
    .map(|c| c.metadata.map)
    .unwrap_output();

    // converting IndexMap into HashMap
    let _ = &(parsed).iter().for_each(|(key, value)| {
        if let (Some(key), Some(value)) = (key.as_str(), value.as_str()) {
            metadata.insert(key.to_string(), value.to_string());
        }
    });

    metadata
}

#[uniffi::export]
pub fn deref_component(recipe: &CooklangRecipe, item: Item) -> Component {
    match item {
        Item::IngredientRef { index } => {
            Component::Ingredient(recipe.ingredients.get(index as usize).unwrap().clone())
        }
        Item::CookwareRef { index } => {
            Component::Cookware(recipe.cookware.get(index as usize).unwrap().clone())
        }
        Item::TimerRef { index } => {
            Component::Timer(recipe.timers.get(index as usize).unwrap().clone())
        }
        Item::Text { value } => Component::Text(value),
    }
}

#[uniffi::export]
pub fn deref_ingredient(recipe: &CooklangRecipe, index: u32) -> Ingredient {
    recipe.ingredients.get(index as usize).unwrap().clone()
}

#[uniffi::export]
pub fn deref_cookware(recipe: &CooklangRecipe, index: u32) -> Cookware {
    recipe.cookware.get(index as usize).unwrap().clone()
}

#[uniffi::export]
pub fn deref_timer(recipe: &CooklangRecipe, index: u32) -> Timer {
    recipe.timers.get(index as usize).unwrap().clone()
}

#[uniffi::export]
pub fn parse_aisle_config(input: String) -> Arc<AisleConf> {
    let mut categories: Vec<AisleCategory> = Vec::new();
    let mut cache: AisleReverseCategory = AisleReverseCategory::default();

    let parsed = parse_aisle_config_original(&input).unwrap();

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

#[uniffi::export]
pub fn combine_ingredients(ingredients: &Vec<Ingredient>) -> IngredientList {
    let indices = (0..ingredients.len()).map(|i| i as u32).collect();
    combine_ingredients_selected(ingredients, &indices)
}

#[uniffi::export]
pub fn combine_ingredients_selected(
    ingredients: &[Ingredient],
    indices: &Vec<u32>,
) -> IngredientList {
    let mut combined: IngredientList = IngredientList::default();

    expand_with_ingredients(ingredients, &mut combined, indices);

    combined
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
        );

        assert_eq!(
            deref_component(&recipe, Item::IngredientRef { index: 1 }),
            Component::Ingredient(Ingredient {
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
                .into_iter()
                .next()
                .expect("No blocks found")
                .blocks
                .into_iter()
                .next()
                .expect("No blocks found")
            {
                Block::Step(step) => step,
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
    fn test_parse_metadata() {
        use crate::parse_metadata;
        use std::collections::HashMap;

        let metadata = parse_metadata(
            r#"---
source: https://google.com
---
a test @step @salt{1%mg} more text
"#
            .to_string(),
        );

        assert_eq!(
            metadata,
            HashMap::from([("source".to_string(), "https://google.com".to_string())])
        );
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
        );

        let first_section = recipe
            .sections
            .into_iter()
            .next()
            .expect("No sections found");

        assert_eq!(first_section.blocks.len(), 2);

        // Check note block
        let mut iterator = first_section.blocks.into_iter();
        let note_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match note_block {
                Block::Note(note) => note,
                _ => panic!("Expected first block to be a Note"),
            }
            .text,
            "This dish is even better the next day, after the flavors have melded overnight."
                .to_string()
        );

        // Check step block
        let step_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match step_block {
                Block::Step(step) => step,
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
        );
        let first_section = recipe
            .sections
            .into_iter()
            .next()
            .expect("No sections found");
        assert_eq!(first_section.blocks.len(), 2);

        // Check first step
        let mut iterator = first_section.blocks.into_iter();
        let first_block = iterator.next().expect("No blocks found");
        let second_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match first_block {
                Block::Step(step) => step,
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
                Block::Step(step) => step,
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
        );

        let mut sections = recipe.sections.into_iter();

        // Check first section
        let first_section = sections.next().expect("No sections found");
        assert_eq!(first_section.title, Some("Dough".to_string()));
        assert_eq!(first_section.blocks.len(), 1);

        let first_block = first_section
            .blocks
            .into_iter()
            .next()
            .expect("No blocks found");
        assert_eq!(
            match first_block {
                Block::Step(step) => step,
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
        let second_section = sections.next().expect("No second section found");
        assert_eq!(second_section.title, Some("Filling".to_string()));
        assert_eq!(second_section.blocks.len(), 1);

        let second_block = second_section
            .blocks
            .into_iter()
            .next()
            .expect("No blocks found");
        assert_eq!(
            match second_block {
                Block::Step(step) => step,
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
}
