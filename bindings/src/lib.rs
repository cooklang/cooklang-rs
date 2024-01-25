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
        metadata.insert(key.to_string(), value.to_string());
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
        use crate::{parse_recipe, Amount, Item, Value};

        let recipe = parse_recipe(
            r#"
a test @step @salt{1%mg} more text
"#
            .to_string(),
        );

        assert_eq!(
            recipe.steps.into_iter().nth(0).unwrap().items,
            vec![
                Item::Text {
                    value: "a test ".to_string()
                },
                Item::Ingredient {
                    name: "step".to_string(),
                    amount: None
                },
                Item::Text {
                    value: " ".to_string()
                },
                Item::Ingredient {
                    name: "salt".to_string(),
                    amount: Some(Amount {
                        quantity: Value::Number { value: 1.0 },
                        units: Some("mg".to_string())
                    })
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
            r#"
>> source: https://google.com
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
}
