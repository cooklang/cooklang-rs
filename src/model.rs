//! Recipe representation

/*

   To make this model compatible with UniFFI
     - Do not use tuple-like enums
     - Enum variant names can't conflict with types or other enums

*/

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    metadata::Metadata,
    quantity::{Quantity, QuantityValue},
};

/// A complete recipe
///
/// A recipe can be [Self::scale] (only once) and only after that [Self::convert]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct Recipe {
    /// Recipe name
    pub name: String,
    /// Metadata
    pub metadata: Metadata,
    /// Each of the sections
    ///
    /// If no sections declared, a section without name
    /// is the default.
    pub sections: Vec<Section>,
    /// All the ingredients
    pub ingredients: Vec<Ingredient>,
    /// All the cookware
    pub cookware: Vec<Cookware>,
    /// All the timers
    pub timers: Vec<Timer>,
}

/// A section holding steps
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Section {
    /// Name of the section
    pub name: Option<String>,
    /// Steps inside
    pub steps: Vec<Step>,
}

impl Section {
    pub(crate) fn new(name: Option<String>) -> Section {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }
}

/// A step holding step [Item]s
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct Step {
    /// [Item]s inside
    pub items: Vec<Item>,
}


/// A step item
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    /// Just plain text
    Text { value: String },
    /// A [Component]
    #[serde(rename = "component")]
    ItemComponent { value: Component },
}

/// A recipe ingredient
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Ingredient {
    /// Name
    ///
    /// This can have the form of a path if the ingredient references a recipe.
    pub name: String,
    /// Quantity
    pub quantity: Option<Quantity>,
    /// Note
    pub note: Option<String>,
}

impl Ingredient {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> Cow<str> {
        Cow::from(&self.name)
    }

}

/// A recipe cookware item
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Cookware {
    /// Name
    pub name: String,
    /// Amount needed
    ///
    /// Note that this is a value, not a quantity, so it doesn't have units.
    pub quantity: Option<QuantityValue>,
    /// Note
    pub note: Option<String>,
}

impl Cookware {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> &str {
        &self.name
    }
}

/// A recipe timer
///
/// If created from parsing, at least one of the fields is guaranteed to be [Some].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Timer {
    /// Name
    pub name: Option<String>,
    /// Time quantity
    ///
    /// If created from parsing the following applies:
    ///
    /// - If the [`ADVANCED_UNITS`](crate::Extensions::ADVANCED_UNITS) extension
    /// is enabled, this is guaranteed to have a time unit.
    ///
    /// - If the [`TIMER_REQUIRES_TIME`](crate::Extensions::TIMER_REQUIRES_TIME)
    /// extension is enabled, this is guaranteed to be [Some].
    pub quantity: Option<Quantity>,
}

/// A component reference
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Component {
    /// What kind of component is
    pub kind: ComponentKind,
}

/// Component kind used in [Component]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ComponentKind {
    #[serde(rename = "ingredient")]
    IngredientKind,
    #[serde(rename = "cookware")]
    CookwareKind,
    #[serde(rename = "timer")]
    TimerKind,
}
