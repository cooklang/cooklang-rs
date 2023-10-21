use std::collections::HashMap;

use cooklang::model::Item as OriginalItem;
use cooklang::quantity::{
    Quantity as OriginalQuantity, ScalableValue as OriginalScalableValue, Value as OriginalValue,
};
use cooklang::ScalableRecipe as OriginalRecipe;

#[derive(uniffi::Record, Debug)]
pub struct CooklangRecipe {
    pub metadata: HashMap<String, String>,
    pub steps: Vec<Step>,
    pub ingredients: IngredientList,
    pub cookware: Vec<Item>,
}

#[derive(uniffi::Record, Debug)]
pub struct Step {
    pub items: Vec<Item>,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Item {
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

pub type IngredientList = HashMap<String, GroupedQuantity>;

pub(crate) fn into_group_quantity(amount: &Option<Amount>) -> GroupedQuantity {
    // options here:
    // - same units:
    //    - same value type
    //    - not the same value type
    // - different units
    // - no units
    // - no amount
    //
    // \
    //  |- <litre,Number> => 1.2
    //  |- <litre,Text> => half
    //  |- <,Text> => pinch
    //  |- <,Empty> => Some
    //
    //
    // TODO define rules on language spec level???
    let empty_units = "".to_string();

    let key = if let Some(amount) = amount {
        let units = amount.units.as_ref().unwrap_or(&empty_units);

        match &amount.quantity {
            Value::Number { .. } => HardToNameWTF {
                name: units.to_string(),
                unit_type: QuantityType::Number,
            },
            Value::Range { .. } => HardToNameWTF {
                name: units.to_string(),
                unit_type: QuantityType::Range,
            },
            Value::Text { .. } => HardToNameWTF {
                name: units.to_string(),
                unit_type: QuantityType::Text,
            },
            Value::Empty => HardToNameWTF {
                name: units.to_string(),
                unit_type: QuantityType::Empty,
            },
        }
    } else {
        HardToNameWTF {
            name: empty_units,
            unit_type: QuantityType::Empty,
        }
    };

    let value = if let Some(amount) = amount {
        amount.quantity.clone()
    } else {
        Value::Empty
    };

    GroupedQuantity::from([(key, value)])
}

#[derive(uniffi::Enum, Debug, Clone, Hash, Eq, PartialEq)]
pub enum QuantityType {
    Number,
    Range, // how to combine ranges?
    Text,
    Empty,
}

#[derive(uniffi::Record, Debug, Clone, Hash, Eq, PartialEq)]
pub struct HardToNameWTF {
    pub name: String,
    pub unit_type: QuantityType,
}

pub type GroupedQuantity = HashMap<HardToNameWTF, Value>;

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Amount {
    pub(crate) quantity: Value,
    pub(crate) units: Option<String>,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Value {
    Number { value: f64 },
    Range { start: f64, end: f64 },
    Text { value: String },
    Empty,
}

pub type CooklangMetadata = HashMap<String, String>;

trait Amountable {
    fn extract_amount(&self) -> Amount;
}

impl Amountable for OriginalQuantity<OriginalScalableValue> {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(&self.value);

        let units = self.unit().as_ref().map(|u| u.to_string());

        Amount { quantity, units }
    }
}

impl Amountable for OriginalScalableValue {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(self);

        Amount {
            quantity,
            units: None,
        }
    }
}

fn extract_quantity(value: &OriginalScalableValue) -> Value {
    match value {
        OriginalScalableValue::Fixed(value) => extract_value(value),
        OriginalScalableValue::Linear(value) => extract_value(value),
        OriginalScalableValue::ByServings(values) => extract_value(values.first().unwrap()),
    }
}

fn extract_value(value: &OriginalValue) -> Value {
    match value {
        OriginalValue::Number(num) => Value::Number { value: num.value() },
        OriginalValue::Range { start, end } => Value::Range {
            start: start.value(),
            end: end.value(),
        },
        OriginalValue::Text(value) => Value::Text {
            value: value.to_string(),
        },
    }
}

// I(dubadub) haven't found a way to export these methods with mutable argument
pub fn add_to_ingredient_list(
    list: &mut IngredientList,
    name: &String,
    quantity_to_add: &GroupedQuantity,
) {
    if let Some(quantity) = list.get_mut(name) {
        merge_grouped_quantities(quantity, quantity_to_add);
    } else {
        list.insert(name.to_string(), quantity_to_add.clone());
    }
}

// O(n2)? find a better way
pub fn merge_ingredient_lists(left: &mut IngredientList, right: &IngredientList) {
    right
        .iter()
        .for_each(|(ingredient_name, grouped_quantity)| {
            let quantity = left
                .entry(ingredient_name.to_string())
                .or_insert(GroupedQuantity::default());

            merge_grouped_quantities(quantity, grouped_quantity);
        });
}

// I(dubadub) haven't found a way to export these methods with mutable argument
// Right should be always smaller?
pub(crate) fn merge_grouped_quantities(left: &mut GroupedQuantity, right: &GroupedQuantity) {
    // options here:
    // - same units:
    //    - same value type
    //    - not the same value type
    // - different units
    // - no units
    // - no amount
    //
    // \
    //  |- <litre,Number> => 1.2
    //  |- <litre,Text> => half
    //  |- <,Text> => pinch
    //  |- <,Empty> => Some
    //
    //
    // TODO define rules on language spec level

    right.iter().for_each(|(key, value)| {
        left
            .entry(key.clone())
            .and_modify(|v| {
                match key.unit_type {
                    QuantityType::Number => {
                        let Value::Number { value: assignable } = value else { panic!("Unexpected type") };
                        let Value::Number { value: stored } = v else { panic!("Unexpected type") };

                        *stored += assignable
                    },
                    QuantityType::Range => {
                        let Value::Range { start, end } = value else { panic!("Unexpected type") };
                        let Value::Range { start: s, end: e } = v else { panic!("Unexpected type") };

                        // is it even correct?
                        *s += start;
                        *e += end;
                    },
                    QuantityType::Text => {
                        let Value::Text { value: ref assignable } = value else { panic!("Unexpected type") };
                        let Value::Text { value: stored } = v else { panic!("Unexpected type") };

                        *stored += assignable;
                    },
                    QuantityType::Empty => {
                        todo!();
                    },

                }
            })
            .or_insert(value.clone());
    });
}

pub(crate) fn into_item(item: &OriginalItem, recipe: &OriginalRecipe) -> Item {
    match item {
        OriginalItem::Text { value } => Item::Text {
            value: value.to_string(),
        },
        OriginalItem::Ingredient { index } => {
            let ingredient = &recipe.ingredients[*index];

            Item::Ingredient {
                name: ingredient.name.clone(),
                amount: ingredient.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        OriginalItem::Cookware { index } => {
            let cookware = &recipe.cookware[*index];
            Item::Cookware {
                name: cookware.name.clone(),
                amount: cookware.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        OriginalItem::Timer { index } => {
            let timer = &recipe.timers[*index];

            Item::Timer {
                name: timer.name.clone(),
                amount: timer.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        // returning an empty block of text as it's not supported by the spec
        OriginalItem::InlineQuantity { index: _ } => Item::Text {
            value: "".to_string(),
        },
    }
}

pub(crate) fn into_simple_recipe(recipe: &OriginalRecipe) -> CooklangRecipe {
    let mut metadata = CooklangMetadata::new();
    let mut steps: Vec<Step> = Vec::new();
    let mut ingredients: IngredientList = IngredientList::default();
    let mut cookware: Vec<Item> = Vec::new();
    let mut items: Vec<Item> = Vec::new();

    recipe.sections.iter().for_each(|section| {
        section.content.iter().for_each(|content| {
            if let cooklang::Content::Step(step) = content {
                step.items.iter().for_each(|i| {
                    let item = into_item(i, recipe);

                    match item {
                        Item::Ingredient {
                            ref name,
                            ref amount,
                        } => {
                            let quantity = into_group_quantity(amount);

                            add_to_ingredient_list(&mut ingredients, name, &quantity);
                        }
                        Item::Cookware { .. } => {
                            cookware.push(item.clone());
                        }
                        // don't need anything if timer or text
                        _ => (),
                    };
                    items.push(item);
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
