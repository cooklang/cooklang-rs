//! Recipe representation

use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::{
    convert::Converter, metadata::Metadata, parser::Modifiers, quantity::Quantity, GroupedQuantity,
};

/// A complete recipe
///
/// The recipes do not have a name. You give it externally or maybe use
/// some metadata key.
///
/// The recipe returned from parsing is a [`ScalableRecipe`].
///
/// The difference between [`ScalableRecipe`] and [`ScaledRecipe`] is in the
/// values of the quantities of ingredients, cookware and timers. The parser
/// returns [`ScalableValue`]s and after scaling, these are converted to regular
/// [`Value`]s.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct Recipe {
    /// Metadata as read from preamble
    #[cfg_attr(feature = "ts", serde(rename = "raw_metadata"))]
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
}

/// A section holding steps
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct Section {
    /// Name of the section
    pub name: Option<String>,
    /// Content inside
    pub content: Vec<Content>,
}

impl Section {
    pub(crate) fn new(name: Option<String>) -> Section {
        Self {
            name,
            content: Vec::new(),
        }
    }

    /// Check if the section is empty
    ///
    /// A section is empty when it has no name and no content.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.content.is_empty()
    }
}

/// Each type of content inside a section
///
/// Serialized as `{"type": "step", "value": <step>}` or, for text,
/// `{"type": "text", "value": <plain text>, "items": [<runs>]}`. The plain
/// `value` is the shape from before notes had formatting, so consumers that
/// read only it keep working. `items` is optional when deserializing.
#[derive(Debug, PartialEq, Clone)]
pub enum Content {
    /// A step
    Step(Step),
    /// A paragraph of just text, no instructions
    Text(Note),
}

impl Serialize for Content {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(tag = "type", rename_all = "camelCase")]
        enum Repr<'a> {
            Step { value: &'a Step },
            Text { value: String, items: &'a [TextRun] },
        }
        match self {
            Content::Step(step) => Repr::Step { value: step },
            Content::Text(note) => Repr::Text {
                value: note.text(),
                items: &note.items,
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "type", rename_all = "camelCase")]
        enum Repr {
            Step {
                value: Step,
            },
            Text {
                value: String,
                #[serde(default)]
                items: Option<Vec<TextRun>>,
            },
        }
        Ok(match Repr::deserialize(deserializer)? {
            Repr::Step { value } => Content::Step(value),
            Repr::Text {
                items: Some(items), ..
            } => Content::Text(Note { items }),
            Repr::Text { value, items: None } => Content::Text(Note::plain(value)),
        })
    }
}

/// TypeScript type of [`Content`], which has its own serialization
#[cfg(feature = "ts")]
mod content_ts {
    use super::{Step, TextRun};
    use tsify::Tsify;

    /// Each type of content inside a section
    #[derive(Tsify)]
    #[serde(tag = "type", rename_all = "camelCase")]
    #[allow(dead_code)]
    enum Content {
        Step {
            value: Step,
        },
        Text {
            /// The plain text
            value: String,
            /// Formatted runs of the text
            items: Vec<TextRun>,
        },
    }

    #[test]
    fn declaration_is_named_content() {
        assert!(<Content as Tsify>::DECL.contains("export type Content = "));
    }
}

impl Content {
    /// Checks if the content is a regular step
    pub fn is_step(&self) -> bool {
        matches!(self, Self::Step(_))
    }

    /// Checks if the content is a text paragraph
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Get's the inner step
    ///
    /// # Panics
    /// If the content is [`Content::Text`]
    pub fn unwrap_step(&self) -> &Step {
        match self {
            Content::Step(s) => s,
            Content::Text(_) => panic!("content is text"),
        }
    }

    /// Get's the inner text
    ///
    /// # Panics
    /// If the content is [`Content::Step`]
    pub fn unwrap_text(&self) -> &Note {
        match self {
            Content::Step(_) => panic!("content is step"),
            Content::Text(t) => t,
        }
    }
}

/// A paragraph of text that is not a step: a note (`> ...`)
///
/// A note is a list of [`TextRun`]s, each a stretch of text with the same
/// formatting. Notes are the only place where inline formatting (emphasis,
/// strong emphasis and links) is read, see spec proposal 0022.
///
/// The plain text of the note, without any formatting, is [`Note::text`].
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
pub struct Note {
    /// Runs of text, in order
    ///
    /// Adjacent runs never have the same formatting and no run is empty, so
    /// a note without formatting has a single run (or none if it is empty).
    pub items: Vec<TextRun>,
}

impl Note {
    /// A note of plain text, without formatting
    pub fn plain(text: impl Into<String>) -> Self {
        let value = text.into();
        let items = if value.is_empty() {
            vec![]
        } else {
            vec![TextRun::plain(value)]
        };
        Self { items }
    }

    /// The plain text of the note
    ///
    /// This is the text of every run joined together, for consumers that do
    /// not show formatting.
    pub fn text(&self) -> String {
        self.items.iter().map(|r| r.value.as_str()).collect()
    }

    /// Checks if any run has formatting or a link
    pub fn is_formatted(&self) -> bool {
        self.items.iter().any(|r| !r.is_plain())
    }
}

impl std::fmt::Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for run in &self.items {
            f.write_str(&run.value)?;
        }
        Ok(())
    }
}

impl From<String> for Note {
    fn from(value: String) -> Self {
        Self::plain(value)
    }
}

impl From<&str> for Note {
    fn from(value: &str) -> Self {
        Self::plain(value)
    }
}

/// A stretch of text in a [`Note`] with the same formatting
///
/// Nested formatting is flattened: the text of `**a *b***` is two runs, both
/// `strong`, the second also `emphasis`.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type", rename = "text")]
pub struct TextRun {
    /// The text, with delimiters and escapes removed
    pub value: String,
    /// Inside strong emphasis (`**text**`, `__text__`)
    #[serde(default, skip_serializing_if = "is_false")]
    pub strong: bool,
    /// Inside emphasis (`*text*`, `_text_`)
    #[serde(default, skip_serializing_if = "is_false")]
    pub emphasis: bool,
    /// Destination of the link (`[text](destination)`) the run is in
    ///
    /// This is returned as written, without resolving or validating it. A
    /// destination is untrusted input: see the proposal for which ones an
    /// application may make active.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl TextRun {
    /// A run without formatting
    pub fn plain(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            ..Default::default()
        }
    }

    /// Checks if the run has no formatting and no link
    pub fn is_plain(&self) -> bool {
        !self.strong && !self.emphasis && self.link.is_none()
    }

    pub(crate) fn same_format(&self, other: &Self) -> bool {
        self.strong == other.strong && self.emphasis == other.emphasis && self.link == other.link
    }
}

/// A step holding step [`Item`]s
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[non_exhaustive]
pub struct Step {
    /// [`Item`]s inside
    pub items: Vec<Item>,

    /// Step number
    ///
    /// The step numbers start at 1 in each section and increase with non
    /// text step.
    pub number: u32,
}

/// A step item
///
/// Except for [`Item::Text`], the value is the index where the item is located
/// in it's corresponding [`Vec`] in the [`Recipe`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Item {
    /// Just plain text
    Text {
        value: String,
    },
    Ingredient {
        index: usize,
    },
    Cookware {
        index: usize,
    },
    Timer {
        index: usize,
    },
    InlineQuantity {
        index: usize,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct RecipeReference {
    pub name: String,
    pub components: Vec<String>,
}

impl RecipeReference {
    pub fn path(&self, separator: &str) -> String {
        self.components.join(separator) + separator + &self.name
    }
}

/// A recipe ingredient
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[cfg_attr(feature = "ts", tsify(into_wasm_abi, from_wasm_abi))]
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
    /// Recipe reference
    pub reference: Option<RecipeReference>,
    /// How the cookware is related to others
    pub relation: IngredientRelation,
    /// Serialized as the flag names, e.g. `"OPT"` or `"HIDDEN | OPT"`
    #[cfg_attr(feature = "ts", serde(default), tsify(type = "string"))]
    pub(crate) modifiers: Modifiers,
}

impl Ingredient {
    /// Gets the name the ingredient should be displayed with
    pub fn display_name(&self) -> Cow<'_, str> {
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

    /// Groups all quantities from itself and it's references (if any).
    /// ```
    /// # #[cfg(feature = "bundled_units")] {
    /// # use cooklang::{CooklangParser, Extensions, Converter, Value, Quantity};
    /// let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    /// let recipe = parser
    ///                 .parse("@flour{1000%g} @&flour{200%g} @&flour{1%bag}")
    ///                 .into_output()
    ///                 .unwrap();
    ///
    /// let flour = &recipe.ingredients[0];
    /// assert_eq!(flour.name, "flour");
    ///
    /// let grouped_flour = flour.group_quantities(
    ///                         &recipe.ingredients,
    ///                         parser.converter()
    ///                     );
    /// assert_eq!(grouped_flour.to_string(), "1.2 kg, 1 bag");
    /// assert_eq!(
    ///     grouped_flour.into_vec(),
    ///     vec![
    ///         Quantity::new(
    ///             Value::from(1.2),
    ///             Some("kg".to_string())  // Unit fit to kilograms
    ///         ),
    ///         Quantity::new(
    ///             Value::from(1.0),
    ///             Some("bag".to_string()) // Can't add this unit to kg
    ///         ),
    ///     ]
    /// );
    /// # }
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
    ) -> impl Iterator<Item = &'a Quantity> {
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
#[cfg_attr(feature = "ts", derive(Tsify))]
#[cfg_attr(feature = "ts", tsify(into_wasm_abi, from_wasm_abi))]
pub struct Cookware {
    /// Name
    pub name: String,
    /// Alias
    pub alias: Option<String>,
    /// Amount needed
    ///
    /// Note that this is a value, not a quantity, so it doesn't have units.
    pub quantity: Option<Quantity>,
    /// Note
    pub note: Option<String>,
    /// How the cookware is related to others
    pub relation: ComponentRelation,
    /// Serialized as the flag names, e.g. `"OPT"` or `"HIDDEN | OPT"`
    #[cfg_attr(feature = "ts", serde(default), tsify(type = "string"))]
    pub(crate) modifiers: Modifiers,
}

impl Cookware {
    /// Gets the name the cookware item should be displayed with
    pub fn display_name(&self) -> &str {
        self.alias.as_ref().unwrap_or(&self.name)
    }

    /// Access the cookware modifiers
    pub fn modifiers(&self) -> Modifiers {
        self.modifiers
    }

    /// Same as [`Ingredient::group_quantities`] but for [`Cookware`]
    pub fn group_quantities(
        &self,
        all_cookware: &[Self],
        converter: &Converter,
    ) -> GroupedQuantity {
        let mut g = GroupedQuantity::empty();
        for q in self.all_quantities(all_cookware) {
            g.add(q, converter);
        }
        let _ = g.fit(converter);
        g
    }

    /// Gets an iterator over all quantities of this ingredient and its references.
    pub fn all_quantities<'a>(
        &'a self,
        all_cookware: &'a [Self],
    ) -> impl Iterator<Item = &'a Quantity> {
        std::iter::once(self.quantity.as_ref())
            .chain(
                self.relation
                    .referenced_from()
                    .iter()
                    .copied()
                    .map(|i| all_cookware[i].quantity.as_ref()),
            )
            .flatten()
    }
}

/// Relation between components
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ComponentRelation {
    /// The component is a definition
    Definition {
        /// List of indices of other components of the same kind referencing this
        /// one
        referenced_from: Vec<usize>,
        /// True if the definition was in a step
        ///
        /// This is only false for components defined in components mode.
        defined_in_step: bool,
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
            ComponentRelation::Definition {
                referenced_from, ..
            } => referenced_from,
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

    /// Checks if the definition was in a step
    ///
    /// Returns None for references
    pub fn is_defined_in_step(&self) -> Option<bool> {
        match self {
            ComponentRelation::Definition {
                defined_in_step, ..
            } => Some(*defined_in_step),
            ComponentRelation::Reference { .. } => None,
        }
    }
}

/// Same as [`ComponentRelation`] but with the ability to reference steps and
/// sections apart from other ingredients.
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct IngredientRelation {
    relation: ComponentRelation,
    reference_target: Option<IngredientReferenceTarget>,
}

/// Target an ingredient reference references to
///
/// This is obtained from [`IngredientRelation::references_to`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy)]
#[cfg_attr(feature = "ts", derive(Tsify))]
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
    pub(crate) fn definition(referenced_from: Vec<usize>, defined_in_step: bool) -> Self {
        Self {
            relation: ComponentRelation::Definition {
                referenced_from,
                defined_in_step,
            },
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
            ComponentRelation::Definition {
                referenced_from, ..
            } => Some(referenced_from),
            ComponentRelation::Reference { .. } => None,
        }
    }

    /// Get the index the relation refrences to and the target
    ///
    /// The first element of the tuple is an index into:
    ///
    /// | Target | Where |
    /// |--------|-------|
    /// | [`Ingredient`] | [`Recipe::ingredients`] |
    /// | [`Step`] | [`Section::content`] in the same section this ingredient is. It's guaranteed that the content is a step. |
    /// | [`Section`] | [`Recipe::sections`] |
    ///
    /// [`Ingredient`]: IngredientReferenceTarget::Ingredient
    /// [`Step`]: IngredientReferenceTarget::Step
    /// [`Section`]: IngredientReferenceTarget::Section
    ///
    /// If the [`INTERMEDIATE_PREPARATIONS`](crate::Extensions::INTERMEDIATE_PREPARATIONS)
    /// extension is disabled, the target will always be
    /// [`IngredientReferenceTarget::Ingredient`].
    pub fn references_to(&self) -> Option<(usize, IngredientReferenceTarget)> {
        self.relation
            .references_to()
            .map(|index| (index, self.reference_target.unwrap()))
    }

    /// Checks if the relation is a regular reference to an ingredient
    pub fn is_regular_reference(&self) -> bool {
        use IngredientReferenceTarget as Target;
        self.references_to()
            .is_some_and(|(_, target)| target == Target::Ingredient)
    }

    /// Checks if the relation is an intermediate reference to a step or section
    pub fn is_intermediate_reference(&self) -> bool {
        use IngredientReferenceTarget as Target;
        self.references_to()
            .is_some_and(|(_, target)| matches!(target, Target::Step | Target::Section))
    }

    /// Check if the relation is a definition
    pub fn is_definition(&self) -> bool {
        self.relation.is_definition()
    }

    /// Checks if the definition was in a step
    ///
    /// Returns None for references
    pub fn is_defined_in_step(&self) -> Option<bool> {
        self.relation.is_defined_in_step()
    }
}

/// A recipe timer
///
/// If created from parsing, at least one of the fields is guaranteed to be
/// [`Some`].
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct Timer {
    /// Name
    pub name: Option<String>,
    /// Time quantity
    ///
    /// If created from parsing the following applies:
    ///
    /// - If the [`ADVANCED_UNITS`](crate::Extensions::ADVANCED_UNITS) extension
    ///   is enabled, this is guaranteed to have a time unit and a non text value.
    ///
    /// - If the [`TIMER_REQUIRES_TIME`](crate::Extensions::TIMER_REQUIRES_TIME)
    ///   extension is enabled, this is guaranteed to be [`Some`].
    pub quantity: Option<Quantity>,
}
