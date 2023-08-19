use cooklang::{CooklangParser, Extensions};
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
    let parser = CooklangParser::new(
        Extensions::all() ^ Extensions::MULTILINE_STEPS,
        Default::default(),
    );
    let r = parser.parse(src, "test").take_output().unwrap();
    let numbers: Vec<Vec<Option<u32>>> = r
        .sections
        .into_iter()
        .map(|sect| sect.steps.into_iter().map(|stp| stp.number).collect())
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
    let r = parser.parse(input, "test").take_output().unwrap();
    assert!(r.sections.is_empty());

    let parser = CooklangParser::new(
        Extensions::all() ^ Extensions::MULTILINE_STEPS,
        Default::default(),
    );
    let r = parser.parse(input, "test").take_output().unwrap();
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
    let r = parser.parse(input, "test").take_output().unwrap();
    assert!(r.sections[0].steps.is_empty());

    let parser = CooklangParser::new(
        Extensions::all() ^ Extensions::MULTILINE_STEPS,
        Default::default(),
    );
    let r = parser.parse(input, "test").take_output().unwrap();
    assert!(r.sections[0].steps.is_empty());
}

#[test]
fn whitespace_line_block_separator() {
    let input = indoc! {r#"
        a step
                 
        another
    "#};

    // should be the same with multiline and without
    let parser = CooklangParser::new(Extensions::all(), Default::default());
    let r = parser.parse(input, "test").take_output().unwrap();
    assert_eq!(r.sections[0].steps.len(), 2);
}
