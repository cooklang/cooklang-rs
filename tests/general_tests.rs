use cooklang::{Content, CooklangParser, Extensions, Item, Value};
use indoc::indoc;
use test_case::test_case;

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
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(src).unwrap_output();
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
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections.is_empty());

    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
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
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());

    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert!(r.sections[0].content.is_empty());
}

#[test]
fn whitespace_line_block_separator() {
    let input = indoc! {r#"
        a step
                 
        another
    "#};

    // should be the same with multiline and without
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
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
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
    assert_eq!(r.sections.len(), 2);
    assert_eq!(r.sections[0].content.len(), 2);
    assert_eq!(r.sections[1].content.len(), 0);
    assert_eq!(r.metadata.map.len(), 1);
}

#[test]
fn multiple_temperatures() {
    let input = "text 2ºC more text 150 F end text";
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input).unwrap_output();
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
