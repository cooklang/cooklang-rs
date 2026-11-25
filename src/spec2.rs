//! Spec 2.0 drafts on top of a parsed [`Recipe`]: the path through choices and
//! toggles (spec proposal 0025), and the graph of steps that products and
//! carried-on steps make (spec proposal 0027).
//!
//! The graph follows the proposal text. Drawing choices (which steps to put
//! above a chart, how to lay it out) are left to the caller.
//!
//! ```
//! use cooklang::{spec2, Converter, CooklangParser, Extensions};
//! let parser = CooklangParser::new(Extensions::all(), Converter::bundled());
//! let src = "Separate @eggs{2}. -> yolks + whites\n\nBeat the @&whites{}.";
//! let recipe = parser.parse(src).into_output().unwrap();
//! let graph = spec2::graph(&recipe, &spec2::Selection::default());
//! assert_eq!(graph.root, Some(1));
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::{
    analysis::product_key,
    model::{Content, IngredientReferenceTarget, Item, Recipe},
    parser::Modifiers,
    quantity::{Quantity, Value},
};

/// What the reader chose: an option per choice, and toggles that are on.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Selection {
    #[serde(default)]
    pub choices: HashMap<String, String>,
    #[serde(default)]
    pub toggles: HashMap<String, bool>,
}

/// A named decision with two or more options; the first is the default.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Choice {
    pub name: String,
    pub options: Vec<String>,
    /// From front matter `choices`, when given
    pub descriptions: Vec<Option<String>>,
}

/// The labels of a recipe: choices, and toggles (off by default).
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct Labels {
    pub choices: Vec<Choice>,
    pub toggles: Vec<String>,
}

/// Collects choices and toggles (0025, "How labels form choices").
pub fn labels(recipe: &Recipe) -> Labels {
    let mut choices: Vec<Choice> = Vec::new();

    // 1. declared in front matter
    if let Some(serde_yaml::Value::Mapping(map)) = recipe.metadata.map.get("choices") {
        for (k, v) in map {
            let Some(name) = k.as_str() else { continue };
            let mut c = Choice {
                name: name.to_lowercase(),
                options: Vec::new(),
                descriptions: Vec::new(),
            };
            match v {
                serde_yaml::Value::Mapping(opts) => {
                    for (o, d) in opts {
                        if let Some(o) = o.as_str() {
                            c.options.push(o.to_lowercase());
                            c.descriptions.push(match d {
                                serde_yaml::Value::String(s) => Some(s.clone()),
                                serde_yaml::Value::Mapping(m) => m
                                    .get("description")
                                    .and_then(|d| d.as_str())
                                    .map(str::to_string),
                                _ => None,
                            });
                        }
                    }
                }
                serde_yaml::Value::Sequence(opts) => {
                    for o in opts.iter().filter_map(|o| o.as_str()) {
                        c.options.push(o.to_lowercase());
                        c.descriptions.push(None);
                    }
                }
                _ => {}
            }
            choices.push(c);
        }
    }

    // every label in the file, in order
    let mut found = Vec::new();
    for s in &recipe.sections {
        found.extend(s.label.clone());
        for c in &s.content {
            if let Content::Step(st) = c {
                found.extend(st.label.clone());
                collect_item_labels(&st.items, &mut found);
            }
        }
    }

    // 2. declared in the text: `?yeast=fresh`
    for l in &found {
        if let Some((c, o)) = l.split_once('=') {
            let idx = match choices.iter().position(|x| x.name == c) {
                Some(i) => i,
                None => {
                    choices.push(Choice {
                        name: c.to_string(),
                        options: Vec::new(),
                        descriptions: Vec::new(),
                    });
                    choices.len() - 1
                }
            };
            if !choices[idx].options.iter().any(|x| x == o) {
                choices[idx].options.push(o.to_string());
                choices[idx].descriptions.push(None);
            }
        }
    }

    // 3 and 4. a bare label is an option of a declared choice, or a toggle
    let mut toggles = Vec::new();
    for l in &found {
        if l.contains('=') || choices.iter().any(|c| c.options.contains(l)) {
            continue;
        }
        if !toggles.contains(l) {
            toggles.push(l.clone());
        }
    }
    Labels { choices, toggles }
}

fn collect_item_labels(items: &[Item], out: &mut Vec<String>) {
    for i in items {
        if let Item::Group { options } = i {
            for o in options {
                out.extend(o.label.clone());
                collect_item_labels(&o.items, out);
            }
        }
    }
}

/// The path chosen through a recipe's labels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivePath {
    /// The active option of each choice
    pub current: HashMap<String, String>,
    /// Toggles and whether they are on
    pub toggles: HashMap<String, bool>,
    #[serde(skip)]
    labels: Labels,
}

impl ActivePath {
    pub fn new(labels: Labels, selection: &Selection) -> Self {
        let mut current = HashMap::new();
        for c in &labels.choices {
            let want = selection.choices.get(&c.name);
            let opt = match want {
                Some(w) if c.options.contains(w) => w.clone(),
                _ => c.options.first().cloned().unwrap_or_default(),
            };
            current.insert(c.name.clone(), opt);
        }
        let toggles = labels
            .toggles
            .iter()
            .map(|t| {
                (
                    t.clone(),
                    selection.toggles.get(t).copied().unwrap_or(false),
                )
            })
            .collect();
        Self {
            current,
            toggles,
            labels,
        }
    }

    /// Whether a label (or no label) is active on this path
    pub fn is_active(&self, label: Option<&str>) -> bool {
        let Some(l) = label else { return true };
        if let Some((c, o)) = l.split_once('=') {
            return self.current.get(c).is_some_and(|x| x == o);
        }
        for c in &self.labels.choices {
            if c.options.iter().any(|o| o == l) {
                return self.current.get(&c.name).is_some_and(|x| x == l);
            }
        }
        self.toggles.get(l).copied().unwrap_or(false)
    }

    /// The active option of a group: the first whose label is active, or the
    /// first with no label; `None` leaves the slot empty on this path.
    pub fn active_option(&self, options: &[crate::model::GroupOption]) -> Option<usize> {
        options
            .iter()
            .position(|o| o.label.is_some() && self.is_active(o.label.as_deref()))
            .or_else(|| options.iter().position(|o| o.label.is_none()))
    }
}

/// The graph of a recipe on one path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub labels: Labels,
    pub path: ActivePath,
    /// Every step of the file in order, active or not. A step's index here is
    /// its place in the file, counting from 0.
    pub steps: Vec<GraphStep>,
    /// The step that makes the dish: the last active step (0027 rule 19)
    pub root: Option<usize>,
    /// Products, or what is left of them, that no later step uses (rule 20)
    pub by_products: Vec<ByProduct>,
    pub reports: Vec<Report>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStep {
    /// Index into [`Recipe::sections`]
    pub section: usize,
    /// Index into that section's [`Section::content`](crate::Section::content)
    pub content: usize,
    /// Used on this path
    pub active: bool,
    /// The step sits in a `[list]` section: its lines define ingredients
    pub list: bool,
    /// What goes into the step, in the order written
    pub inputs: Vec<Input>,
    /// Steps that take something from this one
    pub consumers: Vec<usize>,
    /// For each [`Item::Group`] in the step, in order: the active option
    pub chosen: Vec<Option<usize>>,
    /// Ingredients on this path, in order, with the options chosen
    pub ingredients: Vec<usize>,
    /// The product the section line names, if this is the section's last step
    pub section_product: Option<usize>,
}

/// Something that goes into a step
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Input {
    /// An ingredient written in the step (index into [`Recipe::ingredients`])
    Ingredient {
        ingredient: usize,
        /// Written in a `[list]` section and brought to this step
        from_list: bool,
    },
    /// The result of another step
    Step {
        from: usize,
        /// Carried on from the step before it (0027 rule 9)
        carry: bool,
        /// Named uses that come along this edge
        uses: Vec<Use>,
    },
    /// `@&name{}` with nothing of that name before it (rule 6)
    Missing { ingredient: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Use {
    pub name: String,
    /// The amount or share taken; `None` takes all that is left
    pub quantity: Option<Quantity>,
    /// The `@&` ingredient that wrote it
    pub ingredient: usize,
    /// It names an ingredient written earlier (Part E), not a product
    pub ingredient_ref: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ByProduct {
    pub name: String,
    pub quantity: Option<Quantity>,
    /// The step that made it
    pub step: usize,
    pub kind: ByProductKind,
    /// The amounts that were used, when only part was
    pub used: Vec<Use>,
    /// Steps that used part of it
    pub used_by: Vec<usize>,
    /// For a product made with a count: how many are left
    pub left: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ByProductKind {
    /// Named and never used
    Unused,
    /// Every use took an amount, so some is left
    Rest,
    /// Made with a count; fewer were used
    Count,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub level: ReportLevel,
    /// The rule of spec proposal 0027 it comes from
    pub rule: u8,
    pub message: String,
    pub step: Option<usize>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReportLevel {
    Error,
    Warning,
    Info,
}

struct Entry {
    key: String,
    name: String,
    node: usize,
    quantity: Option<Quantity>,
    uses: Vec<(usize, Use)>,
    used_up_by: Option<usize>,
    handed_on: bool,
    via: Via,
    section: Option<usize>,
}

#[derive(PartialEq, Clone, Copy)]
enum Via {
    Step,
    Handed,
    Section,
}

/// Builds the graph of `recipe` on the path `selection` picks.
pub fn graph(recipe: &Recipe, selection: &Selection) -> Graph {
    let labels = labels(recipe);
    let path = ActivePath::new(labels.clone(), selection);
    let mut reports = Vec::new();

    // every step, in file order
    let mut steps: Vec<GraphStep> = Vec::new();
    let mut by_place: HashMap<(usize, usize), usize> = HashMap::new();
    for (si, s) in recipe.sections.iter().enumerate() {
        let sec_active = path.is_active(s.label.as_deref());
        for (ci, c) in s.content.iter().enumerate() {
            let Content::Step(st) = c else { continue };
            let mut chosen = Vec::new();
            let mut ingredients = Vec::new();
            active_ingredients(&st.items, &path, &mut chosen, &mut ingredients);
            by_place.insert((si, ci), steps.len());
            steps.push(GraphStep {
                section: si,
                content: ci,
                active: sec_active && path.is_active(st.label.as_deref()),
                list: s.is_list(),
                inputs: Vec::new(),
                consumers: Vec::new(),
                chosen,
                ingredients,
                section_product: None,
            });
        }
    }

    let mut names: HashMap<String, usize> = HashMap::new(); // key -> entry
    let mut entries: Vec<Entry> = Vec::new();
    let mut written: HashMap<usize, usize> = HashMap::new(); // ingredient -> step
    let mut written_keys: HashMap<String, usize> = HashMap::new();
    let mut carry_next: HashMap<usize, usize> = HashMap::new();
    let mut live: Vec<usize> = Vec::new();
    let mut pending_list: Vec<usize> = Vec::new();

    let add_input =
        |steps: &mut Vec<GraphStep>, to: usize, from: usize, carry: bool, u: Option<Use>| {
            if from == to {
                return;
            }
            let existing = steps[to]
                .inputs
                .iter()
                .position(|i| matches!(i, Input::Step { from: f, .. } if *f == from));
            let idx = match existing {
                Some(i) => i,
                None => {
                    steps[to].inputs.push(Input::Step {
                        from,
                        carry: false,
                        uses: Vec::new(),
                    });
                    steps[from].consumers.push(to);
                    steps[to].inputs.len() - 1
                }
            };
            if let Input::Step { carry: c, uses, .. } = &mut steps[to].inputs[idx] {
                *c |= carry;
                if let Some(u) = u {
                    uses.push(u);
                }
            }
        };

    // the last step of the line that `start` begins, before `cur` (rule 23)
    let line_end = |carry_next: &HashMap<usize, usize>, mut s: usize, cur: usize| -> usize {
        while let Some(&n) = carry_next.get(&s) {
            if n >= cur {
                break;
            }
            s = n;
        }
        s
    };

    for (si, sec) in recipe.sections.iter().enumerate() {
        if !path.is_active(sec.label.as_deref()) {
            continue;
        }
        let in_sec: Vec<usize> = (0..steps.len())
            .filter(|&k| steps[k].section == si && steps[k].active)
            .collect();
        if sec.is_list() {
            for &k in &in_sec {
                pending_list.extend(
                    steps[k]
                        .ingredients
                        .iter()
                        .copied()
                        .filter(|&i| !recipe.ingredients[i].name.is_empty()),
                );
            }
            continue;
        }
        let sec_key = sec.product.map(|p| product_key(&recipe.products[p].name));
        let mut prev: Option<usize> = None;
        for (n, &k) in in_sec.iter().enumerate() {
            live.push(k);
            if n == 0 && !pending_list.is_empty() {
                for i in pending_list.drain(..) {
                    steps[k].inputs.push(Input::Ingredient {
                        ingredient: i,
                        from_list: true,
                    });
                }
            }
            let Content::Step(step) = &sec.content[steps[k].content] else {
                unreachable!()
            };
            // rule 9: carry on from the step before, unless it named a product
            if let Some(p) = prev {
                let Content::Step(prev_step) = &sec.content[steps[p].content] else {
                    unreachable!()
                };
                if prev_step.products.is_empty() {
                    add_input(&mut steps, k, p, true, None);
                    carry_next.insert(p, k);
                }
            }

            let mut whole_used: Vec<String> = Vec::new();
            for &i in &steps[k].ingredients.clone() {
                let ing = &recipe.ingredients[i];
                if ing.name.is_empty() {
                    continue; // `@{4}`
                }
                let key = product_key(&ing.name);
                let is_ref = ing.modifiers().contains(Modifiers::REF);
                if !is_ref {
                    steps[k].inputs.push(Input::Ingredient {
                        ingredient: i,
                        from_list: false,
                    });
                    written.insert(i, k);
                    written_keys.insert(key.clone(), k);
                    if names.contains_key(&key) {
                        reports.push(report(
                            5,
                            ReportLevel::Warning,
                            k,
                            format!("'{}' is both an ingredient and a product.", ing.name),
                        ));
                    }
                    continue;
                }
                let u = Use {
                    name: display_name(&ing.name),
                    quantity: ing.quantity.clone(),
                    ingredient: i,
                    ingredient_ref: false,
                };
                let whole = ing.quantity.is_none();
                match ing.relation.references_to() {
                    Some((_, IngredientReferenceTarget::Product)) => {
                        // rule 7: inside its own section, the section's name is
                        // what the section has made so far
                        let own = sec_key.as_deref() == Some(key.as_str())
                            && names
                                .get(&key)
                                .map(|&e| entries[e].section != Some(si))
                                .unwrap_or(true);
                        if own {
                            match prev {
                                Some(p) => add_input(&mut steps, k, p, false, Some(u)),
                                None => {
                                    reports.push(report(7, ReportLevel::Error, k, format!(
                                        "'{}' is used in the first step of its own section, before anything is made.", u.name)));
                                    steps[k].inputs.push(Input::Missing { ingredient: i });
                                }
                            }
                            continue;
                        }
                        match names.get(&key).copied() {
                            Some(e) => {
                                if let Some(by) = entries[e].used_up_by {
                                    reports.push(report(
                                        8,
                                        ReportLevel::Error,
                                        k,
                                        format!(
                                            "'{}' is used after step {} used it up.",
                                            u.name,
                                            by + 1
                                        ),
                                    ));
                                }
                                let from = entries[e].node;
                                add_input(&mut steps, k, from, false, Some(u.clone()));
                                entries[e].uses.push((k, u));
                                if whole {
                                    whole_used.push(key);
                                }
                            }
                            None => {
                                // declared later in the file, or on an inactive path
                                reports.push(report(
                                    6,
                                    ReportLevel::Error,
                                    k,
                                    format!(
                                        "'{}' is used, but nothing of that name is made before it.",
                                        u.name
                                    ),
                                ));
                                steps[k].inputs.push(Input::Missing { ingredient: i });
                            }
                        }
                    }
                    Some((def, IngredientReferenceTarget::Ingredient)) => {
                        // Part E: the ingredient written earlier, as it now is
                        let Some(&src) = written.get(&def) else {
                            steps[k].inputs.push(Input::Missing { ingredient: i });
                            continue;
                        };
                        let from = line_end(&carry_next, src, k);
                        let mut u = u;
                        u.ingredient_ref = true;
                        // rule 23: a line that ended by naming a product holds the
                        // ingredient, and the product's name is the one to use
                        {
                            let Content::Step(end_step) =
                                &recipe.sections[steps[from].section].content[steps[from].content]
                            else {
                                unreachable!()
                            };
                            let made = end_step
                                .products
                                .first()
                                .copied()
                                .or(steps[from].section_product);
                            if let Some(p) = made {
                                reports.push(report(23, ReportLevel::Warning, k, format!(
                                    "The {} went into step {}, which makes '{}'. Use @&{}{{}} instead.",
                                    u.name, from + 1, recipe.products[p].name, recipe.products[p].name)));
                            }
                        }
                        if let Some(q) = &u.quantity {
                            reports.push(report(25, ReportLevel::Info, k, format!(
                                "@&{}{{{}}} takes part of the {} written in step {}. If more {} was meant, write @{} or @+{}.",
                                u.name, q, u.name, src + 1, u.name, u.name, u.name)));
                        }
                        add_input(&mut steps, k, from, false, Some(u));
                    }
                    Some(_) => {
                        // the cooklang-rs step and section counting forms
                        steps[k].inputs.push(Input::Missing { ingredient: i });
                    }
                    None => {
                        reports.push(report(6, ReportLevel::Error, k, format!(
                            "'{}' is used, but no product or ingredient of that name comes before it.", u.name)));
                        steps[k].inputs.push(Input::Missing { ingredient: i });
                    }
                }
            }

            // products named at the end of the step
            for &p in &step.products {
                let prod = &recipe.products[p];
                let key = product_key(&prod.name);
                if written_keys.contains_key(&key) {
                    reports.push(report(
                        5,
                        ReportLevel::Warning,
                        k,
                        format!("'{}' names a product and an ingredient.", prod.name),
                    ));
                }
                if let Some(&old) = names.get(&key) {
                    if whole_used.contains(&key) {
                        entries[old].handed_on = true;
                    } else if entries[old].section != Some(si) {
                        reports.push(report(
                            5,
                            ReportLevel::Warning,
                            k,
                            format!("'{}' is named twice.", prod.name),
                        ));
                    }
                }
                names.insert(key.clone(), entries.len());
                entries.push(Entry {
                    key,
                    name: prod.name.clone(),
                    node: k,
                    quantity: prod.quantity.clone(),
                    uses: Vec::new(),
                    used_up_by: None,
                    handed_on: false,
                    via: Via::Step,
                    section: Some(si),
                });
            }
            if !step.products.is_empty() {
                for key in &whole_used {
                    if step
                        .products
                        .iter()
                        .any(|&p| product_key(&recipe.products[p].name) == *key)
                    {
                        continue;
                    }
                    if let Some(&e) = names.get(key) {
                        entries[e].used_up_by = Some(k);
                    }
                }
            } else {
                // rule 8: a name goes on to what the step makes
                for key in whole_used {
                    let Some(&old) = names.get(&key) else {
                        continue;
                    };
                    entries[old].handed_on = true;
                    let (name, section) = (entries[old].name.clone(), entries[old].section);
                    names.insert(key.clone(), entries.len());
                    entries.push(Entry {
                        key,
                        name,
                        node: k,
                        quantity: None,
                        uses: Vec::new(),
                        used_up_by: None,
                        handed_on: false,
                        via: Via::Handed,
                        section,
                    });
                }
            }
            prev = Some(k);
        }

        // rules 2 and 18: the section's product is what its last step makes
        if let Some(p) = sec.product {
            let prod = &recipe.products[p];
            match prev {
                Some(last) => {
                    let Content::Step(last_step) = &sec.content[steps[last].content] else {
                        unreachable!()
                    };
                    if last_step.products.len() >= 2 {
                        reports.push(report(
                            18,
                            ReportLevel::Warning,
                            last,
                            format!(
                                "Section '{}' names '{}', but its last step names {} products.",
                                sec.name.clone().unwrap_or_default(),
                                prod.name,
                                last_step.products.len()
                            ),
                        ));
                    }
                    let key = product_key(&prod.name);
                    if let Some(&old) = names.get(&key) {
                        if entries[old].node != last {
                            entries[old].handed_on = true;
                        }
                    }
                    steps[last].section_product = Some(p);
                    names.insert(key.clone(), entries.len());
                    entries.push(Entry {
                        key,
                        name: prod.name.clone(),
                        node: last,
                        quantity: prod.quantity.clone(),
                        uses: Vec::new(),
                        used_up_by: None,
                        handed_on: false,
                        via: Via::Section,
                        section: None,
                    });
                }
                None => reports.push(Report {
                    level: ReportLevel::Warning,
                    rule: 2,
                    message: format!(
                        "Section '{}' names '{}' but has no steps.",
                        sec.name.clone().unwrap_or_default(),
                        prod.name
                    ),
                    step: None,
                }),
            }
        }
    }

    // rule 19: the dish is what the last step makes
    let root = live.last().copied();

    // rule 20: by-products; rule 14: checks
    let mut by_products: Vec<ByProduct> = Vec::new();
    for e in &entries {
        if Some(e.node) == root || e.handed_on || e.used_up_by.is_some() {
            continue;
        }
        if e.uses.is_empty() && e.via != Via::Step && !steps[e.node].consumers.is_empty() {
            continue;
        }
        let made_count = count_of(e.quantity.as_ref());
        if let Some(made) = made_count {
            let taken: f64 = e
                .uses
                .iter()
                .filter_map(|(_, u)| count_of(u.quantity.as_ref()))
                .sum();
            let rest_whole = e.uses.iter().any(|(_, u)| u.quantity.is_none());
            if taken > made {
                reports.push(report(
                    14,
                    ReportLevel::Warning,
                    e.node,
                    format!("{taken} of '{}' are used, {made} were made.", e.name),
                ));
            }
            if e.uses.is_empty() {
                by_products.push(by_product(e, ByProductKind::Unused, None));
            } else if taken < made && !rest_whole {
                by_products.push(by_product(e, ByProductKind::Count, Some(made - taken)));
            }
            continue;
        }
        if e.uses.is_empty() {
            if !by_products
                .iter()
                .any(|b| b.step == e.node && product_key(&b.name) == e.key)
            {
                by_products.push(by_product(e, ByProductKind::Unused, None));
            }
        } else if e.uses.iter().all(|(_, u)| u.quantity.is_some()) {
            by_products.push(by_product(e, ByProductKind::Rest, None));
        }
        for (k, u) in &e.uses {
            if let Some(q) = &u.quantity {
                if q.unit().is_none() {
                    if let Value::Number(n) = q.value() {
                        if n.value() > 1.0 {
                            reports.push(report(
                                14,
                                ReportLevel::Warning,
                                *k,
                                format!(
                                    "A share of {} of '{}' is more than the whole.",
                                    n.value(),
                                    e.name
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    Graph {
        labels,
        path,
        steps,
        root,
        by_products,
        reports,
    }
}

fn active_ingredients(
    items: &[Item],
    path: &ActivePath,
    chosen: &mut Vec<Option<usize>>,
    out: &mut Vec<usize>,
) {
    for i in items {
        match i {
            Item::Ingredient { index } => out.push(*index),
            Item::Group { options } => {
                let c = path.active_option(options);
                chosen.push(c);
                if let Some(c) = c {
                    let mut inner = Vec::new();
                    active_ingredients(&options[c].items, path, &mut Vec::new(), &mut inner);
                    out.extend(inner);
                }
            }
            _ => {}
        }
    }
}

fn count_of(q: Option<&Quantity>) -> Option<f64> {
    let q = q?;
    if q.unit().is_some() {
        return None;
    }
    match q.value() {
        Value::Number(n) => Some(n.value()),
        _ => None,
    }
}

fn by_product(e: &Entry, kind: ByProductKind, left: Option<f64>) -> ByProduct {
    ByProduct {
        name: e.name.clone(),
        quantity: e.quantity.clone(),
        step: e.node,
        kind,
        used: e.uses.iter().map(|(_, u)| u.clone()).collect(),
        used_by: e.uses.iter().map(|(k, _)| *k).collect(),
        left,
    }
}

fn display_name(name: &str) -> String {
    name.split_once(':')
        .map(|(a, _)| a)
        .unwrap_or(name)
        .trim()
        .to_string()
}

fn report(rule: u8, level: ReportLevel, step: usize, message: String) -> Report {
    Report {
        level,
        rule,
        message,
        step: Some(step),
    }
}
