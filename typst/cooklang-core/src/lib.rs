use cooklang::{Converter, CooklangParser, Extensions};

// Typst WASM protocol boilerplate
use wasm_minimal_protocol::*;
initiate_protocol!();

/// Parse a Cooklang recipe and return it serialized as JSON.
///
/// Errors are returned as a human readable message, which Typst shows as a
/// compile error.
#[wasm_func]
pub fn parse(content: &[u8]) -> Result<Vec<u8>, String> {
    let input =
        std::str::from_utf8(content).map_err(|e| format!("recipe is not valid UTF-8: {e}"))?;

    let parser = CooklangParser::new(Extensions::empty(), Converter::default());

    let (recipe, _warnings) = parser.parse(input).into_result().map_err(|mut report| {
        report.remove_warnings();
        let mut buf = Vec::new();
        match report.write("recipe", input, false, &mut buf) {
            Ok(()) => String::from_utf8_lossy(&buf).into_owned(),
            Err(_) => report.to_string(),
        }
    })?;

    serde_json::to_vec(&recipe).map_err(|e| format!("failed to serialize recipe: {e}"))
}
