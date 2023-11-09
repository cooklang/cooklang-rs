use cooklang::ast::build_ast;
use cooklang::{parser::PullParser, Extensions};
use cooklang::{Converter, CooklangParser, IngredientReferenceTarget, Item};
use std::fmt::Write;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

static EXTENSIONS: Mutex<Extensions> = Mutex::new(Extensions::all());
static COOKLANG_PARSER: Mutex<Option<CooklangParser>> = Mutex::new(None);

#[wasm_bindgen]
pub fn version() -> String {
    include_str!(concat!(env!("OUT_DIR"), "/version")).to_string()
}

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
    let (ast, report) = build_ast(events).into_tuple();
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
    report
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
    let (recipe, report) = parser.parse(input).into_tuple();
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
    report
        .write("playground", input, false, true, &mut buf)
        .unwrap();
    let ansi_error = String::from_utf8_lossy(&buf);
    let error =
        ansi_to_html::convert_escaped(&ansi_error).unwrap_or_else(|_| ansi_error.into_owned());
    FallibleResult { value, error }
}

#[wasm_bindgen]
pub fn parse_render(input: &str, scale: Option<u32>) -> FallibleResult {
    let mut parser_ref = COOKLANG_PARSER.lock().unwrap();
    if parser_ref.is_none() {
        *parser_ref = Some(CooklangParser::new(
            *EXTENSIONS.lock().unwrap(),
            Converter::default(),
        ));
    }
    let parser = parser_ref.as_ref().unwrap();
    let (recipe, report) = parser.parse(input).into_tuple();
    let value = match recipe {
        Some(r) => {
            let r = if let Some(scale) = scale {
                r.scale(scale, parser.converter())
            } else {
                r.default_scale()
            };
            render(r, parser.converter())
        }
        None => "<no ouput>".to_string(),
    };
    let mut buf = Vec::new();
    report
        .write("playground", input, false, true, &mut buf)
        .unwrap();
    let ansi_error = String::from_utf8_lossy(&buf);
    let error =
        ansi_to_html::convert_escaped(&ansi_error).unwrap_or_else(|_| ansi_error.into_owned());
    FallibleResult { value, error }
}

fn render(r: cooklang::ScaledRecipe, converter: &Converter) -> String {
    let ingredient_list = r.group_ingredients(converter);
    maud::html! {
        @if !ingredient_list.is_empty() {
            h2 { "Ingredients:" }
            ul {
                @for entry in &ingredient_list {
                    li {
                        b { (entry.ingredient.display_name()) }
                        @if !entry.quantity.is_empty() {": " (entry.quantity) }
                        @if let Some(n) = &entry.ingredient.note { " (" (n) ")" }
                    }
                }
            }
        }
        @if !r.cookware.is_empty() {
            h2 { "Cookware:" }
            ul {
                @for item in r.cookware.iter().filter(|c| c.modifiers().should_be_listed()) {
                    @let amount = item.group_amounts(&r.cookware).iter()
                                        .map(|q| q.to_string())
                                        .reduce(|s, q| format!("{s}, {q}"))
                                        .unwrap_or(String::new());
                    li {
                        b { (item.display_name()) }
                        @if !amount.is_empty() { ": " (amount) }
                        @if let Some(n) = &item.note { " (" (n) ")" }
                    }
                }
            }
        }
        @if !r.cookware.is_empty() || !ingredient_list.is_empty() {
            hr {}
        }
        @for (s_index, section) in r.sections.iter().enumerate() {
            @let s_num = s_index + 1;
            @if let Some(name) = &section.name {
                h3 { (s_num) " " (name) }
            } @else if s_num > 1 {
                h3 { "Section " (s_num) }
            }

            @for content in &section.content {
                @match content {
                    cooklang::Content::Text(t) => p { (t) },
                    cooklang::Content::Step(s) => p {
                        b { (s.number) ". " }
                        @for item in &s.items {
                            @match item {
                                Item::Ingredient { index } => {
                                    @let igr = &r.ingredients[*index];
                                    span.ingredient {
                                        (igr.display_name())
                                        @if let Some(q) = &igr.quantity {
                                            i { "(" (q) ")" }
                                        }
                                        @if let Some((index, target)) = &igr.relation.references_to() {
                                            @match target {
                                                IngredientReferenceTarget::Step => {
                                                    i { "(from step " (section.content[*index].unwrap_step().number) ")" }
                                                }
                                                IngredientReferenceTarget::Section => {
                                                    @let sect = *index + 1;
                                                    i { "(from section " (sect) ")" }
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                Item::Cookware { index } => {
                                    @let cw = &r.cookware[*index];
                                    span.cookware {
                                        (cw.display_name())
                                        @if let Some(q) = &cw.quantity {
                                            i { "(" (q) ")" }
                                        }
                                    }
                                }
                                Item::Timer { index } => {
                                    @let tm = &r.timers[*index];
                                    span.timer {
                                        @if let Some(name) = &tm.name {
                                            "(" (name) ")"
                                        }
                                        @if let Some(q) = &tm.quantity {
                                            i { (q) }
                                        }
                                    }
                                }
                                Item::InlineQuantity { index } => {
                                    @let q = &r.inline_quantities[*index];
                                    i.temp { (q) }
                                }
                                Item::Text { value } => {
                                    (value)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    .into_string()
}
