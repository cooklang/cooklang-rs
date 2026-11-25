//! Spec 2.0 draft extensions: products (0027), step actions (0029),
//! alternatives and labels (0025), nameless quantities (0030), text timers.

use cooklang::{Converter, CooklangParser, Extensions, Item};

fn parser() -> CooklangParser {
    CooklangParser::new(Extensions::all(), Converter::bundled())
}

#[test]
fn nameless_quantity_parses_and_is_hidden() {
    let r = parser().parse("Cut into @{9} squares.").into_result().unwrap().0;
    assert_eq!(r.ingredients.len(), 1);
    assert_eq!(r.ingredients[0].name, "");
    assert!(!r.ingredients[0].modifiers().should_be_listed());
}

#[test]
fn nameless_quantity_off_is_still_an_error() {
    let p = CooklangParser::new(Extensions::COMPAT, Converter::bundled());
    assert!(p.parse("Cut into @{9} squares.").into_result().is_err());
}

#[test]
fn text_timer_is_a_condition() {
    let r = parser()
        .parse("Fry until ~{golden}, about ~{6%minutes}.")
        .into_result()
        .unwrap()
        .0;
    assert_eq!(r.timers.len(), 2);
    let steps = &r.sections[0].content;
    assert!(matches!(steps[0].unwrap_step().items[1], Item::Timer { index: 0 }));
}
