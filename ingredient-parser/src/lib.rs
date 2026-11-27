//! Turns one typed ingredient line into a name and an amount.
//!
//! "Milk 2 l", "2l milk", "Сахар 200 гр", "2 cans chopped tomatoes": every
//! Cooklang app that takes typed or pasted ingredients reads them through
//! [`parse_ingredient`], so they all agree on the name and write the same
//! stored quantity. See `docs/superpowers/specs/2026-11-27-ingredient-parser-design.md`.
mod number;
mod quantity;
mod units;

pub use quantity::{add_quantities, edit_text, NotAddable};

use serde::{Deserialize, Serialize};

/// What [`parse_ingredient`] made of one line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct ParsedIngredient {
    /// The name to write: cleaned, and in the aisle configuration's spelling
    /// when `aisle_match` is set. `""` only for a blank line.
    pub name: String,
    /// `None` when the line has no amount.
    pub amount: Option<Amount>,
    /// "large", "small". Only a model fills `size`, `prep` and `comment`.
    pub size: Option<String>,
    /// Preparation ("chopped"). Only a model fills it.
    pub prep: Option<String>,
    /// Anything else the line says. Only a model fills it.
    pub comment: Option<String>,
    /// 1.0 for rules; the model's own score otherwise.
    pub confidence: f32,
    pub source: ParseSource,
    pub aisle_match: Option<AisleMatch>,
}

/// A number and an optional unit. No unit means a count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct Amount {
    /// Above zero and finite.
    pub value: f64,
    pub unit: Option<String>,
}

/// The aisle configuration entry a name matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct AisleMatch {
    /// The configuration's spelling of the matched name or alias.
    pub name: String,
    /// The first name of its line, which shopping lists show.
    pub common_name: String,
    /// The category it is listed under.
    pub category: String,
}

/// Who produced a [`ParsedIngredient`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
#[non_exhaustive]
pub enum ParseSource {
    Rules,
    Model,
}
mod name;

pub use name::cleaned_name;
mod list;
mod rules;
