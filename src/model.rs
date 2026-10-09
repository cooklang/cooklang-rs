//! Recipe representation

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecipeReferenceParseError {
    #[error("reference is missing name {0}")]
    MissingName(String),
}

#[cfg(feature = "ts")]
use tsify::Tsify;

use crate::{
    convert::Converter, metadata::Metadata, parser::Modifiers, quantity::Quantity,
    semantic_eq::SemanticEq, GroupedQuantity,
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
    /// The path of the recipe file this was parsed from
    pub path: Option<RecipeReference>,
}

/// Compares two recipes using unit-aware quantity comparison.
///
/// Unlike [`PartialEq`], quantities are compared with their [`SemanticEq`]
/// impl, so `5 dl` equals `0.5 l` and `5 min` equals `300 s`.
impl SemanticEq for Recipe {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        // Specifically does not compare recipe.path as it is
        // runtime dependent and does not have an "equivalent".
        self.metadata.equals(&other.metadata, converter)
            && self.sections == other.sections
            && self.ingredients.equals(&other.ingredients, converter)
            && self.cookware.equals(&other.cookware, converter)
            && self.timers.equals(&other.timers, converter)
            && self
                .inline_quantities
                .equals(&other.inline_quantities, converter)
    }
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
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Content {
    /// A step
    Step(Step),
    /// A paragraph of just text, no instructions
    Text(String),
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

    /// Get's the inner step
    ///
    /// # Panics
    /// If the content is [`Content::Step`]
    pub fn unwrap_text(&self) -> &str {
        match self {
            Content::Step(_) => panic!("content is step"),
            Content::Text(t) => t.as_str(),
        }
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

/// A reference to another recipe, held as a normalized relative path
///
/// The path always has a file name. That is what lets [`Self::name`] be
/// infallible, so every way of building one -- [`FromStr`], [`Deserialize`] and
/// [`Self::resolved_from`] -- has to keep it true.
#[derive(Debug, Serialize, Clone)]
#[cfg_attr(feature = "ts", derive(Tsify))]
pub struct RecipeReference(RelativePathBuf);

impl<'de> Deserialize<'de> for RecipeReference {
    /// Deserializes through [`FromStr`], so a reference read back is normalized
    /// and has a name, like one that came from a parsed recipe.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let path = String::deserialize(deserializer)?;
        path.parse().map_err(serde::de::Error::custom)
    }
}

impl PartialEq for RecipeReference {
    fn eq(&self, other: &Self) -> bool {
        self.0.normalize() == other.0.normalize()
    }
}

impl RecipeReference {
    fn prefix(path: RelativePathBuf) -> RelativePathBuf {
        if path.starts_with("./") || path.starts_with("../") {
            path
        } else {
            RelativePathBuf::from("./").join(&path)
        }
    }
    fn normalize(path: RelativePathBuf) -> RelativePathBuf {
        Self::prefix(path.normalize())
    }
    pub fn name(&self) -> &str {
        self.0
            .file_name()
            .expect("Reference must have a name! This should not be possible")
    }

    pub fn resolved_from(&self, other: &RecipeReference) -> RecipeReference {
        let base = other
            .0
            .parent()
            .unwrap_or_else(|| relative_path::RelativePath::new(""));
        RecipeReference(Self::normalize(base.join(&self.0)))
    }
}

impl FromStr for RecipeReference {
    type Err = RecipeReferenceParseError;

    /// A reference has to name a file, so `..` and `./a/..` are errors
    fn from_str(path: &str) -> Result<Self, Self::Err> {
        match RelativePathBuf::from(path) {
            rp if rp.file_name().is_some() => Ok(RecipeReference(Self::normalize(rp))),
            _ => Err(RecipeReferenceParseError::MissingName(path.to_string())),
        }
    }
}

impl std::fmt::Display for RecipeReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
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
    /// How the ingredient is related to others
    pub relation: IngredientRelation,
    /// Serialized as the flag names, e.g. `"OPT"` or `"HIDDEN | OPT"`
    #[cfg_attr(feature = "ts", serde(default), tsify(type = "string"))]
    pub(crate) modifiers: Modifiers,

    /// The path of the recipe file this ingredient was parsed from
    pub recipe_path: Option<RecipeReference>,
}

impl Ingredient {
    /// The recipe reference resolved against the file this ingredient was
    /// parsed from
    ///
    /// A reference is written relative to its own recipe, so the same target
    /// reads differently from two directories. Resolving it against
    /// [`recipe_path`](Self::recipe_path) gives the one path both agree on.
    ///
    /// Without a recipe path there is nothing to resolve against, so the
    /// reference is returned as written.
    pub fn resolved_reference(&self) -> Option<RecipeReference> {
        let reference = self.reference.as_ref()?;
        Some(match &self.recipe_path {
            Some(path) => reference.resolved_from(path),
            None => reference.clone(),
        })
    }

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

/// Compares two ingredients using unit-aware quantity comparison.
///
/// Unlike [`PartialEq`], the quantity is compared with its [`SemanticEq`]
/// impl, so `500 g` equals `0.5 kg`.
///
/// A recipe reference is compared resolved against
/// [`recipe_path`](Ingredient::recipe_path), so the same target referenced
/// from two different directories compares equal. The recipe path itself is not
/// compared: it records which file the ingredient came from, not what it is.
impl SemanticEq for Ingredient {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.name == other.name
            && self.alias == other.alias
            && self.note == other.note
            && self.relation == other.relation
            && self.resolved_reference() == other.resolved_reference()
            && self.modifiers == other.modifiers
            && self.quantity.equals(&other.quantity, converter)
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

/// Compares two cookware items using unit-aware quantity comparison.
///
/// Unlike [`PartialEq`], the quantity is compared with its [`SemanticEq`] impl.
impl SemanticEq for Cookware {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.name == other.name
            && self.alias == other.alias
            && self.note == other.note
            && self.relation == other.relation
            && self.modifiers == other.modifiers
            && self.quantity.equals(&other.quantity, converter)
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

/// Compares two timers using unit-aware quantity comparison.
///
/// Unlike [`PartialEq`], the quantity is compared with its [`SemanticEq`] impl,
/// so `1 h` equals `60 min`.
impl SemanticEq for Timer {
    fn equals(&self, other: &Self, converter: &Converter) -> bool {
        self.name == other.name && self.quantity.equals(&other.quantity, converter)
    }
}

#[cfg(test)]
mod tests {
    use super::{Recipe, RecipeReference};
    use crate::{Converter, CooklangParser, Extensions, SemanticEq};

    use indoc::indoc;
    use std::sync::LazyLock;

    pub static PARSER: LazyLock<CooklangParser> =
        LazyLock::new(|| CooklangParser::new(Extensions::all(), Converter::default()));

    #[track_caller]
    fn recipe(s: &str) -> Recipe {
        let (recipe, _report) = PARSER.parse(s).into_result().unwrap();
        recipe
    }

    static CONVERTER: LazyLock<Converter> = LazyLock::new(Converter::default);

    fn eq(a: &str, b: &str) -> bool {
        recipe(a).equals(&recipe(b), &CONVERTER)
    }

    fn ne(a: &str, b: &str) -> bool {
        !eq(a, b)
    }

    #[track_caller]
    fn referenced(path: &str, s: &str) -> Recipe {
        let path = path
            .parse::<RecipeReference>()
            .expect("the path has a name");
        let (recipe, _report) = PARSER.parse_with_path(s, Some(path)).into_result().unwrap();
        recipe
    }

    #[track_caller]
    fn rel(reference: &str, other: &str) -> String {
        rref(reference).resolved_from(&rref(other)).to_string()
    }

    #[track_caller]
    fn rref(p: &str) -> RecipeReference {
        p.parse::<RecipeReference>().unwrap()
    }

    #[test]
    fn deserializing_a_reference_goes_through_the_constructor() {
        let parse = |json: &str| serde_json::from_str::<RecipeReference>(json);

        // a reference with no file name has no `name()` to return
        assert!(parse("\"..\"").is_err());
        assert!(parse("\"\"").is_err());

        // and the path is normalized, so `Display` keeps its `./` promise
        assert_eq!(parse("\"a/b\"").unwrap().to_string(), "./a/b");
        assert_eq!(parse("\"./a/../b\"").unwrap().to_string(), "./b");

        // round trips through serde
        let reference = "./sauces/pesto".parse::<RecipeReference>().unwrap();
        let json = serde_json::to_string(&reference).unwrap();
        assert_eq!(json, "\"./sauces/pesto\"");
        assert_eq!(parse(&json).unwrap(), reference);
    }

    #[test]
    fn comparing_ingredients_cookware_and_units() {
        // identical recipes are equal
        assert!(eq(
            "@flour{200%g} and @butter{100%g}",
            "@flour{200%g} and @butter{100%g}",
        ));

        // different ingredient name is not equal
        assert!(ne("@flour{200%g}", "@sugar{200%g}"));

        // different quantity value is not equal
        assert!(ne("@flour{200%g}", "@flour{100%g}"));

        // different number of ingredients is not equal
        assert!(ne("@flour{200%g} and @butter{100%g}", "@flour{200%g}"));

        // --- ingredient modifiers, aliases, notes and references ---

        // an optional ingredient is not the same as a required one
        assert!(ne("@?salt{}", "@salt{}"));
        assert!(eq("@?salt{}", "@?salt{}"));

        // an alias changes how the ingredient reads, so it is part of the recipe
        assert!(ne("@flour|plain flour{200%g}", "@flour{200%g}"));
        assert!(ne("@flour|a{200%g}", "@flour|b{200%g}"));

        // same for a note
        assert!(ne("@onion{1}(diced)", "@onion{1}"));
        assert!(ne("@onion{1}(diced)", "@onion{1}(sliced)"));

        // a recipe reference is not a plain ingredient of the same name
        assert!(ne("@./Guacamole{}", "@Guacamole{}"));
        // but reference paths are normalized before comparing
        assert!(eq("@./a/Guac.cook{}", "@./a/../a/Guac.cook{}"));

        // --- quantity values ---

        // fractions and decimals are the same number
        assert!(eq("@flour{1/2%dl}", "@flour{0.5%dl}"));
        assert!(eq("@flour{1/2}", "@flour{0.5}"));

        // text quantities are compared as text
        assert!(eq("@salt{pinch}", "@salt{pinch}"));
        assert!(ne("@salt{pinch}", "@salt{handful}"));
        assert!(ne("@salt{1}", "@salt{pinch}"));

        // no quantity at all is not a quantity
        assert!(ne("@salt{}", "@salt{1%g}"));

        // --- cookware ---

        // cookware without quantity is equal
        assert!(eq("Use #pan{}", "Use #pan{}"));

        // a cookware amount is compared too, and a text amount as written
        assert!(ne("Use #pan{small}", "Use #pan{big}"));
        assert!(eq("Use #pan{2}", "Use #pan{2}"));
        assert!(ne("Use #pan{2}", "Use #pan{3}"));
        assert!(ne("Use #pan{2}", "Use #pan{}"));

        // --- units ---

        // unit conversion: 5 dl == 0.5 l, 5 min == 300 s
        assert!(eq(
            "Cook @water{5%dl} for ~{5%min}",
            "Cook @water{0.5%l} for ~{300%sec}",
        ));

        // unit names are matched regardless of case
        assert!(eq("@milk{5%dL}", "@milk{5%dl}"));
        // and through their aliases
        assert!(eq("Cook for ~{10%min}", "Cook for ~{10%minutes}"));
        assert!(eq("@milk{1%l}", "@milk{1%liter}"));
        assert!(eq("@oil{3%tbsp}", "@oil{3%tablespoons}"));
        // different units of the same physical quantity convert
        assert!(eq("@oil{3%tsp}", "@oil{1%tbsp}"));
        // units the converter doesn't know are compared as written
        assert!(eq("@garlic{2%cloves}", "@garlic{2%cloves}"));
        assert!(ne("@garlic{2%cloves}", "@garlic{2%pieces}"));
    }

    #[test]
    fn a_reference_is_compared_resolved_against_its_recipe_path() {
        // the same target reached from two directories: `../spices/x.cook` from
        // `./pickles/`, and `./spices/x.cook` from the root
        let a = referenced("./pickles/sour.cook", "@../spices/x.cook{}");
        let b = referenced("./root.cook", "@./spices/x.cook{}");
        assert!(a.equals(&b, &CONVERTER));

        // a reference that resolves elsewhere still differs
        let c = referenced("./root.cook", "@./jams/x.cook{}");
        assert!(!a.equals(&c, &CONVERTER));

        // with no path there is nothing to resolve against, so the reference
        // is compared as written
        assert!(ne("@../spices/x.cook{}", "@./spices/x.cook{}"));
    }

    #[test]
    fn comparing_timers_and_inline_quantities() {
        // timer unit conversion: 1 h == 60 min
        assert!(eq("Cook for ~{1%h}", "Cook for ~{60%min}"));

        // a named timer is not an anonymous one
        assert!(ne("~resting{10%min}", "~{10%min}"));
        assert!(ne("~resting{10%min}", "~proving{10%min}"));
        // a named timer still converts units
        assert!(eq("~resting{1%h}", "~resting{60%min}"));

        // ranges convert unit by unit
        assert!(eq("Cook for ~{1-2%min}", "Cook for ~{60-120%sec}"));
        // a range is not the single value at its start
        assert!(ne("Cook for ~{10-15%min}", "Cook for ~{10%min}"));

        // inline quantities are compared by value
        assert!(eq("Bake at 200ºC", "Bake at 200ºC"));
        assert!(ne("Bake at 200ºC", "Bake at 180ºC"));
    }

    #[test]
    fn comparing_metadata() {
        // different metadata is not equal
        assert!(ne(">> title: Pasta", ">> title: Pizza"));

        // serves is an alias for servings
        assert!(eq(">> serves: 4", ">> servings: 4"));

        // different servings count is not equal
        assert!(ne(">> servings: 4", ">> servings: 8"));

        // yield is a quantity: 500%g == 0.5%kg (unit conversion)
        assert!(eq(">> yield: 500%g", ">> yield: 0.5%kg"));

        // yield with different amounts is not equal
        assert!(ne(">> yield: 500%g", ">> yield: 1000%g"));

        // yield and servings are distinct concepts
        assert!(ne(">> yield: 500%g", ">> servings: 4"));

        // tags are compared as a list, so their order matters
        assert!(eq(">> tags: a, b", ">> tags: a, b"));
        assert!(ne(">> tags: a, b", ">> tags: b, a"));

        // the remaining standard keys are compared as written
        assert!(ne(">> description: x", ">> description: y"));
        assert!(ne(">> difficulty: easy", ">> difficulty: hard"));
        assert!(ne(">> source: a", ">> source: b"));
        assert!(ne(">> author: a", ">> author: b"));

        // key aliases resolve to the same standard key
        assert!(eq(">> time: 10 min", ">> duration: 10 min"));
        assert!(eq(">> course: main", ">> category: main"));

        // a second entry under another alias is still part of the recipe, so it
        // cannot be silently dropped from the comparison
        assert!(ne(">> servings: 4\n>> serves: 99", ">> servings: 4"));
        assert!(ne(
            ">> servings: 4\n>> serves: 99",
            ">> servings: 4\n>> serves: 8"
        ));
        assert!(eq(
            ">> servings: 4\n>> serves: 99",
            ">> servings: 4\n>> serves: 99"
        ));

        // custom keys are compared as written, and must be present in both
        assert!(ne(">> mykey: a", ">> mykey: b"));
        assert!(ne(">> mykey: a", ">> otherkey: a"));
        assert!(ne(">> a: 1\n>> b: 2", ">> a: 1"));

        // the order custom keys are written in carries no meaning
        assert!(eq(">> a: 1\n>> b: 2", ">> b: 2\n>> a: 1"));
        assert!(ne(">> a: 1\n>> b: 2", ">> a: 2\n>> b: 1"));

        // a yield written without a unit is still compared as a quantity, so
        // the tolerance absorbs scaling error but not a written difference
        assert!(eq(">> yield: 2.5", ">> yield: 2.500001"));
        assert!(ne(">> yield: 2.5", ">> yield: 2.6"));

        // yield and yields compare to the same
        assert!(eq(">> yield: 12", ">> yields: 12"));

        // time values are compared as durations, so the same duration
        // spelled two ways is equal
        assert!(eq(">> time: 10 min", ">> time: 10 minutes"));
        assert!(eq(">> time: 1h30m", ">> time: 90 min"));
        assert!(ne(">> time: 10 min", ">> time: 15 min"));

        // same for prep and cook time
        assert!(eq(">> prep time: 1 hour", ">> prep time: 60 min"));
        assert!(ne(">> cook time: 1 hour", ">> cook time: 2 hours"));

        // time values that don't parse are still compared as written
        assert!(eq(">> time: a while", ">> time: a while"));
        assert!(ne(">> time: a while", ">> time: forever"));
    }

    #[test]
    fn comparing_recipe_structure() {
        // the section name is part of the recipe
        assert!(ne(
            indoc! {"
                = Dough =
                @flour{1}
            "},
            indoc! {"
                = Base =
                @flour{1}
            "},
        ));
        assert!(ne(
            indoc! {"
                = Dough =
                @flour{1}
            "},
            "@flour{1}",
        ));

        // so is a text block
        assert!(ne(
            indoc! {"
                > note a
                @flour{1}
            "},
            indoc! {"
                > note b
                @flour{1}
            "},
        ));
    }

    #[test]
    fn sibling_in_same_directory() {
        assert_eq!(
            rel("./tomato", "recipes/pasta/spaghetti"),
            "./recipes/pasta/tomato"
        );
    }

    #[test]
    fn parent_directory_reference() {
        assert_eq!(
            rel("../sauces/tomato", "recipes/pasta/spaghetti"),
            "./recipes/sauces/tomato"
        );
    }

    #[test]
    fn multiple_parent_directory_references() {
        assert_eq!(
            rel("../../sauces/tomato", "recipes/italian/pasta/spaghetti"),
            "./recipes/sauces/tomato"
        );
    }

    #[test]
    fn other_at_root_has_no_parent() {
        // "spaghetti" has no directory component, so the base is the root.
        assert_eq!(rel("./sauce", "spaghetti"), "./sauce");
    }

    #[test]
    fn reference_without_dot_prefix_is_treated_as_relative() {
        // A bare relative path behaves the same as one with a "./" prefix.
        assert_eq!(
            rel("sauces/tomato", "recipes/pasta/spaghetti"),
            "./recipes/pasta/sauces/tomato"
        );
    }

    #[test]
    fn resolving_from_self_gives_own_path() {
        assert_eq!(rel("./spaghetti", "spaghetti"), "./spaghetti");
    }

    #[test]
    fn can_escape_above_the_other_reference_root() {
        // Going up more levels than "other" has just walks above the root;
        // resolved_from does not clamp this.
        assert_eq!(rel("../tomato", "spaghetti"), "../tomato");
    }

    #[test]
    fn preserves_name_of_the_resolved_reference() {
        assert_eq!(
            rref("../sauces/tomato")
                .resolved_from(&rref("./recipes/pasta/spaghetti"))
                .name(),
            "tomato"
        );
    }

    #[test]
    fn bare_path_is_prefixed_with_dot_slash() {
        assert_eq!(rref("spaghetti").to_string(), "./spaghetti");
        assert_eq!(rref("pasta/spaghetti").to_string(), "./pasta/spaghetti");
    }

    #[test]
    fn already_prefixed_path_is_left_unchanged() {
        assert_eq!(rref("./spaghetti").to_string(), "./spaghetti");
        assert_eq!(rref("../spaghetti").to_string(), "../spaghetti");
    }
}
