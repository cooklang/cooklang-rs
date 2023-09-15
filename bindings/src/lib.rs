use std::collections::HashMap;
use cooklang::analysis::{parse_events, RecipeContent};
use cooklang::model::{Item as ModelItem};
use cooklang::parser::{PullParser};
use cooklang::quantity::{
    Quantity as ModelQuantity, ScalableValue as ModelScalableValue, Value as ModelValue,
};
use cooklang::Converter;
use cooklang::Extensions;

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
    Text {
        value: String,
    },
    Ingredient {
        name: String,
        amount: Option<Amount>,
    },
    Cookware {
        name: String,
        amount: Option<Amount>,
    },
    Timer {
        name: Option<String>,
        amount: Option<Amount>,
    },
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


trait Amountable {
    fn extract_amount(&self) -> Amount;
}

impl Amountable for ModelQuantity<ModelScalableValue> {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(&self.value);

        let units = if let Some(u) = &self.unit() {
            Some(u.to_string())
        } else {
            None
        };

        Amount { quantity, units }
    }
}

impl Amountable for ModelScalableValue {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(&self);

        Amount {
            quantity,
            units: None,
        }
    }
}

fn extract_quantity(value: &ModelScalableValue) -> Value {
    match value {
        ModelScalableValue::Fixed { value } => extract_value(value),
        ModelScalableValue::Linear { value } => extract_value(value),
        ModelScalableValue::ByServings { values } => extract_value(values.first().unwrap()),
    }
}

fn extract_value(value: &ModelValue) -> Value {
    match value {
        ModelValue::Number { value } => Value::Number { value: *value },
        ModelValue::Range { value } => Value::Range {
            start: *value.start(),
            end: *value.end(),
        },
        ModelValue::Text { value } => Value::Text {
            value: value.to_string(),
        },
    }
}


fn into_item(item: ModelItem, recipe: &RecipeContent) -> Item {
    match item {
        ModelItem::Text { value } => Item::Text { value },
        ModelItem::ItemIngredient { index } => {
            let ingredient = &recipe.ingredients[index];

            Item::Ingredient {
                name: ingredient.name.clone(),
                amount: if let Some(q) = &ingredient.quantity {
                    Some(q.extract_amount())
                } else {
                    None
                },
            }
        },

        ModelItem::ItemCookware { index } => {
            let cookware = &recipe.cookware[index];
            Item::Cookware {
                name: cookware.name.clone(),
                amount: if let Some(q) = &cookware.quantity {
                    Some(q.extract_amount())
                } else {
                    None
                },
            }
        },

        ModelItem::ItemTimer { index } => {
            let timer = &recipe.timers[index];

            Item::Timer {
                name: timer.name.clone(),
                amount: if let Some(q) = &timer.quantity {
                    Some(q.extract_amount())
                } else {
                    None
                },
            }
        },

        // returning an empty block of text as it's not supported by the spec
        ModelItem::InlineQuantity { .. } => Item::Text {
            value: "".to_string(),
        },
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
        })
    });

    (&recipe.metadata.map).iter().for_each(|(key, value)| {
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
    let result = parse_events(&mut parser, extensions, &converter, None,)
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
        let recipe = cooklang::bindings::parse(
            r#"
a test @step @salt{1%mg} more text
"#
            .to_string(),
        );

        assert_eq!(
            recipe.steps.into_iter().nth(0).unwrap().items,
            vec![
                Item::Text { value: "a test ".to_string() },
                Item::Ingredient {
                    name: "step".to_string(),
                    amount: None
                },
                Item::Text { value: " ".to_string() },
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
}
