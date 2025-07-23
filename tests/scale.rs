use cooklang::{Converter, CooklangParser, Extensions};

#[test]
fn test_scale_updates_servings_metadata() {
    let input = r#">> servings: 4

@flour{200%g}
@eggs{2}
Mix and bake."#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let recipe = parser.parse(input).unwrap_output();

    // Check original servings
    assert_eq!(recipe.servings(), Some(&[4][..]));
    let orig_servings_value = recipe
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(orig_servings_value.as_str(), Some("4"));

    // Scale to 8 servings (2x)
    let scaled = recipe.scale_to_servings(8, &Converter::default());

    // Check that servings in metadata were updated
    let scaled_servings_value = scaled
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
    let recipe = parser.parse(input).unwrap_output();

    // Scale by factor of 3
    let scaled = recipe.scale(3.0, &Converter::default());

    // Check that servings in metadata were updated (2 * 3 = 6)
    let scaled_servings_value = scaled
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(scaled_servings_value.as_u64(), Some(6));
}

#[test]
fn test_scale_without_servings_metadata() {
    // Recipe without servings metadata
    let input = r#"@flour{200%g}
@eggs{2}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let recipe = parser.parse(input).unwrap_output();

    // Should not have servings
    assert_eq!(recipe.servings(), None);

    // Scale by factor of 2
    let scaled = recipe.scale(2.0, &Converter::default());

    // Should still not have servings in metadata
    assert!(scaled
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .is_none());
}

#[test]
fn test_scale_with_fractional_servings() {
    let input = r#">> servings: 3

@milk{300%ml}"#;

    let parser = CooklangParser::new(Extensions::all(), Converter::default());
    let recipe = parser.parse(input).unwrap_output();

    // Scale by factor that results in fractional servings (3 * 1.5 = 4.5, should round to 5)
    let scaled = recipe.scale(1.5, &Converter::default());

    let scaled_servings_value = scaled
        .metadata
        .get(cooklang::metadata::StdKey::Servings)
        .unwrap();
    assert_eq!(scaled_servings_value.as_u64(), Some(5));
}
