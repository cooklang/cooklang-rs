//! Cooklang canonical tests https://github.com/cooklang/spec/blob/main/tests/canonical.yaml

use cooklang::{
    quantity::Value, Content, Converter, CooklangParser, Extensions, Item, Modifiers, Note, Recipe,
};
use serde::Deserialize;

#[derive(Deserialize, PartialEq, Debug)]
struct TestCase {
    source: String,
    result: TestResult,
}

#[derive(Deserialize, PartialEq, Debug)]
struct TestResult {
    steps: Vec<TestStep>,
    /// Notes in order, each a list of text runs (spec proposal 0022)
    #[serde(default)]
    notes: Vec<TestNote>,
    metadata: serde_yaml::Mapping,
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(transparent)]
struct TestNote(Vec<TestNoteRun>);

#[derive(Deserialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TestNoteRun {
    Text {
        value: String,
        #[serde(default)]
        strong: bool,
        #[serde(default)]
        emphasis: bool,
        #[serde(default)]
        link: Option<String>,
    },
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(transparent)]
struct TestStep(Vec<TestStepItem>);

#[derive(Deserialize, PartialEq, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TestStepItem {
    Text {
        value: String,
    },
    Ingredient {
        name: String,
        quantity: TestValue,
        units: String,
        #[serde(default)]
        optional: bool,
    },
    Cookware {
        name: String,
        quantity: TestValue,
        #[serde(default)]
        optional: bool,
    },
    Timer {
        name: String,
        quantity: TestValue,
        units: String,
    },
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(untagged)]
enum TestValue {
    Number(f64),
    Text(String),
    /// `quantity: {start: 2, end: 3}`
    Range {
        start: f64,
        end: f64,
    },
}

mod canonical_cases;

fn runner(input: TestCase) {
    let parser = CooklangParser::new(Extensions::empty(), Converter::empty());
    let got = parser
        .parse(&input.source)
        .into_output()
        .expect("Failed to parse");
    let got_result = TestResult::from_cooklang(got);
    assert_eq!(got_result, input.result);
}

impl TestResult {
    fn from_cooklang(value: Recipe) -> Self {
        assert!(value.sections.len() <= 1);
        let mut steps = vec![];
        let mut notes = vec![];
        if let Some(section) = value.sections.first().cloned() {
            assert!(section.name.is_none());
            for content in section.content {
                match content {
                    Content::Text(note) => notes.push(TestNote::from_cooklang_note(note)),
                    step => steps.push(TestStep::from_cooklang_step(step, &value)),
                }
            }
        }
        Self {
            steps,
            notes,
            metadata: value.metadata.map,
        }
    }
}

impl TestNote {
    fn from_cooklang_note(note: Note) -> Self {
        Self(
            note.items
                .into_iter()
                .map(|r| TestNoteRun::Text {
                    value: r.value,
                    strong: r.strong,
                    emphasis: r.emphasis,
                    link: r.link,
                })
                .collect(),
        )
    }
}

impl TestStep {
    fn from_cooklang_step(value: Content, recipe: &Recipe) -> Self {
        let Content::Step(step) = value else {
            panic!("unexpected non step block")
        };

        let items = join_text_items(&step.items);
        let items = items
            .into_iter()
            .map(|v| TestStepItem::from_cooklang_item(v, recipe))
            .collect();
        Self(items)
    }
}

impl TestStepItem {
    fn from_cooklang_item(value: Item, recipe: &Recipe) -> Self {
        match value {
            Item::Text { value } => Self::Text { value },
            Item::Ingredient { index } => {
                let i = &recipe.ingredients[index];
                assert!(i.relation.is_definition());
                assert!(i.relation.referenced_from().is_empty());
                assert!((i.modifiers() - Modifiers::OPT).is_empty());
                assert!(i.alias.is_none());
                assert!(i.note.is_none());
                let quantity = i
                    .quantity
                    .as_ref()
                    .map(|q| TestValue::from_cooklang_value(q.value().clone()))
                    .unwrap_or(TestValue::Text("some".into()));
                let units = i
                    .quantity
                    .as_ref()
                    .and_then(|q| q.unit().map(|s| s.into()))
                    .unwrap_or_default();
                Self::Ingredient {
                    name: i.name.clone(),
                    quantity,
                    units,
                    optional: i.modifiers().is_optional(),
                }
            }
            Item::Cookware { index } => {
                let i = &recipe.cookware[index];
                assert!(i.relation.is_definition());
                assert!(i.relation.referenced_from().is_empty());
                assert!((i.modifiers() - Modifiers::OPT).is_empty());
                assert!(i.alias.is_none());
                assert!(i.note.is_none());
                let quantity = i
                    .quantity
                    .as_ref()
                    .map(|q| TestValue::from_cooklang_value(q.value().clone()))
                    .unwrap_or(TestValue::Number(1.0));
                Self::Cookware {
                    name: i.name.clone(),
                    quantity,
                    optional: i.modifiers().is_optional(),
                }
            }
            Item::Timer { index } => {
                let i = &recipe.timers[index];
                let quantity = i
                    .quantity
                    .as_ref()
                    .map(|q| TestValue::from_cooklang_value(q.value().clone()))
                    .unwrap_or(TestValue::Text("".into()));
                let units = i
                    .quantity
                    .as_ref()
                    .and_then(|q| q.unit().map(|s| s.into()))
                    .unwrap_or_default();
                Self::Timer {
                    name: i.name.clone().unwrap_or_default(),
                    quantity,
                    units,
                }
            }
            Item::InlineQuantity { index: _ } => panic!("Unexpected inline quantity"),
        }
    }
}

impl TestValue {
    fn from_cooklang_value(value: Value) -> Self {
        match value {
            Value::Number(num) => TestValue::Number(num.value()),
            Value::Range { start, end } => TestValue::Range {
                start: start.value(),
                end: end.value(),
            },
            Value::Text(value) => TestValue::Text(value),
        }
    }
}

// The parser may return text items splitted, but the tests don't account for that
fn join_text_items(items: &[cooklang::model::Item]) -> Vec<cooklang::model::Item> {
    let mut out = Vec::new();
    for item in items {
        if let Item::Text { value: current } = item {
            if let Some(Item::Text { value: last }) = out.last_mut() {
                last.push_str(current);
                continue;
            }
        }
        out.push(item.clone());
    }
    out
}
