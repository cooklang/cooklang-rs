//! Optional ingredients and cookware (`@?name`, `#?name`), spec proposal 0018.

use cooklang::{Converter, CooklangParser, Extensions, Modifiers};
use indoc::indoc;
use test_case::test_case;

fn names_and_optional(extensions: Extensions, input: &str) -> Vec<(String, bool)> {
    let parser = CooklangParser::new(extensions, Converter::empty());
    let r = parser.parse(input).unwrap_output();
    r.ingredients
        .iter()
        .map(|i| (i.name.clone(), i.modifiers().is_optional()))
        .collect()
}

#[test_case(Extensions::empty(); "no extensions")]
#[test_case(Extensions::all(); "all extensions")]
fn optional_marker_is_core_syntax(extensions: Extensions) {
    let got = names_and_optional(
        extensions,
        "Season with @salt and @?chilli flakes{1%pinch}, top with @?chives.",
    );
    assert_eq!(
        got,
        vec![
            ("salt".to_string(), false),
            ("chilli flakes".to_string(), true),
            ("chives".to_string(), true),
        ]
    );
}

#[test_case(Extensions::empty(); "no extensions")]
#[test_case(Extensions::all(); "all extensions")]
fn optional_cookware(extensions: Extensions) {
    let parser = CooklangParser::new(extensions, Converter::empty());
    let r = parser
        .parse("Use a #?splatter guard{} and a #pan.")
        .unwrap_output();
    let got: Vec<_> = r
        .cookware
        .iter()
        .map(|c| (c.name.as_str(), c.modifiers().is_optional()))
        .collect();
    assert_eq!(got, vec![("splatter guard", true), ("pan", false)]);
}

#[test]
fn optional_recipe_reference() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::empty());
    let r = parser
        .parse("Serve with @?./sauces/chimichurri{}.")
        .unwrap_output();
    let igr = &r.ingredients[0];
    assert!(igr.modifiers().is_optional());
    assert_eq!(igr.name, "chimichurri");
    let reference = igr.reference.as_ref().expect("recipe reference");
    assert_eq!(reference.name(), "chimichurri");
    assert_eq!(reference.to_string(), "./sauces/chimichurri");
}

#[test]
fn only_question_mark_without_extension() {
    // Other modifiers stay behind the extension flag
    let got = names_and_optional(Extensions::empty(), "Add @-salt{} and @?+pepper{}");
    assert_eq!(
        got,
        vec![("-salt".to_string(), false), ("+pepper".to_string(), true)]
    );
}

#[test]
fn only_one_optional_marker_with_extension() {
    let parser = CooklangParser::new(Extensions::all(), Converter::empty());
    let r = parser.parse("Add @??thyme{}").unwrap_output();
    assert_eq!(r.ingredients[0].name, "?thyme");
    assert_eq!(r.ingredients[0].modifiers(), Modifiers::OPT);
}

#[test]
fn timers_are_unaffected() {
    let parser = CooklangParser::new(Extensions::empty(), Converter::empty());
    let r = parser.parse("Wait ~?{5%minutes}").unwrap_output();
    assert_eq!(r.timers.len(), 1);
    assert_eq!(r.timers[0].name.as_deref(), Some("?"));
}

#[test]
fn optional_and_required_are_listed_separately() {
    let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    let r = parser
        .parse(indoc! {r#"
            Stir @parmesan{100%g} into the sauce.

            Top with extra @?parmesan{50%g} before serving.
        "#})
        .unwrap_output();
    let grouped = r.group_ingredients(parser.converter());
    let got: Vec<_> = grouped
        .iter()
        .map(|g| {
            (
                g.ingredient.name.as_str(),
                g.ingredient.modifiers().is_optional(),
                g.quantity.to_string(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("parmesan", false, "100 g".to_string()),
            ("parmesan", true, "50 g".to_string()),
        ]
    );
}

#[test]
fn duplicate_reference_mode_groups_by_optionality() {
    let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    let r = parser
        .parse(indoc! {r#"
            >> [duplicate]: ref
            Stir @parmesan{100%g} into the sauce, then @parmesan{20%g}.

            Top with @?parmesan{50%g}, then @?parmesan{10%g}.
        "#})
        .into_result()
        .expect("mixing optional and required occurrences is not an error");
    let (recipe, warnings) = r;
    assert!(!warnings.has_warnings(), "{warnings:?}");
    let grouped = recipe.group_ingredients(parser.converter());
    let got: Vec<_> = grouped
        .iter()
        .map(|g| {
            (
                g.ingredient.modifiers().is_optional(),
                g.quantity.to_string(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![(false, "120 g".to_string()), (true, "60 g".to_string())]
    );
}

#[test]
fn explicit_reference_inherits_optional() {
    let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
    let r = parser
        .parse("Top with @?parmesan{50%g}, then @&parmesan{10%g}.")
        .unwrap_output();
    assert!(r.ingredients.iter().all(|i| i.modifiers().is_optional()));
    let grouped = r.group_ingredients(parser.converter());
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0].quantity.to_string(), "60 g");
}
