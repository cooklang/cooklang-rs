//! Plugging in a line-level model, such as `ingredient-tagger` in
//! `cook-md/db`. The rules decide first; see `should_ask`.

use crate::name::cleaned_name;
use crate::number::{number, typed_number, whole};
use crate::rules::{glued_amount, RulesResult};
use crate::units::{match_unit, UnitMatch};
use crate::{Amount, ParseSource, ParsedIngredient};
use serde::{Deserialize, Serialize};

/// A model that splits one ingredient line into its parts.
pub trait IngredientModel {
    fn parse_line(&self, line: &str) -> Result<ModelParse, ModelError>;
}

/// What a model returns; field for field `ingredient_tagger::ParsedIngredient`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelParse {
    /// "2", "1 1/2", "1-2".
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub size: Option<String>,
    pub name: String,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ingredient model failed: {0}")]
pub struct ModelError(pub String);

/// The lowest model confidence [`crate::parse_ingredient`] accepts.
pub const MODEL_CONFIDENCE_THRESHOLD: f32 = 0.80;

/// Ask only about lines whose first word is a number, and only when the
/// rules did not already take an amount from the end of the line.
pub(crate) fn should_ask(line: &str, rules: &RulesResult) -> bool {
    if rules.amount.is_some() && rules.from_end {
        return false;
    }
    let Some(first) = line.split_whitespace().next() else {
        return false;
    };
    typed_number(first).is_some() || glued_amount(first).is_some()
}

/// The model's answer as a parse, or `None` when it must not be used.
pub(crate) fn accept(answer: Result<ModelParse, ModelError>) -> Option<ParsedIngredient> {
    let answer = answer.ok()?;
    // `>=` is false for NaN, so a NaN confidence is refused too.
    if !(answer.confidence >= MODEL_CONFIDENCE_THRESHOLD) {
        return None;
    }
    let name = cleaned_name(&answer.name);
    if name.is_empty() {
        return None;
    }
    let value = match answer
        .quantity
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
    {
        None => None,
        Some(quantity) => Some(model_number(quantity)?),
    };
    let amount = value.map(|value| Amount {
        value,
        unit: answer.unit.as_deref().and_then(model_unit),
    });
    Some(ParsedIngredient {
        name,
        amount,
        size: answer.size,
        prep: answer.prep,
        comment: answer.comment,
        confidence: answer.confidence,
        source: ParseSource::Model,
        aisle_match: None,
    })
}

/// One positive number: whole, decimal, any fraction, or mixed ("1 1/2").
fn model_number(quantity: &str) -> Option<f64> {
    let parts: Vec<&str> = quantity.split_whitespace().collect();
    match parts.as_slice() {
        [single] => number(single),
        [whole_part, fraction] if fraction.contains('/') => {
            Some(whole(whole_part)? + number(fraction)?)
        }
        _ => None,
    }
}

/// A table unit in its short form; a piece word is a count (`None`); any
/// other unit as given, without `%`, `{`, `}` and with single spaces.
fn model_unit(unit: &str) -> Option<String> {
    match match_unit(unit) {
        Some(UnitMatch::Unit(stored)) => Some(stored.to_owned()),
        Some(UnitMatch::Count) => None,
        None => {
            let kept: String = unit
                .chars()
                .filter(|c| !matches!(c, '%' | '{' | '}'))
                .collect();
            let kept = kept.split_whitespace().collect::<Vec<_>>().join(" ");
            (!kept.is_empty()).then_some(kept)
        }
    }
}
