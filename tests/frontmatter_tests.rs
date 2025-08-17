use cooklang;
use indoc::indoc;

#[test]
fn test_invalid_yaml_frontmatter_becomes_warning() {
    let input = indoc! {r#"
        ---
        title: Test Recipe
        tags: [test
        invalid yaml here
        ---
        
        This is a test recipe with invalid YAML frontmatter.
        
        @eggs{2} and @butter{1%tbsp}
    "#};

    let result = cooklang::parse(input);
    
    // Should not fail to parse
    assert!(result.output().is_some(), "Recipe should parse successfully despite invalid YAML");
    
    let recipe = result.output().unwrap();
    let warnings = result.report();
    
    // Should have warnings about invalid YAML
    assert!(!warnings.is_empty(), "Should have warnings about invalid YAML");
    
    // The frontmatter should be ignored, so metadata should be empty
    assert!(recipe.metadata.map.is_empty(), "Metadata should be empty when YAML is invalid");
    
    // Should still parse the recipe content
    assert_eq!(recipe.ingredients.len(), 2, "Should still parse ingredients");
    assert_eq!(recipe.ingredients[0].name, "eggs");
    assert_eq!(recipe.ingredients[1].name, "butter");
}

#[test]
fn test_valid_yaml_frontmatter_still_works() {
    let input = indoc! {r#"
        ---
        title: Test Recipe
        tags: [test, recipe]
        prep_time: 10 min
        ---
        
        This is a test recipe with valid YAML frontmatter.
        
        @eggs{2} and @butter{1%tbsp}
    "#};

    let result = cooklang::parse(input);
    
    // Should parse successfully
    assert!(result.output().is_some(), "Recipe should parse successfully");
    
    let recipe = result.output().unwrap();
    
    // Metadata should be parsed
    assert!(!recipe.metadata.map.is_empty(), "Metadata should not be empty");
    assert_eq!(
        recipe.metadata.map.get("title").and_then(|v| v.as_str()),
        Some("Test Recipe"),
        "Title should be parsed correctly"
    );
    
    // Should still parse the recipe content
    assert_eq!(recipe.ingredients.len(), 2, "Should parse ingredients");
    assert_eq!(recipe.ingredients[0].name, "eggs");
    assert_eq!(recipe.ingredients[1].name, "butter");
}

#[test]
fn test_invalid_yaml_with_colon_in_value() {
    let input = indoc! {r#"
        ---
        title: Recipe: with colon
        description: This has: many: colons
        tags: [unclosed
        ---
        
        @flour{2%cups}
    "#};

    let result = cooklang::parse(input);
    
    // Should not fail to parse
    assert!(result.output().is_some(), "Recipe should parse successfully despite invalid YAML");
    
    let recipe = result.output().unwrap();
    let warnings = result.report();
    
    // Should have warnings
    assert!(!warnings.is_empty(), "Should have warnings about invalid YAML");
    
    // Metadata should be empty due to invalid YAML
    assert!(recipe.metadata.map.is_empty(), "Metadata should be empty when YAML is invalid");
    
    // Should still parse ingredients
    assert_eq!(recipe.ingredients.len(), 1);
    assert_eq!(recipe.ingredients[0].name, "flour");
}

#[test]
fn test_completely_malformed_yaml() {
    let input = indoc! {r#"
        ---
        { this is not valid yaml at all }}}
        : : : :
        ---
        
        Simple recipe with @salt and @pepper
    "#};

    let result = cooklang::parse(input);
    
    // Should not fail to parse
    assert!(result.output().is_some(), "Recipe should parse successfully despite malformed YAML");
    
    let recipe = result.output().unwrap();
    let warnings = result.report();
    
    // Should have warnings
    assert!(!warnings.is_empty(), "Should have warnings about invalid YAML");
    
    // Should contain information about invalid YAML in warning
    let warning_str = format!("{:?}", warnings);
    assert!(
        warning_str.contains("Invalid YAML") || warning_str.contains("invalid YAML"),
        "Warning should mention invalid YAML"
    );
    
    // Metadata should be empty
    assert!(recipe.metadata.map.is_empty(), "Metadata should be empty when YAML is invalid");
    
    // Should still parse the recipe
    assert_eq!(recipe.ingredients.len(), 2);
    assert_eq!(recipe.ingredients[0].name, "salt");
    assert_eq!(recipe.ingredients[1].name, "pepper");
}
