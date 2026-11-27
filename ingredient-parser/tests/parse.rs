use cooklang_ingredient_parser::{
    parse_ingredient, parse_ingredient_list, Amount, IngredientModel, ModelError, ModelParse,
    ParseSource,
};
use std::cell::RefCell;

/// Answers every line with `answer` and records what it was asked.
struct FakeModel {
    answer: Result<ModelParse, ModelError>,
    asked: RefCell<Vec<String>>,
}

impl FakeModel {
    fn new(answer: Result<ModelParse, ModelError>) -> Self {
        Self {
            answer,
            asked: RefCell::new(Vec::new()),
        }
    }
}

impl IngredientModel for FakeModel {
    fn parse_line(&self, line: &str) -> Result<ModelParse, ModelError> {
        self.asked.borrow_mut().push(line.to_owned());
        self.answer.clone()
    }
}

fn tomatoes(confidence: f32) -> ModelParse {
    ModelParse {
        quantity: Some("2".into()),
        unit: Some("cans".into()),
        size: None,
        name: "tomatoes".into(),
        prep: Some("chopped".into()),
        comment: None,
        confidence,
    }
}

#[test]
fn rules_alone() {
    let parsed = parse_ingredient("Sugar 1.5kg", None, None);
    assert_eq!(parsed.name, "Sugar");
    assert_eq!(
        parsed.amount,
        Some(Amount {
            value: 1.5,
            unit: Some("kg".into())
        })
    );
    assert_eq!(parsed.source, ParseSource::Rules);
    assert_eq!(parsed.confidence, 1.0);
}

#[test]
fn the_name_is_cleaned() {
    assert_eq!(
        parse_ingredient("./Milk {fresh} 2 l", None, None).name,
        "Milk fresh"
    );
}

#[test]
fn the_model_is_not_asked_when_the_rules_found_an_amount_at_the_end_or_no_number_leads() {
    let model = FakeModel::new(Ok(tomatoes(0.99)));
    for line in ["Milk 2 l", "Bread", "7up", "Lemons 3"] {
        let parsed = parse_ingredient(line, None, Some(&model));
        assert_eq!(parsed.source, ParseSource::Rules, "{line}");
    }
    assert!(model.asked.borrow().is_empty());
}

#[test]
fn a_confident_model_answer_replaces_the_rules() {
    let model = FakeModel::new(Ok(tomatoes(0.91)));
    let parsed = parse_ingredient("2 cans chopped tomatoes", None, Some(&model));
    assert_eq!(parsed.name, "tomatoes");
    assert_eq!(
        parsed.amount,
        Some(Amount {
            value: 2.0,
            unit: Some("cans".into())
        })
    );
    assert_eq!(parsed.prep.as_deref(), Some("chopped"));
    assert_eq!(parsed.source, ParseSource::Model);
    assert_eq!(parsed.confidence, 0.91);
    assert_eq!(model.asked.borrow().as_slice(), ["2 cans chopped tomatoes"]);
}

#[test]
fn a_model_unit_in_the_table_is_stored_short() {
    let answer = ModelParse {
        unit: Some("teaspoons".into()),
        quantity: Some("1 1/2".into()),
        ..tomatoes(0.9)
    };
    let parsed = parse_ingredient(
        "1 1/2 teaspoons tomatoes",
        None,
        Some(&FakeModel::new(Ok(answer))),
    );
    assert_eq!(
        parsed.amount,
        Some(Amount {
            value: 1.5,
            unit: Some("tsp".into())
        })
    );
}

#[test]
fn a_weak_or_unusable_model_answer_is_ignored() {
    let line = "2 cans chopped tomatoes";
    let rules = parse_ingredient(line, None, None);
    let answers = [
        Ok(tomatoes(0.79)),
        Ok(ModelParse {
            name: "  {} ".into(),
            ..tomatoes(0.99)
        }),
        Ok(ModelParse {
            quantity: Some("1-2".into()),
            ..tomatoes(0.99)
        }),
        Ok(ModelParse {
            quantity: Some("a few".into()),
            ..tomatoes(0.99)
        }),
        Ok(tomatoes(f32::NAN)),
        Err(ModelError("no runtime".into())),
    ];
    for answer in answers {
        let parsed = parse_ingredient(line, None, Some(&FakeModel::new(answer.clone())));
        assert_eq!(parsed, rules, "{answer:?}");
    }
}

#[test]
fn the_aisle_match_runs_on_the_final_name() {
    let aisle = cooklang::aisle::parse("[canned]\ntomatoes\n\n[dairy]\nmilk\n").unwrap();
    let model = FakeModel::new(Ok(ModelParse {
        name: "Tomato".into(),
        ..tomatoes(0.95)
    }));
    let parsed = parse_ingredient("2 cans chopped tomatoes", Some(&aisle), Some(&model));
    assert_eq!(parsed.name, "tomatoes");
    assert_eq!(parsed.aisle_match.unwrap().category, "canned");

    let parsed = parse_ingredient("Milk 2 l", Some(&aisle), None);
    assert_eq!(parsed.name, "milk");
}

#[test]
fn a_list_is_one_result_per_kept_line() {
    let parsed = parse_ingredient_list("## Dairy\n- Milk 2 l\n\n* [x] 3 lemons\n", None, None);
    let names: Vec<_> = parsed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Milk", "lemons"]);
}

#[test]
fn a_blank_line_has_an_empty_name() {
    assert_eq!(parse_ingredient("   ", None, None).name, "");
}
