// use crate::*;
use crate::analysis::*;
use crate::model::{Component, ComponentKind, Item as ModelItem};
use crate::quantity::{QuantityValue, Value as ModelValue};
use crate::parser::parse as canonical_parse;
use crate::Converter;
use crate::Extensions;
use std::collections::HashMap;

#[derive(uniffi::Record, Debug)]
pub struct CooklangRecipe {
    metadata: HashMap<String, String>,
    steps: Vec<Step>,
    ingredients: Vec<Item>,
    cookware: Vec<Item>,
}

#[derive(uniffi::Record, Debug)]
struct Step {
    items: Vec<Item>,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
enum Item {
    Text { value: String },
    Ingredient { name: String, amount: Option<Amount> },
    Cookware { name: String, amount: Option<Amount> },
    Timer { name: Option<String>, amount: Option<Amount> },
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
struct Amount {
    quantity: Value,
    units: Option<String>,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
enum Value {
    Number { value: f64 },
    Range { start: f64, end: f64 },
    Text { value: String },
}

fn into_item(item: ModelItem, recipe: &RecipeContent) -> Item {
    match item {
        ModelItem::Text { value } => Item::Text { value },
        ModelItem::ItemComponent { value } => {
            let Component { index, kind } = value;

            match kind {
                ComponentKind::IngredientKind => {
                    let igredient = &recipe.ingredients[index];
                    let amount: Option<Amount>;

                    let amount = if let Some(q) = &igredient.quantity {
                        let quantity = match &q.value {
                            // TODO remove duplication
                            QuantityValue::Fixed { value } => {
                                match value {
                                    ModelValue::Number { value } => Value::Number { value: *value },
                                    ModelValue::Range { value } => Value::Range { start: *value.start(), end: *value.end() },
                                    ModelValue::Text { value } => Value::Text { value: value.to_string() },
                                }
                            },
                            QuantityValue::Linear { value } => {
                                match value {
                                    ModelValue::Number { value } => Value::Number { value: *value },
                                    ModelValue::Range { value } => Value::Range { start: *value.start(), end: *value.end() },
                                    ModelValue::Text { value } => Value::Text { value: value.to_string() },
                                }
                            },
                            QuantityValue::ByServings { values } => {
                                match values.first().unwrap() {
                                    ModelValue::Number { value } => Value::Number { value: *value },
                                    ModelValue::Range { value } => Value::Range { start: *value.start(), end: *value.end() },
                                    ModelValue::Text { value } => Value::Text { value: value.to_string() },
                                }
                            }
                        };

                        let units = if let Some(u) = &q.unit {
                            Some(u.to_string())
                        } else {
                            None
                        };

                        Some(Amount {
                            quantity: quantity,
                            units: units,
                        })
                    } else {
                        None
                    };

                    Item::Ingredient {
                        name: igredient.name.clone(),
                        amount: amount,
                    }
                }

                // TODO amount
                ComponentKind::CookwareKind => {
                    let cookware = &recipe.cookware[index];
                    Item::Cookware {
                        name: cookware.name.clone(),
                        amount: None,
                    }
                }

                // TODO amount
                ComponentKind::TimerKind => {
                    let timer = &recipe.timers[index];

                    if let Some(name) = &timer.name {
                        Item::Timer {
                            name: Some(name.to_string()),
                            amount: None,
                        }
                    } else {
                        Item::Timer {
                            name: None,
                            amount: None,
                        }
                    }

                }
            }
        }
        ModelItem::InlineQuantity { .. } => Item::Text { value: "".to_string(),},
    }
}

fn simplify_recipe_data(recipe: &RecipeContent) -> CooklangRecipe {
    let mut metadata = HashMap::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut ingredients: Vec<Item> = Vec::new();
    let mut cookware: Vec<Item> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    (&recipe.sections).iter().for_each(|section| {
        (&section.steps).iter().for_each(|step| {
            (&step.items).iter().for_each(|item| {
                let i = into_item(item.clone(), &recipe);

                match i {
                    Item::Ingredient { .. } => ingredients.push(i.clone()),
                    Item::Cookware { .. } => cookware.push(i.clone()),
                    _ => (),
                };

                items.push(i);
            });
            // TODO: think how to make it faster as we probably
            // can switch items content into the step without cloning it
            steps.push(Step {
                items: items.clone(),
            });
            items.clear();
        })
    });

    (&recipe.metadata.map).iter().for_each(|(key, value)| {
        metadata.insert(key.to_string(), value.to_string());
    });

    CooklangRecipe {
        metadata: metadata,
        steps: steps,
        ingredients: ingredients,
        cookware: cookware,
    }
}

#[uniffi::export]
pub fn parse(input: String) -> CooklangRecipe {
    let extensions = Extensions::empty();
    let converter = Converter::empty();

    let ast = canonical_parse(&input, extensions).take_output().unwrap();
    let result = parse_ast(ast, extensions, &converter, None)
        .take_output()
        .unwrap();

    simplify_recipe_data(&result)
}

uniffi::setup_scaffolding!();

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn just_kidding() {
        let recipe = crate::bindings::parse(
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
                    amount: Amount {
                        quantity: "".to_string(),
                        units: "".to_string()
                    }
                },
                Item::Text {
                    value: " ".to_string()
                },
                Item::Ingredient {
                    name: "salt".to_string(),
                    amount: Amount {
                        quantity: "1".to_string(),
                        units: "".to_string()
                    }
                },
                Item::Text {
                    value: " more text".to_string()
                }
            ]
        );
    }
}
