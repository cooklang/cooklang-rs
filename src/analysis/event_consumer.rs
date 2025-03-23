use std::collections::HashMap;
use std::str::FromStr;

use crate::convert::{Converter, PhysicalQuantity};
use crate::error::{label, CowStr, PassResult, SourceDiag, SourceReport};
use crate::located::Located;
use crate::metadata::{check_std_entry, StdKey};
use crate::parser::{
    self, BlockKind, Event, IntermediateData, IntermediateRefMode, IntermediateTargetKind,
    Modifiers,
};
use crate::quantity::{Quantity, QuantityValue, ScalableValue, Value};
use crate::span::Span;
use crate::text::Text;
use crate::{model::*, Extensions, ParseOptions};

use super::{AnalysisResult, CheckOptions, DefineMode, DuplicateMode};

macro_rules! error {
    ($msg:expr, $label:expr $(,)?) => {
        $crate::error::SourceDiag::error($msg, $label, $crate::error::Stage::Analysis)
    };
    ($msg:expr) => {
        $crate::error::SourceDiag::unlabeled(
            $msg,
            $crate::error::Severity::Error,
            $crate::error::Stage::Analysis,
        )
    };
}

macro_rules! warning {
    ($msg:expr, $label:expr $(,)?) => {
        $crate::error::SourceDiag::warning($msg, $label, $crate::error::Stage::Analysis)
    };
    ($msg:expr) => {
        $crate::error::SourceDiag::unlabeled(
            $msg,
            $crate::error::Severity::Warning,
            $crate::error::Stage::Analysis,
        )
    };
}

/// Takes an iterator of [events](`Event`) and converts to a full recipe.
///
/// The `input` must be the same that the [events](`Event`) are generated from.
///
/// Probably the iterator you want is an instance of [`PullParser`](crate::parser::PullParser).
#[tracing::instrument(level = "debug", skip_all, target = "cooklang::analysis")]
pub fn parse_events<'i, 'c>(
    events: impl Iterator<Item = Event<'i>>,
    input: &'i str,
    extensions: Extensions,
    converter: &Converter,
    parse_options: ParseOptions,
) -> AnalysisResult {
    let col = RecipeCollector {
        input,
        extensions,
        converter,
        parse_options,

        content: ScalableRecipe {
            metadata: Default::default(),
            sections: Default::default(),
            ingredients: Default::default(),
            cookware: Default::default(),
            timers: Default::default(),
            inline_quantities: Default::default(),
            data: crate::scale::Servings(None),
        },
        current_section: Section::default(),

        define_mode: DefineMode::All,
        duplicate_mode: DuplicateMode::New,
        old_style_metadata: true,
        old_style_metadata_used: vec![],
        ctx: SourceReport::empty(),

        locations: Default::default(),
        step_counter: 1,
    };
    col.parse_events(events)
}

struct RecipeCollector<'i, 'c> {
    input: &'i str,
    extensions: Extensions,
    converter: &'c Converter,
    parse_options: ParseOptions<'c>,

    content: ScalableRecipe,
    current_section: Section,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    old_style_metadata: bool,
    old_style_metadata_used: Vec<Span>,
    ctx: SourceReport,

    locations: Locations<'i>,
    step_counter: u32,
}

#[derive(Default)]
struct Locations<'i> {
    ingredients: Vec<Located<parser::Ingredient<'i>>>,
    cookware: Vec<Located<parser::Cookware<'i>>>,
    metadata: HashMap<StdKey, (Text<'i>, Text<'i>)>,
}

const IMPLICIT_REF_WARN: &str = "The reference (&) is implicit";

impl<'i> RecipeCollector<'i, '_> {
    fn parse_events(mut self, mut events: impl Iterator<Item = Event<'i>>) -> AnalysisResult {
        enum BlockBuffer {
            Step(Vec<Item>),
            Text(String),
        }
        let mut current_block = None;

        let events = events.by_ref();
        while let Some(event) = events.next() {
            match event {
                Event::YAMLFrontMatter(yaml_text) => {
                    self.old_style_metadata = true;
                    self.process_frontmatter(yaml_text);
                }
                Event::Metadata { key, value } => self.metadata(key, value),
                Event::Section { name } => {
                    self.step_counter = 1;
                    if !self.current_section.is_empty() {
                        self.content.sections.push(self.current_section);
                    }
                    self.current_section =
                        Section::new(name.map(|t| t.text_trimmed().into_owned()));
                }
                Event::Start(kind) => {
                    let buffer = if self.define_mode == DefineMode::Text {
                        BlockBuffer::Text(String::new())
                    } else {
                        match kind {
                            BlockKind::Step => BlockBuffer::Step(Vec::new()),
                            BlockKind::Text => BlockBuffer::Text(String::new()),
                        }
                    };
                    current_block = Some(buffer)
                }
                Event::End(kind) => {
                    let new_content = match current_block {
                        Some(BlockBuffer::Step(items)) => {
                            assert_eq!(kind, BlockKind::Step);
                            Content::Step(Step {
                                items,
                                number: self.step_counter,
                            })
                        }
                        Some(BlockBuffer::Text(text)) => {
                            assert!(
                                kind == BlockKind::Text || self.define_mode == DefineMode::Text,
                            );
                            Content::Text(text)
                        }
                        None => panic!("End event without Start"),
                    };

                    // If define mode is ingredients, don't add the
                    // step to the section. The components should have been
                    // added to their lists
                    if self.define_mode != DefineMode::Components || new_content.is_text() {
                        if new_content.is_step() {
                            self.step_counter += 1;
                        }
                        self.current_section.content.push(new_content);
                    }

                    current_block = None;
                }
                item @ (Event::Text(_)
                | Event::Ingredient(_)
                | Event::Cookware(_)
                | Event::Timer(_)) => match &mut current_block {
                    Some(BlockBuffer::Step(items)) => self.in_step(item, items),
                    Some(BlockBuffer::Text(text)) => self.in_text(item, text),
                    None => panic!("Content outside block"),
                },

                Event::Error(e) => {
                    // on a parser error, collect all other parser errors and
                    // warnings
                    self.ctx.error(e);
                    events.for_each(|e| match e {
                        Event::Error(e) | Event::Warning(e) => self.ctx.push(e),
                        _ => {}
                    });
                    // discard non parser errors/warnings
                    self.ctx.retain(|e| e.stage == crate::error::Stage::Parse);
                    // return no output
                    return PassResult::new(None, self.ctx);
                }
                Event::Warning(w) => self.ctx.warn(w),
            }
        }
        if !self.current_section.is_empty() {
            self.content.sections.push(self.current_section);
        }

        if !self.old_style_metadata_used.is_empty() {
            let mut diag =
                warning!("The '>>' syntax for metadata is deprecated, use a YAML frontmatter");
            for span in self.old_style_metadata_used {
                diag.add_label(label!(span));
            }
            if let Ok(yaml_hint) = serde_yaml::to_string(&self.content.metadata.map) {
                diag.add_hint(format!("Replace the entries with this at the top of the document:\n---\n{yaml_hint}---\n"));
            }
            self.ctx.warn(diag);
        }

        PassResult::new(Some(self.content), self.ctx)
    }

    fn process_frontmatter(&mut self, yaml_text: Text<'i>) {
        self.old_style_metadata = false;
        let yaml_str = yaml_text.text();
        let mut yaml_map = match serde_yaml::from_str::<serde_yaml::Mapping>(&yaml_str) {
            Ok(yaml_map) => yaml_map,
            Err(err) => {
                // ! This message (can) contains line and column number, but line numbers
                // ! are off by one thanks to the starting `---`
                let mut diag = error!(err.to_string());
                let err_span = err
                    .location()
                    .map(|loc| Span::pos(yaml_text.span().start() + loc.index()));
                if let Some(loc) = err_span {
                    diag = diag.label(label!(loc));
                }
                self.ctx.error(diag);
                return;
            }
        };

        let mut to_remove = Vec::new();
        for (key, value) in yaml_map.iter() {
            let mut action = CheckOptions::default();
            // run custom validator if any
            if let Some(validator) = self.parse_options.metadata_validator.as_mut() {
                let res = validator(key, value, &mut action);
                if let Some(mut diag) = res.into_source_diag(|| "Invalid metadata entry") {
                    if let Some(key_s) = key.as_str() {
                        if let Some(pos) = yaml_find_key_position(&yaml_str, key_s) {
                            diag.add_label(label!(Span::pos(yaml_text.span().start() + pos)));
                        }
                    }
                    self.ctx.push(diag);
                }
                if !action.include {
                    to_remove.push(key.clone());
                    continue;
                }
            }

            if !action.run_std_checks {
                continue;
            }
            if let Some(sk) = key.as_str().and_then(|s| StdKey::from_str(s).ok()) {
                match check_std_entry(sk, value, self.converter) {
                    Ok(Some(servings)) => self.content.data = servings,
                    Ok(None) => {}
                    Err(err) => {
                        let mut diag = warning!(format!(
                            "Unsupported value for key: '{}'",
                            key.as_str().unwrap()
                        ))
                        .set_source(err);
                        if let Some(key_s) = key.as_str() {
                            if let Some(pos) = yaml_find_key_position(&yaml_str, key_s) {
                                diag.add_label(label!(Span::pos(yaml_text.span().start() + pos)));
                            }
                        }
                        self.ctx.warn(diag);
                    }
                }
            }
        }
        for key in &to_remove {
            yaml_map.shift_remove(key);
        }

        if yaml_map.contains_key(StdKey::Time.as_ref()) {
            // ? I guess I could group calls to `yaml_find_key_pos` into a single
            // iteration of yaml_str but doesn't really matter

            let loc = |key: StdKey| -> Option<usize> {
                yaml_map
                    .contains_key(key.as_ref())
                    .then(|| yaml_find_key_position(&yaml_str, key.as_ref()))
                    .flatten()
            };

            let prep = loc(StdKey::PrepTime);
            let cook = loc(StdKey::CookTime);

            const OVERRIDEN: &str = "this entry is overriden";
            const OVERRIDES: &str = "this entry has preference";

            if prep.is_some() || cook.is_some() {
                let mut w = warning!("Time overriden");
                if let Some(p) = prep {
                    w.add_label(label!(Span::pos(yaml_text.span().start() + p), OVERRIDEN));
                }
                if let Some(p) = cook {
                    w.add_label(label!(Span::pos(yaml_text.span().start() + p), OVERRIDEN));
                }
                if let Some(p) = yaml_find_key_position(&yaml_str, StdKey::Time.as_ref()) {
                    w.add_label(label!(Span::pos(yaml_text.span().start() + p), OVERRIDES));
                }
                w.add_hint(
                    "Top level 'prep time' and/or 'cook time' are not compatible with 'time'",
                );
                self.ctx.warn(w);
            }
        }

        self.content.metadata.map = yaml_map;
    }

    fn metadata(&mut self, key: Text<'i>, value: Text<'i>) {
        let key_t = key.text_trimmed();
        let value_t = value.text_outer_trimmed();
        let invalid_value = |possible| {
            error!(
                format!("Invalid value for config key '{key_t}': {value_t}"),
                label!(value.span(), "this value")
            )
            .label(label!(key.span(), "this key does not support"))
            .hint(format!("Possible values are: {possible:?}"))
        };

        if self.extensions.contains(Extensions::MODES)
            && key_t.starts_with('[')
            && key_t.ends_with(']')
        {
            let config_key = &key_t[1..key_t.len() - 1];
            match config_key {
                "define" | "mode" => match value_t.as_ref() {
                    "all" | "default" => self.define_mode = DefineMode::All,
                    "components" | "ingredients" => self.define_mode = DefineMode::Components,
                    "steps" => self.define_mode = DefineMode::Steps,
                    "text" => self.define_mode = DefineMode::Text,
                    _ => self
                        .ctx
                        .error(invalid_value(vec!["all", "components", "steps", "text"])),
                },
                "duplicate" => match value_t.as_ref() {
                    "new" | "default" => self.duplicate_mode = DuplicateMode::New,
                    "reference" | "ref" => self.duplicate_mode = DuplicateMode::Reference,
                    _ => self.ctx.error(invalid_value(vec!["new", "reference"])),
                },
                _ => {
                    self.ctx.warn(
                        warning!(
                            format!("Unknown config metadata key: {key_t}"),
                            label!(key.span())
                        )
                        .hint(
                            "Possible config keys are '[mode]' and '[duplicate]''",
                        ),
                    );
                    if self.old_style_metadata {
                        self.content.metadata.map.insert(
                            serde_yaml::Value::String(key_t.into_owned()),
                            serde_yaml::Value::String(value_t.into_owned()),
                        );
                    }
                }
            }
            return;
        }

        self.old_style_metadata_used
            .push(Span::new(key.span().start(), value.span().end()));

        let yaml_key = serde_yaml::Value::String(key_t.to_string());
        let yaml_value = serde_yaml::Value::String(value_t.to_string());

        // run custom validator if any
        let mut action = CheckOptions::default();
        if let Some(validator) = self.parse_options.metadata_validator.as_mut() {
            let res = validator(&yaml_key, &yaml_value, &mut action);
            if let Some(mut diag) = res.into_source_diag(|| "Invalid metadata entry") {
                diag.add_label(label!(key.span()));
                diag.add_label(label!(value.span()));
                self.ctx.push(diag);
            }
            if !action.include {
                return;
            }
        }

        // insert the value into the map
        self.content.metadata.map.insert(yaml_key, yaml_value);

        // check if it's a std key
        if !action.run_std_checks {
            return;
        }
        if let Ok(sp_key) = StdKey::from_str(&key_t) {
            let check_result = crate::metadata::check_std_entry(
                sp_key,
                self.content.metadata.map.get(key_t.as_ref()).unwrap(),
                self.converter,
            );

            match check_result {
                Ok(Some(servings)) => self.content.data = servings,
                Ok(None) => {}
                Err(err) => {
                    self.ctx.warn(
                        warning!(
                            format!("Unsupported value for key: '{}'", key.text_trimmed()),
                            label!(value.span(), "this value"),
                        )
                        .label(label!(key.span(), "this key does not support"))
                        .hint("It will be a regular metadata entry")
                        .set_source(err),
                    );
                    return;
                }
            }

            // store it's location if it was inserted
            self.locations
                .metadata
                .insert(sp_key, (key.clone(), value.clone()));

            if matches!(sp_key, StdKey::Time | StdKey::PrepTime | StdKey::CookTime) {
                self.time_override_check(sp_key)
            }
        }
    }

    fn time_override_check(&mut self, new: StdKey) {
        let locs = |keys: &[StdKey]| {
            assert!(!keys.is_empty());
            let mut v = keys
                .iter()
                .filter_map(|k| {
                    self.locations
                        .metadata
                        .get(k)
                        .map(|e| Span::new(e.0.span().start(), e.1.span().end()))
                })
                .collect::<Vec<_>>();
            v.sort_unstable();
            v
        };

        let overrides = locs(&[new])[0];
        let overriden_keys: &[StdKey] = match new {
            StdKey::Time => &[StdKey::PrepTime, StdKey::CookTime],
            StdKey::PrepTime | StdKey::CookTime => &[StdKey::Time],
            _ => panic!("unknown time special key"),
        };
        let overriden = locs(overriden_keys);
        for k in overriden_keys {
            self.locations.metadata.remove(k); // remove the overriden keys
        }
        if overriden.is_empty() {
            return;
        }
        let mut overriden = overriden.iter();

        const OVERRIDEN: &str = "this entry is overriden";
        const OVERRIDES: &str = "by this entry";

        let mut warn = warning!(
            "Time overridden",
            label!(overriden.next().unwrap(), OVERRIDEN)
        );
        for e in overriden {
            warn.add_label(label!(e, OVERRIDEN));
        }
        warn.add_label(label!(overrides, OVERRIDES));
        warn.add_hint("Prep time and/or cook time overrides total time and vice versa");
        self.ctx.warn(warn);
    }

    fn in_step(&mut self, item: Event<'i>, items: &mut Vec<Item>) {
        match item {
            Event::Text(text) => {
                let t = text.text();
                if self.define_mode == DefineMode::Components {
                    // only issue warnings for alphanumeric characters
                    // so that the user can format the text with spaces,
                    // hypens or whatever.
                    if t.contains(|c: char| c.is_alphanumeric()) {
                        self.ctx.warn(warning!(
                            "Ignoring text in define components mode",
                            label!(text.span())
                        ));
                    }
                    return; // ignore text
                }

                if self.extensions.contains(Extensions::INLINE_QUANTITIES) {
                    let mut haystack = t.as_ref();
                    while let Some((before, temperature, after)) =
                        find_inline_quantity(haystack, self.converter)
                    {
                        if !before.is_empty() {
                            items.push(Item::Text {
                                value: before.to_string(),
                            });
                        }

                        items.push(Item::InlineQuantity {
                            index: self.content.inline_quantities.len(),
                        });
                        self.content.inline_quantities.push(temperature);

                        haystack = after;
                    }
                    if !haystack.is_empty() {
                        items.push(Item::Text {
                            value: haystack.to_string(),
                        });
                    }
                } else {
                    items.push(Item::Text {
                        value: t.into_owned(),
                    });
                }
            }

            Event::Ingredient(i) => items.push(Item::Ingredient {
                index: self.ingredient(i),
            }),
            Event::Cookware(i) => items.push(Item::Cookware {
                index: self.cookware(i),
            }),
            Event::Timer(i) => items.push(Item::Timer {
                index: self.timer(i),
            }),

            _ => panic!("Unexpected event in step: {item:?}"),
        };
    }

    fn in_text(&mut self, ev: Event<'i>, s: &mut String) {
        match ev {
            Event::Text(t) => s.push_str(t.text().as_ref()),
            Event::Ingredient(_) | Event::Cookware(_) | Event::Timer(_) => {
                assert_eq!(
                    self.define_mode,
                    DefineMode::Text,
                    "Non text event in text block outside define mode text"
                );

                // ignore component
                let (c, span) = match ev {
                    Event::Ingredient(i) => ("ingredient", i.span()),
                    Event::Cookware(c) => ("cookware", c.span()),
                    Event::Timer(t) => ("timer", t.span()),
                    _ => unreachable!(),
                };
                self.ctx
                    .warn(warning!(format!("Ignoring {c} in text mode"), label!(span)));
                s.push_str(&self.input[span.range()]);
            }
            _ => panic!("Unexpected event in text block: {ev:?}"),
        }
    }

    fn ingredient(&mut self, ingredient: Located<parser::Ingredient<'i>>) -> usize {
        let located_ingredient = ingredient.clone();
        let (ingredient, location) = ingredient.take_pair();

        let mut name = ingredient.name.text_trimmed();
        let reference = parse_reference(&name);

        if let Some(reference) = &reference {
            name = reference.name.clone().into();
        }

        let mut new_igr = Ingredient {
            name: name.into_owned(),
            alias: ingredient.alias.map(|t| t.text_trimmed().into_owned()),
            quantity: ingredient.quantity.clone().map(|q| self.quantity(q, true)),
            note: ingredient.note.map(|n| n.text_trimmed().into_owned()),
            reference,
            modifiers: ingredient.modifiers.into_inner(),
            relation: IngredientRelation::definition(
                Vec::new(),
                self.define_mode != DefineMode::Components,
            ),
        };

        if let Some(inter_data) = ingredient.intermediate_data {
            assert!(new_igr.modifiers().contains(Modifiers::REF));
            let invalid_modifiers = Modifiers::RECIPE | Modifiers::HIDDEN | Modifiers::NEW;
            if new_igr.modifiers().intersects(invalid_modifiers) {
                self.ctx.error(
                    error!(
                        "Conflicting modifiers with intermediate preparation reference",
                        label!(ingredient.modifiers.span())
                    )
                    .hint(format!(
                        "Remove the following modifiers: {}",
                        new_igr.modifiers() & invalid_modifiers
                    )),
                );
            }
            match self.resolve_intermediate_ref(inter_data) {
                Ok(relation) => new_igr.relation = relation,
                Err(error) => self.ctx.error(error),
            }
        } else if let Some((references_to, implicit)) =
            self.resolve_reference(&mut new_igr, location, located_ingredient.modifiers.span())
        {
            assert!(ingredient.intermediate_data.is_none()); // now unreachable, but just to be safe in the future

            let definition = &self.content.ingredients[references_to];
            let definition_location = &self.locations.ingredients[references_to];
            assert!(definition.relation.is_definition());

            if self.extensions.contains(Extensions::ADVANCED_UNITS) {
                if let Some(new_quantity) = &new_igr.quantity {
                    let all_quantities = std::iter::once(references_to)
                        .chain(definition.relation.referenced_from().iter().copied())
                        .filter_map(|index| {
                            self.content.ingredients[index]
                                .quantity
                                .as_ref()
                                .map(|q| (index, q))
                        });
                    for (index, q) in all_quantities {
                        if let Err(e) = q.compatible_unit(new_quantity, self.converter) {
                            let old_q_loc =
                                self.locations.ingredients[index].quantity.as_ref().unwrap();
                            let old = old_q_loc
                                .unit
                                .as_ref()
                                .map(|l| l.span())
                                .unwrap_or(old_q_loc.span());
                            let new_q_loc = located_ingredient.quantity.as_ref().unwrap();
                            let new = new_q_loc
                                .unit
                                .as_ref()
                                .map(|l| l.span())
                                .unwrap_or(new_q_loc.span());

                            let (main_label, support_label) = match &e {
                                crate::quantity::IncompatibleUnits::MissingUnit { lhs, .. } => {
                                    let m = "value missing unit";
                                    let f = "found unit";
                                    if *lhs {
                                        // new is mising
                                        (label!(new, m), label!(old, f))
                                    } else {
                                        // old is missing
                                        (label!(new, f), label!(old, m))
                                    }
                                }
                                crate::quantity::IncompatibleUnits::DifferentPhysicalQuantities {
                                    a: a_q,
                                    b: b_q,
                                } => {
                                    (label!(new, b_q.to_string()), label!(old, a_q.to_string()))
                                }
                                crate::quantity::IncompatibleUnits::UnknownDifferentUnits { .. } => {
                                    (label!(new), label!(old))
                                }
                            };

                            self.ctx.warn(
                                warning!(
                                    "Incompatible units prevent calculating total amount",
                                    main_label
                                )
                                .label(support_label)
                                .set_source(e),
                            )
                        }
                    }
                }
            }

            if let Some(note) = &located_ingredient.note {
                self.ctx.error(note_reference_error(
                    note.span(),
                    implicit,
                    definition_location.span(),
                    definition_location.note.as_ref().map(|n| n.span()),
                ));
            }

            // When the ingredient is not defined in a step, only the definition
            // or the references can have quantities.
            // This is to avoid confusion when calculating the total amount.
            //  - If the user defines the ingredient in a ingredient list with
            //    a quantity and later references it with a quantity, what does
            //    the definition quantity mean? total? partial and the reference
            //    a portion used? Too messy. This situation is prohibited
            //  - If the user defines the ingredient directly in a step, it's
            //    quantity is used there, and the total is the sum of itself and
            //    all of its references. All clear.
            if definition.quantity.is_some()
                && new_igr.quantity.is_some()
                && !definition
                    .relation
                    .is_defined_in_step()
                    .expect("definition")
            {
                self.ctx.error(conflicting_reference_quantity_error(
                    ingredient.quantity.unwrap().span(),
                    definition_location.span(),
                    implicit,
                ));
            }

            // text value warning
            if let Some((ref_q, def_q)) =
                &new_igr.quantity.as_ref().zip(definition.quantity.as_ref())
            {
                let ref_is_text = ref_q.value().is_text();
                let def_is_text = def_q.value().is_text();

                if ref_is_text != def_is_text {
                    let ref_q_loc = located_ingredient.quantity.as_ref().unwrap().span();
                    let def_q_loc = definition_location.quantity.as_ref().unwrap().span();

                    let (text_quantity_span, number_quantity_span) = if ref_is_text {
                        (ref_q_loc, def_q_loc)
                    } else {
                        (def_q_loc, ref_q_loc)
                    };

                    self.ctx.warn(text_val_in_ref_warn(
                        text_quantity_span,
                        number_quantity_span,
                        implicit,
                    ));
                }
            }

            Ingredient::set_referenced_from(&mut self.content.ingredients, references_to);
        }

        if new_igr.modifiers.contains(Modifiers::RECIPE)
            && !new_igr.modifiers.contains(Modifiers::REF)
        {
            if let Some(checker) = self.parse_options.recipe_ref_check.as_mut() {
                let res = checker(&new_igr.name);
                if let Some(mut diag) = res
                    .into_source_diag(|| format!("Referenced recipe not found: {}", new_igr.name))
                {
                    diag.add_label(label!(location));
                    self.ctx.push(diag);
                }
            }
        }

        self.locations.ingredients.push(located_ingredient);
        self.content.ingredients.push(new_igr);
        self.content.ingredients.len() - 1
    }

    fn resolve_intermediate_ref(
        &mut self,
        inter_data: Located<IntermediateData>,
    ) -> Result<IngredientRelation, SourceDiag> {
        use IntermediateRefMode as Mode;
        use IntermediateTargetKind as Kind;
        assert!(!inter_data.val.is_negative());
        let val = inter_data.val as u32;

        const INVALID: &str = "Invalid intermediate preparation reference";

        if val == 0 {
            match inter_data.ref_mode {
                Mode::Number => {
                    return Err(error!(
                        format!("{INVALID}: number is 0"),
                        label!(inter_data.span())
                    )
                    .hint("Step and section numbers start at 1"));
                }
                Mode::Relative => {
                    return Err(error!(
                        format!("{INVALID}: relative reference to self"),
                        label!(inter_data.span())
                    )
                    .hint("Relative reference value has to be greater than 0"));
                }
            }
        }

        let bounds = |help: String| {
            Err(error!(
                format!("{INVALID}: value out of bounds"),
                label!(inter_data.span())
            )
            .hint(help))
        };

        let relation = match (inter_data.target_kind, inter_data.ref_mode) {
            (Kind::Step, Mode::Number) => {
                let index = self
                    .current_section
                    .content
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.is_step().then_some(i))
                    .nth((val - 1) as usize);

                if index.is_none() {
                    return bounds(format!(
                        "The value has to be a previous step number: {}",
                        // -1 because step_counter holds the current step number
                        match self.step_counter.saturating_sub(1) {
                            0 => "no steps before this one".to_string(),
                            1 => "1".to_string(),
                            max => format!("1 to {max}"),
                        }
                    ));
                }

                IngredientRelation::reference(index.unwrap(), IngredientReferenceTarget::Step)
            }
            (Kind::Step, Mode::Relative) => {
                let index = self
                    .current_section
                    .content
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.is_step().then_some(i))
                    .nth_back((val - 1) as usize);
                if index.is_none() {
                    return bounds(format!(
                        "The current section {} steps before this one",
                        match self.step_counter.saturating_sub(1) {
                            0 => "has no".to_string(),
                            before => format!("only has {before}"),
                        }
                    ));
                }

                IngredientRelation::reference(index.unwrap(), IngredientReferenceTarget::Step)
            }
            (Kind::Section, Mode::Number) => {
                let index = (val - 1) as usize; // direct index, but make it 0 indexed

                if index >= self.content.sections.len() {
                    return bounds(format!(
                        "The value has to be a previous section number: {}",
                        match self.content.sections.len() {
                            0 => "no sections before this one".to_string(),
                            1 => "1".to_string(),
                            max => format!("1 to {max}"),
                        }
                    ));
                }

                IngredientRelation::reference(index, IngredientReferenceTarget::Section)
            }
            (Kind::Section, Mode::Relative) => {
                let val = val as usize; // number of sections to go back

                // content.sections holds the past sections
                if val > self.content.sections.len() {
                    return bounds(format!(
                        "The recipe {} sections before this one",
                        match self.content.sections.len() {
                            0 => "has no".to_string(),
                            before => format!("only has {before}"),
                        }
                    ));
                }

                // number of past sections - number to go back
                // val is at least 1, so the first posibility is the prev section index
                // val is checked to be smaller or equal, if equal, get 0, the index
                let index = self.content.sections.len().saturating_sub(val);
                IngredientRelation::reference(index, IngredientReferenceTarget::Section)
            }
        };
        Ok(relation)
    }

    fn cookware(&mut self, cookware: Located<parser::Cookware<'i>>) -> usize {
        let located_cookware = cookware.clone();
        let (cookware, location) = cookware.take_pair();

        let mut new_cw = Cookware {
            name: cookware.name.text_trimmed().into_owned(),
            alias: cookware.alias.map(|t| t.text_trimmed().into_owned()),
            quantity: cookware.quantity.map(|q| self.value(q.into_inner(), false)),
            note: cookware.note.map(|n| n.text_trimmed().into_owned()),
            modifiers: cookware.modifiers.into_inner(),
            relation: ComponentRelation::Definition {
                referenced_from: Vec::new(),
                defined_in_step: self.define_mode != DefineMode::Components,
            },
        };

        if let Some((references_to, implicit)) =
            self.resolve_reference(&mut new_cw, location, located_cookware.modifiers.span())
        {
            let definition = &self.content.cookware[references_to];
            let definition_location = &self.locations.cookware[references_to];
            assert!(definition.relation.is_definition());

            if let Some(note) = &located_cookware.note {
                self.ctx.error(note_reference_error(
                    note.span(),
                    implicit,
                    definition_location.span(),
                    definition_location.note.as_ref().map(|n| n.span()),
                ));
            }

            // See ingredients for explanation
            if definition.quantity.is_some()
                && new_cw.quantity.is_some()
                && !definition
                    .relation
                    .is_defined_in_step()
                    .expect("definition")
            {
                self.ctx.error(conflicting_reference_quantity_error(
                    located_cookware.quantity.as_ref().unwrap().span(),
                    definition_location.span(),
                    implicit,
                ));
            }

            // text value warning
            if let Some((ref_q, def_q)) =
                &new_cw.quantity.as_ref().zip(definition.quantity.as_ref())
            {
                let ref_is_text = ref_q.is_text();
                let def_is_text = def_q.is_text();

                if ref_is_text != def_is_text {
                    let ref_q_loc = located_cookware.quantity.as_ref().unwrap().span();
                    let def_q_loc = definition_location.quantity.as_ref().unwrap().span();

                    let (text_quantity_span, number_quantity_span) = if ref_is_text {
                        (ref_q_loc, def_q_loc)
                    } else {
                        (def_q_loc, ref_q_loc)
                    };

                    self.ctx.warn(text_val_in_ref_warn(
                        text_quantity_span,
                        number_quantity_span,
                        implicit,
                    ));
                }
            }

            Cookware::set_referenced_from(&mut self.content.cookware, references_to);
        }

        self.locations.cookware.push(located_cookware);
        self.content.cookware.push(new_cw);
        self.content.cookware.len() - 1
    }

    fn timer(&mut self, timer: Located<parser::Timer<'i>>) -> usize {
        let located_timer = timer.clone();
        let (timer, _span) = timer.take_pair();
        let quantity = timer.quantity.map(|q| {
            let quantity = self.quantity(q, false);
            if self.extensions.contains(Extensions::ADVANCED_UNITS) {
                let located_quantity = located_timer.quantity.as_ref().unwrap();
                if quantity.value().is_text() {
                    self.ctx.error(error!(
                        format!("Timer value is text: {}", quantity.value()),
                        label!(located_quantity.value.span(), "expected a number here")
                    ));
                }
                if let Some(unit_text) = quantity.unit() {
                    let unit_span = located_quantity.unit.as_ref().unwrap().span();
                    match quantity.unit_info(self.converter) {
                        Some(unit) => {
                            if unit.physical_quantity != PhysicalQuantity::Time {
                                self.ctx.error(error!(
                                    format!("Timer unit is not time: {unit}"),
                                    label!(
                                        unit_span,
                                        "expected time, not {}",
                                        unit.physical_quantity
                                    )
                                ));
                            }
                        }
                        None => self.ctx.error(error!(
                            format!("Unknown timer unit: {unit_text}"),
                            label!(unit_span, "expected time unit")
                        )),
                    }
                }
            }
            quantity
        });

        let new_timer = Timer {
            name: timer.name.map(|t| t.text_trimmed().into_owned()),
            quantity,
        };

        self.content.timers.push(new_timer);
        self.content.timers.len() - 1
    }

    fn quantity(
        &mut self,
        quantity: Located<parser::Quantity<'i>>,
        is_ingredient: bool,
    ) -> Quantity<ScalableValue> {
        let parser::Quantity { value, unit, .. } = quantity.into_inner();
        Quantity::new(
            self.value(value, is_ingredient),
            unit.map(|t| t.text_trimmed().into_owned()),
        )
    }

    fn value(&mut self, value: parser::QuantityValue, is_ingredient: bool) -> ScalableValue {
        let parser::QuantityValue { value, scaling_lock } = value;
        let has_scaling_lock = scaling_lock.is_some();
        let is_text = value.is_text();

        // For ingredients without text values and without scaling lock, use Linear
        if is_ingredient && !is_text && !has_scaling_lock {
            return ScalableValue::Linear(value.into_inner());
        }

        // Warn if scaling lock is used unnecessarily (on non-ingredients or text values)
        if has_scaling_lock {
            let mut warning = warning!(
                "Unnecessary scaling lock modifier",
                label!(value.span(), "this scaling lock has no effect")
            );

            if !is_ingredient {
                warning.add_hint("Only ingredients can be scaled, scaling lock is not needed here");
            } else if is_text {
                warning.add_hint("Text values cannot be scaled, scaling lock is not needed here");
            }

            self.ctx.warn(warning);
        }

        // Everything else uses Fixed
        ScalableValue::Fixed(value.into_inner())
    }

    fn resolve_reference<C: RefComponent>(
        &mut self,
        new: &mut C,
        location: Span,
        modifiers_location: Span,
    ) -> Option<(usize, bool)> {
        let new_name = unicase::UniCase::new(new.name());

        let all = C::all(&self.content);
        // find the LAST component with the same name, lazy
        let same_name_cell = std::cell::OnceCell::new();
        let same_name = || {
            *same_name_cell.get_or_init(|| {
                C::all(&self.content).iter().rposition(|other: &C| {
                    !other.modifiers().contains(Modifiers::REF)
                        && new_name == unicase::UniCase::new(other.name())
                })
            })
        };

        let conflicing_modifiers = |conflict: Modifiers, help: CowStr, implicit: bool| {
            let mut e = error!(
                format!("Unsupported modifier combination with reference: {conflict}"),
                label!(modifiers_location)
            )
            .hint(help);
            if implicit {
                e.add_hint(IMPLICIT_REF_WARN);
            }
            e
        };

        let redundant_modifier = |redundant: &'static str, help: String| {
            warning!(
                format!("Redundant {redundant} modifier"),
                label!(modifiers_location)
            )
            .hint(help)
            .hint(format!(
                "In the current mode, by default, {}",
                match (self.define_mode, self.duplicate_mode) {
                    (DefineMode::Steps, _) => "all components are references",
                    (_, DuplicateMode::Reference) =>
                        "components are definitions but duplicates are references",
                    _ => "all components are definitions",
                }
            ))
        };

        // no new and ref -> error
        if new.modifiers().contains(Modifiers::NEW | Modifiers::REF) {
            self.ctx.error(conflicing_modifiers(
                *new.modifiers(),
                "New (+) can never be combined with ref (&)".into(),
                false,
            ));
            return None;
        }

        // no new -> maybe warning for redundant
        if new.modifiers().contains(Modifiers::NEW) {
            if self.define_mode != DefineMode::Steps {
                if self.duplicate_mode == DuplicateMode::Reference && same_name().is_none() {
                    self.ctx.warn(redundant_modifier(
                        "new (+)",
                        format!("There are no {}s with the same name before", C::container()),
                    ));
                } else if self.duplicate_mode == DuplicateMode::New {
                    self.ctx.warn(redundant_modifier(
                        "new (+)",
                        format!("This {} is already a definition", C::container()),
                    ));
                }
            }
            return None;
        }

        // warning for redundant ref
        if (self.duplicate_mode == DuplicateMode::Reference
            || self.define_mode == DefineMode::Steps)
            && new.modifiers().contains(Modifiers::REF)
        {
            self.ctx.warn(redundant_modifier(
                "reference (&)",
                format!("This {} is already a reference", C::container()),
            ));
        }

        let treat_as_reference = new.modifiers().contains(Modifiers::REF)
            || self.define_mode == DefineMode::Steps
            || self.duplicate_mode == DuplicateMode::Reference && same_name().is_some();

        if !treat_as_reference {
            return None;
        }

        // the reference is implicit if we are here (is a reference) and the
        // reference modifier is not set
        let implicit = !new.modifiers().contains(Modifiers::REF);

        if let Some(references_to) = same_name() {
            let referenced = &all[references_to];
            assert!(!referenced.modifiers().contains(Modifiers::REF));

            // Set of inherited modifiers from the definition
            let inherited = *referenced.modifiers() & C::inherit_modifiers();
            // Set of conflict modifiers
            //   - any modifiers not inherited
            //   - is not ref
            // except ref and new, the only modifiers a reference can have is those inherited
            // from the definition. And if it has it's not treated as a reference.
            let conflict = *new.modifiers() & !inherited & !Modifiers::REF;

            // Apply inherited
            *new.modifiers_mut() |= inherited;

            // Set it as a reference
            *new.modifiers_mut() |= Modifiers::REF;
            new.set_reference(references_to);

            if !conflict.is_empty() {
                let help = {
                    let extra = conflict
                        .iter_names()
                        .map(|(s, _)| s.to_lowercase())
                        .collect::<Vec<_>>()
                        .join(", ");
                    if implicit {
                        format!("Mark the definition as {extra} or add new (+) to this")
                    } else {
                        format!("Mark the definition as {extra} or remove the reference (&)")
                    }
                };
                self.ctx
                    .error(conflicing_modifiers(conflict, help.into(), implicit));
            }

            // extra reference checks
            Some((references_to, implicit))
        } else {
            self.ctx.error({
                let mut e = error!(
                    format!("Reference not found: {}", new.name()),
                    label!(location)
                )
                .hint(format!(
                    "A non reference {} with the same name defined BEFORE cannot be found",
                    C::container()
                ));
                if implicit {
                    e.add_hint(IMPLICIT_REF_WARN);
                }
                e
            });
            None
        }
    }
}

trait RefComponent: Sized {
    fn name(&self) -> &str;
    fn modifiers(&self) -> &Modifiers;
    fn modifiers_mut(&mut self) -> &mut Modifiers;

    fn inherit_modifiers() -> Modifiers;

    fn container() -> &'static str;

    fn set_reference(&mut self, references_to: usize);
    fn set_referenced_from(all: &mut [Self], references_to: usize);

    fn all(content: &ScalableRecipe) -> &[Self];
}

impl RefComponent for Ingredient<ScalableValue> {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    #[inline]
    fn modifiers_mut(&mut self) -> &mut Modifiers {
        &mut self.modifiers
    }

    #[inline]
    fn inherit_modifiers() -> Modifiers {
        Modifiers::HIDDEN | Modifiers::OPT | Modifiers::RECIPE
    }

    #[inline]
    fn container() -> &'static str {
        "ingredient"
    }

    #[inline]
    fn set_reference(&mut self, references_to: usize) {
        self.relation =
            IngredientRelation::reference(references_to, IngredientReferenceTarget::Ingredient);
    }

    fn set_referenced_from(all: &mut [Self], references_to: usize) {
        let new_index = all.len();
        match all[references_to].relation.referenced_from_mut() {
            Some(referenced_from) => {
                referenced_from.push(new_index);
            }
            None => panic!("Reference to reference"),
        }
    }

    #[inline]
    fn all(content: &ScalableRecipe) -> &[Self] {
        &content.ingredients
    }
}

impl RefComponent for Cookware<ScalableValue> {
    #[inline]
    fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    fn modifiers(&self) -> &Modifiers {
        &self.modifiers
    }

    #[inline]
    fn modifiers_mut(&mut self) -> &mut Modifiers {
        &mut self.modifiers
    }

    #[inline]
    fn inherit_modifiers() -> Modifiers {
        Modifiers::HIDDEN | Modifiers::OPT
    }

    #[inline]
    fn container() -> &'static str {
        "cookware item"
    }

    #[inline]
    fn set_reference(&mut self, references_to: usize) {
        self.relation = ComponentRelation::Reference { references_to };
    }

    fn set_referenced_from(all: &mut [Self], references_to: usize) {
        let new_index = all.len();
        match &mut all[references_to].relation {
            ComponentRelation::Definition {
                referenced_from, ..
            } => referenced_from.push(new_index),
            ComponentRelation::Reference { .. } => panic!("Reference to reference"),
        }
    }

    #[inline]
    fn all(content: &ScalableRecipe) -> &[Self] {
        &content.cookware
    }
}

fn find_inline_quantity<'a>(
    text: &'a str,
    converter: &Converter,
) -> Option<(&'a str, Quantity<Value>, &'a str)> {
    let mut i = 0;

    fn eat_word<'a>(text: &'a str, i: &mut usize) -> Option<&'a str> {
        let s = &text[*i..];
        let offset = s.find(|c: char| c.is_whitespace()).or({
            // if no whitespace until the end if there is anything left
            if !s.is_empty() {
                Some(s.len())
            } else {
                None
            }
        })?;
        let word = &s[..offset];
        *i += offset;
        Some(word)
    }

    fn eat_whitespace<'a>(text: &'a str, i: &mut usize) -> Option<&'a str> {
        let offset = text[*i..].find(|c: char| !c.is_whitespace())?;
        let ws = &text[*i..*i + offset];
        *i += offset;
        Some(ws)
    }

    #[cfg(debug_assertions)]
    let mut prev = 0;
    while let Some(offset) = text[i..].find(|c: char| c.is_ascii_digit()) {
        i += offset;

        // get "before" slice and check negative
        let before: &str;
        let neg: bool;
        if i > 0 && text.as_bytes()[i - 1] == b'-' {
            before = &text[..i - 1];
            neg = true;
        } else {
            before = &text[..i];
            neg = false;
        }

        let w1 = eat_word(text, &mut i)?; // if no words, no more quantities
        let first_non_digit =
            w1.find(|c: char| !c.is_ascii_digit() && c != '.' && !c.is_whitespace());
        let (mut number, mut unit) = if let Some(mid) = first_non_digit {
            // split after number for unit
            w1.split_at(mid)
        } else {
            // or take next word as unit
            let _ = eat_whitespace(text, &mut i);
            let w2 = eat_word(text, &mut i)?; // if no words, no more quantities
            (w1, w2)
        };

        number = number.trim();
        unit = unit.trim();

        #[cfg(debug_assertions)]
        {
            debug_assert!(prev < i); // to be sure no infinite loop
            prev = i;
        }

        let after = &text[i..];
        let Ok(mut number) = number.parse::<f64>() else {
            continue;
        };
        if converter.find_unit(unit).is_none() {
            continue;
        };

        if neg {
            number = -number;
        }

        let q = Quantity::new(Value::from(number), Some(unit.to_string()));
        return Some((before, q, after));
    }

    None
}

fn note_reference_error(
    span: Span,
    implicit: bool,
    def_span: Span,
    def_note_span: Option<Span>,
) -> SourceDiag {
    let span = Span::new(span.start().saturating_sub(1), span.end() + 1);

    let mut e = error!("Note not allowed in reference", label!(span, "remove this"));

    if let Some(sp) = def_note_span {
        e.add_label(label!(sp, "the definition already has a note"));
    } else {
        e.add_hint("Add the note in the definition of the ingredient");
        e.add_label(label!(Span::pos(def_span.end()), "add the note here"));
    }

    if implicit {
        e.add_hint(IMPLICIT_REF_WARN);
    }
    e
}

fn conflicting_reference_quantity_error(
    ref_quantity_span: Span,
    def_span: Span,
    implicit: bool,
) -> SourceDiag {
    let mut e = error!(
        "Conflicting component reference quantities",
        label!(ref_quantity_span, "reference with quantity")
    )
    .label(label!(
        def_span,
        "definition with quantity outside a step"
    ))
    .hint("If the component is not defined in a step and has a quantity, its references cannot have a quantity");
    if implicit {
        e.add_hint(IMPLICIT_REF_WARN);
    }
    e
}

fn text_val_in_ref_warn(
    text_quantity_span: Span,
    number_quantity_span: Span,
    implicit: bool,
) -> SourceDiag {
    let mut w = warning!(
        "Text value may prevent calculating total amount",
        label!(text_quantity_span, "can't operate with text value")
    )
    .label(label!(number_quantity_span, "numeric value"))
    .hint("Use numeric values so they can be added together");
    if implicit {
        w.add_hint(IMPLICIT_REF_WARN);
    }
    w
}

fn yaml_find_key_position(text: &str, key: &str) -> Option<usize> {
    // This is a bit of a hack, but it will work almost always and if it doesn't
    // it only tells the user a bad position

    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let l_offset = offset;
        offset += line.len();
        let line = line.trim_start();

        let Some((k, _)) = line.split_once(':') else {
            continue;
        };
        let Some(start) = k.find(|c: char| !c.is_ascii_whitespace()) else {
            continue;
        };
        if k[start..].trim_ascii_end() == key {
            return Some(l_offset + start);
        }
    }
    None
}

fn parse_reference(name: &str) -> Option<RecipeReference> {
    if name.starts_with("./") || name.starts_with("../") || name.starts_with(".\\") || name.starts_with("..\\") {
        let path = name.replace('\\', "/");
        let mut components: Vec<String> = path.split('/').map(String::from).skip(1).collect();
        let file_stem = components.pop().unwrap();
        Some(RecipeReference {
            components,
            name: file_stem.into()
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reference() {
        // Test Unix-style paths
        assert_eq!(
            parse_reference("./pasta/spaghetti"),
            Some(RecipeReference {
                components: vec!["pasta".to_string()],
                name: "spaghetti".into()
            })
        );

        assert_eq!(
            parse_reference("../sauces/tomato"),
            Some(RecipeReference {
                components: vec!["sauces".to_string()],
                name: "tomato".into()
            })
        );

        // Test Windows-style paths
        assert_eq!(
            parse_reference(r#".\pasta\spaghetti"#),
            Some(RecipeReference {
                components: vec!["pasta".to_string()],
                name: "spaghetti".into()
            })
        );

        assert_eq!(
            parse_reference(r#"..\sauces\tomato"#),
            Some(RecipeReference {
                components: vec!["sauces".to_string()],
                name: "tomato".into()
            })
        );

        // Test deeper paths
        assert_eq!(
            parse_reference("./recipes/italian/pasta/spaghetti"),
            Some(RecipeReference {
                components: vec!["recipes".to_string(), "italian".to_string(), "pasta".to_string()],
                name: "spaghetti".into()
            })
        );

        // Test paths with no components (just file)
        assert_eq!(
            parse_reference("./spaghetti"),
            Some(RecipeReference {
                components: vec![],
                name: "spaghetti".into()
            })
        );

        // Test non-path names (should return None)
        assert_eq!(parse_reference("spaghetti"), None);
        assert_eq!(parse_reference("pasta/spaghetti"), None);
        assert_eq!(parse_reference("pasta\\spaghetti"), None);
        assert_eq!(parse_reference("/pasta/spaghetti"), None);
        assert_eq!(parse_reference("\\pasta\\spaghetti"), None);
    }
}
