//! Checks if serializing and deserialing a recipe with all the possible
//! features end with with an equal recipe
//!
//! Deserializing a recipe with serde is not recommended as it
//! will use more memory in comparison to using the cooklang parser

use cooklang::parse;

const RECIPE: &str = r#"

>> description: desc
>> time: 3 min

A step with @ingredients{}. References to @&ingredients{}, #cookware,
~timers{3%min}.

"#;

#[test]
#[ignore]
fn serde_test() {
    let recipe = parse(RECIPE).into_output().unwrap();

    let serialized = serde_json::to_string(&recipe).unwrap();
    println!("{}", serialized);
    let deserialized = serde_json::from_str(&serialized).unwrap();

    assert_eq!(recipe, deserialized);
}
