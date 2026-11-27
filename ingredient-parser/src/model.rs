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
    /// "2", "1.5", "1 1/2". Ranges such as "1-2" are refused.
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub size: Option<String>,
    pub name: String,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
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
    if first.contains(',') {
        return false;
    }
    typed_number(first).is_some() || glued_amount(first).is_some() || is_vulgar_amount(first)
}

/// "½", "1½", "2 ⅓" style first words: ASCII digits, ".", "/" and at least
/// one vulgar fraction character.
fn is_vulgar_amount(word: &str) -> bool {
    let vulgar = |c: char| matches!(c, '\u{00BC}'..='\u{00BE}' | '\u{2153}'..='\u{215E}');
    word.chars().any(vulgar)
        && word
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | '/') || vulgar(c))
}

/// The model's answer as a parse, or `None` when it must not be used.
pub(crate) fn accept(answer: Result<ModelParse, ModelError>) -> Option<ParsedIngredient> {
    let answer = answer.ok()?;
    if answer.confidence.is_nan() || answer.confidence < MODEL_CONFIDENCE_THRESHOLD {
        return None;
    }
    let name = cleaned_name(&answer.name);
    if name.is_empty() {
        return None;
    }
    // The model is only asked about lines that start with a number.
    let quantity = answer
        .quantity
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())?;
    let value = model_number(quantity)?;
    // The tagger copies a size into the unit slot when there is no unit.
    let unit = match (answer.unit.as_deref(), answer.size.as_deref()) {
        (Some(unit), Some(size)) if unit.trim().eq_ignore_ascii_case(size.trim()) => None,
        (Some(unit), _) => model_unit(unit),
        (None, _) => None,
    };
    let amount = Some(Amount { value, unit });
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
            let whole_part = whole(whole_part).filter(|w| *w > 0.0)?;
            Some(whole_part + typed_number(fraction)?)
        }
        _ => None,
    }
}

/// A table unit in its short form; a piece word is a count (`None`); any
/// other unit cleaned like a name, without `%`.
fn model_unit(unit: &str) -> Option<String> {
    match match_unit(unit) {
        Some(UnitMatch::Unit(stored)) => Some(stored.to_owned()),
        Some(UnitMatch::Count) => None,
        None => {
            let kept = cleaned_name(unit).replace('%', "");
            let kept = kept.trim();
            (!kept.is_empty()).then(|| kept.to_owned())
        }
    }
}
