use crate::{error::Recover, located::Located, quantity::Value, span::Span, text::Text};

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Lines that form a recipe.
///
/// They may not be just 1 line in the file, as a single step can be parsed from
/// multiple lines.
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum Block<'a> {
    /// Metadata entry
    Metadata { key: Text<'a>, value: Text<'a> },
    /// Section divider
    ///
    /// In the ast, a section does not own steps, it just exists in between.
    Section { name: Option<Text<'a>> },
    /// Recipe step
    Step {
        /// Items that compose the step.
        ///
        /// This is in order, so to form the representation of the step just
        /// iterate over the items and process them in that order.
        items: Vec<Item<'a>>,
    },
    /// A paragraph of instructions
    TextBlock(Vec<Text<'a>>),
}

/// An item of a [`Block::Step`].
#[derive(Debug, Serialize, PartialEq, Clone)]
pub enum Item<'a> {
    /// Plain text
    Text(Text<'a>),
    Ingredient(Box<Located<Ingredient<'a>>>),
    Cookware(Box<Located<Cookware<'a>>>),
    Timer(Box<Located<Timer<'a>>>),
}

impl Item<'_> {
    /// Returns the location of the item in the original input
    pub fn span(&self) -> Span {
        match self {
            Item::Text(t) => t.span(),
            Item::Ingredient(c) => c.span(),
            Item::Cookware(c) => c.span(),
            Item::Timer(c) => c.span(),
        }
    }
}

/// Ingredient [`Item`]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Ingredient<'a> {
    /// Ingredient modifiers
    ///
    /// If there are no modifiers, this will be [`Modifiers::empty`] and the
    /// location of where the modifiers would be.
    pub modifiers: Located<Modifiers>,
    /// Data for intermediate references
    ///
    /// If any of those modifiers is present, this will be.
    pub intermediate_data: Option<Located<IntermediateData>>,
    pub name: Text<'a>,
    pub alias: Option<Text<'a>>,
    pub quantity: Option<Located<Quantity<'a>>>,
    pub note: Option<Text<'a>>,
}

/// Cookware [`Item`]
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Cookware<'a> {
    /// Cookware modifiers
    ///
    /// If there are no modifiers, this will be [`Modifiers::empty`] and the
    /// location of where the modifiers would be.
    pub modifiers: Located<Modifiers>,
    pub name: Text<'a>,
    pub alias: Option<Text<'a>>,
    /// This it's just a [`QuantityValue`], because cookware cannot not have
    /// a unit.
    pub quantity: Option<Located<QuantityValue>>,
    pub note: Option<Text<'a>>,
}

/// Timer [`Item`]
///
/// At least one of the fields is guaranteed to be [`Some`].
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Timer<'a> {
    pub name: Option<Text<'a>>,
    /// If the [`TIMER_REQUIRES_TIME`](crate::Extensions::TIMER_REQUIRES_TIME)
    /// extension is enabled, this is guaranteed to be [`Some`].
    pub quantity: Option<Located<Quantity<'a>>>,
}

/// Quantity used in [items](Item)
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Quantity<'a> {
    /// Value or values
    pub value: QuantityValue,
    /// Unit text
    ///
    /// It's just the text, no checks
    pub unit: Option<Text<'a>>,
}

/// Quantity value(s)
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuantityValue {
    pub value: Located<Value>,
    pub scaling_lock: Option<Span>
}

impl QuantityValue {
    /// Calculates the span of the value
    pub fn span(&self) -> Span {
        self.value.span()
    }
}

impl Recover for Text<'_> {
    fn recover() -> Self {
        Self::empty(0)
    }
}

impl Recover for Quantity<'_> {
    fn recover() -> Self {
        Self {
            value: Recover::recover(),
            unit: Recover::recover(),
        }
    }
}

impl Recover for QuantityValue {
    fn recover() -> Self {
        Self {
            value: Recover::recover(),
            scaling_lock: None,
        }
    }
}

impl Recover for Value {
    fn recover() -> Self {
        1.0.into()
    }
}

bitflags! {
    /// Component modifiers
    ///
    /// Sadly, for now this can represent invalid combinations of modifiers.
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct Modifiers: u16 {
        /// refers to a recipe with the same name
        const RECIPE         = 1 << 0;
        /// references another igr with the same name, if amount given will sum
        const REF            = 1 << 1;
        /// not shown in the ingredient list, only inline
        const HIDDEN         = 1 << 2;
        /// mark as optional
        const OPT            = 1 << 3;
        /// forces to create a new ingredient
        const NEW            = 1 << 4;
    }
}

impl Modifiers {
    /// Returns true if the component should be diplayed in a list
    pub fn should_be_listed(self) -> bool {
        !self.intersects(Modifiers::HIDDEN | Modifiers::REF)
    }

    pub fn is_hidden(&self) -> bool {
        self.contains(Modifiers::HIDDEN)
    }

    pub fn is_optional(&self) -> bool {
        self.contains(Modifiers::OPT)
    }

    pub fn is_recipe(&self) -> bool {
        self.contains(Modifiers::RECIPE)
    }

    pub fn is_reference(&self) -> bool {
        self.contains(Modifiers::REF)
    }
}

impl std::fmt::Display for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

/// Data for interemediate references
///
/// This is not checked, and may point to inexistent or future steps/sections
/// which is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntermediateData {
    /// The mode in which `val` works
    pub ref_mode: IntermediateRefMode,
    /// The target of the reference
    pub target_kind: IntermediateTargetKind,
    /// Value
    ///
    /// This means:
    ///
    /// | `ref_mode`/`target_kind` | [`Step`]                               | [`Section`]               |
    /// |:-------------------------|:---------------------------------------|:--------------------------|
    /// | [`Number`]               | Step number **in the current section** | Section number            |
    /// | [`Relative`]             | Number of non text steps back          | Number of sections back   |
    ///
    /// [`Step`]: IntermediateTargetKind::Step
    /// [`Section`]: IntermediateTargetKind::Section
    /// [`Number`]: IntermediateRefMode::Number
    /// [`Relative`]: IntermediateRefMode::Relative
    pub val: i16,
}

/// How to treat the value in [`IntermediateData`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntermediateRefMode {
    /// Step or section number
    Number,
    /// Relative backwards
    ///
    /// When it is steps, is number of non text steps back.
    Relative,
}

/// What the target of [`IntermediateData`] is
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntermediateTargetKind {
    /// A step in the current section
    Step,
    /// A section of the recipe
    Section,
}
