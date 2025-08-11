use std::collections::HashMap;

use cooklang::metadata::{
    NameAndUrl as OriginalNameAndUrl, RecipeTime as OriginalRecipeTime,
    Servings as OriginalServings, StdKey as OriginalStdKey,
};
use cooklang::model::Item as OriginalItem;
use cooklang::quantity::{Quantity as OriginalQuantity, Value as OriginalValue};
use cooklang::Recipe as OriginalRecipe;

/// A parsed Cooklang recipe containing all recipe components
#[derive(uniffi::Object, Debug)]
pub struct CooklangRecipe {
    pub(crate) metadata: cooklang::metadata::Metadata,
    pub sections: Vec<Section>,
    pub ingredients: Vec<Ingredient>,
    pub cookware: Vec<Cookware>,
    pub timers: Vec<Timer>,
}

#[uniffi::export]
impl CooklangRecipe {
    /// Returns all sections in the recipe
    pub fn sections(&self) -> Vec<Section> {
        self.sections.clone()
    }

    /// Returns all ingredients used in the recipe
    pub fn ingredients(&self) -> Vec<Ingredient> {
        self.ingredients.clone()
    }

    /// Returns all cookware used in the recipe
    pub fn cookware(&self) -> Vec<Cookware> {
        self.cookware.clone()
    }

    /// Returns all timers in the recipe
    pub fn timers(&self) -> Vec<Timer> {
        self.timers.clone()
    }
}

pub type ComponentRef = u32;

/// Represents a distinct section of a recipe, optionally with a title
#[derive(uniffi::Record, Debug, Clone)]
pub struct Section {
    pub title: Option<String>,
    pub blocks: Vec<Block>,
    pub ingredient_refs: Vec<ComponentRef>,
    pub cookware_refs: Vec<ComponentRef>,
    pub timer_refs: Vec<ComponentRef>,
}

/// A block can either be a cooking step or a note
#[derive(uniffi::Enum, Debug, Clone)]
pub enum Block {
    StepBlock(Step),
    NoteBlock(BlockNote),
}

/// Types of components that can be referenced in a recipe
#[derive(uniffi::Enum, Debug, PartialEq)]
pub enum Component {
    IngredientComponent(Ingredient),
    CookwareComponent(Cookware),
    TimerComponent(Timer),
    TextComponent(String),
}

/// Represents a single cooking instruction step
#[derive(uniffi::Record, Debug, Clone)]
pub struct Step {
    pub items: Vec<Item>,
    pub ingredient_refs: Vec<ComponentRef>,
    pub cookware_refs: Vec<ComponentRef>,
    pub timer_refs: Vec<ComponentRef>,
}

/// A text note within the recipe
#[derive(uniffi::Record, Debug, Clone)]
pub struct BlockNote {
    pub text: String,
}

/// Represents an ingredient in the recipe
#[derive(uniffi::Record, Debug, PartialEq, Clone)]
pub struct Ingredient {
    pub name: String,
    pub amount: Option<Amount>,
    pub descriptor: Option<String>,
}

/// Represents a piece of cookware used in the recipe
#[derive(uniffi::Record, Debug, PartialEq, Clone)]
pub struct Cookware {
    pub name: String,
    pub amount: Option<Amount>,
}

/// Represents a timer in the recipe
#[derive(uniffi::Record, Debug, PartialEq, Clone)]
pub struct Timer {
    pub name: Option<String>,
    pub amount: Option<Amount>,
}

/// Elements that can appear in a recipe step
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Item {
    Text { value: String },
    IngredientRef { index: ComponentRef },
    CookwareRef { index: ComponentRef },
    TimerRef { index: ComponentRef },
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
            Value::Number { .. } => GroupedQuantityKey {
                name: units.to_string(),
                unit_type: QuantityType::Number,
            },
            Value::Range { .. } => GroupedQuantityKey {
                name: units.to_string(),
                unit_type: QuantityType::Range,
            },
            Value::Text { .. } => GroupedQuantityKey {
                name: units.to_string(),
                unit_type: QuantityType::Text,
            },
            Value::Empty => GroupedQuantityKey {
                name: units.to_string(),
                unit_type: QuantityType::Empty,
            },
        }
    } else {
        GroupedQuantityKey {
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

/// Type of quantity value in a grouped quantity
#[derive(uniffi::Enum, Debug, Clone, Hash, Eq, PartialEq)]
pub enum QuantityType {
    Number,
    Range, // how to combine ranges?
    Text,
    Empty,
}

/// Key for grouping quantities by unit and type
#[derive(uniffi::Record, Debug, Clone, Hash, Eq, PartialEq)]
pub struct GroupedQuantityKey {
    pub name: String,
    pub unit_type: QuantityType,
}

pub type GroupedQuantity = HashMap<GroupedQuantityKey, Value>;

/// Represents a quantity with optional units
#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct Amount {
    pub(crate) quantity: Value,
    pub(crate) units: Option<String>,
}

/// Types of values that can represent quantities
#[derive(uniffi::Enum, Debug, Clone, PartialEq)]
pub enum Value {
    Number { value: f64 },
    Range { start: f64, end: f64 },
    Text { value: String },
    Empty,
}

// Metadata types
/// Standard metadata keys from the Cooklang specification
#[derive(uniffi::Enum, Debug, Clone)]
pub enum StdKey {
    Title,
    Description,
    Tags,
    Author,
    Source,
    Course,
    Time,
    PrepTime,
    CookTime,
    Servings,
    Difficulty,
    Cuisine,
    Diet,
    Images,
    Locale,
}

impl From<&OriginalStdKey> for StdKey {
    fn from(key: &OriginalStdKey) -> Self {
        match key {
            OriginalStdKey::Title => StdKey::Title,
            OriginalStdKey::Description => StdKey::Description,
            OriginalStdKey::Tags => StdKey::Tags,
            OriginalStdKey::Author => StdKey::Author,
            OriginalStdKey::Source => StdKey::Source,
            OriginalStdKey::Course => StdKey::Course,
            OriginalStdKey::Time => StdKey::Time,
            OriginalStdKey::PrepTime => StdKey::PrepTime,
            OriginalStdKey::CookTime => StdKey::CookTime,
            OriginalStdKey::Servings => StdKey::Servings,
            OriginalStdKey::Difficulty => StdKey::Difficulty,
            OriginalStdKey::Cuisine => StdKey::Cuisine,
            OriginalStdKey::Diet => StdKey::Diet,
            OriginalStdKey::Images => StdKey::Images,
            OriginalStdKey::Locale => StdKey::Locale,
        }
    }
}

/// Recipe servings as either a number or text description
#[derive(uniffi::Enum, Debug, Clone)]
pub enum Servings {
    Number { value: u32 },
    Text { value: String },
}

impl From<OriginalServings> for Servings {
    fn from(servings: OriginalServings) -> Self {
        match servings {
            OriginalServings::Number(n) => Servings::Number { value: n },
            OriginalServings::Text(t) => Servings::Text { value: t },
        }
    }
}

/// A name with an optional URL (used for author/source)
#[derive(uniffi::Record, Debug, Clone)]
pub struct NameAndUrl {
    pub name: Option<String>,
    pub url: Option<String>,
}

impl From<OriginalNameAndUrl> for NameAndUrl {
    fn from(nu: OriginalNameAndUrl) -> Self {
        NameAndUrl {
            name: nu.name().map(|s| s.to_string()),
            url: nu.url().map(|s| s.to_string()),
        }
    }
}

/// Recipe time as either total minutes or separate prep/cook times
#[derive(uniffi::Enum, Debug, Clone)]
pub enum RecipeTime {
    Total {
        minutes: u32,
    },
    Composed {
        prep_time: Option<u32>,
        cook_time: Option<u32>,
    },
}

impl From<OriginalRecipeTime> for RecipeTime {
    fn from(time: OriginalRecipeTime) -> Self {
        match time {
            OriginalRecipeTime::Total(m) => RecipeTime::Total { minutes: m },
            OriginalRecipeTime::Composed {
                prep_time,
                cook_time,
            } => RecipeTime::Composed {
                prep_time,
                cook_time,
            },
        }
    }
}

trait Amountable {
    fn extract_amount(&self) -> Amount;
}

impl Amountable for OriginalQuantity {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_value(self.value());

        let units = self.unit().as_ref().map(|u| u.to_string());

        Amount { quantity, units }
    }
}

impl Amountable for OriginalValue {
    fn extract_amount(&self) -> Amount {
        let quantity = extract_value(self);

        Amount {
            quantity,
            units: None,
        }
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

pub fn expand_with_ingredients(
    ingredients: &[Ingredient],
    base: &mut IngredientList,
    addition: &Vec<ComponentRef>,
) {
    for index in addition {
        let ingredient = ingredients.get(*index as usize).unwrap().clone();
        let quantity = into_group_quantity(&ingredient.amount);
        add_to_ingredient_list(base, &ingredient.name, &quantity);
    }
}

// I(dubadub) haven't found a way to export these methods with mutable argument
fn add_to_ingredient_list(
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
            let quantity = left.entry(ingredient_name.to_string()).or_default();

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
    //  |- <litre,Number> => 1.2 litre
    //  |- <litre,Text> => half litre
    //  |- <,Text> => pinch
    //  |- <,Empty> => Some
    //
    //
    // TODO define rules on language spec level

    right.iter().for_each(|(key, value)| {
        left.entry(key.clone()) // isn't really necessary?
            .and_modify(|v| {
                match key.unit_type {
                    QuantityType::Number => {
                        let Value::Number { value: assignable } = value else {
                            panic!("Unexpected type")
                        };
                        let Value::Number { value: stored } = v else {
                            panic!("Unexpected type")
                        };

                        *stored += assignable
                    }
                    QuantityType::Range => {
                        let Value::Range { start, end } = value else {
                            panic!("Unexpected type")
                        };
                        let Value::Range { start: s, end: e } = v else {
                            panic!("Unexpected type")
                        };

                        // is it even correct?
                        *s += start;
                        *e += end;
                    }
                    QuantityType::Text => {
                        let Value::Text {
                            value: ref assignable,
                        } = value
                        else {
                            panic!("Unexpected type")
                        };
                        let Value::Text { value: stored } = v else {
                            panic!("Unexpected type")
                        };

                        *stored += assignable;
                    }
                    QuantityType::Empty => {} // nothing is required to do, Some + Some = Some
                }
            })
            .or_insert(value.clone());
    });
}

pub(crate) fn into_item(item: &OriginalItem) -> Item {
    match item {
        OriginalItem::Text { value } => Item::Text {
            value: value.to_string(),
        },
        OriginalItem::Ingredient { index } => Item::IngredientRef {
            index: *index as u32,
        },
        OriginalItem::Cookware { index } => Item::CookwareRef {
            index: *index as u32,
        },
        OriginalItem::Timer { index } => Item::TimerRef {
            index: *index as u32,
        },
        // returning an empty block of text as it's not supported by the spec
        OriginalItem::InlineQuantity { index: _ } => Item::Text {
            value: "".to_string(),
        },
    }
}

pub(crate) fn into_simple_recipe(recipe: &OriginalRecipe) -> CooklangRecipe {
    let metadata = recipe.metadata.clone();
    let ingredients: Vec<Ingredient> = recipe.ingredients.iter().map(|i| i.into()).collect();
    let cookware: Vec<Cookware> = recipe.cookware.iter().map(|i| i.into()).collect();
    let timers: Vec<Timer> = recipe.timers.iter().map(|i| i.into()).collect();
    let mut sections: Vec<Section> = Vec::new();

    // Process each section
    for section in &recipe.sections {
        let mut blocks: Vec<Block> = Vec::new();

        let mut ingredient_refs: Vec<u32> = Vec::new();
        let mut cookware_refs: Vec<u32> = Vec::new();
        let mut timer_refs: Vec<u32> = Vec::new();

        // Process content within each section
        for content in &section.content {
            match content {
                cooklang::Content::Step(step) => {
                    let mut step_ingredient_refs: Vec<u32> = Vec::new();
                    let mut step_cookware_refs: Vec<u32> = Vec::new();
                    let mut step_timer_refs: Vec<u32> = Vec::new();

                    let mut items: Vec<Item> = Vec::new();
                    // Process step items
                    for item in &step.items {
                        let item = into_item(item);

                        // Handle ingredients and cookware tracking
                        match &item {
                            Item::IngredientRef { index } => {
                                step_ingredient_refs.push(*index);
                            }
                            Item::CookwareRef { index } => {
                                step_cookware_refs.push(*index);
                            }
                            Item::TimerRef { index } => {
                                step_timer_refs.push(*index);
                            }
                            _ => (),
                        };
                        items.push(item);
                    }
                    blocks.push(Block::StepBlock(Step {
                        items,
                        ingredient_refs: step_ingredient_refs.clone(),
                        cookware_refs: step_cookware_refs.clone(),
                        timer_refs: step_timer_refs.clone(),
                    }));
                    ingredient_refs.extend(step_ingredient_refs);
                    cookware_refs.extend(step_cookware_refs);
                    timer_refs.extend(step_timer_refs);
                }

                cooklang::Content::Text(text) => {
                    blocks.push(Block::NoteBlock(BlockNote {
                        text: text.to_string(),
                    }));
                }
            }
        }

        sections.push(Section {
            title: section.name.clone(),
            blocks,
            ingredient_refs,
            cookware_refs,
            timer_refs,
        });
    }

    CooklangRecipe {
        metadata,
        sections,
        ingredients,
        cookware,
        timers,
    }
}

impl From<&cooklang::Ingredient> for Ingredient {
    fn from(ingredient: &cooklang::Ingredient) -> Self {
        Ingredient {
            name: ingredient.name.clone(),
            amount: ingredient.quantity.as_ref().map(|q| q.extract_amount()),
            descriptor: ingredient.note.clone(),
        }
    }
}

impl From<&cooklang::Cookware> for Cookware {
    fn from(cookware: &cooklang::Cookware) -> Self {
        Cookware {
            name: cookware.name.clone(),
            amount: cookware.quantity.as_ref().map(|q| q.extract_amount()),
        }
    }
}

impl From<&cooklang::Timer> for Timer {
    fn from(timer: &cooklang::Timer) -> Self {
        Timer {
            name: Some(timer.name.clone().unwrap_or_default()),
            amount: timer.quantity.as_ref().map(|q| q.extract_amount()),
        }
    }
}
