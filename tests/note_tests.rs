//! Inline formatting in notes (spec proposal 0022), through the whole parser

use cooklang::{Content, Converter, CooklangParser, Extensions, Item, Note, Recipe, TextRun};
use indoc::indoc;

fn parse(input: &str, extensions: Extensions) -> Recipe {
    CooklangParser::new(extensions, Converter::empty())
        .parse(input)
        .into_output()
        .expect("parse failed")
}

fn notes(input: &str) -> Vec<Note> {
    parse(input, Extensions::empty())
        .sections
        .into_iter()
        .flat_map(|s| s.content)
        .filter_map(|c| match c {
            Content::Text(note) => Some(note),
            Content::Step(_) => None,
        })
        .collect()
}

fn one_note(input: &str) -> Note {
    let mut all = notes(input);
    assert_eq!(all.len(), 1, "expected one note in {input:?}");
    all.remove(0)
}

fn run(value: &str) -> TextRun {
    TextRun::plain(value)
}

fn strong(value: &str) -> TextRun {
    TextRun {
        strong: true,
        ..run(value)
    }
}

fn emphasis(value: &str) -> TextRun {
    TextRun {
        emphasis: true,
        ..run(value)
    }
}

fn link(value: &str, destination: &str) -> TextRun {
    TextRun {
        link: Some(destination.into()),
        ..run(value)
    }
}

#[test]
fn proposal_example() {
    let note = one_note(indoc! {"
        > **Make ahead:** the dough keeps *up to 3 days* in the fridge.
        > See the [shaping guide](https://example.com/shaping) before you start.

        Mix @flour{500%g} with @water{350%ml} and @salt{10%g}.
    "});
    assert_eq!(
        note.items,
        vec![
            strong("Make ahead:"),
            run(" the dough keeps "),
            emphasis("up to 3 days"),
            run(" in the fridge. See the "),
            link("shaping guide", "https://example.com/shaping"),
            run(" before you start."),
        ]
    );
    assert_eq!(
        note.text(),
        "Make ahead: the dough keeps up to 3 days in the fridge. See the shaping guide before you start."
    );
    assert!(note.is_formatted());
}

#[test]
fn plain_note_is_one_run_with_todays_text() {
    let note = one_note(indoc! {"
        > a text step
        > with 2 lines
    "});
    assert_eq!(note.items, vec![run("a text step with 2 lines")]);
    assert!(!note.is_formatted());
}

#[test]
fn formatting_continues_across_lines() {
    let note = one_note(indoc! {"
        > It is *very
        > important* to rest
        > the [dough
        > overnight](./Dough.cook).
    "});
    assert_eq!(
        note.items,
        vec![
            run("It is "),
            emphasis("very important"),
            run(" to rest the "),
            link("dough overnight", "./Dough.cook"),
            run("."),
        ]
    );
}

#[test]
fn formatting_does_not_cross_note_blocks() {
    let all = notes(indoc! {"
        > *one

        > two*
    "});
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].items, vec![run("*one")]);
    assert_eq!(all[1].items, vec![run("two*")]);
}

#[test]
fn escaped_characters_are_text() {
    let note = one_note(r"> Use 2\*250 g tins, not 3\*100 g jars, and \[brackets\](x) \_here\_.");
    assert_eq!(
        note.items,
        vec![run(
            "Use 2*250 g tins, not 3*100 g jars, and [brackets](x) _here_."
        )]
    );
}

#[test]
fn escape_at_the_start_of_a_line() {
    let note = one_note(indoc! {r"
        > \*a*
        > \\*b*
    "});
    assert_eq!(note.items, vec![run(r"*a* \"), emphasis("b")]);
}

#[test]
fn comments_are_removed_first() {
    let note = one_note("> *a [- hidden -]b* c -- gone");
    assert_eq!(note.items, vec![emphasis("a b"), run(" c ")]);

    // `--` in a destination starts a comment, unless escaped
    let note = one_note(r"> [x](https://example.com/a\--b)");
    assert_eq!(note.items, vec![link("x", "https://example.com/a--b")]);
}

#[test]
fn cooklang_markers_in_notes_are_text() {
    let note = one_note("> Add *@salt{1%tsp}* and #pan ~{5%minutes}");
    assert_eq!(
        note.items,
        vec![
            run("Add "),
            emphasis("@salt{1%tsp}"),
            run(" and #pan ~{5%minutes}")
        ]
    );
}

#[test]
fn steps_are_not_formatted() {
    let recipe = parse("Mix *well* and see [this](x).", Extensions::empty());
    let Content::Step(step) = &recipe.sections[0].content[0] else {
        panic!("not a step")
    };
    assert_eq!(
        step.items,
        vec![Item::Text {
            value: "Mix *well* and see [this](x).".into()
        }]
    );
}

#[test]
fn ingredient_notes_are_not_formatted() {
    let recipe = parse("Add @flour{200%g}(*sifted*).", Extensions::all());
    assert_eq!(recipe.ingredients[0].note.as_deref(), Some("*sifted*"));
}

#[test]
fn formatting_with_all_extensions() {
    let note = one_note_with("> **Tip:** use *fresh* herbs", Extensions::all());
    assert_eq!(
        note.items,
        vec![
            strong("Tip:"),
            run(" use "),
            emphasis("fresh"),
            run(" herbs")
        ]
    );
}

fn one_note_with(input: &str, extensions: Extensions) -> Note {
    let recipe = parse(input, extensions);
    let mut all: Vec<Note> = recipe
        .sections
        .into_iter()
        .flat_map(|s| s.content)
        .filter_map(|c| match c {
            Content::Text(note) => Some(note),
            Content::Step(_) => None,
        })
        .collect();
    assert_eq!(all.len(), 1);
    all.remove(0)
}

#[test]
fn json_keeps_the_plain_value_and_adds_items() {
    let recipe = parse("> **Tip:** see [this](x)", Extensions::empty());
    let json = serde_json::to_value(&recipe.sections[0].content[0]).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "type": "text",
            "value": "Tip: see this",
            "items": [
                { "type": "text", "value": "Tip:", "strong": true },
                { "type": "text", "value": " see " },
                { "type": "text", "value": "this", "link": "x" },
            ]
        })
    );
}

#[test]
fn json_without_items_reads_as_plain_note() {
    let content: Content =
        serde_json::from_value(serde_json::json!({ "type": "text", "value": "**old**" })).unwrap();
    assert_eq!(content, Content::Text(Note::plain("**old**")));
}

#[test]
fn json_roundtrip() {
    let recipe = parse(
        "> **a** *b* [c](d)\n\nA step with @salt.",
        Extensions::all(),
    );
    let json = serde_json::to_string(&recipe).unwrap();
    let back: Recipe = serde_json::from_str(&json).unwrap();
    assert_eq!(recipe, back);
}

#[test]
fn note_display_and_conversions() {
    let note = one_note("> a *b* c");
    assert_eq!(note.to_string(), "a b c");
    assert_eq!(Note::from("x"), Note::plain("x"));
    assert_eq!(Note::plain("").items, vec![]);
    assert_eq!(Content::Text(note.clone()).unwrap_text().items, note.items);
}
