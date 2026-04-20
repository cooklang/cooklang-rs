use cooklang::{
    Converter, CooklangParser, Extensions, Recipe,
};
use std::str;

// Typst WASM protocol boilerplate
use wasm_minimal_protocol::*;
initiate_protocol!();

#[wasm_func]
pub fn parse(content: &[u8]) -> Vec<u8> {
    // initiate cooklang parser
    let parser: CooklangParser = CooklangParser::new(Extensions::empty(), Converter::default());

    // parse the recipe
    let parsed: cooklang::error::PassResult<Recipe> = parser.parse(str::from_utf8(content).unwrap());

    // unwrap the result
    let (recipe, _warnings) = parsed.into_result().unwrap();

    // serialize to json
    serde_json::to_vec(&recipe).unwrap()
}