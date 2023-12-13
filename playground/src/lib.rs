use cooklang::ast::build_ast;
use cooklang::error::SourceReport;
use cooklang::{parser::PullParser, Extensions};
use cooklang::{Converter, CooklangParser, IngredientReferenceTarget, Item};
use std::fmt::Write;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    include_str!(concat!(env!("OUT_DIR"), "/version")).to_string()
}

#[wasm_bindgen]
pub struct State {
    parser: CooklangParser,
    load_units: bool,
    extensions: Extensions,
}

#[wasm_bindgen]
impl State {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            parser: CooklangParser::new(Extensions::all(), Converter::bundled()),
            load_units: true,
            extensions: Extensions::all(),
        }
    }

    #[wasm_bindgen(getter)]
    pub fn load_units(&self) -> bool {
        self.load_units
    }
    #[wasm_bindgen(setter)]
    pub fn set_load_units(&mut self, load: bool) {
        self.load_units = load;
        self.update_parser();
    }

    #[wasm_bindgen(getter)]
    pub fn extensions(&self) -> u32 {
        self.extensions.bits()
    }
    #[wasm_bindgen(setter)]
    pub fn set_extensions(&mut self, bits: u32) {
        self.extensions = Extensions::from_bits_truncate(bits);
        self.update_parser();
    }

    pub fn parse_events(&self, input: &str) -> String {
        let mut s = String::new();
        let events = PullParser::new(input, self.extensions);
        for e in events {
            writeln!(s, "{e:#?}").unwrap();
        }
        s
    }

    pub fn parse_ast(&self, input: &str, json: bool) -> FallibleResult {
        let events = PullParser::new(input, self.extensions);
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
        FallibleResult::new(value, report, input)
    }

    pub fn parse_full(&self, input: &str, json: bool) -> FallibleResult {
        let (recipe, report) = self.parser.parse(input).into_tuple();
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
        FallibleResult::new(value, report, input)
    }

    pub fn parse_render(&self, input: &str, scale: Option<u32>) -> FallibleResult {
        let (recipe, report) = self.parser.parse(input).into_tuple();
        let value = match recipe {
            Some(r) => {
                let r = if let Some(scale) = scale {
                    r.scale(scale, self.parser.converter())
                } else {
                    r.default_scale()
                };
                render(r, self.parser.converter())
            }
            None => "<no ouput>".to_string(),
        };
        FallibleResult::new(value, report, input)
    }
}

impl State {
    fn build_parser(&self) -> CooklangParser {
        let ext = self.extensions;
        let converter = if self.load_units {
            Converter::bundled()
        } else {
            Converter::empty()
        };
        CooklangParser::new(ext, converter)
    }

    fn update_parser(&mut self) {
        self.parser = self.build_parser();
    }
}

#[wasm_bindgen(getter_with_clone)]
pub struct FallibleResult {
    pub value: String,
    pub error: String,
}

impl FallibleResult {
    pub fn new(value: String, report: SourceReport, input: &str) -> Self {
        let mut buf = Vec::new();
        report.write("playground", input, true, &mut buf).unwrap();
        let ansi_error = String::from_utf8_lossy(&buf);
        let error = ansi_to_html::convert(&ansi_error).unwrap_or_else(|_| ansi_error.into_owned());
        FallibleResult { value, error }
    }
}

fn render(r: cooklang::ScaledRecipe, converter: &Converter) -> String {
    let ingredient_list = r.group_ingredients(converter);
    maud::html! {
        @if !r.metadata.map.is_empty() {
            ul {
                @for (key, value) in &r.metadata.map {
                    li.metadata {
                        span.key { (key) } ":" (value)
                    }
                }
            }

            hr {}
        }

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
                h3 { "(" (s_num) ") " (name) }
            } @else if r.sections.len() > 1 {
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
