//! Range quantities (`{2-3%cloves}`), spec proposal 0011.

use cooklang::{
    ingredient_list::IngredientList, quantity::Value, Converter, CooklangParser, Extensions,
    Quantity, Recipe,
};

/// Parses with no extensions and no units, as the canonical parser does, and
/// checks there are no errors or warnings.
fn parse(input: &str) -> Recipe {
    let parser = CooklangParser::new(Extensions::empty(), Converter::empty());
    let (recipe, report) = parser.parse(input).into_result().expect("parse failed");
    assert!(report.is_empty(), "unexpected diagnostics: {report:?}");
    recipe
}

fn ends(q: Option<&Quantity>) -> (f64, f64, Option<&str>) {
    let q = q.expect("no quantity");
    let Value::Range { start, end } = q.value() else {
        panic!("not a range: {:?}", q.value());
    };
    (start.value(), end.value(), q.unit())
}

const RECIPE: &str = "Crush @garlic{2-3%cloves}, season with @salt{1-2%tsp}, \
                      fill #ramekins{4-6} and bake for ~{25–30%minutes}.";

#[test]
fn ranges_on_ingredients_cookware_and_timers() {
    let r = parse(RECIPE);
    assert_eq!(
        ends(r.ingredients[0].quantity.as_ref()),
        (2.0, 3.0, Some("cloves"))
    );
    assert_eq!(
        ends(r.ingredients[1].quantity.as_ref()),
        (1.0, 2.0, Some("tsp"))
    );
    assert_eq!(ends(r.cookware[0].quantity.as_ref()), (4.0, 6.0, None));
    assert_eq!(
        ends(r.timers[0].quantity.as_ref()),
        (25.0, 30.0, Some("minutes"))
    );
}

#[test]
fn scaling_multiplies_both_ends() {
    let mut r = parse(RECIPE);
    r.scale(2.0, &Converter::empty());
    assert_eq!(
        ends(r.ingredients[0].quantity.as_ref()),
        (4.0, 6.0, Some("cloves"))
    );
    assert_eq!(
        ends(r.ingredients[1].quantity.as_ref()),
        (2.0, 4.0, Some("tsp"))
    );
    // cookware and timers don't scale
    assert_eq!(ends(r.cookware[0].quantity.as_ref()), (4.0, 6.0, None));
    assert_eq!(
        ends(r.timers[0].quantity.as_ref()),
        (25.0, 30.0, Some("minutes"))
    );
}

#[test]
fn a_scaling_lock_locks_the_whole_range() {
    let mut r = parse("@salt{=1-2%tsp}");
    r.scale(2.0, &Converter::empty());
    assert_eq!(
        ends(r.ingredients[0].quantity.as_ref()),
        (1.0, 2.0, Some("tsp"))
    );
}

#[test]
fn ranges_add_end_to_end_and_numbers_add_to_both_ends() {
    let r =
        parse("@garlic{2} @garlic{1-2} @water{100-150%ml} @water{50-100%ml} @eggs{1-2} @eggs{3}");
    let mut list = IngredientList::new();
    list.add_recipe(&r, &Converter::empty(), false);
    let total = |name: &str| {
        let (_, q) = list.iter().find(|(n, _)| *n == name).unwrap();
        let q: Vec<_> = q.iter().collect();
        assert_eq!(q.len(), 1, "{name}: {q:?}");
        ends(Some(q[0]))
    };
    assert_eq!(total("garlic"), (3.0, 4.0, None));
    assert_eq!(total("water"), (150.0, 250.0, Some("ml")));
    assert_eq!(total("eggs"), (4.0, 5.0, None));
}

#[test]
fn range_like_text_is_still_text() {
    for (input, text) in [
        ("@eggs{2-}", "2-"),
        ("@eggs{1-2-3}", "1-2-3"),
        ("@water{1-2 handfuls}", "1-2 handfuls"),
        ("@eggs{2\u{2212}3}", "2\u{2212}3"),
    ] {
        let r = parse(input);
        assert_eq!(
            r.ingredients[0].quantity.as_ref().unwrap().value(),
            &Value::Text(text.into()),
            "{input}"
        );
    }
}

#[test]
fn extensions_do_not_change_a_range() {
    let parser = CooklangParser::new(Extensions::all(), Converter::empty());
    let r = parser
        .parse("@garlic{2–3%cloves} @flour{1-2 cups}")
        .unwrap_output();
    assert_eq!(
        ends(r.ingredients[0].quantity.as_ref()),
        (2.0, 3.0, Some("cloves"))
    );
    // a unit without `%` is still the ADVANCED_UNITS extension
    assert_eq!(
        ends(r.ingredients[1].quantity.as_ref()),
        (1.0, 2.0, Some("cups"))
    );
}
