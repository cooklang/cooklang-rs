use cooklang::{Converter, CooklangParser, Extensions};

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
        Some(4)
    );

    let orig_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(orig_servings_value.as_u64(), Some(4));

    // Scale to 8 servings (2x)
    recipe.scale_to_servings(8, &Converter::default()).unwrap();

    // Check that servings in metadata were updated
    let scaled_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(scaled_servings_value.as_u64(), Some(8));
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
        serde_yaml::Value::Number(n) => assert_eq!(n.as_u64(), Some(6)),
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

    // Scale by factor that results in fractional servings (3 * 1.5 = 4.5, should round to 5)
    recipe.scale(1.5, &Converter::default());

    let scaled_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    // Handle both string and number formats
    match scaled_servings_value {
        serde_yaml::Value::String(s) => assert_eq!(s, "5"),
        serde_yaml::Value::Number(n) => assert_eq!(n.as_u64(), Some(5)),
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
    let result = recipe.scale_to_servings(8, &Converter::default());
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
    assert_eq!(servings.as_ref().and_then(|s| s.as_number()), Some(4));

    // scale_to_servings should succeed
    let result = recipe.scale_to_servings(8, &Converter::default());
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
    let result = recipe.scale_to_servings(8, &Converter::default());

    // Check that it returns an error
    assert!(result.is_err());
    match result.unwrap_err() {
        cooklang::scale::ScaleError::InvalidServings => {
            // Expected error
        }
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
