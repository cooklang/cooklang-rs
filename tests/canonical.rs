//! Cooklang canonical tests https://github.com/cooklang/spec/blob/main/tests/canonical.yaml

use cooklang::{
    model::ComponentKind,
    quantity::{QuantityValue, Value},
};
use yaml_rust::{Yaml, YamlLoader};

#[test]
fn canonical_tests() {
    let file = std::fs::read_to_string("tests/canonical.yaml").unwrap();
    let docs = YamlLoader::load_from_str(&file).unwrap();
    let doc = &docs[0];
    let tests = doc["tests"].as_hash().unwrap();

    for (name, test) in tests.iter() {
        let name = name.as_str().unwrap();
        run_test(name, test);
    }
}

fn run_test(name: &str, test: &Yaml) {
    eprintln!("Running test {name}");
    let got = cooklang::CooklangParser::builder()
        .with_extensions(cooklang::Extensions::empty())
        .finish()
        .parse(test["source"].as_str().unwrap(), name)
        .into_output()
        .expect("Failed to parse");
    let expected = &test["result"];

    compare_metadata(&expected["metadata"], &got.metadata);
    compare_steps(&expected["steps"], &got.sections, &got);
}

fn compare_metadata(expected: &Yaml, got: &cooklang::metadata::Metadata) {
    let expected = expected.as_hash().unwrap();
    assert_eq!(expected.len(), got.map.len());

    for (e_key, e_value) in expected.iter() {
        let e_key = e_key.as_str().unwrap();
        let e_value = e_value.as_str().unwrap();

        let got_val = got.map[e_key].as_str();
        assert_eq!(e_value, got_val);
    }
}

fn compare_steps(expected: &Yaml, got: &[cooklang::model::Section], recipe: &cooklang::Recipe) {
    let expected = expected.as_vec().unwrap();
    if expected.is_empty() {
        assert!(got.is_empty());
        return;
    }
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].name, None);
    let got = &got[0].steps;
    assert_eq!(expected.len(), got.len());
    // for each step
    for (expected, got) in expected.iter().zip(got.iter()) {
        let expected = expected.as_vec().unwrap();
        assert_eq!(got.is_text(), false);
        let got = join_text_items(&got.items);
        eprintln!("{got:#?}");
        assert_eq!(expected.len(), got.len()); // same number of items
                                               // for each item
        for (expected, got) in expected.iter().zip(got.iter()) {
            compare_items(expected, got, recipe);
        }
    }
}

fn join_text_items(items: &[cooklang::model::Item]) -> Vec<cooklang::model::Item> {
    use cooklang::model::Item;
    use Item::*;

    let mut out = Vec::new();
    for item in items {
        if let Text { value: current } = item {
            if let Some(Text { value: last }) = out.last_mut() {
                last.push_str(&current);
                continue;
            }
        }
        out.push(item.clone());
    }
    out
}

fn compare_items(expected: &Yaml, got: &cooklang::model::Item, recipe: &cooklang::Recipe) {
    use cooklang::model::Item;

    let tyype = expected["type"].as_str().unwrap();

    match got {
        Item::Text { value: text } => {
            assert_eq!(tyype, "text");
            assert_eq!(expected["value"].as_str().unwrap(), text);
        }
        Item::ItemComponent { value: component } => match component.kind {
            ComponentKind::IngredientKind => {
                let i = &recipe.ingredients[component.index];
                assert_eq!(tyype, "ingredient");
                assert!(i.alias.is_none());
                assert!(i.note.is_none());
                assert_eq!(i.name, expected["name"].as_str().unwrap());
                match &i.quantity {
                    Some(quantity) => {
                        compare_value(&expected["quantity"], &quantity.value);
                        match quantity.unit_text() {
                            Some(u) => assert_eq!(u, expected["units"].as_str().unwrap()),
                            None => assert!(expected["units"].as_str().unwrap().is_empty()),
                        }
                    }
                    None => assert_eq!("some", expected["quantity"].as_str().unwrap()),
                }
            }
            ComponentKind::CookwareKind => {
                let c = &recipe.cookware[component.index];
                assert_eq!(tyype, "cookware");
                assert_eq!(c.name, expected["name"].as_str().unwrap());
                match &c.quantity {
                    Some(v) => compare_value(&expected["quantity"], v),
                    None => assert_eq!(expected["quantity"].as_i64().unwrap(), 1),
                }
            }
            ComponentKind::TimerKind => {
                let t = &recipe.timers[component.index];
                assert_eq!(tyype, "timer");
                match &t.name {
                    Some(n) => assert_eq!(n, expected["name"].as_str().unwrap()),
                    None => assert!(expected["name"].as_str().unwrap().is_empty()),
                }
                match &t.quantity {
                    Some(quantity) => {
                        compare_value(&expected["quantity"], &quantity.value);
                        match quantity.unit_text() {
                            Some(u) => assert_eq!(u, expected["units"].as_str().unwrap()),
                            None => assert!(expected["units"].as_str().unwrap().is_empty()),
                        }
                    }
                    None => assert_eq!("", expected["quantity"].as_str().unwrap()),
                }
            }
        },
        _ => panic!("unexpected item kind"),
    }
}

fn compare_value(expected: &Yaml, got: &QuantityValue) {
    let value = match got {
        QuantityValue::Fixed { value: v } => v,
        _ => {
            panic!("scalable values not supported by cooklang currently");
        }
    };
    match value {
        Value::Number { value: n } => {
            assert_eq!(
                *n as f64,
                expected
                    .as_i64()
                    .map(|n| n as f64)
                    .or_else(|| expected.as_f64())
                    .unwrap()
            )
        }
        Value::Range { value: _ } => panic!("Unexpected range value"),
        Value::Text { value: t } => assert_eq!(t, expected.as_str().unwrap()),
    };
}
