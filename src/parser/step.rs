use crate::{
    ast::{self, Text},
    context::Recover,
    error::label,
    lexer::T,
    located::Located,
    span::Span,
    Extensions,
};

use super::{
    quantity::parse_quantity, token_stream::Token, LineParser, ParserError,
    ParserWarning,
};

pub struct ParsedStep<'input> {
    pub items: Vec<ast::Item<'input>>,
}

pub(crate) fn step<'input>(
    line: &mut LineParser<'_, 'input>,
) -> ParsedStep<'input> {
    let mut items: Vec<ast::Item> = vec![];

    while !line.rest().is_empty() {
        let start = line.current_offset();
        let component = match line.peek() {
            T![@] => line
                .with_recover(ingredient)
                .map(ast::Component::Ingredient),
            T![#] => line.with_recover(cookware).map(ast::Component::Cookware),
            T![~] => line.with_recover(timer).map(ast::Component::Timer),
            _ => None,
        };
        if let Some(component) = component {
            let end = line.current_offset();
            items.push(ast::Item::Component(Box::new(Located::new(
                component,
                Span::new(start, end),
            ))));
        } else {
            let tokens_start = line.tokens_consumed();
            line.bump_any(); // consume the first token, this avoids entering an infinite loop
            line.consume_while(|t| !matches!(t, T![@] | T![#] | T![~]));
            let tokens_end = line.tokens_consumed();
            let tokens = &line.tokens()[tokens_start..tokens_end];

            items.push(ast::Item::Text(line.text(start, tokens)));
        }
    }


    // trim the line

    // TODO uncomment or remove when super::parse TODO is solved
    // if let Some(ast::Item::Text(text)) = items.last_mut() {
    //     text.trim_fragments_end();
    //     if text.fragments().is_empty() {
    //         items.pop();
    //     }
    // }
    if let Some(ast::Item::Text(text)) = items.first_mut() {
        text.trim_fragments_start();
        if text.fragments().is_empty() {
            items.remove(0);
        }
    }

    ParsedStep { items }
}

struct Body<'t> {
    name: &'t [Token],
    close: Option<Span>,
    quantity: Option<&'t [Token]>,
}

fn comp_body<'t>(line: &mut LineParser<'t, '_>) -> Option<Body<'t>> {
    line.with_recover(|line| {
        let name = line.until(|t| matches!(t, T!['{'] | T![@] | T![#] | T![~]))?;
        let close_span_start = line.consume(T!['{'])?.span.start();
        let quantity = line.until(|t| t == T!['}'])?;
        let close_span_end = line.bump(T!['}']).span.end();
        let close_span = Span::new(close_span_start, close_span_end);
        if quantity
            .iter()
            .any(|t| !matches!(t.kind, T![ws] | T![block comment]))
        {
            Some(Body {
                name,
                close: Some(close_span),
                quantity: Some(quantity),
            })
        } else {
            Some(Body {
                name,
                close: Some(close_span),
                quantity: None,
            })
        }
    })
    .or_else(|| {
        line.with_recover(|line| {
            let tokens = line.consume_while(|t| matches!(t, T![word] | T![int] | T![float]));
            if tokens.is_empty() {
                return None;
            }
            Some(Body {
                name: tokens,
                close: None,
                quantity: None,
            })
        })
    })
}

fn note<'input>(line: &mut LineParser<'_, 'input>) -> Option<Text<'input>> {
    line.extension(Extensions::COMPONENT_NOTE)
        .then(|| {
            line.with_recover(|line| {
                line.consume(T!['('])?;
                let offset = line.current_offset();
                let note = line.until(|t| t == T![')'])?;
                line.bump(T![')']);
                Some(line.text(offset, note))
            })
        })
        .flatten()
}

const INGREDIENT: &str = "ingredient";
const COOKWARE: &str = "cookware";
const TIMER: &str = "timer";

fn ingredient<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Ingredient<'input>> {
    // Parse
    line.consume(T![@])?;
    let name_offset = line.current_offset();
    let body = comp_body(line)?;
    let note = note(line);
    let name = line.text(name_offset, body.name);

    if name.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: INGREDIENT,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name.span(), "add a name here")],
            help: None,
        });
    }


    let quantity = body.quantity.map(|tokens| {
        parse_quantity(tokens, line.input, line.extensions, &mut line.context).quantity
    });

    Some(ast::Ingredient {
        name,
        quantity,
        note,
    })
}

fn cookware<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Cookware<'input>> {
    // Parse
    line.consume(T![#])?;
    let name_offset = line.current_offset();
    let body = comp_body(line)?;
    let note = note(line);
    let name = line.text(name_offset, body.name);

    // Errors
    if name.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
            container: COOKWARE,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name, "add a name here")],
            help: None,
        });
    }

    let quantity = body.quantity.map(|tokens| {
        let q = parse_quantity(tokens, line.input, line.extensions, &mut line.context);
        if let Some(unit) = &q.quantity.unit {
            let span = if let Some(sep) = q.unit_separator {
                Span::new(sep.start(), unit.span().end())
            } else {
                unit.span()
            };
            line.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "unit in quantity",
                to_remove: span,
                help: Some("Cookware quantity can't have an unit."),
            });
        }
        if let ast::QuantityValue::Single {
            auto_scale: Some(auto_scale),
            ..
        } = &q.quantity.value
        {
            line.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: *auto_scale,
                help: Some("Cookware quantity can't be auto scaled."),
            });
        }
        q.quantity.map_inner(|q| q.value)
    });

    Some(ast::Cookware {
        name,
        quantity,
        note,
    })
}

fn timer<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Timer<'input>> {
    // Parse
    let name_offset = line.current_offset();
    line.consume(T![~])?;
    let body = comp_body(line)?;

    // Errors
    check_note(line, TIMER);

    let name = line.text(name_offset, body.name);

    let mut quantity = body.quantity.map(|tokens| {
        let q = parse_quantity(tokens, line.input, line.extensions, &mut line.context);
        if let ast::QuantityValue::Single {
            auto_scale: Some(auto_scale),
            ..
        } = &q.quantity.value
        {
            line.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "auto scale marker",
                to_remove: *auto_scale,
                help: Some("Timer quantity can't be auto scaled."),
            });
        }
        if q.quantity.unit.is_none() {
            line.error(ParserError::ComponentPartMissing {
                container: TIMER,
                what: "quantity unit",
                expected_pos: Span::pos(q.quantity.value.span().end()),
            });
        }
        q.quantity
    });

    if quantity.is_none() && line.extension(Extensions::TIMER_REQUIRES_TIME) {
        let span = body.close.unwrap_or_else(|| Span::pos(name.span().end()));
        line.error(ParserError::ComponentPartMissing {
            container: TIMER,
            what: "quantity",
            expected_pos: span,
        });
        quantity = Some(Recover::recover());
    }

    let name = if name.is_text_empty() {
        None
    } else {
        Some(name)
    };

    if name.is_none() && quantity.is_none() {
        let span = if let Some(s) = body.close {
            Span::new(name_offset, s.end())
        } else {
            Span::pos(name_offset)
        };
        line.error(ParserError::ComponentPartMissing {
            container: TIMER,
            what: "quantity OR name",
            expected_pos: span,
        });
        quantity = Some(Recover::recover()); // could be also name, but whatever
    }

    Some(ast::Timer { name, quantity })
}


fn check_note(line: &mut LineParser, container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if !line.extension(Extensions::COMPONENT_NOTE) {
        return;
    }

    assert!(line
        .with_recover(|line| {
            let start = line.consume(T!['('])?.span.start();
            let _ = line.until(|t| t == T![')'])?;
            let end = line.bump(T![')']).span.end();
            line.warn(ParserWarning::ComponentPartIgnored {
                container,
                what: "note",
                ignored: Span::new(start, end),
                help: Some("Notes are only available in ingredients"),
            });
            None::<()> // always backtrack
        })
        .is_none());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ast::Item, parser::token_stream::TokenStream};
    use test_case::test_case;

    macro_rules! t {
        ($input:expr) => {
            t!($input, $crate::Extensions::all())
        };
        ($input:expr, $extensions:expr) => {{
            let input = $input;
            let tokens = TokenStream::new(input).collect::<Vec<_>>();
            let mut line = LineParser::new(0, &tokens, input, $extensions);
            let r = step(&mut line, false);
            (r, line.finish())
        }};
    }

    macro_rules! igr {
        ($item:expr) => {
            match $item {
                Item::Component(comp) => comp.clone().map_inner(|comp| match comp {
                    ast::Component::Ingredient(igr) => igr,
                    _ => panic!(),
                }),
                _ => panic!(),
            }
        };
    }

    #[test_case("@&(~1)one step back{}" => (
        Located::new(Modifiers::REF | Modifiers::REF_TO_STEP, 1..6),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Relative,
            target_kind: IntermediateTargetKind::Step,
            val: 1
            }, 2..6)
    ); "step relative")]
    #[test_case("@&(1)step index 1{}" => (
        Located::new(Modifiers::REF | Modifiers::REF_TO_STEP, 1..5),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Index,
            target_kind: IntermediateTargetKind::Step,
            val: 1
        }, 2..5)
    ); "step index")]
    #[test_case("@&(=~1)one section back{}" => (
        Located::new(Modifiers::REF | Modifiers::REF_TO_SECTION, 1..7),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Relative,
            target_kind: IntermediateTargetKind::Section,
            val: 1
        }, 2..7)
    ); "section relative")]
    #[test_case("@&(=1)section index 1{}" => (
        Located::new(Modifiers::REF | Modifiers::REF_TO_SECTION, 1..6),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Index,
            target_kind: IntermediateTargetKind::Section,
            val: 1
        }, 2..6)
    ); "section index")]
    fn intermediate_ref(input: &str) -> (Located<Modifiers>, Located<IntermediateData>) {
        let (s, ctx) = t!(input);
        let igr = igr!(&s.items[0]);
        assert!(ctx.is_empty());
        (igr.modifiers, igr.intermediate_data.unwrap())
    }

    #[test_case("@&(~=1)name{}"; "swap ~ =")]
    #[test_case("@&(9999999999999999999999999999999999999999)name{}"; "number too big")]
    #[test_case("@&(awebo)name{}"; "unexpected syntax")]
    fn intermediate_ref_errors(input: &str) {
        let (_, ctx) = t!(input);
        assert_eq!(ctx.errors.len(), 1);
    }
}
