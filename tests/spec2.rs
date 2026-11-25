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

// ---------------------------------------------------------------- helpers

use cooklang::spec2::{self, ByProductKind, Input, ReportLevel, Selection};
use cooklang::{Content, Recipe};

fn recipe(src: &str) -> Recipe {
    parser().parse(src).into_output().expect("no output")
}

/// "eggs | S1[yolks 1]" for each step, as 1-based step numbers
fn inputs(r: &Recipe, sel: &Selection) -> Vec<String> {
    let g = spec2::graph(r, sel);
    g.steps
        .iter()
        .filter(|s| s.active)
        .map(|s| {
            s.inputs
                .iter()
                .map(|i| match i {
                    Input::Ingredient { ingredient, .. } => r.ingredients[*ingredient].name.clone(),
                    Input::Missing { ingredient } => format!("?{}", r.ingredients[*ingredient].name),
                    Input::Step { from, uses, .. } => {
                        let u: Vec<String> = uses
                            .iter()
                            .map(|u| match &u.quantity {
                                Some(q) => format!("{} {}", u.name, q),
                                None => u.name.clone(),
                            })
                            .collect();
                        if u.is_empty() {
                            format!("S{}", from + 1)
                        } else {
                            format!("S{}[{}]", from + 1, u.join(", "))
                        }
                    }
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect()
}

fn step_text(r: &Recipe, section: usize, content: usize) -> String {
    let Content::Step(st) = &r.sections[section].content[content] else { panic!() };
    st.items
        .iter()
        .map(|i| match i {
            Item::Text { value } => value.clone(),
            Item::Action { value } => format!("`{value}`"),
            Item::Ingredient { index } => format!("<{}>", r.ingredients[*index].name),
            Item::Cookware { index } => format!("[{}]", r.cookware[*index].name),
            Item::Timer { .. } => "(t)".into(),
            Item::Group { options } => format!("<group of {}>", options.len()),
            Item::InlineQuantity { .. } => "(q)".into(),
        })
        .collect()
}

// ---------------------------------------------------------------- 0027 products

const TART: &str = "---
title: Lemon Meringue Tart
servings: 8
---

Separate @eggs{4}. -> yolks{4} + whites{4}

= Shell -> shell

Rub @butter{120%g}(cold, cubed) into @plain flour{220%g} and @icing sugar{40%g}. Bind with @&yolks{1} and @cold water{2%tbsp}.

Chill for ~{30%minutes}, then roll out and line a #tart tin{}.

Bake blind at 180 °C for ~{25%minutes}.

= Curd -> curd{600%g}

Whisk @lemon juice{150%ml} with @caster sugar{150%g}, the @&yolks{} and @eggs{2} in a #saucepan{}.

Cook over a low heat for ~{8%minutes} until thick, then beat in @butter{80%g}.

= Assemble

Fill the @&shell{} with the @&curd{} and chill for ~{2%hours}. -> tart

Whisk @&whites{3} with @caster sugar{150%g} to stiff peaks. -> meringue

Spread the @&meringue{} over the @&tart{} and brown under a #grill{}.
";

#[test]
fn tart_matches_the_proposal_table() {
    let r = recipe(TART);
    assert_eq!(
        inputs(&r, &Selection::default()),
        vec![
            "eggs",
            "butter | plain flour | icing sugar | S1[yolks 1] | cold water",
            "S2",
            "S3",
            "lemon juice | caster sugar | S1[yolks] | eggs",
            "S5 | butter",
            "S4[shell] | S6[curd]",
            "S1[whites 3] | caster sugar",
            "S8[meringue] | S7[tart]",
        ]
    );
    let g = spec2::graph(&r, &Selection::default());
    assert_eq!(g.root, Some(8));
    assert_eq!(g.by_products.len(), 1);
    assert_eq!(g.by_products[0].name, "whites");
    assert_eq!(g.by_products[0].kind, ByProductKind::Count);
    assert_eq!(g.by_products[0].left, Some(1.0));
    assert!(g.reports.iter().all(|r| r.level == ReportLevel::Info), "{:?}", g.reports);
    // products stay off the ingredient list
    assert!(r.ingredients.iter().filter(|i| i.relation.is_product_reference()).count() == 7);
}

#[test]
fn tail_is_not_step_text() {
    let r = recipe("Mix @flour{200%g} and @water{120%ml}. -> dough");
    assert_eq!(step_text(&r, 0, 0), "Mix <flour> and <water>. ");
    assert_eq!(r.products[0].name, "dough");
}

#[test]
fn section_product_and_name() {
    let r = recipe("== Dough -> dough ==\n\nMix @flour{}.\n");
    assert_eq!(r.sections[0].name.as_deref(), Some("Dough"));
    assert_eq!(r.products[r.sections[0].product.unwrap()].name, "dough");
}

#[test]
fn arrows_in_prose() {
    let r = recipe("Turn 180 -> 160 and bake.");
    assert_eq!(r.products[0].name, "160 and bake.");
    let r = recipe("Heat 60 -> 80 °C, then strain. -> custard");
    assert_eq!(r.products.len(), 1);
    assert_eq!(r.products[0].name, "custard");
    let r = recipe("Whisk.-> dough");
    assert!(r.products.is_empty());
    let r = recipe("Whisk. ->dough");
    assert_eq!(r.products[0].name, "dough");
    let r = recipe("Sear the meat. -> #&pan{}");
    assert!(r.products.is_empty());
}

#[test]
fn two_products_with_amounts() {
    let r = recipe("Drain the @pasta{500%g}. -> pasta + pasta water{250%ml}");
    assert_eq!(r.products.len(), 2);
    assert_eq!(r.products[1].name, "pasta water");
    assert!(r.products[1].quantity.is_some());
}

#[test]
fn a_use_with_nothing_before_it_is_reported_not_fatal() {
    let res = parser().parse("Knead the @&dough{}.");
    assert!(!res.report().has_errors());
    let r = res.into_output().unwrap();
    let g = spec2::graph(&r, &Selection::default());
    assert!(g.reports.iter().any(|x| x.rule == 6));
    assert!(!r.ingredients[0].modifiers().should_be_listed());
}

#[test]
fn name_used_after_it_was_used_up() {
    let r = recipe("Mix @flour{} and @water{}. -> wet mix\n\nMix the @&wet mix{} with @eggs{2}. -> batter\n\nAdd the @&wet mix{}.");
    let g = spec2::graph(&r, &Selection::default());
    assert!(g.reports.iter().any(|x| x.rule == 8), "{:?}", g.reports);
}

#[test]
fn product_named_like_an_ingredient() {
    let r = recipe("Melt @butter{50%g}. -> butter");
    let g = spec2::graph(&r, &Selection::default());
    assert!(g.reports.iter().any(|x| x.rule == 5));
}

#[test]
fn own_section_name_is_what_is_made_so_far() {
    let r = recipe("= Dough -> dough\n\nMix @flour{}.\n\nRest the @&dough{}.\n\n= Bake\n\nBake the @&dough{}.");
    assert_eq!(inputs(&r, &Selection::default()), vec!["flour", "S1 | S1[dough]", "S2[dough]"]
        .into_iter().map(|s| s.replace("S1 | S1[dough]", "S1[dough]")).collect::<Vec<_>>());
}

#[test]
fn part_e_ingredient_written_earlier() {
    let r = recipe("Melt @butter{100%g} in a #small pan{}.\n\nWhisk @eggs{2} with @sugar{80%g}. -> egg mix\n\nPour the @&butter{} into the @&egg mix{}.");
    let ins = inputs(&r, &Selection::default());
    // rule 9 carries "Whisk" on from "Melt", so the butter's line ends at the
    // step that makes the egg mix, and rule 23 reports it
    assert_eq!(ins[2], "S2[butter, egg mix]", "{ins:?}");
    let g = spec2::graph(&r, &Selection::default());
    assert!(g.reports.iter().any(|x| x.rule == 23), "{:?}", g.reports);
    // with the butter named, nothing is reported
    let r = recipe("Melt @butter{100%g} in a #small pan{}. -> melted butter\n\nWhisk @eggs{2} with @sugar{80%g}. -> egg mix\n\nPour the @&melted butter{} into the @&egg mix{}.");
    assert_eq!(inputs(&r, &Selection::default())[2], "S1[melted butter] | S2[egg mix]");
}

#[test]
fn shares_and_rest() {
    let r = recipe("Boil @potatoes{1%kg}. -> potatoes\n\nMash @&potatoes{1/2}.");
    let g = spec2::graph(&r, &Selection::default());
    // a share was taken, so the rest is a by-product
    assert!(g.by_products.iter().any(|b| b.kind == ByProductKind::Rest));
}

// ---------------------------------------------------------------- 0029 actions

#[test]
fn step_actions() {
    let r = recipe("In a #bowl{}, `whisk` the @eggs{3}. `Fold in` the @flour{100%g}.");
    assert_eq!(step_text(&r, 0, 0), "In a [bowl], `whisk` the <eggs>. `Fold in` the <flour>.");
}

#[test]
fn backticks_that_are_text() {
    // a pair holding an ingredient is text, and its closer opens nothing
    let r = recipe("Add `the @salt{}` now and `stir`.");
    assert_eq!(step_text(&r, 0, 0), "Add `the <salt>` now and `stir`.");
    let Content::Step(st) = &r.sections[0].content[0] else { panic!() };
    assert_eq!(st.items.iter().filter(|i| matches!(i, Item::Action { .. })).count(), 1);
    // a lone backtick and an empty pair are text
    for src in ["A lone ` stays.", "Empty `` too.", "Across `two\nlines` is text."] {
        let r = recipe(src);
        assert!(!r.sections[0].content[0].unwrap_step().items.iter().any(|i| matches!(i, Item::Action { .. })), "{src}");
    }
}

// ---------------------------------------------------------------- 0025 alternatives

#[test]
fn groups_and_labels() {
    let r = recipe("Fry @smoked bacon{200%g}|@pancetta|@smoked tofu{}?vegetarian in a #pan{}.");
    let Content::Step(st) = &r.sections[0].content[0] else { panic!() };
    let Item::Group { options } = &st.items[1] else { panic!("{:?}", st.items) };
    assert_eq!(options.len(), 3);
    assert_eq!(options[2].label.as_deref(), Some("vegetarian"));
    assert_eq!(inputs(&r, &Selection::default()), vec!["smoked bacon"]);
    let veg = Selection { toggles: [("vegetarian".into(), true)].into(), ..Default::default() };
    assert_eq!(inputs(&r, &veg), vec!["smoked tofu"]);
}

#[test]
fn bracket_option_and_reason() {
    let r = recipe("Add @egg{1}|(@flax{1%tbsp} + @water{3%tbsp})[if vegan] and stir.");
    let Content::Step(st) = &r.sections[0].content[0] else { panic!() };
    let Item::Group { options } = &st.items[1] else { panic!("{:?}", st.items) };
    assert_eq!(options[1].items.len(), 2);
    assert_eq!(options[1].reason.as_deref(), Some("if vegan"));
}

#[test]
fn brackets_without_a_pipe_are_text() {
    let r = recipe("Season (@salt{} and @pepper{}) to taste.");
    assert_eq!(step_text(&r, 0, 0), "Season (<salt> and <pepper>) to taste.");
}

#[test]
fn reason_only_after_an_option() {
    let r = recipe("Soak @urad dal{150%grams}[black lentils] overnight.");
    assert!(step_text(&r, 0, 0).contains("[black lentils]"));
}

#[test]
fn name_taken_from_first_option() {
    let r = recipe("Add @bread flour{500%g}|{450%g}?sourdough.");
    let Content::Step(st) = &r.sections[0].content[0] else { panic!() };
    let Item::Group { options } = &st.items[1] else { panic!() };
    let Item::Ingredient { index } = options[1].items[0] else { panic!() };
    assert_eq!(r.ingredients[index].name, "bread flour");
}

#[test]
fn step_and_section_labels_form_choices() {
    let src = "---\nchoices:\n  pastry:\n    homemade: Make it\n    shop-bought: Buy it\n---\n\n= Pastry ?homemade -> pastry\n\nRoll out @shortcrust{350%g}.\n\n= Pastry ?shop-bought -> pastry\n\nUnroll @ready-rolled pastry{1%sheet}.\n\n= Case\n\nLine a #tin{} with the @&pastry{}.\n\n?yeast=fresh: Crumble @fresh yeast{21%g}.\n\n?yeast=dry: Stir in @dry yeast{7%g}.";
    let r = recipe(src);
    let g = spec2::graph(&r, &Selection::default());
    assert_eq!(g.labels.choices.len(), 2);
    assert_eq!(g.labels.choices[0].name, "pastry");
    assert_eq!(g.labels.choices[1].options, vec!["fresh", "dry"]);
    assert_eq!(inputs(&r, &Selection::default()), vec!["shortcrust", "S1[pastry]", "S3 | fresh yeast"]);
    let shop = Selection { choices: [("pastry".into(), "shop-bought".into()), ("yeast".into(), "dry".into())].into(), ..Default::default() };
    assert_eq!(inputs(&r, &shop), vec!["ready-rolled pastry", "S2[pastry]", "S3 | dry yeast"]);
    assert!(g.reports.iter().all(|x| x.rule != 5));
}
