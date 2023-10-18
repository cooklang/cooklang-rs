use std::sync::Arc;

use cooklang::aisle::parse as parse_aisle_config_original;
use cooklang::analysis::{parse_events};
use cooklang::parser::PullParser;
use cooklang::Extensions;
use cooklang::{Converter, ScalableRecipe};

pub mod aisle;
pub mod model;

use aisle::*;
use model::*;

fn simplify_recipe_data(recipe: &ScalableRecipe) -> CooklangRecipe {
    let mut metadata = CooklangMetadata::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut ingredients: IngredientList = IngredientList::default();
    let mut cookware: Vec<Item> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    (&recipe.sections).iter().for_each(|section| {
        (&section.content).iter().for_each(|content| {
            if let cooklang::Content::Step(step) = content {
                (&step.items).iter().for_each(|item| {
                    let i = into_item(item.clone(), recipe);

                    match i {
                        // TODO
                        Item::Ingredient {
                            ref name,
                            ref amount,
                        } => {
                            add_to_ingredient_list(&mut ingredients, name.to_string(), amount);
                        }
                        Item::Cookware { .. } => {
                            cookware.push(i.clone());
                        }
                        // don't need anything if timer or text
                        _ => (),
                    };
                        items.push(i);
                });
                // TODO: think how to make it faster as we probably
                // can switch items content directly into the step object without cloning it
                steps.push(Step {
                    items: items.clone(),
                });

                items.clear();
            }
        });
    });


    recipe.metadata.map.iter().for_each(|(key, value)| {
        metadata.insert(key.to_string(), value.to_string());
    });

    CooklangRecipe {
        metadata,
        steps,
        ingredients,
        cookware,
    }
}

#[uniffi::export]
pub fn parse(input: String) -> CooklangRecipe {
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    let mut parser = PullParser::new(&input, extensions);
    let parsed = parse_events(&mut parser, extensions, &converter, None)
        .take_output()
        .unwrap();

    simplify_recipe_data(&parsed)
}

#[uniffi::export]
pub fn parse_metadata(input: String) -> CooklangMetadata {
    let mut metadata = CooklangMetadata::new();
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    let parser = PullParser::new(&input, extensions);

    let parsed = parse_events(parser.into_meta_iter(), extensions, &converter, None)
        .map(|c| c.metadata.map)
        .take_output()
        .unwrap();

    let _ = &(parsed).iter().for_each(|(key, value)| {
        metadata.insert(key.to_string(), value.to_string());
    });

    metadata
}

#[uniffi::export]
pub fn parse_aisle_config(input: String) -> Arc<AisleConf> {
    let mut categories: Vec<AisleCategory> = Vec::new();
    let mut ingredients: Vec<AisleIngredient> = Vec::new();

    let parsed = parse_aisle_config_original(&input).unwrap();

    let _ = &(parsed).categories.iter().for_each(|c| {
        c.ingredients.iter().for_each(|i| {
            let mut it = i.names.iter();

            let name = it.next().unwrap().to_string();
            let aliases = it.map(|v| v.to_string()).collect();

            ingredients.push(AisleIngredient { name, aliases });
        });

        let category = AisleCategory {
            name: c.name.to_string(),
            ingredients: ingredients.clone(),
        };

        ingredients.clear();
        categories.push(category);
    });

    let config = AisleConf { categories };

    Arc::new(config)
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {

    #[test]
    fn parse() {
        use crate::{parse, Amount, Item, Value};

        let recipe = parse(
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
    fn parse_metadata() {
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
}
