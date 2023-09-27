use std::collections::HashMap;
use std::collections::BTreeMap;

use cooklang::model::{Item as ModelItem};
use cooklang::analysis::{RecipeContent};
use cooklang::quantity::{
    Quantity as ModelQuantity, ScalableValue as ModelScalableValue, Value as ModelValue,
};



#[derive(uniffi::Record, Debug)]
pub struct CooklangRecipe {
    pub metadata: HashMap<String, String>,
    pub steps: Vec<Step>,
    pub ingredients: Vec<Item>,
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

#[derive(uniffi::Record, Debug)]
pub struct IngredientList(BTreeMap<String, GroupedQuantity>);

// #[derive(uniffi::Record, Debug)]
pub struct GroupedQuantity {

}


#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Amount {
    quantity: Value,
    units: Option<String>,
}

#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Value {
    Number { value: f64 },
    Range { start: f64, end: f64 },
    Text { value: String },
}

pub type CooklangMetadata = HashMap<String, String>;

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


pub fn into_item(item: ModelItem, recipe: &RecipeContent) -> Item {
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
