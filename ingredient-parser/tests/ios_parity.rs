//! The iOS recogniser's test cases (mobile-app-ios a6f8a1d1,
//! Packages/ShoppingList/Tests/ShoppingListTests/IngredientLineRecognizerTests.swift).
//! The two implementations must agree on every line.

use cooklang_ingredient_parser::parse_ingredient;
use test_case::test_case;

fn parsed(line: &str) -> (String, Option<f64>, Option<String>) {
    let p = parse_ingredient(line, None, None);
    let value = p.amount.as_ref().map(|a| a.value);
    (p.name, value, p.amount.and_then(|a| a.unit))
}

fn expect(
    name: &str,
    value: Option<f64>,
    unit: Option<&str>,
) -> (String, Option<f64>, Option<String>) {
    (name.to_owned(), value, unit.map(str::to_owned))
}

#[test_case("Milk 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk\u{00A0}2\u{202F}l", "Milk", Some(2.0), Some("l"))]
#[test_case("2 l milk", "milk", Some(2.0), Some("l"))]
#[test_case("Milk 2l", "Milk", Some(2.0), Some("l"))]
#[test_case("Sugar 1.5kg", "Sugar", Some(1.5), Some("kg"))]
#[test_case("Sugar 500 grams", "Sugar", Some(500.0), Some("g"))]
#[test_case("Сахар 200 гр", "Сахар", Some(200.0), Some("g"); "russian sugar in grams")]
#[test_case("Lemons 3", "Lemons", Some(3.0), None)]
#[test_case("3 lemons", "lemons", Some(3.0), None)]
#[test_case("Milk2l", "Milk2l", None, None)]
#[test_case("7up", "7up", None, None)]
#[test_case("Sugar500 grams", "Sugar500 grams", None, None)]
#[test_case("Diet 7up", "Diet 7up", None, None)]
#[test_case("7up 6", "7up", Some(6.0), None)]
#[test_case("Milk2l 3", "Milk2l", Some(3.0), None)]
#[test_case("2l milk", "milk", Some(2.0), Some("l"))]
#[test_case("3шт яйца", "яйца", Some(3.0), None)]
#[test_case("Lemons 1/2", "Lemons", Some(0.5), None)]
#[test_case("Milk 0", "Milk 0", None, None)]
#[test_case("Eggs 0 pcs", "Eggs 0 pcs", None, None)]
#[test_case("Flour 0/5 kg", "Flour 0/5 kg", None, None)]
#[test_case("Sugar", "Sugar", None, None)]
#[test_case("Milk 2 packs", "Milk 2 packs", None, None)]
#[test_case("Flour 1,5 kg", "Flour", Some(1.5), Some("kg"))]
#[test_case("1/2 kg flour", "flour", Some(0.5), Some("kg"))]
#[test_case("Flour 1/0 kg", "Flour 1/0 kg", None, None)]
#[test_case("Flour 1/2kg", "Flour", Some(0.5), Some("kg"))]
#[test_case("1/2kg flour", "flour", Some(0.5), Some("kg"))]
#[test_case("24/7 pizza", "24/7 pizza", None, None)]
#[test_case("500 g", "500 g", None, None)]
#[test_case("2", "2", None, None)]
#[test_case("", "", None, None)]
#[test_case("2 milk 3", "2 milk", Some(3.0), None)]
#[test_case("  Olive   oil  1 l ", "Olive oil", Some(1.0), Some("l"))]
#[test_case("Salt 1 TSP.", "Salt", Some(1.0), Some("tsp"))]
#[test_case("Соль 2 ч.л.", "Соль", Some(2.0), Some("tsp"); "russian salt in teaspoons")]
#[test_case("Eggs 6 pcs", "Eggs", Some(6.0), None)]
#[test_case("Яйца 10 шт", "Яйца", Some(10.0), None; "russian eggs in pieces")]
#[test_case("Eier 6 Stück", "Eier", Some(6.0), None)]
#[test_case("potato, 2 kg", "potato", Some(2.0), Some("kg"))]
#[test_case("onion, 1", "onion", Some(1.0), None)]
#[test_case("Milk: 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk - 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk – 2l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk , 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Vitamin C: 500", "Vitamin C", Some(500.0), None)]
#[test_case(
    "olive oil, 2 Tablespoons, 50 ml",
    "olive oil, 2 Tablespoons",
    Some(50.0),
    Some("ml")
)]
#[test_case("2 kg - potatoes", "potatoes", Some(2.0), Some("kg"))]
#[test_case("2 kg , potatoes", "potatoes", Some(2.0), Some("kg"))]
#[test_case("bread,", "bread", None, None)]
#[test_case("bread,,", "bread", None, None)]
#[test_case("bread;,", "bread", None, None)]
#[test_case(",bread", ",bread", None, None)]
#[test_case("Milk ,2", "Milk ,2", None, None)]
#[test_case("Milk 2 l,", "Milk", Some(2.0), Some("l"))]
#[test_case("Lemons 3;", "Lemons", Some(3.0), None)]
#[test_case("garlic, 1 head", "garlic, 1 head", None, None)]
#[test_case("Mix 50:50", "Mix 50:50", None, None)]
#[test_case("Note: buy early", "Note: buy early", None, None)]
#[test_case("Coca-Cola 2", "Coca-Cola", Some(2.0), None)]
#[test_case(", 2 kg", ", 2 kg", None, None)]
#[test_case("Milk, 2 l", "Milk", Some(2.0), Some("l"))]
fn recognises(line: &str, name: &str, value: Option<f64>, unit: Option<&str>) {
    assert_eq!(parsed(line), expect(name, value, unit));
}

#[test_case("g", &["g", "gr", "grams", "Gramm", "gramos", "grammes", "gramas", "г", "грамм"])]
#[test_case("kg", &["kg", "kgs", "kilo", "Kilogramm", "kilos", "quilos", "кг", "килограмм"])]
#[test_case("ml", &["ml", "milliliter", "millilitre", "mililitros", "мл"])]
#[test_case("l", &["l", "L", "liter", "litre", "litros", "л", "литр"])]
#[test_case("tsp", &["tsp", "tsps", "teaspoon", "TL", "cdta", "cdtas", "ч.л."])]
#[test_case("tbsp", &["tbsp", "tbsps", "tbs", "tablespoon", "EL", "cda", "cdas", "ст.л."])]
#[test_case("cup", &["cup", "cups", "Tasse", "taza", "tasses", "xícara", "xi\u{301}cara", "стакан"])]
#[test_case("oz", &["oz", "ounces", "Unze", "onzas", "once", "унция"])]
#[test_case("lb", &["lb", "lbs", "pounds", "Pfund", "libras", "livre", "фунт"])]
fn recognises_every_unit(stored: &str, words: &[&str]) {
    for word in words {
        assert_eq!(
            parsed(&format!("Thing 2 {word}")),
            expect("Thing", Some(2.0), Some(stored)),
            "{word}"
        );
    }
}

#[test_case("pc")]
#[test_case("Stk")]
#[test_case("unidad")]
#[test_case("pièce")]
#[test_case("unidade")]
#[test_case("штук")]
fn a_piece_word_makes_a_count(word: &str) {
    assert_eq!(
        parsed(&format!("Thing 2 {word}")),
        expect("Thing", Some(2.0), None)
    );
}

#[test]
fn a_number_too_big_for_f64_is_not_an_amount() {
    let line = format!("Flour {} l", "9".repeat(400));
    assert_eq!(parse_ingredient(&line, None, None).amount, None);
}
