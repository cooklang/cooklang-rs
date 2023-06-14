//! Recipe representation

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    ast::Modifiers,
    convert::Converter,
    metadata::Metadata,
    quantity::{GroupedQuantity, Quantity, QuantityAddError, QuantityValue},
    scale::ScaleOutcome,
};

/// A complete recipe
///
/// A recipe can be [Self::scale] (only once) and [Self::convert] to other units.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Recipe<D = ()> {
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
    /// All the inline quantities
    pub inline_quantities: Vec<Quantity>,
    #[serde(skip_deserializing)]
    pub(crate) data: D,
}

impl<D> PartialEq for Recipe<D> {
    fn eq(&self, other: &Self) -> bool {
        // data ignored
        self.name == other.name
            && self.metadata == other.metadata
            && self.sections == other.sections
            && self.ingredients == other.ingredients
            && self.cookware == other.cookware
            && self.timers == other.timers
            && self.inline_quantities == other.inline_quantities
    }
}

/// A recipe after being scaled
///
/// Note that this doesn't implement [Recipe::scale]. A recipe can only be
/// scaled once.
pub type ScaledRecipe = Recipe<crate::scale::Scaled>;

impl Recipe {
    pub(crate) fn from_content(name: String, content: crate::analysis::RecipeContent) -> Self {
        Recipe {
            name,
            metadata: content.metadata,
            sections: content.sections,
            ingredients: content.ingredients,
            cookware: content.cookware,
            timers: content.timers,
            inline_quantities: content.inline_quantities,
            data: (),
        }
    }

    pub(crate) fn with_data<D>(self, data: D) -> Recipe<D> {
        Recipe {
            name: self.name,
            metadata: self.metadata,
            sections: self.sections,
            ingredients: self.ingredients,
            cookware: self.cookware,
            timers: self.timers,
            inline_quantities: self.inline_quantities,
            data,
        }
    }
}

impl ScaledRecipe {
    pub fn ingredient_list(&self, converter: &Converter) -> IngredientList {
        let mut list = Vec::new();
        let data = self.scaled_data();
        for (index, ingredient) in self
            .ingredients
            .iter()
            .enumerate()
            .filter(|(_, i)| !i.is_reference())
        {
            let mut grouped = GroupedQuantity::default();
            for q in ingredient.all_quantities(&self.ingredients) {
                grouped.add(q, converter);
            }
            let outcome: Option<ScaleOutcome> = data
                .as_ref()
                .map(|data| {
                    // Color in list depends on outcome of definition and all references
                    let mut outcome = &data.ingredients[index]; // temp value
                    let all_indices = std::iter::once(index)
                        .chain(ingredient.relation.referenced_from().iter().copied());
                    for index in all_indices {
                        match &data.ingredients[index] {
                            e @ ScaleOutcome::Error(_) => return e, // if err, return
                            e @ ScaleOutcome::Fixed => outcome = e, // if fixed, store
                            _ => {}
                        }
                    }
                    outcome
                })
                .cloned();
            list.push(IngredientListEntry {
                index,
                quantity: grouped,
                outcome,
            });
        }
        list
    }
}

pub type IngredientList = Vec<IngredientListEntry>;
#[derive(Debug, Clone, Serialize)]
pub struct IngredientListEntry {
    /// Index into the recipe ingredients (ingredient definition)
    pub index: usize,
    /// Total grouped quantity
    pub quantity: GroupedQuantity,
    /// Scale outcome, if scaled to a custom target
    pub outcome: Option<ScaleOutcome>,
}

/// A section holding steps
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Step {
    /// [Item]s inside
    pub items: Vec<Item>,
    /// Flag that indicates the step is a text step.
    ///
    /// A text step should not increase the step counter, and there are only
    /// text items inside.
    pub is_text: bool,
}

/// A step item
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Item {
    /// Just plain text
    Text(String),
    /// A [Component]
    Component(Component),
    /// An inline quantity.
    ///
    /// The number inside is an index into [Recipe::inline_quantities].
    InlineQuantity(usize),
}

/// A recipe ingredient
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Ingredient {
    /// Name
    ///
    /// This can have the form of a path if the ingredient references a recipe.
    pub name: String,
    /// Alias
    pub alias: Option<String>,
    /// Quantity
    pub quantity: Option<Quantity>,
    /// Note
    pub note: Option<String>,
    /// How the cookware is related to others
    pub relation: ComponentRelation,
    pub(crate) modifiers: Modifiers,
    pub(crate) defined_in_step: bool, // TODO maybe move this into analysis?, is not needed in the model
}

impl Ingredient {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> Cow<str> {
        let mut name = Cow::from(&self.name);
        if self.modifiers.contains(Modifiers::RECIPE) {
            if let Some(recipe_name) = std::path::Path::new(&self.name)
                .file_stem()
                .and_then(|s| s.to_str())
            {
                name = recipe_name.into();
            }
        }
        self.alias.as_ref().map(Cow::from).unwrap_or(name)
    }

    /// Access the ingredient modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn is_hidden(&self) -> bool {
        self.modifiers.contains(Modifiers::HIDDEN)
    }

    pub fn is_optional(&self) -> bool {
        self.modifiers.contains(Modifiers::OPT)
    }

    pub fn is_recipe(&self) -> bool {
        self.modifiers.contains(Modifiers::RECIPE)
    }

    pub fn is_reference(&self) -> bool {
        self.modifiers.contains(Modifiers::REF)
    }

    /// Calculates the total quantity adding all the quantities from the
    /// references.
    pub fn total_quantity<'a>(
        &'a self,
        all_ingredients: &'a [Self],
        converter: &Converter,
    ) -> Result<Option<Quantity>, QuantityAddError> {
        let mut quantities = self.all_quantities(all_ingredients);

        let Some(mut total) = quantities.next().cloned() else { return Ok(None); };
        for q in quantities {
            total = total.try_add(q, converter)?;
        }
        let _ = total.fit(converter);

        Ok(Some(total))
    }

    /// Gets an iterator over all quantities of this ingredient and its references.
    pub fn all_quantities<'a>(
        &'a self,
        all_ingredients: &'a [Self],
    ) -> impl Iterator<Item = &Quantity> {
        std::iter::once(self.quantity.as_ref())
            .chain(
                self.relation
                    .referenced_from()
                    .iter()
                    .copied()
                    .map(|i| all_ingredients[i].quantity.as_ref()),
            )
            .flatten()
    }
}

/// A recipe cookware item
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Cookware {
    /// Name
    pub name: String,
    /// Alias
    pub alias: Option<String>,
    /// Amount needed
    ///
    /// Note that this is a value, not a quantity, so it doesn't have units.
    pub quantity: Option<QuantityValue>,
    /// Note
    pub note: Option<String>,
    /// How the cookware is related to others
    pub relation: ComponentRelation,
    pub(crate) modifiers: Modifiers,
}

impl Cookware {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> &str {
        self.alias.as_ref().unwrap_or(&self.name)
    }

    /// Access the ingredient modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    pub fn is_hidden(&self) -> bool {
        self.modifiers.contains(Modifiers::HIDDEN)
    }

    pub fn is_optional(&self) -> bool {
        self.modifiers.contains(Modifiers::OPT)
    }

    pub fn is_reference(&self) -> bool {
        self.modifiers.contains(Modifiers::REF)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ComponentRelation {
    Definition { referenced_from: Vec<usize> },
    Reference { references_to: usize },
}

impl ComponentRelation {
    /// Gets a list of the components referencing this one.
    ///
    /// Returns a list of indices to the corresponding vec in [Recipe].
    pub fn referenced_from(&self) -> &[usize] {
        match self {
            ComponentRelation::Definition { referenced_from } => referenced_from,
            ComponentRelation::Reference { .. } => &[],
        }
    }

    /// Get the index the relations references to
    pub fn references_to(&self) -> Option<usize> {
        match self {
            ComponentRelation::Definition { .. } => None,
            ComponentRelation::Reference { references_to } => Some(*references_to),
        }
    }
}

/// A recipe timer
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Timer {
    /// Name
    pub name: Option<String>,
    /// Time quantity
    ///
    /// If created from parsing and the advanced units extension is enabled,
    /// this is guaranteed to have a time unit.
    pub quantity: Quantity,
}

/// A component reference
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Component {
    /// What kind of component is
    pub kind: ComponentKind,
    /// The index in the corresponding [Vec] in the [Recipe] struct.
    pub index: usize,
}

/// Component kind used in [Component]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    Ingredient,
    Cookware,
    Timer,
}
