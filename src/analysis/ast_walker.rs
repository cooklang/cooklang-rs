use std::borrow::Cow;
use std::collections::HashMap;

use regex::Regex;

use crate::ast::{self, IntermediateData, Modifiers, Text};
use crate::context::Context;
use crate::convert::{Converter, PhysicalQuantity};
use crate::error::{CooklangError, CooklangWarning};
use crate::located::Located;
use crate::parser::{BlockKind, Event};
use crate::quantity::{Quantity, QuantityValue, ScalableValue, UnitInfo, Value};
use crate::span::Span;
use crate::{model::*, Extensions, RecipeRefCheckResult, RecipeRefChecker};

use super::{AnalysisError, AnalysisResult, AnalysisWarning, DefineMode, DuplicateMode};

#[tracing::instrument(level = "debug", skip_all, target = "cooklang::analysis")]
pub fn parse_events<'i>(
    events: impl Iterator<Item = Event<'i>>,
    extensions: Extensions,
    converter: &Converter,
    recipe_ref_checker: Option<RecipeRefChecker>,
) -> AnalysisResult {
    let mut ctx = Context::default();
    let temperature_regex = extensions
        .contains(Extensions::TEMPERATURE)
        .then(|| match converter.temperature_regex() {
            Ok(re) => Some(re),
            Err(source) => {
                ctx.warn(AnalysisWarning::TemperatureRegexCompile { source });
                None
            }
        })
        .flatten();

    let col = RecipeCollector {
        extensions,
        temperature_regex,
        converter,
        recipe_ref_checker,

        content: ScalableRecipe {
            metadata: Default::default(),
            sections: Default::default(),
            ingredients: Default::default(),
            cookware: Default::default(),
            timers: Default::default(),
            inline_quantities: Default::default(),
            data: (),
        },
        current_section: Section::default(),

        define_mode: DefineMode::All,
        duplicate_mode: DuplicateMode::New,
        auto_scale_ingredients: false,
        ctx,

        locations: Default::default(),
        step_counter: 1,
    };
    col.parse_events(events)
}

struct RecipeCollector<'i, 'c> {
    extensions: Extensions,
    temperature_regex: Option<&'c Regex>,
    converter: &'c Converter,
    recipe_ref_checker: Option<RecipeRefChecker<'c>>,

    content: ScalableRecipe,
    current_section: Section,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    auto_scale_ingredients: bool,
    ctx: Context<CooklangError, CooklangWarning>,

    locations: Locations<'i>,
    step_counter: u32,
}

#[derive(Default)]
struct Locations<'i> {
    ingredients: Vec<Located<ast::Ingredient<'i>>>,
    cookware: Vec<Located<ast::Cookware<'i>>>,
    metadata: HashMap<Cow<'i, str>, (Text<'i>, Text<'i>)>,
}

impl<'i, 'c> RecipeCollector<'i, 'c> {
    fn parse_events(mut self, mut events: impl Iterator<Item = Event<'i>>) -> AnalysisResult {
        let mut items = Vec::new();
        let events = events.by_ref();
        while let Some(event) = events.next() {
            match event {
                Event::Metadata { key, value } => self.metadata(key, value),
                Event::Section { name } => {
                    self.step_counter = 1;
                    if !self.current_section.is_empty() {
                        self.content.sections.push(self.current_section);
                    }
                    self.current_section =
                        Section::new(name.map(|t| t.text_trimmed().into_owned()));
                }
                Event::Start(_kind) => items.clear(),
                Event::End(kind) => {
                    let new_content = match kind {
                        _ if self.define_mode == DefineMode::Text => {
                            Content::Text(self.text_block(items))
                        }
                        BlockKind::Step => Content::Step(self.step(items)),
                        BlockKind::Text => Content::Text(self.text_block(items)),
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

                    items = Vec::new();
                }
                item @ (Event::Text(_)
                | Event::Ingredient(_)
                | Event::Cookware(_)
                | Event::Timer(_)) => items.push(item),

                Event::Error(e) => {
                    // on a parser error, collect all other parser errors and
                    // warnings
                    self.ctx.error(e);
                    events.for_each(|e| match e {
                        Event::Error(e) => self.ctx.error(e),
                        Event::Warning(w) => self.ctx.warn(w),
                        _ => {}
                    });
                    // discard non parser errors/warnings
                    self.ctx
                        .errors
                        .retain(|e| matches!(e, CooklangError::Parser(_)));
                    self.ctx
                        .warnings
                        .retain(|e| matches!(e, CooklangWarning::Parser(_)));
                    // return no output
                    return self.ctx.finish(None);
                }
                Event::Warning(w) => self.ctx.warn(w),
            }
        }
        if !self.current_section.is_empty() {
            self.content.sections.push(self.current_section);
        }
        self.ctx.finish(Some(self.content))
    }

    fn metadata(&mut self, key: Text<'i>, value: Text<'i>) {
        self.locations
            .metadata
            .insert(key.text_trimmed(), (key.clone(), value.clone()));

        let invalid_value = |possible_values| AnalysisError::InvalidSpecialMetadataValue {
            key: key.located_string_trimmed(),
            value: value.located_string_trimmed(),
            possible_values,
        };

        let key_t = key.text_trimmed();
        let value_t = value.text_trimmed();
        if self.extensions.contains(Extensions::MODES)
            && key_t.starts_with('[')
            && key_t.ends_with(']')
        {
            let special_key = &key_t[1..key_t.len() - 1];
            match special_key {
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
                "auto scale" | "auto_scale" => match value_t.as_ref() {
                    "true" => self.auto_scale_ingredients = true,
                    "false" | "default" => self.auto_scale_ingredients = false,
                    _ => self.ctx.error(invalid_value(vec!["true", "false"])),
                },
                _ => {
                    self.ctx.warn(AnalysisWarning::UnknownSpecialMetadataKey {
                        key: key.located_string_trimmed(),
                    });
                    self.content
                        .metadata
                        .map
                        .insert(key_t.into_owned(), value_t.into_owned());
                }
            }
        } else if let Err(warn) = self
            .content
            .metadata
            .insert(key_t.into_owned(), value_t.into_owned())
        {
            self.ctx.warn(AnalysisWarning::InvalidMetadataValue {
                key: key.located_string_trimmed(),
                value: value.located_string_trimmed(),
                source: warn,
            });
        }
    }

    fn step(&mut self, items: Vec<Event<'i>>) -> Step {
        let mut new_items = Vec::new();

        for item in items {
            match item {
                Event::Text(text) => {
                    let t = text.text();
                    if self.define_mode == DefineMode::Components {
                        // only issue warnings for alphanumeric characters
                        // so that the user can format the text with spaces,
                        // hypens or whatever.
                        if t.contains(|c: char| c.is_alphanumeric()) {
                            self.ctx.warn(AnalysisWarning::TextDefiningIngredients {
                                text_span: text.span(),
                            });
                        }
                        continue; // ignore text
                    }

                    // it's only some if the extension is enabled
                    if let Some(re) = &self.temperature_regex {
                        debug_assert!(self.extensions.contains(Extensions::TEMPERATURE));

                        let mut haystack = t.as_ref();
                        while let Some((before, temperature, after)) =
                            find_temperature(haystack, re)
                        {
                            if !before.is_empty() {
                                new_items.push(Item::Text {
                                    value: before.to_string(),
                                });
                            }

                            new_items.push(Item::InlineQuantity {
                                index: self.content.inline_quantities.len(),
                            });
                            self.content.inline_quantities.push(temperature);

                            haystack = after;
                        }
                        if !haystack.is_empty() {
                            new_items.push(Item::Text {
                                value: haystack.to_string(),
                            });
                        }
                    } else {
                        new_items.push(Item::Text {
                            value: t.into_owned(),
                        });
                    }
                }

                Event::Ingredient(..) | Event::Cookware(..) | Event::Timer(..) => {
                    let new_component = match item {
                        Event::Ingredient(i) => Item::Ingredient {
                            index: self.ingredient(i),
                        },
                        Event::Cookware(c) => Item::Cookware {
                            index: self.cookware(c),
                        },
                        Event::Timer(t) => Item::Timer {
                            index: self.timer(t),
                        },
                        _ => unreachable!(),
                    };
                    new_items.push(new_component);
                }
                _ => panic!("Unexpected event in step: {item:?}"),
            };
        }

        Step {
            items: new_items,
            number: self.step_counter,
        }
    }

    fn text_block(&mut self, items: Vec<Event<'i>>) -> String {
        let mut s = String::new();
        for ev in items {
            match ev {
                Event::Text(t) => s += t.text().as_ref(),
                Event::Ingredient(_) | Event::Cookware(_) | Event::Timer(_) => {
                    assert_eq!(
                        self.define_mode,
                        DefineMode::Text,
                        "Non text event in text block outside define mode text"
                    );

                    // ignore component
                    self.ctx.warn(AnalysisWarning::ComponentInTextMode {
                        component_span: match ev {
                            Event::Ingredient(i) => i.span(),
                            Event::Cookware(c) => c.span(),
                            Event::Timer(t) => t.span(),
                            _ => unreachable!(),
                        },
                    });
                }
                _ => panic!("Unexpected event in text block: {ev:?}"),
            }
        }
        s
    }

    fn ingredient(&mut self, ingredient: Located<ast::Ingredient<'i>>) -> usize {
        let located_ingredient = ingredient.clone();
        let (ingredient, location) = ingredient.take_pair();

        let name = ingredient.name.text_trimmed();

        let mut new_igr = Ingredient {
            name: name.into_owned(),
            alias: ingredient.alias.map(|t| t.text_trimmed().into_owned()),
            quantity: ingredient.quantity.clone().map(|q| self.quantity(q, true)),
            note: ingredient.note.map(|n| n.text_trimmed().into_owned()),
            modifiers: ingredient.modifiers.into_inner(),
            relation: IngredientRelation::definition(
                Vec::new(),
                self.define_mode != DefineMode::Components,
            ),
        };

        if let Some(inter_data) = ingredient.intermediate_data {
            match self.resolve_intermediate_ref(inter_data) {
                Ok(relation) => {
                    new_igr.relation = relation;
                    assert!(new_igr.modifiers().contains(Modifiers::REF));
                    let invalid_modifiers = Modifiers::RECIPE | Modifiers::HIDDEN | Modifiers::NEW;
                    if new_igr.modifiers().intersects(invalid_modifiers) {
                        self.ctx.error(AnalysisError::InvalidIntermediateReference {
                            reference_span: ingredient.modifiers.span(),
                            reason: "invalid combination of modifiers",
                            help: format!(
                                "Remove the following modifiers: {}",
                                new_igr.modifiers() & invalid_modifiers
                            )
                            .into(),
                        })
                    }
                }
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
                            let a = old_q_loc
                                .unit
                                .as_ref()
                                .map(|l| l.span())
                                .unwrap_or(old_q_loc.span());
                            let new_q_loc = located_ingredient.quantity.as_ref().unwrap();
                            let b = new_q_loc
                                .unit
                                .as_ref()
                                .map(|l| l.span())
                                .unwrap_or(new_q_loc.span());
                            self.ctx
                                .warn(AnalysisWarning::IncompatibleUnits { a, b, source: e });
                        }
                    }
                }
            }

            if let Some(note) = &located_ingredient.note {
                self.ctx
                    .error(AnalysisError::ComponentPartNotAllowedInReference {
                        container: "ingredient",
                        what: "note",
                        to_remove: note.span(),
                        implicit,
                    })
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
                self.ctx
                    .error(AnalysisError::ConflictingReferenceQuantities {
                        component_name: new_igr.name.to_string(),
                        definition_span: definition_location.span(),
                        reference_span: location,
                    });
            }

            // text value warning
            if let Some((ref_q, def_q)) =
                &new_igr.quantity.as_ref().zip(definition.quantity.as_ref())
            {
                let ref_is_text = ref_q.value.is_text();
                let def_is_text = def_q.value.is_text();

                if ref_is_text != def_is_text {
                    let ref_q_loc = located_ingredient.quantity.as_ref().unwrap().span();
                    let def_q_loc = definition_location.quantity.as_ref().unwrap().span();

                    let (text_quantity_span, number_quantity_span) = if ref_is_text {
                        (ref_q_loc, def_q_loc)
                    } else {
                        (def_q_loc, ref_q_loc)
                    };

                    self.ctx.warn(AnalysisWarning::TextValueInReference {
                        text_quantity_span,
                        number_quantity_span,
                    });
                }
            }

            Ingredient::set_referenced_from(&mut self.content.ingredients, references_to);
        }

        if new_igr.modifiers.contains(Modifiers::RECIPE)
            && !new_igr.modifiers.contains(Modifiers::REF)
        {
            if let Some(checker) = &self.recipe_ref_checker {
                let res = (*checker)(&new_igr.name);

                if let RecipeRefCheckResult::NotFound { help, note } = res {
                    self.ctx.warn(AnalysisWarning::RecipeNotFound {
                        ref_span: location,
                        name: new_igr.name.clone(),
                        help,
                        note,
                    })
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
    ) -> Result<IngredientRelation, AnalysisError> {
        use ast::IntermediateRefMode::*;
        use ast::IntermediateTargetKind::*;
        assert!(!inter_data.val.is_negative());
        let val = inter_data.val as u32;

        if val == 0 {
            match inter_data.ref_mode {
                Number => {
                    return Err(AnalysisError::InvalidIntermediateReference {
                        reference_span: inter_data.span(),
                        reason: "number is 0",
                        help: "Step and section numbers start at 1".into(),
                    })
                }
                Relative => {
                    return Err(AnalysisError::InvalidIntermediateReference {
                        reference_span: inter_data.span(),
                        reason: "relative reference to self",
                        help: "Relative reference value has to be greater than 0".into(),
                    })
                }
            }
        }

        let bounds = |help| {
            Err(AnalysisError::InvalidIntermediateReference {
                reference_span: inter_data.span(),
                reason: "value out of bounds",
                help,
            })
        };

        let relation = match (inter_data.target_kind, inter_data.ref_mode) {
            (Step, Number) => {
                let index = self
                    .current_section
                    .content
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.is_step().then_some(i))
                    .nth((val - 1) as usize);

                if index.is_none() {
                    return bounds(
                        format!(
                            "The value has to be a previous step number: {}",
                            // -1 because step_counter holds the current step number
                            match self.step_counter.saturating_sub(1) {
                                0 => "no steps before this one".to_string(),
                                1 => "1".to_string(),
                                max => format!("1 to {max}"),
                            }
                        )
                        .into(),
                    );
                }

                IngredientRelation::reference(index.unwrap(), IngredientReferenceTarget::Step)
            }
            (Step, Relative) => {
                let index = self
                    .current_section
                    .content
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| c.is_step().then_some(i))
                    .nth_back((val - 1) as usize);
                if index.is_none() {
                    return bounds(
                        format!(
                            "The current section {} steps before this one",
                            match self.step_counter.saturating_sub(1) {
                                0 => "has no".to_string(),
                                before => format!("only has {before}"),
                            }
                        )
                        .into(),
                    );
                }

                IngredientRelation::reference(index.unwrap(), IngredientReferenceTarget::Step)
            }
            (Section, Number) => {
                let index = (val - 1) as usize; // direct index, but make it 0 indexed

                if index >= self.content.sections.len() {
                    return bounds(
                        format!(
                            "The value has to be a previous section number: {}",
                            match self.content.sections.len() {
                                0 => "no sections before this one".to_string(),
                                1 => "1".to_string(),
                                max => format!("1 to {max}"),
                            }
                        )
                        .into(),
                    );
                }

                IngredientRelation::reference(index, IngredientReferenceTarget::Section)
            }
            (Section, Relative) => {
                let val = val as usize; // number of sections to go back

                // content.sections holds the past sections
                if val > self.content.sections.len() {
                    return bounds(
                        format!(
                            "The recipe {} sections before this one",
                            match self.content.sections.len() {
                                0 => "has no".to_string(),
                                before => format!("only has {before}"),
                            }
                        )
                        .into(),
                    );
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

    fn cookware(&mut self, cookware: Located<ast::Cookware<'i>>) -> usize {
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
                self.ctx
                    .error(AnalysisError::ComponentPartNotAllowedInReference {
                        container: "cookware",
                        what: "note",
                        to_remove: note.span(),
                        implicit,
                    });
            }

            // See ingredients for explanation
            if definition.quantity.is_some()
                && new_cw.quantity.is_some()
                && !definition
                    .relation
                    .is_defined_in_step()
                    .expect("definition")
            {
                self.ctx
                    .error(AnalysisError::ConflictingReferenceQuantities {
                        component_name: new_cw.name.to_string(),
                        definition_span: definition_location.span(),
                        reference_span: location,
                    });
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

                    self.ctx.warn(AnalysisWarning::TextValueInReference {
                        text_quantity_span,
                        number_quantity_span,
                    });
                }
            }

            Cookware::set_referenced_from(&mut self.content.cookware, references_to);
        }

        self.locations.cookware.push(located_cookware);
        self.content.cookware.push(new_cw);
        self.content.cookware.len() - 1
    }

    fn timer(&mut self, timer: Located<ast::Timer<'i>>) -> usize {
        let located_timer = timer.clone();
        let (timer, span) = timer.take_pair();
        let quantity = timer.quantity.map(|q| {
            let quantity = self.quantity(q, false);
            if self.extensions.contains(Extensions::ADVANCED_UNITS) {
                if let Some(unit) = quantity.unit() {
                    match unit.unit_info_or_parse(self.converter) {
                        UnitInfo::Known(unit) => {
                            if unit.physical_quantity != PhysicalQuantity::Time {
                                self.ctx.error(AnalysisError::BadTimerUnit {
                                    unit: Box::new(unit.as_ref().clone()),
                                    timer_span: located_timer
                                        .quantity
                                        .as_ref()
                                        .unwrap()
                                        .unit
                                        .as_ref()
                                        .unwrap()
                                        .span(),
                                })
                            }
                        }
                        UnitInfo::Unknown => self.ctx.error(AnalysisError::UnknownTimerUnit {
                            unit: unit.text().to_string(),
                            timer_span: span,
                        }),
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
        quantity: Located<ast::Quantity<'i>>,
        is_ingredient: bool,
    ) -> Quantity<ScalableValue> {
        let ast::Quantity { value, unit, .. } = quantity.into_inner();
        Quantity::new(
            self.value(value, is_ingredient),
            unit.map(|t| t.text_trimmed().into_owned()),
        )
    }

    fn value(&mut self, value: ast::QuantityValue, is_ingredient: bool) -> ScalableValue {
        match &value {
            ast::QuantityValue::Single {
                value,
                auto_scale: Some(auto_scale_marker),
            } => {
                self.ctx.error(AnalysisError::ScaleTextValue {
                    value_span: value.span(),
                    auto_scale_marker: *auto_scale_marker,
                });
            }
            ast::QuantityValue::Many(v) => {
                if let Some(s) = &self.content.metadata.servings {
                    let servings_meta_span = self
                        .locations
                        .metadata
                        .get("servings")
                        .map(|(_, value)| value.span());
                    if s.len() != v.len() {
                        self.ctx.error(AnalysisError::ScalableValueManyConflict {
                            reason: format!(
                                "{} servings defined but {} values in the quantity",
                                s.len(),
                                v.len()
                            )
                            .into(),
                            value_span: value.span(),
                            servings_meta_span,
                        });
                    }
                } else {
                    self.ctx.error(AnalysisError::ScalableValueManyConflict {
                        reason: format!("no servings defined but {} values in quantity", v.len())
                            .into(),
                        value_span: value.span(),
                        servings_meta_span: None,
                    })
                }
            }
            _ => {}
        }
        let value_span = value.span();
        let mut v = ScalableValue::from_ast(value);

        if is_ingredient && self.auto_scale_ingredients {
            match v {
                ScalableValue::Fixed(value) if !value.is_text() => v = ScalableValue::Linear(value),
                ScalableValue::Linear(_) => {
                    self.ctx.warn(AnalysisWarning::RedundantAutoScaleMarker {
                        quantity_span: Span::new(value_span.end(), value_span.end() + 1),
                    });
                }
                _ => {}
            };
        }

        v
    }

    fn resolve_reference<C: RefComponent>(
        &mut self,
        new: &mut C,
        location: Span,
        modifiers_location: Span,
    ) -> Option<(usize, bool)> {
        let all = C::all(&self.content);
        let new_name = new.name().to_lowercase();
        // find the LAST component with the same name, lazy
        let same_name_cell = std::cell::OnceCell::new();
        let same_name = || {
            *same_name_cell.get_or_init(|| {
                C::all(&self.content).iter().rposition(|other: &C| {
                    !other.modifiers().contains(Modifiers::REF)
                        && new_name == other.name().to_lowercase()
                })
            })
        };

        // no new and ref -> error
        if new.modifiers().contains(Modifiers::NEW | Modifiers::REF) {
            self.ctx
                .error(AnalysisError::ConflictingModifiersInReference {
                    modifiers: Located::new(*new.modifiers(), modifiers_location),
                    conflict: *new.modifiers(),
                    implicit: false,
                });
            return None;
        }

        // no new -> maybe warning for redundant
        if new.modifiers().contains(Modifiers::NEW) {
            if self.define_mode != DefineMode::Steps {
                if self.duplicate_mode == DuplicateMode::Reference && same_name().is_none() {
                    self.ctx.warn(AnalysisWarning::RedundantModifier {
                        what: "new ('+')",
                        help: format!("There are no {}s with the same name before", C::container())
                            .into(),
                        define_mode: self.define_mode,
                        duplicate_mode: self.duplicate_mode,
                        modifiers: Located::new(*new.modifiers(), modifiers_location),
                    });
                } else if self.duplicate_mode == DuplicateMode::New {
                    self.ctx.warn(AnalysisWarning::RedundantModifier {
                        what: "new ('+')",
                        help: format!("This {} is already a definition", C::container()).into(),
                        define_mode: self.define_mode,
                        duplicate_mode: self.duplicate_mode,
                        modifiers: Located::new(*new.modifiers(), modifiers_location),
                    });
                }
            }
            return None;
        }

        // warning for redundant ref
        if (self.duplicate_mode == DuplicateMode::Reference
            || self.define_mode == DefineMode::Steps)
            && new.modifiers().contains(Modifiers::REF)
        {
            self.ctx.warn(AnalysisWarning::RedundantModifier {
                what: "reference ('&')",
                help: format!("This {} is already a reference", C::container()).into(),
                define_mode: self.define_mode,
                duplicate_mode: self.duplicate_mode,
                modifiers: Located::new(*new.modifiers(), modifiers_location),
            });
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
                self.ctx
                    .error(AnalysisError::ConflictingModifiersInReference {
                        modifiers: Located::new(*new.modifiers(), modifiers_location),
                        conflict,
                        implicit,
                    });
            }

            // extra reference checks
            Some((references_to, implicit))
        } else {
            self.ctx.error(AnalysisError::ReferenceNotFound {
                name: new.name().to_string(),
                reference_span: location,
                implicit,
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

fn find_temperature<'a>(text: &'a str, re: &Regex) -> Option<(&'a str, Quantity<Value>, &'a str)> {
    let Some(caps) = re.captures(text) else {
        return None;
    };

    let value = caps[1].replace(',', ".").parse::<f64>().ok()?;
    let unit = caps.get(3).unwrap().range();
    let unit_text = text[unit].to_string();
    let temperature = Quantity::new(Value::Number(value.into()), Some(unit_text));

    let range = caps.get(0).unwrap().range();
    let (before, after) = (&text[..range.start], &text[range.end..]);

    Some((before, temperature, after))
}
