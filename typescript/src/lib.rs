use cooklang::ast::build_ast;
use cooklang::error::SourceReport;
use cooklang::metadata::{CooklangValueExt, NameAndUrl, RecipeTime, Servings, StdKey};
use cooklang::parser::Quantity;
use cooklang::{parser::PullParser, quantity, Cookware, Extensions, GroupedQuantity, Ingredient};
use cooklang::{Converter, CooklangParser, IngredientReferenceTarget, Item};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Write;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    include_str!(concat!(env!("OUT_DIR"), "/version")).to_string()
}

// wasm cannot export pure tuples yet
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
pub struct GroupedIndexAndQuantity {
    index: usize,
    quantity: GroupedQuantity,
}

#[wasm_bindgen]
pub struct Parser {
    parser: CooklangParser,
    load_units: bool,
    extensions: Extensions,
}

#[derive(Tsify, Serialize, Deserialize)]
pub struct InterpretedMetadata {
    #[tsify(optional)]
    title: Option<String>,
    #[tsify(optional)]
    description: Option<String>,
    #[tsify(optional)]
    tags: Option<Vec<String>>,
    #[tsify(optional)]
    author: Option<NameAndUrl>,
    #[tsify(optional)]
    source: Option<NameAndUrl>,
    #[tsify(optional, type = "any")]
    course: Option<serde_yaml::Value>,
    #[tsify(optional)]
    time: Option<RecipeTime>,
    #[tsify(optional)]
    servings: Option<Servings>,
    #[tsify(optional, type = "any")]
    difficulty: Option<serde_yaml::Value>,
    #[tsify(optional, type = "any")]
    cuisine: Option<serde_yaml::Value>,
    #[tsify(optional, type = "any")]
    diet: Option<serde_yaml::Value>,
    #[tsify(optional, type = "any")]
    images: Option<serde_yaml::Value>,
    #[tsify(optional)]
    locale: Option<(String, Option<String>)>,

    #[tsify(type = "Record<any, any>")]
    custom: HashMap<serde_yaml::Value, serde_yaml::Value>,
}

#[derive(Tsify, Serialize, Deserialize)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub struct ScaledRecipeWithReport {
    recipe: cooklang::Recipe,
    metadata: InterpretedMetadata,
    report: String,
}

#[wasm_bindgen]
impl Parser {
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
            None => "<no output>".to_string(),
        };
        FallibleResult::new(value, report, input)
    }

    pub fn parse(&self, input: &str) -> ScaledRecipeWithReport {
        let (recipe, _report) = self.parser.parse(input).into_tuple();
        let mut recipe = recipe.expect("expected recipe");
        recipe.scale(1., self.parser.converter());

        let metadata = InterpretedMetadata {
            title: recipe.metadata.title().map(str::to_string),
            description: recipe.metadata.description().map(str::to_string),
            tags: recipe
                .metadata
                .tags()
                .map(|v| v.iter().map(|s| s.to_string()).collect()),
            author: recipe.metadata.author(),
            source: recipe.metadata.source(),
            course: recipe.metadata.get(StdKey::Course).cloned(),
            time: recipe.metadata.time(self.parser.converter()),
            servings: recipe.metadata.servings(),
            difficulty: recipe.metadata.get(StdKey::Difficulty).cloned(),
            cuisine: recipe.metadata.get(StdKey::Cuisine).cloned(),
            diet: recipe.metadata.get(StdKey::Diet).cloned(),
            images: recipe.metadata.get(StdKey::Images).cloned(),
            locale: recipe
                .metadata
                .locale()
                .map(|(a, b)| (a.to_string(), b.map(str::to_string))),
            custom: recipe
                .metadata
                .map_filtered()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        };

        let data = ScaledRecipeWithReport {
            recipe,
            metadata,
            report: "<no output>".to_string(),
        };
        data
    }

    /// returns vector of indices in r.recipe.ingredients and their quantities
    pub fn group_ingredients(&self, r: &ScaledRecipeWithReport) -> Vec<GroupedIndexAndQuantity> {
        r.recipe
            .group_ingredients(self.parser.converter())
            .into_iter()
            .map(|r| GroupedIndexAndQuantity {
                index: r.index,
                quantity: r.quantity,
            })
            .collect()
    }

    /// returns vector of indices in r.recipe.cookware and their quantities
    pub fn group_cookware(&self, r: &ScaledRecipeWithReport) -> Vec<GroupedIndexAndQuantity> {
        r.recipe
            .group_cookware(self.parser.converter())
            .into_iter()
            .map(|r| GroupedIndexAndQuantity {
                index: r.index,
                quantity: r.quantity,
            })
            .collect()
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
            None => "<no output>".to_string(),
        };
        FallibleResult::new(value, report, input)
    }

    pub fn parse_render(&self, input: &str, scale: Option<f64>) -> FallibleResult {
        let (recipe, report) = self.parser.parse(input).into_tuple();
        let value = match recipe {
            Some(mut r) => {
                if let Some(scale) = scale {
                    r.scale(scale, self.parser.converter())
                }
                render(r, self.parser.converter())
            }
            None => "<no output>".to_string(),
        };
        FallibleResult::new(value, report, input)
    }

    pub fn std_metadata(&self, input: &str) -> FallibleResult {
        let (meta, report) = self.parser.parse_metadata(input).into_tuple();
        let value = match meta {
            Some(m) => {
                #[derive(Debug)]
                #[allow(dead_code)]
                struct StdMeta<'a> {
                    tags: Option<Vec<std::borrow::Cow<'a, str>>>,
                    author: Option<NameAndUrl>,
                    source: Option<NameAndUrl>,
                    time: Option<RecipeTime>,
                    servings: Option<cooklang::metadata::Servings>,
                    locale: Option<(&'a str, Option<&'a str>)>,
                }
                let val = StdMeta {
                    tags: m.tags(),
                    author: m.author(),
                    source: m.source(),
                    time: m.time(self.parser.converter()),
                    servings: m.servings(),
                    locale: m.locale(),
                };
                format!("{val:#?}")
            }
            None => "<no output>".to_string(),
        };
        FallibleResult::new(value, report, input)
    }
}

impl Parser {
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

#[wasm_bindgen]
pub fn ingredient_should_be_listed(this: &Ingredient) -> bool {
    this.modifiers().should_be_listed()
}

#[wasm_bindgen]
pub fn ingredient_display_name(this: &Ingredient) -> String {
    this.display_name().to_string()
}

#[wasm_bindgen]
pub fn cookware_should_be_listed(this: &Cookware) -> bool {
    this.modifiers().should_be_listed()
}

#[wasm_bindgen]
pub fn cookware_display_name(this: &Cookware) -> String {
    this.display_name().to_string()
}

#[wasm_bindgen]
pub fn grouped_quantity_is_empty(this: &GroupedQuantity) -> bool {
    this.is_empty()
}

#[wasm_bindgen]
pub fn grouped_quantity_display(this: &GroupedQuantity) -> String {
    this.to_string()
}

#[wasm_bindgen]
pub fn quantity_display(this: &quantity::Quantity) -> String {
    this.to_string()
}

fn render(r: cooklang::Recipe, converter: &Converter) -> String {
    let ingredient_list = r.group_ingredients(converter);
    let cookware_list = r.group_cookware(converter);
    maud::html! {
        @if !r.metadata.map.is_empty() {
            ul {
                @for (key, value) in &r.metadata.map {
                    li.metadata {
                        span.key { (key.as_str_like().unwrap_or_else(|| format!("{key:?}").into())) } ":" (value.as_str_like().unwrap_or_else(|| format!("{value:?}").into()))
                    }
                }
            }

            hr {}
        }

        @if !ingredient_list.is_empty() {
            h2 { "Ingredients:" }
            ul {
                @for entry in &ingredient_list {
                    @if entry.ingredient.modifiers().should_be_listed() {
                        li {
                            b { (entry.ingredient.display_name()) }
                            @if !entry.quantity.is_empty() {": " (entry.quantity) }
                            @if let Some(n) = &entry.ingredient.note { " (" (n) ")" }
                        }
                    }
                }
            }
        }
        @if !r.cookware.is_empty() {
            h2 { "Cookware:" }
            ul {
                @for entry in &cookware_list {
                    @if entry.cookware.modifiers().should_be_listed() {
                        li {
                            b { (entry.cookware.display_name()) }
                            @if !entry.quantity.is_empty() { ": " (entry.quantity) }
                            @if let Some(n) = &entry.cookware.note { " (" (n) ")" }
                        }
                    }
                }
            }
        }
        @if !cookware_list.is_empty() || !ingredient_list.is_empty() {
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
