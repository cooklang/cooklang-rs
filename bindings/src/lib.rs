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
    let _ = &(parsed)
        .iter()
        .for_each(|(key, value)| match (key.as_str(), value.as_str()) {
            (Some(key), Some(value)) => {
                metadata.insert(key.to_string(), value.to_string());
            }
            _ => {}
        });

    metadata
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
pub fn combine_ingredient_lists(lists: Vec<IngredientList>) -> IngredientList {
    let mut combined: IngredientList = IngredientList::default();

    lists
        .iter()
        .for_each(|l| merge_ingredient_lists(&mut combined, l));

    combined
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {

    #[test]
    fn test_parse_recipe() {
        use crate::{parse_recipe, Amount, Item, Value, Block};

        let recipe = parse_recipe(
            r#"
a test @step @salt{1%mg} more text
"#
            .to_string(),
        );

        assert_eq!(
            match recipe.sections.into_iter().nth(0).expect("No blocks found").blocks.into_iter().nth(0).expect("No blocks found") {
                Block::Step(step) => step,
                _ => panic!("Expected first block to be a Step"),
            }.items,
            vec![
                Item::Text {
                    value: "a test ".to_string()
                },
                Item::Ingredient {
                    name: "step".to_string(),
                    amount: None,
                    preparation: None
                },
                Item::Text {
                    value: " ".to_string()
                },
                Item::Ingredient {
                    name: "salt".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 1.0 },
                        units: Some("mg".to_string())
                    }),
                    preparation: None
                },
                Item::Text {
                    value: " more text".to_string()
                }
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
    fn test_combine_ingredient_lists() {
        use crate::{combine_ingredient_lists, GroupedQuantityKey, QuantityType, Value};
        use std::collections::HashMap;

        let combined = combine_ingredient_lists(vec![
            HashMap::from([
                (
                    "salt".to_string(),
                    HashMap::from([
                        (
                            GroupedQuantityKey {
                                name: "g".to_string(),
                                unit_type: QuantityType::Number,
                            },
                            Value::Number { value: 5.0 },
                        ),
                        (
                            GroupedQuantityKey {
                                name: "tsp".to_string(),
                                unit_type: QuantityType::Number,
                            },
                            Value::Number { value: 1.0 },
                        ),
                    ]),
                ),
                (
                    "pepper".to_string(),
                    HashMap::from([
                        (
                            GroupedQuantityKey {
                                name: "mg".to_string(),
                                unit_type: QuantityType::Number,
                            },
                            Value::Number { value: 5.0 },
                        ),
                        (
                            GroupedQuantityKey {
                                name: "tsp".to_string(),
                                unit_type: QuantityType::Number,
                            },
                            Value::Number { value: 1.0 },
                        ),
                    ]),
                ),
            ]),
            HashMap::from([(
                "salt".to_string(),
                HashMap::from([
                    (
                        GroupedQuantityKey {
                            name: "kg".to_string(),
                            unit_type: QuantityType::Number,
                        },
                        Value::Number { value: 0.005 },
                    ),
                    (
                        GroupedQuantityKey {
                            name: "tsp".to_string(),
                            unit_type: QuantityType::Number,
                        },
                        Value::Number { value: 1.0 },
                    ),
                ]),
            )]),
        ]);

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
                        name: "tsp".to_string(),
                        unit_type: QuantityType::Number
                    },
                    Value::Number { value: 2.0 }
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
        use crate::{parse_recipe, Amount, Item, Value, Block};

        let recipe = parse_recipe(
            r#"
> This dish is even better the next day, after the flavors have melded overnight.

Cook @onions{3%large} until brown
"#
            .to_string(),
        );

        let first_section = recipe.sections.into_iter().nth(0).expect("No sections found");

        assert_eq!(first_section.blocks.len(), 2);

        // Check note block
        let mut iterator = first_section.blocks.into_iter();
        let note_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match note_block {
                Block::Note(note) => note,
                _ => panic!("Expected first block to be a Note"),
            }.text,
            "This dish is even better the next day, after the flavors have melded overnight.".to_string()
        );

        // Check step block
        let step_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match step_block {
                Block::Step(step) => step,
                _ => panic!("Expected second block to be a Step"),
            }.items,
            vec![
                Item::Text { value: "Cook ".to_string() },
                Item::Ingredient {
                    name: "onions".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 3.0 },
                        units: Some("large".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: " until brown".to_string() }
            ]
        );
    }

    #[test]
    fn test_parse_recipe_with_multiline_steps() {
        use crate::{parse_recipe, Amount, Item, Value, Block};

        let recipe = parse_recipe(
            r#"
add @onions{2} to pan
heat until golden

add @tomatoes{400%g}
simmer for 10 minutes
"#
            .to_string(),
        );
        let first_section = recipe.sections.into_iter().nth(0).expect("No sections found");
        assert_eq!(first_section.blocks.len(), 2);

        // Check first step
        let mut iterator = first_section.blocks.into_iter();
        let first_block = iterator.next().expect("No blocks found");
        let second_block = iterator.next().expect("No blocks found");

        assert_eq!(
            match first_block {
                Block::Step(step) => step,
                _ => panic!("Expected first block to be a Step"),
            }.items,
            vec![
                Item::Text { value: "add ".to_string() },
                Item::Ingredient {
                    name: "onions".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 2.0 },
                        units: None
                    }),
                    preparation: None
                },
                Item::Text { value: " to pan heat until golden".to_string() }
            ]
        );

        // Check second step
        assert_eq!(
            match second_block {
                Block::Step(step) => step,
                _ => panic!("Expected second block to be a Step"),
            }.items,
            vec![
                Item::Text { value: "add ".to_string() },
                Item::Ingredient {
                    name: "tomatoes".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 400.0 },
                        units: Some("g".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: " simmer for 10 minutes".to_string() }
            ]
        );
    }

    #[test]
    fn test_parse_recipe_with_sections() {
        use crate::{parse_recipe, Amount, Item, Value, Block};

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

        let first_block = first_section.blocks.into_iter().next().expect("No blocks found");
        assert_eq!(
            match first_block {
                Block::Step(step) => step,
                _ => panic!("Expected block to be a Step"),
            }.items,
            vec![
                Item::Text { value: "Mix ".to_string() },
                Item::Ingredient {
                    name: "flour".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 200.0 },
                        units: Some("g".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: " and ".to_string() },
                Item::Ingredient {
                    name: "water".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 50.0 },
                        units: Some("ml".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: " together until smooth.".to_string() }
            ]
        );

        // Check second section
        let second_section = sections.next().expect("No second section found");
        assert_eq!(second_section.title, Some("Filling".to_string()));
        assert_eq!(second_section.blocks.len(), 1);

        let second_block = second_section.blocks.into_iter().next().expect("No blocks found");
        assert_eq!(
            match second_block {
                Block::Step(step) => step,
                _ => panic!("Expected block to be a Step"),
            }.items,
            vec![
                Item::Text { value: "Combine ".to_string() },
                Item::Ingredient {
                    name: "cheese".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 100.0 },
                        units: Some("g".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: " and ".to_string() },
                Item::Ingredient {
                    name: "spinach".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 50.0 },
                        units: Some("g".to_string())
                    }),
                    preparation: None
                },
                Item::Text { value: ", then season to taste.".to_string() }
            ]
        );
    }
}
