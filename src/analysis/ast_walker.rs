use std::borrow::Cow;
use std::collections::HashMap;

use crate::ast::{self, Text};
use crate::context::Context;
use crate::convert::{Converter, PhysicalQuantity};
use crate::located::Located;
use crate::metadata::Metadata;
use crate::quantity::{Quantity, QuantityValue, UnitInfo};
use crate::{model::*, Extensions};

use super::{AnalysisError, AnalysisResult, AnalysisWarning};

#[derive(Default, Debug)]
pub struct RecipeContent {
    pub metadata: Metadata,
    pub sections: Vec<Section>,
    pub ingredients: Vec<Ingredient>,
    pub cookware: Vec<Cookware>,
    pub timers: Vec<Timer>,
}

#[tracing::instrument(level = "debug", skip_all, target = "cooklang::analysis", fields(ast_lines = ast.lines.len()))]
pub fn parse_ast<'a>(
    ast: ast::Ast<'a>,
    extensions: Extensions,
    converter: &Converter,
) -> AnalysisResult {
    let context = Context::default();

    let walker = Walker {
        extensions,
        converter,

        content: Default::default(),
        current_section: Section::default(),

        define_mode: DefineMode::All,
        duplicate_mode: DuplicateMode::New,
        context,

        ingredient_locations: Default::default(),
        metadata_locations: Default::default(),
        step_counter: 0,
    };
    walker.ast(ast)
}

struct Walker<'a, 'c> {
    extensions: Extensions,
    converter: &'c Converter,

    content: RecipeContent,
    current_section: Section,

    define_mode: DefineMode,
    duplicate_mode: DuplicateMode,
    context: Context<AnalysisError, AnalysisWarning>,

    ingredient_locations: Vec<Located<ast::Ingredient<'a>>>,
    metadata_locations: HashMap<Cow<'a, str>, (Text<'a>, Text<'a>)>,
    step_counter: u32,
}

#[derive(PartialEq)]
enum DefineMode {
    All,
    Components,
    Steps,
    Text,
}

#[derive(PartialEq)]
enum DuplicateMode {
    New,
    Reference,
}

crate::context::impl_deref_context!(Walker<'_, '_>, AnalysisError, AnalysisWarning);

impl<'a, 'r> Walker<'a, 'r> {
    fn ast(mut self, ast: ast::Ast<'a>) -> AnalysisResult {
        for line in ast.lines {
            match line {
                ast::Line::Metadata { key, value } => self.metadata(key, value),
                ast::Line::Step { items } => {
                    let new_step = self.step(items);

                    // If define mode is ingredients, don't add the
                    // step to the section. The components should have been
                    // added to their lists
                    if self.define_mode != DefineMode::Components {
                        self.current_section.steps.push(new_step);
                    }
                }
                ast::Line::Section { name } => {
                    if !self.current_section.is_empty() {
                        self.content.sections.push(self.current_section);
                    }
                    self.current_section =
                        Section::new(name.map(|t| t.text_trimmed().into_owned()));
                }
            }
        }
        if !self.current_section.is_empty() {
            self.content.sections.push(self.current_section);
        }
        self.context.finish(Some(self.content))
    }

    fn metadata(&mut self, key: Text<'a>, value: Text<'a>) {
        self.metadata_locations
            .insert(key.text_trimmed(), (key.clone(), value.clone()));

        let key_t = key.text_trimmed();
        let value_t = value.text_trimmed();

        if let Err(warn) = self
            .content
            .metadata
            .insert(key_t.into_owned(), value_t.into_owned())
        {
            self.warn(AnalysisWarning::InvalidMetadataValue {
                key: key.located_string(),
                value: value.located_string(),
                source: warn,
            });
        }
    }

    fn step(&mut self, items: Vec<ast::Item<'a>>) -> Step {
        let mut new_items = Vec::new();

        for item in items {
            match item {
                ast::Item::Text(text) => {
                    let t = text.text();
                    if self.define_mode == DefineMode::Components {
                        // only issue warnings for alphanumeric characters
                        // so that the user can format the text with spaces,
                        // hypens or whatever.
                        if t.contains(|c: char| c.is_alphanumeric()) {
                            self.warn(AnalysisWarning::TextDefiningIngredients {
                                text_span: text.span(),
                            });
                        }
                        continue; // ignore text
                    }


                    new_items.push(Item::Text {
                        value: t.into_owned(),
                    });
                }
                ast::Item::Component(c) => {
                    let new_component = self.component(c);
                    new_items.push(Item::ItemComponent {
                        value: new_component,
                    })
                }
            };
        }

        Step {
            items: new_items,
        }
    }

    fn component(&mut self, component: Box<Located<ast::Component<'a>>>) -> Component {
        let (inner, span) = component.take_pair();

        match inner {
            ast::Component::Ingredient(_) => Component {
                kind: ComponentKind::IngredientKind,
            },
            ast::Component::Cookware(_) => Component {
                kind: ComponentKind::CookwareKind,
            },
            ast::Component::Timer(_) => Component {
                kind: ComponentKind::TimerKind,
            },
        }
    }

    fn ingredient(&mut self, ingredient: Located<ast::Ingredient<'a>>) -> usize {
        let located_ingredient = ingredient.clone();
        let (ingredient, location) = ingredient.take_pair();

        let name = ingredient.name.text_trimmed();

        let mut new_igr = Ingredient {
            name: name.into_owned(),
            quantity: ingredient.quantity.clone().map(|q| self.quantity(q, true)),
            note: ingredient.note.map(|n| n.text_trimmed().into_owned()),
        };

        self.ingredient_locations.push(located_ingredient);
        self.content.ingredients.push(new_igr);
        self.content.ingredients.len() - 1
    }


    fn cookware(&mut self, cookware: Located<ast::Cookware<'a>>) -> usize {
        let located_cookware = cookware.clone();
        let (cookware, location) = cookware.take_pair();

        let mut new_cw = Cookware {
            name: cookware.name.text_trimmed().into_owned(),
            quantity: cookware.quantity.map(|q| self.value(q.inner, false)),
            note: cookware.note.map(|n| n.text_trimmed().into_owned()),
        };

        self.content.cookware.push(new_cw);
        self.content.cookware.len() - 1
    }

    fn timer(&mut self, timer: Located<ast::Timer<'a>>) -> usize {
        let located_timer = timer.clone();
        let (timer, span) = timer.take_pair();
        let quantity = timer.quantity.map(|q| {
            let quantity = self.quantity(q, false);
            if self.extensions.contains(Extensions::ADVANCED_UNITS) {
                if let Some(unit) = quantity.unit() {
                    match unit.unit_or_parse(self.converter) {
                        UnitInfo::Known(unit) => {
                            if unit.physical_quantity != PhysicalQuantity::Time {
                                self.error(AnalysisError::BadTimerUnit {
                                    unit: unit.as_ref().clone(),
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
                        UnitInfo::Unknown => self.error(AnalysisError::UnknownTimerUnit {
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

    fn quantity(&mut self, quantity: Located<ast::Quantity<'a>>, is_ingredient: bool) -> Quantity {
        let ast::Quantity { value, unit, .. } = quantity.take();
        Quantity::new(
            self.value(value, is_ingredient),
            unit.map(|t| t.text_trimmed().into_owned()),
        )
    }

    fn value(&mut self, value: ast::QuantityValue, is_ingredient: bool) -> QuantityValue {
        let value_span = value.span();
        let mut v = QuantityValue::from_ast(value);

        v
    }


}

