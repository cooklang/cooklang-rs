//! A cleaned name, written as a free-hand `.shopping-list` line, reads back
//! as one item with that name and quantity; `edit_text` reads back through
//! the rules as the same item.

use cooklang::shopping_list::{self, IngredientItem, ShoppingList, ShoppingListItem};
use cooklang_ingredient_parser::{cleaned_name, edit_text, parse_ingredient};
use test_case::test_case;

#[test_case("Tomatoes {ripe}")]
#[test_case("salt -- sea")]
#[test_case("a [- b -] c")]
#[test_case("./Milk")]
#[test_case("50% cream")]
#[test_case("7up")]
#[test_case("Crème fraîche")]
#[test_case("a [-\u{301} b")]
#[test_case("a --\u{301} b")]
#[test_case("./\u{301}Milk")]
#[test_case("- salt")]
#[test_case("-]")]
#[test_case("a\tb")]
#[test_case("? milk")]
fn a_cleaned_name_reads_back_from_the_shopping_list(raw: &str) {
    let name = cleaned_name(raw);
    let items = vec![
        ShoppingListItem::Ingredient(IngredientItem {
            name: name.clone(),
            quantity: Some("2%l".into()),
            optional: false,
        }),
        ShoppingListItem::Ingredient(IngredientItem {
            name: name.clone(),
            quantity: None,
            optional: false,
        }),
    ];
    let list = ShoppingList { items };
    let mut text = Vec::new();
    shopping_list::write(&list, &mut text).unwrap();
    let text = String::from_utf8(text).unwrap();
    assert_eq!(shopping_list::parse(&text).unwrap(), list, "{text}");
}

#[test_case("Milk", Some("2%l"))]
#[test_case("Baking powder", Some("1"))]
#[test_case("Sugar", Some("1.5%kg"))]
#[test_case("Bread", None)]
fn edit_text_reads_back_as_the_same_item(name: &str, quantity: Option<&str>) {
    let parsed = parse_ingredient(&edit_text(name, quantity), None, None);
    assert_eq!(parsed.name, name);
    assert_eq!(
        parsed.amount.and_then(|a| a.to_quantity()).as_deref(),
        quantity
    );
}
