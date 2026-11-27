//! The text forms of an amount: stored ("2%l"), shown ("2 l", chip "x3"),
//! and the line that edits an item ("Milk 2 l"). Ported from the iOS
//! `ManualItemQuantity.swift`.

use cooklang::quantity::{Number, Quantity, Value};

use crate::number::format_number;
use crate::units::{match_unit, UnitMatch};
use crate::Amount;

impl Amount {
    /// The quantity to store, written `"2%l"` (or `"3"` for a count) by
    /// [`Quantity::to_cooklang`]. The value is rounded to six decimals, as
    /// the iOS app stores it. `None` when the value is not above zero or
    /// rounds to `"0"` at six decimals.
    pub fn to_quantity(&self) -> Option<Quantity> {
        if !(self.value.is_finite() && self.value > 0.0) {
            return None;
        }
        let number = format_number(self.value);
        if number == "0" {
            return None;
        }
        let value: f64 = number.parse().ok()?;
        Some(Quantity::new(
            Value::Number(Number::Regular(value)),
            self.unit.clone(),
        ))
    }

    /// `"2 l"`, `"3"` for a count.
    pub fn display(&self) -> String {
        match &self.unit {
            Some(unit) => format!("{} {unit}", format_number(self.value)),
            None => format_number(self.value),
        }
    }

    /// `"2 l"`, `"x3"` for a count.
    pub fn chip_text(&self) -> String {
        match &self.unit {
            Some(_) => self.display(),
            None => format!("x{}", format_number(self.value)),
        }
    }

    /// Reads a stored quantity back: a single number above zero, with or
    /// without a unit. `None` for text ("a pinch", "200 ml", "1,000%g") and
    /// ranges ("1-2%cup").
    pub fn from_quantity(quantity: &Quantity) -> Option<Amount> {
        let Value::Number(number) = quantity.value() else {
            return None;
        };
        let value = number.value();
        if !(value.is_finite() && value > 0.0) {
            return None;
        }
        Some(Amount {
            value,
            unit: quantity.unit().map(str::to_owned),
        })
    }
}

/// Two stored quantities cannot be added: different units, text, or one
/// with a quantity and one without.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the two quantities cannot be added")]
pub struct NotAddable;

/// The sum of two stored quantities with the same unit (compared exactly).
/// Two `None`s give `None`.
pub fn add_quantities(
    a: Option<&Quantity>,
    b: Option<&Quantity>,
) -> Result<Option<Quantity>, NotAddable> {
    match (a, b) {
        (None, None) => Ok(None),
        (Some(a), Some(b)) => {
            let a = Amount::from_quantity(a).ok_or(NotAddable)?;
            let b = Amount::from_quantity(b).ok_or(NotAddable)?;
            if a.unit != b.unit {
                return Err(NotAddable);
            }
            let sum = Amount {
                value: a.value + b.value,
                unit: a.unit,
            };
            sum.to_quantity().map(Some).ok_or(NotAddable)
        }
        _ => Err(NotAddable),
    }
}

/// The stored amount when the rules read its display form back the same.
fn editable_amount(quantity: &Quantity) -> Option<Amount> {
    let amount = Amount::from_quantity(quantity)?;
    match &amount.unit {
        None => Some(amount),
        Some(unit) => matches!(match_unit(unit), Some(UnitMatch::Unit(stored)) if stored == unit)
            .then_some(amount),
    }
}

/// The line to put in an input field so the rules read it back as the same
/// item: `"Milk 2 l"`, `"Lemons 3"`; the name alone when the quantity has a
/// unit outside the table ("2%bunches"), a unit in another spelling ("2%L"),
/// or is text ("a pinch").
pub fn edit_text(name: &str, quantity: Option<&Quantity>) -> String {
    match quantity.and_then(editable_amount) {
        Some(amount) => format!("{name} {}", amount.display()),
        None => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cooklang::shopping_list::{self, ShoppingListItem};

    fn amount(value: f64, unit: Option<&str>) -> Amount {
        Amount {
            value,
            unit: unit.map(str::to_owned),
        }
    }

    /// The quantity a shopping list line `x{text}` holds.
    fn q(text: &str) -> Option<Quantity> {
        let list = shopping_list::parse(&format!("x{{{text}}}\n")).unwrap();
        match list.items.into_iter().next()? {
            ShoppingListItem::Ingredient(item) if item.name == "x" => item.quantity,
            _ => None,
        }
    }

    fn stored(amount: Amount) -> Option<String> {
        amount.to_quantity().map(|q| q.to_cooklang())
    }

    fn read(text: &str) -> Option<Amount> {
        q(text).as_ref().and_then(Amount::from_quantity)
    }

    fn add(a: Option<&str>, b: Option<&str>) -> Result<Option<String>, NotAddable> {
        let a = a.map(|a| q(a).expect(a));
        let b = b.map(|b| q(b).expect(b));
        add_quantities(a.as_ref(), b.as_ref()).map(|sum| sum.map(|q| q.to_cooklang()))
    }

    fn edit(name: &str, quantity: Option<&str>) -> String {
        let quantity = quantity.map(|text| q(text).expect(text));
        edit_text(name, quantity.as_ref())
    }

    #[test]
    fn stored_form() {
        assert_eq!(stored(amount(2.0, Some("l"))).as_deref(), Some("2%l"));
        assert_eq!(stored(amount(1.5, Some("kg"))).as_deref(), Some("1.5%kg"));
        assert_eq!(stored(amount(0.5, Some("kg"))).as_deref(), Some("0.5%kg"));
        assert_eq!(stored(amount(3.0, None)).as_deref(), Some("3"));
        assert_eq!(amount(0.0, Some("g")).to_quantity(), None);
        assert_eq!(amount(-1.0, Some("l")).to_quantity(), None);
        assert_eq!(amount(f64::INFINITY, None).to_quantity(), None);
        assert_eq!(amount(0.0000001, Some("g")).to_quantity(), None);
        assert_eq!(
            stored(amount(0.000001, Some("g"))).as_deref(),
            Some("0.000001%g")
        );
    }

    #[test]
    fn stores_six_decimals() {
        assert_eq!(
            stored(amount(2.0 / 3.0, Some("cup"))).as_deref(),
            Some("0.666667%cup")
        );
    }

    #[test]
    fn reads_stored_form() {
        assert_eq!(read("2%l"), Some(amount(2.0, Some("l"))));
        assert_eq!(read("1.5%kg"), Some(amount(1.5, Some("kg"))));
        assert_eq!(read("3"), Some(amount(3.0, None)));
        assert_eq!(read("a pinch"), None);
        assert_eq!(read("200 ml"), None);
        assert_eq!(read(""), None);
        assert_eq!(read("1/2%cup"), Some(amount(0.5, Some("cup"))));
        assert_eq!(read("3/2%cup"), Some(amount(1.5, Some("cup"))));
        assert_eq!(read("0%g"), None);
        assert_eq!(read("-1%l"), None);
        assert_eq!(read("0x10"), None);
        assert_eq!(read(" 2 % l "), Some(amount(2.0, Some("l"))));
        assert_eq!(read("2%"), Some(amount(2.0, None)));
        assert_eq!(read("1,000%g"), None);
        assert_eq!(read("1,5%kg"), None);
        assert_eq!(read("1-2%cup"), None);
    }

    #[test]
    fn a_small_amount_survives_storing() {
        let small = amount(0.0004, Some("kg"));
        let quantity = small.to_quantity().unwrap();
        assert_eq!(Amount::from_quantity(&quantity), Some(small.clone()));
        assert_eq!(read(&quantity.to_cooklang()), Some(small));
    }

    #[test]
    fn display_and_chip() {
        assert_eq!(amount(500.0, Some("g")).chip_text(), "500 g");
        assert_eq!(amount(1.5, Some("kg")).chip_text(), "1.5 kg");
        assert_eq!(amount(3.0, None).chip_text(), "x3");
        assert_eq!(amount(3.0, None).display(), "3");
        assert_eq!(amount(2.0, Some("l")).display(), "2 l");
    }

    #[test]
    fn adds_quantities() {
        assert_eq!(add(Some("2%l"), Some("1%l")), Ok(Some("3%l".into())));
        assert_eq!(add(Some("1.5%kg"), Some("0.5%kg")), Ok(Some("2%kg".into())));
        assert_eq!(add(Some("3"), Some("2")), Ok(Some("5".into())));
        assert_eq!(add(None, None), Ok(None));
        assert_eq!(add(Some("2%l"), Some("500%ml")), Err(NotAddable));
        assert_eq!(add(Some("2%l"), Some("2")), Err(NotAddable));
        assert_eq!(add(Some("2%l"), None), Err(NotAddable));
        assert_eq!(add(None, Some("2")), Err(NotAddable));
        assert_eq!(add(Some("a pinch"), Some("a pinch")), Err(NotAddable));
        assert_eq!(add(Some("2%L"), Some("2%l")), Err(NotAddable));
    }

    #[test]
    fn edit_text_is_the_line_that_makes_the_item() {
        assert_eq!(edit("Milk", Some("2%l")), "Milk 2 l");
        assert_eq!(edit("Baking powder", Some("1")), "Baking powder 1");
        assert_eq!(edit("Bread", None), "Bread");
        assert_eq!(edit("olive oil", Some("a splash")), "olive oil");
        assert_eq!(edit("Oil", Some("1-2%cup")), "Oil");
        assert_eq!(edit("parsley", Some("2%bunches")), "parsley");
        assert_eq!(edit("Milk", Some("2%L")), "Milk");
    }
}
