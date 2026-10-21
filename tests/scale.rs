use cooklang::{Converter, CooklangParser, Extensions, Recipe};
use std::sync::LazyLock;

static PARSER: LazyLock<CooklangParser> =
    LazyLock::new(|| CooklangParser::new(Extensions::all(), Converter::default()));
static CONVERTER: LazyLock<Converter> = LazyLock::new(Converter::default);

#[track_caller]
fn parse(src: &str) -> Recipe {
    PARSER.parse(src).unwrap_output()
}

fn servings(recipe: &Recipe) -> Option<f64> {
    recipe.metadata.servings().and_then(|s| s.as_number())
}

fn recipe_yield(recipe: &Recipe) -> Option<String> {
    recipe.metadata.yield_quantity().map(|q| q.to_string())
}

fn first_amount(recipe: &Recipe) -> Option<String> {
    recipe.ingredients[0]
        .quantity
        .as_ref()
        .map(|q| q.to_string())
}

#[test]
fn test_scale_updates_servings_metadata() {
    let input = r#"---
servings: 4
---

@flour{200%g}
@eggs{2}
Mix and bake."#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Check original servings
    assert_eq!(
        recipe.metadata.servings().and_then(|s| s.as_number()),
        Some(4.0)
    );

    let orig_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(orig_servings_value.as_u64(), Some(4));

    // Scale to 8 servings (2x)
    recipe
        .scale_to_servings(8.0, &Converter::default())
        .unwrap();

    // Check that servings in metadata were updated
    let scaled_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(scaled_servings_value.as_f64(), Some(8.0));
}

#[test]
fn test_scale_by_factor_updates_servings_metadata() {
    let input = r#">> servings: 2

@butter{100%g}
@sugar{50%g}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Scale by factor of 3
    recipe.scale(3.0, &Converter::default());

    // Check that servings in metadata were updated (2 * 3 = 6)
    let scaled_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    // Handle both string and number formats
    match scaled_servings_value {
        serde_yaml::Value::String(s) => assert_eq!(s, "6"),
        serde_yaml::Value::Number(n) => assert_eq!(n.as_f64(), Some(6.0)),
        _ => panic!("Unexpected servings value type"),
    }
}

#[test]
fn test_scale_without_servings_metadata() {
    // Recipe without servings metadata
    let input = r#"@flour{200%g}
@eggs{2}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Should not have servings
    assert_eq!(recipe.metadata.servings(), None);

    // Scale by factor of 2
    recipe.scale(2.0, &Converter::default());

    // Should still not have servings in metadata
    assert!(recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .is_none());
}

#[test]
fn test_scale_with_fractional_servings() {
    let input = r#">> servings: 3

@milk{300%ml}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Scale by factor that results in fractional servings (3 * 1.5 = 4.5)
    recipe.scale(1.5, &Converter::default());

    let scaled_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    // Handle both string and number formats
    match scaled_servings_value {
        serde_yaml::Value::String(s) => assert_eq!(s, "4.5"),
        serde_yaml::Value::Number(n) => assert_eq!(n.as_f64(), Some(4.5)),
        _ => panic!("Unexpected servings value type"),
    }
}

#[test]
fn test_scale_with_non_numeric_servings() {
    let input = r#">> servings: two

@flour{200%g}
@butter{100%g}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Should parse "two" as text servings
    let servings = recipe.metadata.servings();
    assert!(servings.is_some());
    assert_eq!(servings.as_ref().and_then(|s| s.as_number()), None);
    assert_eq!(servings.as_ref().and_then(|s| s.as_text()), Some("two"));

    // Scale by factor of 2
    recipe.scale(2.0, &Converter::default());

    // Servings should remain unchanged as a string
    let servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(servings_value.as_str(), Some("two"));
}

#[test]
fn test_scale_to_servings_with_parseable_string_servings() {
    let input = r#"---
servings: "serves 4 people"
---

@rice{2%cups}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Should parse "serves 4 people" as Servings::Text("serves 4 people") since we don't do complex parsing
    let servings = recipe.metadata.servings();
    assert!(servings.is_some());
    assert_eq!(servings.as_ref().and_then(|s| s.as_number()), None);
    assert_eq!(
        servings.as_ref().and_then(|s| s.as_text()),
        Some("serves 4 people")
    );

    // scale_to_servings should fail since "serves 4 people" is not parsed as a number
    let result = recipe.scale_to_servings(8.0, &Converter::default());
    assert!(result.is_err());

    // Recipe should remain unchanged
    let ingredient_quantity = &recipe.ingredients[0].quantity.as_ref().unwrap();
    match ingredient_quantity.value() {
        cooklang::quantity::Value::Number(n) => {
            assert_eq!(n.value(), 2.0); // Original value unchanged
        }
        _ => panic!("Expected numeric value"),
    }
}

#[test]
fn test_scale_to_servings_with_numeric_string() {
    let input = r#"---
servings: "4"
---

@rice{2%cups}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Should parse "4" as Servings::Number(4)
    let servings = recipe.metadata.servings();
    assert!(servings.is_some());
    assert_eq!(servings.as_ref().and_then(|s| s.as_number()), Some(4.0));

    // scale_to_servings should succeed
    let result = recipe.scale_to_servings(8.0, &Converter::default());
    assert!(result.is_ok());

    // Recipe should be scaled from 4 to 8 (factor of 2)
    let ingredient_quantity = &recipe.ingredients[0].quantity.as_ref().unwrap();
    match ingredient_quantity.value() {
        cooklang::quantity::Value::Number(n) => {
            assert_eq!(n.value(), 4.0); // Original 2 * 2
        }
        _ => panic!("Expected numeric value"),
    }
}

#[test]
fn test_scale_to_servings_with_non_numeric_servings() {
    let input = r#"---
servings: "varies"
---

@rice{2%cups}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let mut recipe = parser.parse(input).unwrap_output();

    // Should parse "varies" as Servings::Text("varies")
    let servings = recipe.metadata.servings();
    assert!(servings.is_some());
    assert_eq!(servings.as_ref().and_then(|s| s.as_number()), None);
    assert_eq!(servings.as_ref().and_then(|s| s.as_text()), Some("varies"));

    // scale_to_servings should fail when servings can't be parsed to number
    let result = recipe.scale_to_servings(8.0, &Converter::default());

    // Check that it returns an error
    assert!(result.is_err());
    match result.unwrap_err() {
        cooklang::scale::ScaleError::InvalidServings => {
            // Expected error
        }
        _ => panic!("Expected InvalidServings error, got a different error"),
    }

    // Recipe should remain unchanged
    let ingredient_quantity = &recipe.ingredients[0].quantity.as_ref().unwrap();
    match ingredient_quantity.value() {
        cooklang::quantity::Value::Number(n) => {
            assert_eq!(n.value(), 2.0); // Original value
        }
        _ => panic!("Expected numeric value"),
    }
}

#[test]
fn scaling_updates_servings_written_as_an_alias() {
    let mut recipe = parse(">> serves: 4\n@flour{100%g}");

    recipe.scale(2.0, &CONVERTER);

    // the reader resolves the alias, so the writer has to update the same entry
    assert_eq!(servings(&recipe), Some(8.0));
    assert_eq!(first_amount(&recipe).as_deref(), Some("200 g"));
}

#[test]
fn scaling_updates_yield_written_as_an_alias() {
    let mut recipe = parse(">> yields: 500%g\n@flour{100%g}");

    recipe
        .scale_to_yield(1000.0, "g", &CONVERTER)
        .expect("the recipe has a yield");

    assert_eq!(recipe_yield(&recipe).as_deref(), Some("1000 g"));
}

#[test]
fn scaling_by_a_factor_scales_yield_too() {
    let mut recipe = parse(">> yield: 500%g\n@flour{100%g}");

    recipe.scale(2.0, &CONVERTER);

    // the yield is an amount the recipe makes, so it cannot contradict the
    // ingredients it was scaled with
    assert_eq!(recipe_yield(&recipe).as_deref(), Some("1 kg"));
    assert_eq!(first_amount(&recipe).as_deref(), Some("200 g"));
}

#[track_caller]
fn refuses_to_scale_servings(value: &str) {
    let mut recipe = parse(&format!("---\nservings: {value}\n---\n@flour{{100%g}}"));

    assert!(recipe.scale_to_servings(8.0, &CONVERTER).is_err());
    assert_eq!(first_amount(&recipe).as_deref(), Some("100 g"));
    assert_eq!(servings(&recipe), None);
}

#[test]
fn scaling_refuses_servings_that_are_not_a_count() {
    refuses_to_scale_servings("-4");
    refuses_to_scale_servings("0");
    refuses_to_scale_servings(".nan");
    refuses_to_scale_servings(".inf");
}

#[track_caller]
fn refuses_servings_target(target: f64) {
    let mut recipe = parse("---\nservings: 4\n---\n@flour{100%g}");

    assert!(recipe.scale_to_servings(target, &CONVERTER).is_err());
    assert_eq!(first_amount(&recipe).as_deref(), Some("100 g"));
    assert_eq!(servings(&recipe), Some(4.0));
}

#[test]
fn scaling_refuses_a_target_that_is_not_a_count() {
    refuses_servings_target(0.0);
    refuses_servings_target(-8.0);
    refuses_servings_target(f64::NAN);
    refuses_servings_target(f64::INFINITY);
}

#[track_caller]
fn refuses_to_scale_yield(value: &str) {
    let mut recipe = parse(&format!("---\nyield: {value}\n---\n@flour{{100%g}}"));

    assert!(recipe.scale_to_yield(1000.0, "g", &CONVERTER).is_err());
    assert_eq!(first_amount(&recipe).as_deref(), Some("100 g"));
    assert_eq!(recipe_yield(&recipe), None);
}

#[test]
fn scaling_refuses_a_yield_that_cannot_be_made() {
    refuses_to_scale_yield("-3%g");
    refuses_to_scale_yield("0");
    refuses_to_scale_yield(".nan");
}

#[track_caller]
fn refuses_yield_target(target: f64) {
    let mut recipe = parse("---\nyield: 500%g\n---\n@flour{100%g}");

    assert!(recipe.scale_to_yield(target, "g", &CONVERTER).is_err());
    assert_eq!(first_amount(&recipe).as_deref(), Some("100 g"));
    assert_eq!(recipe_yield(&recipe).as_deref(), Some("500 g"));
}

#[test]
fn scaling_refuses_a_yield_target_that_cannot_be_made() {
    refuses_yield_target(0.0);
    refuses_yield_target(-1000.0);
    refuses_yield_target(f64::NAN);
    refuses_yield_target(f64::INFINITY);
}

/// The yield and the first amount after scaling a recipe with `yield_` to
/// `target` `unit`
#[track_caller]
fn scaled_to_yield(yield_: &str, target: f64, unit: &str) -> (String, String) {
    let mut recipe = parse(&format!("---\nyield: {yield_}\n---\n@flour{{100%g}}"));
    recipe.scale_to_yield(target, unit, &CONVERTER).unwrap();
    (
        recipe_yield(&recipe).unwrap(),
        first_amount(&recipe).unwrap(),
    )
}

#[track_caller]
fn yield_unit_mismatch(yield_: &str, unit: &str) {
    let mut recipe = parse(&format!("---\nyield: {yield_}\n---\n@flour{{100%g}}"));

    assert!(matches!(
        recipe.scale_to_yield(1.0, unit, &CONVERTER),
        Err(cooklang::scale::ScaleError::UnitMismatch { .. })
    ));
    assert_eq!(first_amount(&recipe).as_deref(), Some("100 g"));
}

#[test]
fn scaling_to_a_yield_converts_the_target_into_the_yields_unit() {
    let s = |a: &str, b: &str| (a.to_string(), b.to_string());

    // the yield is written back as asked, not refitted
    assert_eq!(scaled_to_yield("500%g", 1000.0, "g"), s("1000 g", "200 g"));
    assert_eq!(scaled_to_yield("500%g", 2.0, "kg"), s("2 kg", "400 g"));
    assert_eq!(scaled_to_yield("2.5", 5.0, ""), s("5", "200 g"));

    // across systems: 1 lb is 453.59237 g
    let mut recipe = parse("---\nyield: 500%g\n---\n@flour{100%g}");
    recipe.scale_to_yield(1.0, "lb", &CONVERTER).unwrap();
    assert_eq!(recipe_yield(&recipe).as_deref(), Some("1 lb"));
    match recipe.ingredients[0].quantity.as_ref().unwrap().value() {
        cooklang::quantity::Value::Number(n) => assert!((n.value() - 90.718474).abs() < 1e-6),
        other => panic!("expected a number, got {other:?}"),
    }

    // a unit that cannot convert, or a unit on only one side, is a mismatch
    yield_unit_mismatch("500%g", "l");
    yield_unit_mismatch("500%g", "");
    yield_unit_mismatch("2.5", "g");
}

#[test]
fn scaling_twice_scales_the_ingredients_twice() {
    let mut recipe = parse("---\nservings: 1\nyield: 500%g\n---\n@flour{100%g}");

    recipe.scale(3.0, &CONVERTER);
    recipe.scale(0.5, &CONVERTER);

    assert_eq!(first_amount(&recipe).as_deref(), Some("150 g"));
    assert_eq!(servings(&recipe), Some(1.5));
    assert_eq!(recipe_yield(&recipe).as_deref(), Some("750 g"));
}

#[test]
fn scaling_to_servings_writes_the_count_rounded_as_scaling_does() {
    let mut recipe = parse("---\nservings: 4\n---\n@flour{100%g}");
    recipe.scale_to_servings(4.567, &CONVERTER).unwrap();

    assert_eq!(servings(&recipe), Some(4.6));
}
