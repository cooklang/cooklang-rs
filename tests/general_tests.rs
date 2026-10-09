use cooklang::analysis::{CheckResult, RecipeRefTarget};
use cooklang::{
    Content, Converter, CooklangParser, Extensions, Item, ParseOptions, RecipeReference, Value,
};
use indoc::indoc;
use std::cell::RefCell;
use std::sync::LazyLock;
use test_case::test_case;

static PARSER: LazyLock<CooklangParser> =
    LazyLock::new(|| CooklangParser::new(Extensions::all(), Converter::default()));

#[test_case(
    indoc! {r#"
        first

        second
    "#} => vec![vec![Some(1), Some(2)]]; "basic"
)]
#[test_case(
    indoc! {r#"
        > text

        first

        second
    "#} => vec![vec![None,  Some(1), Some(2)]]; "text start"
)]
#[test_case(
    indoc! {r#"
        first

        > text

        second
    "#} => vec![vec![Some(1), None, Some(2)]]; "text middle"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        first again
    "#} => vec![vec![Some(1), Some(2)], vec![Some(1)]]; "section reset"
)]
#[test_case(
    indoc! {r#"
        > text

        first

        second
        == sect ==
        first again
    "#} => vec![vec![None, Some(1), Some(2)], vec![Some(1)]]; "complex 1"
)]
#[test_case(
    indoc! {r#"
        first

        > text

        second
        == sect ==
        first again
    "#} => vec![vec![Some(1), None, Some(2)], vec![Some(1)]]; "complex 2"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        > text

        first again
    "#} => vec![vec![Some(1), Some(2)], vec![None, Some(1)]]; "complex 3"
)]
#[test_case(
    indoc! {r#"
        first

        second
        == sect ==
        first again

        > text
    "#} => vec![vec![Some(1), Some(2)], vec![Some(1), None]]; "complex 4"
)]
#[test_case(
    indoc! {r#"
        > just text

        == sect ==

        > text

        first again
    "#} => vec![vec![None], vec![None, Some(1)]]; "complex 5"
)]
fn step_number(src: &str) -> Vec<Vec<Option<u32>>> {
    let r = PARSER.parse(src).unwrap_output();
    let numbers: Vec<Vec<Option<u32>>> = r
        .sections
        .into_iter()
        .map(|sect| {
            sect.content
                .into_iter()
                .map(|c| match c {
                    Content::Step(s) => Some(s.number),
                    Content::Text(_) => None,
                })
                .collect()
        })
        .collect();
    numbers
}

#[test]
fn empty_not_empty() {
    let input = indoc! {r#"
        -- "empty" recipe

           -- with spaces

        -- that should actually be empty 
            -- and not produce empty steps   
        
        [- not even this -]
    "#};

    // should be the same with multiline and without
    let r = PARSER.parse(input).unwrap_output();
    assert!(r.sections.is_empty());
    let r = PARSER.parse(input).unwrap_output();
    assert!(r.sections.is_empty());
}

#[test]
fn empty_steps() {
    let input = indoc! {r#"
        == Section name to force the section ==

        -- "empty" recipe

           -- with spaces

        -- that should actually be empty 
            -- and not produce empty steps   
        
        [- not even this -]
    "#};

    // should be the same with multiline and without
    let r = PARSER.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());
    let r = PARSER.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());
}

#[test]
fn whitespace_line_block_separator() {
    let input = indoc! {r#"
        a step
                 
        another
    "#};

    // should be the same with multiline and without
    let r = PARSER.parse(input).unwrap_output();
    assert_eq!(r.sections[0].content.len(), 2);
}

#[test]
fn single_line_no_separator() {
    let input = indoc! {r#"
        a step
        >> meta: val
        another step
        = section
    "#};
    let r = PARSER.parse(input).unwrap_output();
    assert_eq!(r.sections.len(), 2);
    assert_eq!(r.sections[0].content.len(), 2);
    assert_eq!(r.sections[1].content.len(), 0);
    assert_eq!(r.metadata.map.len(), 1);
}

#[test]
fn multiple_temperatures() {
    let input = "text 2ºC more text 150 F end text";
    let r = PARSER.parse(input).unwrap_output();
    assert_eq!(r.inline_quantities.len(), 2);
    assert_eq!(r.inline_quantities[0].value(), &Value::from(2.0));
    assert_eq!(r.inline_quantities[0].unit(), Some("ºC"));
    assert_eq!(r.inline_quantities[1].value(), &Value::from(150.0));
    assert_eq!(r.inline_quantities[1].unit(), Some("F"));
    let Content::Step(first_step) = &r.sections[0].content[0] else {
        panic!()
    };
    assert_eq!(
        first_step.items,
        vec![
            Item::Text {
                value: "text ".into()
            },
            Item::InlineQuantity { index: 0 },
            Item::Text {
                value: " more text ".into()
            },
            Item::InlineQuantity { index: 1 },
            Item::Text {
                value: " end text".into()
            }
        ]
    );
}

#[test]
fn no_steps_component_mode() {
    let input = indoc! {r#"
        >> [mode]: components
        @igr
        >> [mode]: steps
        = section
        step
    "#};
    let r = cooklang::parse(input).unwrap_output();
    assert_eq!(r.sections.len(), 1);
    assert_eq!(r.sections[0].name.as_deref(), Some("section"));
    assert!(matches!(
        r.sections[0].content.as_slice(),
        [Content::Step(_)]
    ));
}

#[test]
fn text_steps_extension() {
    let input = "> text";

    let r = CooklangParser::canonical().parse(input).unwrap_output();
    assert!(matches!(
        r.sections[0].content.as_slice(),
        [Content::Text(_)]
    ));
}

#[test]
fn timer_missing_unit_warning() {
    let input = "Cook for ~{30}";

    let parser = CooklangParser::canonical();
    let result = parser.parse(input);

    // Check that we get a warning, not an error
    let report = result.report();

    // Verify we have exactly one warning
    assert_eq!(report.iter().count(), 1);

    // Check that it's a warning, not an error
    let diag = report.iter().next().unwrap();
    assert_eq!(diag.severity, cooklang::error::Severity::Warning);

    // Verify the warning message
    assert!(diag
        .message
        .contains("Invalid timer quantity: missing unit"));

    // Should parse successfully (not error)
    let _r = result.unwrap_output();
}

/// Collects what [`ParseOptions::recipe_ref_check`] is handed for `input`,
/// rendered by `describe`.
fn checked_refs<F>(input: &str, path: Option<&str>, describe: F) -> Vec<String>
where
    F: Fn(RecipeRefTarget) -> String,
{
    let seen = RefCell::new(Vec::new());

    let options = ParseOptions {
        recipe_ref_check: Some(Box::new(|target: RecipeRefTarget| {
            seen.borrow_mut().push(describe(target));
            CheckResult::Ok
        })),
        ..Default::default()
    };

    let path = path.map(|s| s.parse::<RecipeReference>().unwrap());
    let _ = PARSER.parse_with_options(input, options, path);
    seen.into_inner()
}

fn tagged(target: RecipeRefTarget) -> String {
    match target {
        RecipeRefTarget::Resolved(reference) => format!("resolved {reference}"),
        RecipeRefTarget::Unresolved(reference) => format!("unresolved {reference}"),
    }
}

#[test]
fn what_the_recipe_ref_check_is_handed() {
    let handed = |input: &str, path: Option<&str>| checked_refs(input, path, tagged).join(", ");

    // resolved against the recipe's path
    assert_eq!(
        handed("@@../spices/x.cook{}", Some("./pickles/sour.cook")),
        "resolved ./spices/x.cook"
    );
    assert_eq!(
        handed("@@./spices/x.cook{}", Some("./root.cook")),
        "resolved ./spices/x.cook"
    );
    // told when there is no path to resolve against
    assert_eq!(
        handed("@@../spices/x.cook{}", None),
        "unresolved ../spices/x.cook"
    );
    // backslash separators read as a path
    assert_eq!(
        handed(r#"@@..\\spices\\x.cook{}"#, Some("./pickles/sour.cook")),
        "resolved ./spices/x.cook"
    );
    assert_eq!(
        handed(r#"@@.\\spices\\x.cook{}"#, None),
        "unresolved ./spices/x.cook"
    );
    // not called for a bare name
    assert_eq!(handed("@@plain{}", Some("./root.cook")), "");
    assert_eq!(handed("@@plain{}", None), "");
}

#[test]
fn every_recipe_ref_target_carries_a_name() {
    let name = |target: RecipeRefTarget| target.name().to_string();

    assert_eq!(
        checked_refs("@@../spices/x.cook{}", Some("./pickles/sour.cook"), name),
        ["x.cook"]
    );
    assert_eq!(checked_refs("@@../spices/x.cook{}", None, name), ["x.cook"]);
}

#[test]
fn a_backslash_that_separates_no_path_stays_in_the_name() {
    let recipe = PARSER.parse(r#"@@my\\recipe{}"#).unwrap_output();

    assert_eq!(recipe.ingredients[0].name, r#"my\recipe"#);
    assert!(recipe.ingredients[0].reference.is_none());
}
