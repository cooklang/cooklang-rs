// use crate::*;
use crate::analysis::*;
use crate::parser::{parse as canonical_parse};
use crate::model::{ComponentKind, Component, Item as ModelItem};
use crate::Converter;
use crate::Extensions;
use std::collections::HashMap;


#[derive(uniffi::Record, Debug)]
pub struct CooklangRecipe {
    metadata: HashMap<String,String>,
    steps: Vec<Step>,
    ingredients: Vec<Item>,
    cookware: Vec<Item>,
}

#[derive(uniffi::Record, Debug)]
struct Step {
    items: Vec<Item>
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
struct Amount {
    quantity: String,
    units: String
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
enum Item {
    Text { value: String },
    Ingredient { name: String, amount: Amount },
    Cookware { name: String },
    Timer { name: String },
}

fn into_item(item: ModelItem, recipe: &RecipeContent) -> Item {
     match item {
        ModelItem::Text { value } => Item::Text { value },
        ModelItem::ItemComponent { value } => {
            let Component { index, kind } = value;

            match kind {
                ComponentKind::IngredientKind => {
                    let igredient = &recipe.ingredients[index];
                    let mut q: String = "".to_string();
                    let u: String = "".to_string();

                    if let Some(quantity) = &igredient.quantity {
                        q = quantity.value.to_string();

                        // if Some(unit) = quantity.unit {
                        //     q = match unit {
                        //         QuantityValue::Fixed { value } =>
                        //     }
                        // }
                    }
                    let amount = Amount { quantity: q, units: u };

                    Item::Ingredient { name: igredient.name.clone(), amount: amount }
                }

                ComponentKind::CookwareKind => {
                    let cookware = &recipe.cookware[index];
                    Item::Cookware { name: cookware.name.clone() }
                }

                ComponentKind::TimerKind => {
                    let timer = &recipe.timers[index];

                    if let Some(name) = &timer.name {
                        Item::Timer { name: name.to_string() }
                    } else {
                        Item::Timer { name: "".to_string() }
                    }
                    // if let Some(quantity) = &t.quantity {

                    // }
                }
            }
        },
        _ => Item::Text { value: "".to_string() }
    }
}

fn dumb_down_recipe(recipe: &RecipeContent) -> CooklangRecipe {
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
                        Item::Ingredient { name: _, amount: _ } => ingredients.push(i.clone()),
                        Item::Cookware { name: _ } => cookware.push(i.clone()),
                        _ => (),
                    };

                    items.push(i);
            });
            // TODO: think how to make it faster as we probably
            // can switch items content into the step without cloning it
            steps.push(Step { items: items.clone() });
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

    dumb_down_recipe(&result)
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
"#.to_string(),
        );

        assert_eq!(
            recipe.steps.into_iter().nth(0).unwrap().items,
            vec![
                Item::Text { value: "a test ".to_string() },
                Item::Ingredient { name: "step".to_string(), amount: Amount { quantity: "".to_string(), units: "".to_string() } },
                Item::Text { value: " ".to_string() },
                Item::Ingredient { name: "salt".to_string(), amount: Amount { quantity: "1".to_string(), units: "".to_string() } },
                Item::Text { value: " more text".to_string() }
            ]
        );
    }
}

