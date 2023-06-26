//! Recipe representation

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    ast::Modifiers,
    convert::Converter,
    metadata::Metadata,
    quantity::{Quantity, QuantityAddError, QuantityValue},
};

/// A complete recipe
///
/// A recipe can be [Self::scale] (only once) and only after that [Self::convert]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

/// A recipe after being scaled
///
/// Note that this doesn't implement [Recipe::scale]. A recipe can only be
/// scaled once.
pub type ScaledRecipe = Recipe<crate::scale::Scaled>;

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
    Text { value: String },
    /// A [Component]
    ItemComponent { value: Component },
    /// An inline quantity.
    ///
    /// The number inside is an index into [Recipe::inline_quantities].
    InlineQuantity { value: usize },
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
    pub relation: IngredientRelation,
    pub(crate) modifiers: Modifiers,
    // ? maybe move this into analysis?, is not needed in the model
    // ? however I will keep it here for now. Because of alignment it does
    // ? not increase the size of the struct. Maybe in the future it can even be
    // ? be public.
    pub(crate) defined_in_step: bool,
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

/// Same as [ComponentRelation] but with the ability to reference steps and
/// sections apart from other ingredients.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct IngredientRelation {
    #[serde(flatten)]
    relation: ComponentRelation,
    reference_target: Option<IngredientReferenceTarget>,
}

/// Target an ingredient reference references to
///
/// This is obtained from [IngredientRelation::references_to]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum IngredientReferenceTarget {
    /// Ingredient definition
    Ingredient,
    /// Step in the current section
    Step,
    /// Section in the current recipe
    Section,
}

impl IngredientRelation {
    /// Creates a new ingredient relation
    ///
    /// # Panics
    /// If `relation` is [ComponentRelation::Reference] and `reference_target`
    /// is not [Some].
    pub(crate) fn new(
        relation: ComponentRelation,
        reference_target: Option<IngredientReferenceTarget>,
    ) -> Self {
        assert!(
            matches!(relation, ComponentRelation::Definition { .. }) || reference_target.is_some(),
            "ingredient relation reference without reference target defined. this is a bug."
        );
        Self {
            relation,
            reference_target,
        }
    }

    /// Gets a list of the components referencing this one.
    ///
    /// Returns a list of indices to the corresponding vec in [Recipe].
    pub fn referenced_from(&self) -> &[usize] {
        self.relation.referenced_from()
    }

    pub(crate) fn referenced_from_mut(&mut self) -> Option<&mut Vec<usize>> {
        match &mut self.relation {
            ComponentRelation::Definition { referenced_from } => Some(referenced_from),
            ComponentRelation::Reference { .. } => None,
        }
    }

    /// Get the index the relation refrences to and the target
    ///
    /// If the [INTERMEDIATE_INGREDIENTS](crate::Extensions::INTERMEDIATE_INGREDIENTS)
    /// extension is disabled, the target will always be [IngredientReferenceTarget::Ingredient].
    pub fn references_to(&self) -> Option<(usize, IngredientReferenceTarget)> {
        self.relation
            .references_to()
            .map(|index| (index, self.reference_target.unwrap()))
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
    /// The index in the corresponding [Vec] in the [Recipe] struct.
    pub index: usize,
}

/// Component kind used in [Component]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum ComponentKind {
    IngredientKind,
    CookwareKind,
    TimerKind,
}
