//! Cooklang canonical tests https://github.com/cooklang/spec/blob/main/tests/canonical.yaml

use cooklang::{
    quantity::{ScalableValue, Value},
    Content, Converter, CooklangParser, Extensions, Item, ScalableRecipe,
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
    metadata: serde_yaml::Mapping,
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
    },
    Cookware {
        name: String,
        quantity: TestValue,
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
    fn from_cooklang(value: ScalableRecipe) -> Self {
        assert!(value.sections.len() <= 1);
        let steps = if let Some(section) = value.sections.first().cloned() {
            assert!(section.name.is_none());
            section
                .content
                .into_iter()
                .map(|v| TestStep::from_cooklang_step(v, &value))
                .collect()
        } else {
            vec![]
        };
        Self {
            steps,
            metadata: value.metadata.map,
        }
    }
}

impl TestStep {
    fn from_cooklang_step(value: Content, recipe: &cooklang::ScalableRecipe) -> Self {
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
    fn from_cooklang_item(value: Item, recipe: &cooklang::ScalableRecipe) -> Self {
        match value {
            Item::Text { value } => Self::Text { value },
            Item::Ingredient { index } => {
                let i = &recipe.ingredients[index];
                assert!(i.relation.is_definition());
                assert!(i.relation.referenced_from().is_empty());
                assert!(i.modifiers().is_empty());
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
                }
            }
            Item::Cookware { index } => {
                let i = &recipe.cookware[index];
                assert!(i.relation.is_definition());
                assert!(i.relation.referenced_from().is_empty());
                assert!(i.modifiers().is_empty());
                assert!(i.alias.is_none());
                assert!(i.note.is_none());
                let quantity = i
                    .quantity
                    .as_ref()
                    .map(|q| TestValue::from_cooklang_value(q.clone()))
                    .unwrap_or(TestValue::Number(1.0));
                Self::Cookware {
                    name: i.name.clone(),
                    quantity,
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
    fn from_cooklang_value(value: ScalableValue) -> Self {
        match value {
            ScalableValue::Fixed(value) | ScalableValue::Linear(value) => match value {
                Value::Number(num) => TestValue::Number(num.value()),
                Value::Range { .. } => panic!("unexpected range value"),
                Value::Text(value) => TestValue::Text(value),
            }
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
