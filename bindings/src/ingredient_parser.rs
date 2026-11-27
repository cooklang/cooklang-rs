//! Typed ingredient lines for the apps: see the `cooklang-ingredient-parser`
//! crate. The aisle configuration is passed as its text, because this
//! crate's own `AisleConf` record is a flattened cache.

use cooklang::aisle::parse_lenient;
use cooklang_ingredient_parser as parser;

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IngredientAmount {
    pub value: f64,
    pub unit: Option<String>,
    /// Cooklang quantity syntax ("2%l"); `None` if it rounds to zero.
    pub quantity: Option<String>,
    /// "2 l", "x3".
    pub chip_text: String,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IngredientAisleMatch {
    pub name: String,
    pub common_name: String,
    pub category: String,
}

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq)]
pub enum IngredientParseSource {
    Rules,
    Model,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ParsedIngredient {
    pub name: String,
    pub amount: Option<IngredientAmount>,
    pub size: Option<String>,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
    pub source: IngredientParseSource,
    pub aisle_match: Option<IngredientAisleMatch>,
}

impl From<parser::ParsedIngredient> for ParsedIngredient {
    fn from(p: parser::ParsedIngredient) -> Self {
        Self {
            name: p.name,
            amount: p.amount.map(|a| IngredientAmount {
                quantity: a.to_quantity(),
                chip_text: a.chip_text(),
                value: a.value,
                unit: a.unit,
            }),
            size: p.size,
            prep: p.prep,
            comment: p.comment,
            confidence: p.confidence,
            source: match p.source {
                parser::ParseSource::Model => IngredientParseSource::Model,
                _ => IngredientParseSource::Rules,
            },
            aisle_match: p.aisle_match.map(|m| IngredientAisleMatch {
                name: m.name,
                common_name: m.common_name,
                category: m.category,
            }),
        }
    }
}

/// Parses one typed ingredient line. `aisle_text` is the aisle.conf content.
#[uniffi::export]
pub fn parse_ingredient(line: String, aisle_text: Option<String>) -> ParsedIngredient {
    let aisle = aisle_text
        .as_deref()
        .and_then(|t| parse_lenient(t).into_output());
    parser::parse_ingredient(&line, aisle.as_ref(), None).into()
}

/// Parses pasted text, one result per kept line.
#[uniffi::export]
pub fn parse_ingredient_list(text: String, aisle_text: Option<String>) -> Vec<ParsedIngredient> {
    let aisle = aisle_text
        .as_deref()
        .and_then(|t| parse_lenient(t).into_output());
    parser::parse_ingredient_list(&text, aisle.as_ref(), None)
        .into_iter()
        .map(Into::into)
        .collect()
}

/// Two quantities that cannot be summed (different units, or no number).
#[derive(uniffi::Error, Debug, thiserror::Error)]
pub enum IngredientQuantityError {
    #[error("the two quantities cannot be added")]
    NotAddable,
}

/// The sum of two stored quantities ("2%l" + "1%l" = "3%l"). `None` only
/// means "no quantity" (both absent); quantities that cannot be added are
/// an error, so a caller keeps two lines instead of losing an amount.
#[uniffi::export]
pub fn add_ingredient_quantities(
    a: Option<String>,
    b: Option<String>,
) -> Result<Option<String>, IngredientQuantityError> {
    parser::add_quantities(a.as_deref(), b.as_deref())
        .map_err(|_| IngredientQuantityError::NotAddable)
}

#[uniffi::export]
pub fn ingredient_edit_text(name: String, quantity: Option<String>) -> String {
    parser::edit_text(&name, quantity.as_deref())
}

#[uniffi::export]
pub fn cleaned_ingredient_name(raw: String) -> String {
    parser::cleaned_name(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_aisle_text() {
        let parsed = parse_ingredient("Tomato 2".into(), Some("[veg]\ntomatoes\n".into()));
        assert_eq!(parsed.name, "tomatoes");
        assert_eq!(parsed.amount.unwrap().chip_text, "x2");
        assert_eq!(parsed.aisle_match.unwrap().category, "veg");
    }

    #[test]
    fn adds_quantities() {
        assert_eq!(
            add_ingredient_quantities(Some("2%l".into()), Some("1%l".into())).unwrap(),
            Some("3%l".into())
        );
        assert!(matches!(
            add_ingredient_quantities(Some("2%l".into()), Some("2".into())),
            Err(IngredientQuantityError::NotAddable)
        ));
        assert_eq!(add_ingredient_quantities(None, None).unwrap(), None);
    }
}
