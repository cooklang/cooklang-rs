use cooklang::error::Report;
use cooklang::parser::build_ast;
use cooklang::{parser::PullParser, Extensions};
use cooklang::{Converter, CooklangParser};
use std::fmt::Write;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static EXTENSIONS: Mutex<Extensions> = Mutex::new(Extensions::all());
static COOKLANG_PARSER: Mutex<Option<CooklangParser>> = Mutex::new(None);

#[wasm_bindgen]
pub fn set_extensions(bits: u32) {
    let extensions = Extensions::from_bits_truncate(bits);
    *EXTENSIONS.lock().unwrap() = extensions;
    *COOKLANG_PARSER.lock().unwrap() = Some(CooklangParser::new(extensions, Converter::default()));
}

#[wasm_bindgen]
pub fn get_extensions() -> u32 {
    EXTENSIONS.lock().unwrap().bits()
}

#[wasm_bindgen]
pub fn parse_events(input: &str) -> String {
    let mut s = String::new();
    let events = PullParser::new(input, *EXTENSIONS.lock().unwrap());
    for e in events {
        writeln!(s, "{e:#?}").unwrap();
    }
    s
}

#[wasm_bindgen(getter_with_clone)]
pub struct FallibleResult {
    pub value: String,
    pub error: String,
}

#[wasm_bindgen]
pub fn parse_ast(input: &str, json: bool) -> FallibleResult {
    let events = PullParser::new(input, *EXTENSIONS.lock().unwrap());
    let (ast, warnings, errors) = build_ast(events).into_tuple();
    let value = match ast {
        Some(ast) => {
            if json {
                serde_json::to_string_pretty(&ast).unwrap()
            } else {
                format!("{ast:#?}")
            }
        }
        None => "<no ouput>".to_string(),
    };
    let mut buf = Vec::new();
    Report::new(errors, warnings)
        .write("playground", input, false, true, &mut buf)
        .unwrap();
    let ansi_error = String::from_utf8_lossy(&buf);
    let error =
        ansi_to_html::convert_escaped(&ansi_error).unwrap_or_else(|_| ansi_error.into_owned());
    FallibleResult { value, error }
}

#[wasm_bindgen]
pub fn parse_full(input: &str, json: bool) -> FallibleResult {
    let mut parser_ref = COOKLANG_PARSER.lock().unwrap();
    if parser_ref.is_none() {
        *parser_ref = Some(CooklangParser::new(
            *EXTENSIONS.lock().unwrap(),
            Converter::default(),
        ));
    }
    let parser = parser_ref.as_ref().unwrap();
    let (recipe, warnings, errors) = parser.parse(input, "playground").into_tuple();
    let value = match recipe {
        Some(r) => {
            if json {
                serde_json::to_string_pretty(&r).unwrap()
            } else {
                format!("{r:#?}")
            }
        }
        None => "<no ouput>".to_string(),
    };
    let mut buf = Vec::new();
    Report::new(errors, warnings)
        .write("playground", input, false, true, &mut buf)
        .unwrap();
    let ansi_error = String::from_utf8_lossy(&buf);
    let error =
        ansi_to_html::convert_escaped(&ansi_error).unwrap_or_else(|_| ansi_error.into_owned());
    FallibleResult { value, error }
}
