//! Typed ingredient lines for the apps: see the `cooklang-ingredient-parser`
//! crate. The aisle configuration is passed as its text, because this
//! crate's own `AisleConf` record is a flattened cache.

use cooklang::aisle::parse_lenient;
use cooklang_ingredient_parser as parser;

use crate::model::{amount_to_quantity, quantity_to_amount, Amount};

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IngredientAmount {
    pub value: f64,
    pub unit: Option<String>,
    /// The quantity to store, its value rounded to six decimals; `None` if it
    /// rounds to zero. [`amount_to_cooklang`] writes it ("2%l").
    pub quantity: Option<Amount>,
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
                quantity: a.to_quantity().as_ref().map(quantity_to_amount),
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

/// The sum of two stored quantities (2 l + 1 l = 3 l). `None` only means
/// "no quantity" (both absent); quantities that cannot be added are an
/// error, so a caller keeps two lines instead of losing an amount.
#[uniffi::export]
pub fn add_ingredient_quantities(
    a: Option<Amount>,
    b: Option<Amount>,
) -> Result<Option<Amount>, IngredientQuantityError> {
    let a = a.as_ref().and_then(amount_to_quantity);
    let b = b.as_ref().and_then(amount_to_quantity);
    parser::add_quantities(a.as_ref(), b.as_ref())
        .map(|sum| sum.as_ref().map(quantity_to_amount))
        .map_err(|_| IngredientQuantityError::NotAddable)
}

/// The line that edits an item ("Milk 2 l"), or the name alone when the
/// rules would not read the quantity back the same.
#[uniffi::export]
pub fn ingredient_edit_text(name: String, quantity: Option<Amount>) -> String {
    let quantity = quantity.as_ref().and_then(amount_to_quantity);
    parser::edit_text(&name, quantity.as_ref())
}

/// An amount in Cooklang brace syntax, without the braces, as a shopping
/// list stores it: "2%l", "3", "a pinch". Numbers are decimals, so a
/// fraction is written "0.5%cup". `""` for `Value::Empty`.
#[uniffi::export]
pub fn amount_to_cooklang(amount: Amount) -> String {
    amount_to_quantity(&amount)
        .map(|q| q.to_cooklang())
        .unwrap_or_default()
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

    fn amount(value: f64, units: Option<&str>) -> Amount {
        Amount {
            quantity: crate::model::Value::Number { value },
            units: units.map(str::to_owned),
        }
    }

    #[test]
    fn adds_quantities() {
        let sum =
            add_ingredient_quantities(Some(amount(2.0, Some("l"))), Some(amount(1.0, Some("l"))))
                .unwrap();
        assert_eq!(sum, Some(amount(3.0, Some("l"))));
        assert_eq!(amount_to_cooklang(sum.unwrap()), "3%l");
        assert!(matches!(
            add_ingredient_quantities(Some(amount(2.0, Some("l"))), Some(amount(2.0, None))),
            Err(IngredientQuantityError::NotAddable)
        ));
        assert_eq!(add_ingredient_quantities(None, None).unwrap(), None);
    }

    #[test]
    fn parsed_quantity_is_stored_at_six_decimals() {
        let parsed = parse_ingredient("Sugar 1.5kg".into(), None);
        let quantity = parsed.amount.unwrap().quantity.unwrap();
        assert_eq!(amount_to_cooklang(quantity), "1.5%kg");
    }

    #[test]
    fn edit_text_takes_an_amount() {
        assert_eq!(
            ingredient_edit_text("Milk".into(), Some(amount(2.0, Some("l")))),
            "Milk 2 l"
        );
        assert_eq!(ingredient_edit_text("Bread".into(), None), "Bread");
    }

    #[test]
    fn amounts_in_cooklang_syntax() {
        use crate::model::Value;
        assert_eq!(amount_to_cooklang(amount(2.0, None)), "2");
        assert_eq!(amount_to_cooklang(amount(0.5, Some("cup"))), "0.5%cup");
        assert_eq!(
            amount_to_cooklang(Amount {
                quantity: Value::Range {
                    start: 1.0,
                    end: 2.0
                },
                units: Some("cup".into()),
            }),
            "1-2%cup"
        );
        assert_eq!(
            amount_to_cooklang(Amount {
                quantity: Value::Text {
                    value: "a pinch".into()
                },
                units: None,
            }),
            "a pinch"
        );
        assert_eq!(
            amount_to_cooklang(Amount {
                quantity: Value::Empty,
                units: None,
            }),
            ""
        );
    }
}
