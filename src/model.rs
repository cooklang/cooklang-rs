//! Recipe representation

/*

   To make this model compatible with UniFFI
     - Do not use tuple-like enums
     - Enum variant names can't conflict with types or other enums

*/

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{
    ast::Modifiers,
    convert::Converter,
    metadata::Metadata,
    quantity::{Quantity, QuantityAddError, QuantityValue, ScalableValue, ScaledQuantity},
    GroupedQuantity, Value,
};

/// A complete recipe
///
/// The recipe returned from parsing is a [`ScalableRecipe`].
///
/// The difference between [`ScalableRecipe`] and [`ScaledRecipe`] is in the
/// values of the quantities of ingredients, cookware and timers. The parser
/// returns [`ScalableValue`]s and after scaling, these are converted to regular
/// [`Value`]s.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Recipe<D, V: QuantityValue> {
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
    pub ingredients: Vec<Ingredient<V>>,
    /// All the cookware
    pub cookware: Vec<Cookware<V>>,
    /// All the timers
    pub timers: Vec<Timer<V>>,
    /// All the inline quantities
    pub inline_quantities: Vec<ScaledQuantity>,
    pub(crate) data: D,
}

/// A recipe before being scaled
///
/// Note that this doesn't implement [`Recipe::convert`]. Only scaled recipes
/// can be converted.
pub type ScalableRecipe = Recipe<(), ScalableValue>;

/// A recipe after being scaled
///
/// Note that this doesn't implement [`Recipe::scale`]. A recipe can only be
/// scaled once.
pub type ScaledRecipe = Recipe<crate::scale::Scaled, Value>;

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

    /// Check if the section is empty
    ///
    /// A section is empty when it has no name and no steps.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty()
    }
}

/// A step holding step [`Item`]s
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[non_exhaustive]
pub struct Step {
    /// [`Item`]s inside
    pub items: Vec<Item>,

    /// Step number
    ///
    /// The step numbers start at 1 in each section and increase with every non
    /// text step. Text steps do not have a number. If this is not a text step,
    /// it will always be [`Some`].
    pub number: Option<u32>,
}

impl Step {
    /// Flag that indicates the step is a text step.
    ///
    /// A text step does not increase the step counter, so, if this method
    /// returns `true`, the step does not have a number. There are only
    /// [`Item::Text`] in [`items`](Self::items).
    pub fn is_text(&self) -> bool {
        self.number.is_none()
    }
}

/// A step item
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    /// Just plain text
    Text { value: String },
    /// A [`Component`]
    #[serde(rename = "component")] // UniFFI
    ItemComponent { value: Component },
    /// An inline quantity.
    ///
    /// The number inside is an index into [`Recipe::inline_quantities`].
    InlineQuantity { value: usize },
}

/// A recipe ingredient
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Ingredient<V: QuantityValue = Value> {
    /// Name
    ///
    /// This can have the form of a path if the ingredient references a recipe.
    pub name: String,
    /// Alias
    pub alias: Option<String>,
    /// Quantity
    pub quantity: Option<Quantity<V>>,
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

impl<V: QuantityValue> Ingredient<V> {
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
}

impl Ingredient<Value> {
    /// Calculates the total quantity adding all the quantities from the
    /// references.
    pub fn total_quantity<'a>(
        &'a self,
        all_ingredients: &'a [Self],
        converter: &Converter,
    ) -> Result<Option<ScaledQuantity>, QuantityAddError> {
        let mut quantities = self.all_quantities(all_ingredients);

        let Some(mut total) = quantities.next().cloned() else { return Ok(None); };
        for q in quantities {
            total = total.try_add(q, converter)?;
        }
        let _ = total.fit(converter);

        Ok(Some(total))
    }

    /// Groups all quantities from itself and it's references (if any).
    /// ```
    /// # use cooklang::{CooklangParser, Extensions, Converter, TotalQuantity, Value, Quantity};
    /// let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    /// let recipe = parser.parse("@flour{1000%g} @&flour{100%g}", "name")
    ///                 .into_output()
    ///                 .unwrap()
    ///                 .default_scale();
    ///
    /// let flour = &recipe.ingredients[0];
    /// assert_eq!(flour.name, "flour");
    ///
    /// let grouped_flour = recipe.ingredients[0].group_quantities(
    ///                         &recipe.ingredients,
    ///                         parser.converter()
    ///                     );
    ///
    /// assert_eq!(
    ///     grouped_flour.total(),
    ///     TotalQuantity::Single(
    ///         Quantity::new(
    ///             Value::from(1.1),
    ///             Some("kg".to_string()) // Unit fit to kilograms
    ///         )
    ///     )
    /// );
    /// ```
    pub fn group_quantities(
        &self,
        all_ingredients: &[Self],
        converter: &Converter,
    ) -> GroupedQuantity {
        let mut grouped = GroupedQuantity::default();
        for q in self.all_quantities(all_ingredients) {
            grouped.add(q, converter);
        }
        let _ = grouped.fit(converter);
        grouped
    }

    /// Gets an iterator over all quantities of this ingredient and its references.
    pub fn all_quantities<'a>(
        &'a self,
        all_ingredients: &'a [Self],
    ) -> impl Iterator<Item = &ScaledQuantity> {
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
pub struct Cookware<V: QuantityValue = Value> {
    /// Name
    pub name: String,
    /// Alias
    pub alias: Option<String>,
    /// Amount needed
    ///
    /// Note that this is a value, not a quantity, so it doesn't have units.
    pub quantity: Option<V>,
    /// Note
    pub note: Option<String>,
    /// How the cookware is related to others
    pub relation: ComponentRelation,
    pub(crate) modifiers: Modifiers,
}

impl<V: QuantityValue> Cookware<V> {
    /// Gets the name the cookware item should be displayed with
    pub fn display_name(&self) -> &str {
        self.alias.as_ref().unwrap_or(&self.name)
    }

    /// Access the cookware modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }
}

/// Relation between components
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ComponentRelation {
    /// The component is a definition
    Definition {
        /// List of indices of other components of the same kind referencing this
        /// one
        referenced_from: Vec<usize>,
    },
    /// The component is a reference
    Reference {
        /// Index of the definition component
        references_to: usize,
    },
}

impl ComponentRelation {
    /// Gets a list of the components referencing this one.
    ///
    /// Returns a list of indices to the corresponding vec in [`Recipe`].
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

    /// Check if the relation is a reference
    pub fn is_reference(&self) -> bool {
        matches!(self, ComponentRelation::Reference { .. })
    }

    /// Check if the relation is a definition
    pub fn is_definition(&self) -> bool {
        matches!(self, ComponentRelation::Definition { .. })
    }
}

/// Same as [`ComponentRelation`] but with the ability to reference steps and
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
pub enum IngredientReferenceTarget {
    /// Ingredient definition
    #[serde(rename = "ingredient")]
    IngredientTarget,
    /// Step in the current section
    #[serde(rename = "step")]
    StepTarget,
    /// Section in the current recipe
    #[serde(rename = "section")]
    SectionTarget,
}

impl IngredientRelation {
    pub(crate) fn definition(referenced_from: Vec<usize>) -> Self {
        Self {
            relation: ComponentRelation::Definition { referenced_from },
            reference_target: None,
        }
    }

    pub(crate) fn reference(
        references_to: usize,
        reference_target: IngredientReferenceTarget,
    ) -> Self {
        Self {
            relation: ComponentRelation::Reference { references_to },
            reference_target: Some(reference_target),
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
    /// extension is disabled, the target will always be
    /// [IngredientReferenceTarget::IngredientTarget].
    pub fn references_to(&self) -> Option<(usize, IngredientReferenceTarget)> {
        self.relation
            .references_to()
            .map(|index| (index, self.reference_target.unwrap()))
    }

    /// Checks if the relation is a regular reference to an ingredient
    pub fn is_regular_reference(&self) -> bool {
        use IngredientReferenceTarget::*;
        self.references_to()
            .map(|(_, target)| target == IngredientTarget)
            .unwrap_or(false)
    }

    /// Checks if the relation is an intermediate reference to a step or section
    pub fn is_intermediate_reference(&self) -> bool {
        use IngredientReferenceTarget::*;
        self.references_to()
            .map(|(_, target)| matches!(target, StepTarget | SectionTarget))
            .unwrap_or(false)
    }

    /// Check if the relation is a definition
    pub fn is_definition(&self) -> bool {
        self.relation.is_definition()
    }
}

/// A recipe timer
///
/// If created from parsing, at least one of the fields is guaranteed to be
/// [`Some`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Timer<V: QuantityValue = Value> {
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
    /// extension is enabled, this is guaranteed to be [`Some`].
    pub quantity: Option<Quantity<V>>,
}

/// A component reference
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Component {
    /// What kind of component is
    pub kind: ComponentKind,
    /// The index in the corresponding vec in the [`Recipe`] struct.
    pub index: usize,
}

/// Component kind used in [`Component`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ComponentKind {
    #[serde(rename = "ingredient")]
    IngredientKind,
    #[serde(rename = "cookware")]
    CookwareKind,
    #[serde(rename = "timer")]
    TimerKind,
}
