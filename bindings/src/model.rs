use std::collections::HashMap;

use cooklang::model::Item as ModelItem;
use cooklang::quantity::{
    Quantity as ModelQuantity, ScalableValue as ModelScalableValue, Value as ModelValue,
};
use cooklang::ScalableRecipe;

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

// I(dubadub) haven't found a way to export these methods with mutable argument
pub fn add_to_ingredient_list(list: &mut IngredientList, name: &String, quantity_to_add: &GroupedQuantity) {
    if let Some(quantity) = list.get_mut(name) {
        merge_grouped_quantities(quantity, quantity_to_add);
    } else {
        list.insert(name.to_string(), quantity_to_add.clone());
    }
}


// O(n2)? find a better way
pub fn merge_ingredient_lists(left: &mut IngredientList, right: &IngredientList) {
    right.iter().for_each(|(ingredient_name, grouped_quantity)| {
        let quantity = left.entry(ingredient_name.to_string()).or_insert(GroupedQuantity::default());

        merge_grouped_quantities(quantity, grouped_quantity);
    });
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

impl Amountable for ModelQuantity<ModelScalableValue> {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(&self.value);

        let units = self.unit().as_ref().map(|u| u.to_string());

        Amount { quantity, units }
    }
}

impl Amountable for ModelScalableValue {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_quantity(self);

        Amount {
            quantity,
            units: None,
        }
    }
}

fn extract_quantity(value: &ModelScalableValue) -> Value {
    match value {
        ModelScalableValue::Fixed(value) => extract_value(value),
        ModelScalableValue::Linear(value) => extract_value(value),
        ModelScalableValue::ByServings(values) => extract_value(values.first().unwrap()),
    }
}

fn extract_value(value: &ModelValue) -> Value {
    match value {
        ModelValue::Number(num) => Value::Number { value: num.value() },
        ModelValue::Range { start, end } => Value::Range {
            start: start.value(),
            end: end.value(),
        },
        ModelValue::Text(value) => Value::Text {
            value: value.to_string(),
        },
    }
}

pub(crate) fn into_item(item: ModelItem, recipe: &ScalableRecipe) -> Item {
    match item {
        ModelItem::Text { value } => Item::Text { value },
        ModelItem::Ingredient { index } => {
            let ingredient = &recipe.ingredients[index];

            Item::Ingredient {
                name: ingredient.name.clone(),
                amount: ingredient.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        ModelItem::Cookware { index } => {
            let cookware = &recipe.cookware[index];
            Item::Cookware {
                name: cookware.name.clone(),
                amount: cookware.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        ModelItem::Timer { index } => {
            let timer = &recipe.timers[index];

            Item::Timer {
                name: timer.name.clone(),
                amount: timer.quantity.as_ref().map(|q| q.extract_amount()),
            }
        }

        // returning an empty block of text as it's not supported by the spec
        ModelItem::InlineQuantity { index: _ } => Item::Text {
            value: "".to_string(),
        },
    }
}
