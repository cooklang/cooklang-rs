use smallvec::SmallVec;

use crate::{
    ast::{self, IntermediateData, IntermediateRefMode, IntermediateTargetKind, Modifiers, Text},
    context::Recover,
    error::label,
    lexer::T,
    located::Located,
    span::Span,
    Extensions,
};

use super::{
    mt, quantity::parse_quantity, token_stream::Token, tokens_span, BlockParser, Event,
    ParserError, ParserWarning,
};

pub(crate) fn step<'input>(bp: &mut BlockParser<'_, 'input>) {
    let is_text = bp.consume(T![>]).is_some();

    let is_empty = bp.tokens().iter().all(|t| {
        matches!(
            t.kind,
            T![ws] | T![line comment] | T![block comment] | T![newline]
        )
    });
    if is_empty {
        bp.consume_rest();
        return;
    }

    bp.event(Event::StartStep { is_text });

    if is_text {
        let start = bp.current_offset();
        while !bp.rest().is_empty() {
            let _ = bp.consume(T![>]);
            let tokens = bp.consume_while(|t| t != T![newline]);
            let text = bp.text(start, tokens);
            if !text.is_text_empty() {
                bp.event(Event::Text(text));
            }
        }
    } else {
        while !bp.rest().is_empty() {
            let component = match bp.peek() {
                T![@] => bp.with_recover(ingredient),
                T![#] => bp.with_recover(cookware),
                T![~] => bp.with_recover(timer),
                _ => None,
            };
            if let Some(ev) = component {
                bp.event(ev)
            } else {
                let start = bp.current_offset();
                let tokens_start = bp.tokens_consumed();
                bp.bump_any(); // consume the first token, this avoids entering an infinite loop
                bp.consume_while(|t| !matches!(t, T![@] | T![#] | T![~]));
                let tokens_end = bp.tokens_consumed();
                let tokens = &bp.tokens()[tokens_start..tokens_end];
                let text = bp.text(start, tokens);
                if !text.fragments().is_empty() {
                    bp.event(Event::Text(text));
                }
            }
        }
    }

    bp.event(Event::EndStep { is_text });
}

struct Body<'t> {
    name: &'t [Token],
    close: Option<Span>,
    quantity: Option<&'t [Token]>,
}

fn comp_body<'t>(bp: &mut BlockParser<'t, '_>) -> Option<Body<'t>> {
    bp.with_recover(|line| {
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
        bp.with_recover(|line| {
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

fn modifiers<'t>(bp: &mut BlockParser<'t, '_>) -> &'t [Token] {
    if !bp.extension(Extensions::COMPONENT_MODIFIERS) {
        return &[];
    }

    let start = bp.current;
    loop {
        match bp.peek() {
            T![@] | T![?] | T![+] | T![-] => {
                bp.bump_any();
            }
            T![&] => {
                bp.bump_any();
                if bp.extension(Extensions::INTERMEDIATE_INGREDIENTS) {
                    bp.with_recover(|line| {
                        line.consume(T!['('])?;
                        let intermediate = line.until(|t| t == T![')'])?;
                        line.bump(T![')']);
                        Some(intermediate)
                    });
                }
            }
            _ => break,
        }
    }
    &bp.tokens()[start..bp.current]
}

fn note<'input>(bp: &mut BlockParser<'_, 'input>) -> Option<Text<'input>> {
    bp.extension(Extensions::COMPONENT_NOTE)
        .then(|| {
            bp.with_recover(|line| {
                line.consume(T!['('])?;
                let offset = line.current_offset();
                let note = line.until(|t| t == T![')'])?;
                line.bump(T![')']);
                Some(line.text(offset, note))
            })
        })
        .flatten()
}

struct ParsedModifiers {
    flags: Located<Modifiers>,
    intermediate_data: Option<Located<IntermediateData>>,
}

// Parsing is defered so there are no errors for components that doesn't support modifiers
fn parse_modifiers(
    bp: &mut BlockParser,
    modifiers_tokens: &[Token],
    modifiers_pos: usize,
) -> ParsedModifiers {
    if modifiers_tokens.is_empty() {
        ParsedModifiers {
            flags: Located::new(Modifiers::empty(), Span::pos(modifiers_pos)),
            intermediate_data: None,
        }
    } else {
        let modifiers_span = tokens_span(modifiers_tokens);
        let mut modifiers = Modifiers::empty();
        let mut intermediate_data = None;

        let mut tokens = modifiers_tokens.iter();

        while let Some(tok) = tokens.next() {
            let new_m = match tok.kind {
                T![@] => Modifiers::RECIPE,
                T![&] => {
                    if bp.extension(Extensions::INTERMEDIATE_INGREDIENTS) {
                        intermediate_data = parse_intermediate_ref_data(bp, &mut tokens);
                    }

                    Modifiers::REF
                }
                T![?] => Modifiers::OPT,
                T![+] => Modifiers::NEW,
                T![-] => Modifiers::HIDDEN,
                _ => panic!("Bad modifiers token sequence. Unexpected token: {tok:?}"),
            };

            if modifiers.contains(new_m) {
                bp.error(ParserError::DuplicateModifiers {
                    modifiers_span,
                    dup: bp.as_str(*tok).to_string(),
                });
            } else {
                modifiers |= new_m;
            }
        }

        ParsedModifiers {
            flags: Located::new(modifiers, modifiers_span),
            intermediate_data,
        }
    }
}

fn parse_intermediate_ref_data(
    bp: &mut BlockParser,
    tokens: &mut std::slice::Iter<Token>,
) -> Option<Located<IntermediateData>> {
    use IntermediateRefMode::*;
    use IntermediateTargetKind::*;
    const CONTAINER: &str = "modifiers";
    const WHAT: &str = "intermediate reference";
    const INTER_REF_HELP: &str = "The reference is something like: `~1`, `1`, `=1` or `=~1`.";

    // if '(' has been taken as a modifier token, it has taken until
    // a closing ')'

    if !matches!(tokens.clone().next(), Some(mt!['('])) {
        return None;
    }

    let slice = {
        let slice = tokens.as_slice();
        let end_pos = tokens
            .position(|t| t.kind == T![')']) // consumes until and including ')'
            .expect("No closing paren in intermediate ingredient ref");
        &slice[..=end_pos]
    };
    let inner_slice = &slice[1..slice.len() - 1];

    if inner_slice.is_empty() {
        bp.error(ParserError::ComponentPartInvalid {
            container: CONTAINER,
            what: WHAT,
            reason: "empty",
            labels: vec![label!(tokens_span(slice), "add the reference here")],
            help: Some(INTER_REF_HELP),
        });
        return None;
    }

    let filtered_tokens: SmallVec<[Token; 3]> = inner_slice
        .iter()
        .filter(|t| !matches!(t.kind, T![ws] | T![block comment]))
        .copied()
        .collect();

    let (i, ref_mode, target_kind) = match *filtered_tokens.as_slice() {
        [i @ mt![int]] => (i, Index, Step),
        [mt![~], i @ mt![int]] => (i, Relative, Step),
        [mt![=], i @ mt![int]] => (i, Index, Section),
        [mt![=], mt![~], i @ mt![int]] => (i, Relative, Section),

        // common errors
        [rel @ mt![~], sec @ mt![=], mt![int]] => {
            bp.error(ParserError::ComponentPartInvalid {
                container: "modifiers",
                what: "intermediate reference",
                reason: "Wrong relative section order",
                labels: vec![
                    label!(rel.span, "this should be"),
                    label!(sec.span, "after this"),
                ],
                help: Some("Swap the `~` and the `=`"),
            });
            return None;
        }
        [.., s @ mt![- | +], mt![int]] => {
            bp.error(ParserError::ComponentPartNotAllowed {
                container: "modifiers",
                what: "intermediate reference sign",
                to_remove: s.span,
                help: Some(
                    "The value cannot have a sign. They are indexes or relative always backwards.",
                ),
            });
            return None;
        }
        _ => {
            bp.error(ParserError::ComponentPartInvalid {
                container: "modifiers",
                what: "intermediate reference",
                reason: "Invalid reference syntax",
                labels: vec![label!(
                    tokens_span(inner_slice),
                    "this reference is not valid"
                )],
                help: Some(INTER_REF_HELP),
            });
            return None;
        }
    };

    let val = match bp.as_str(i).parse::<i16>() {
        Ok(val) => val,
        Err(err) => {
            bp.error(ParserError::ParseInt {
                bad_bit: i.span,
                source: err,
            });
            return None;
        }
    };

    let data = IntermediateData {
        ref_mode,
        target_kind,
        val,
    };

    Some(Located::new(data, tokens_span(slice)))
}

fn parse_alias<'input>(
    container: &'static str,
    bp: &mut BlockParser<'_, 'input>,
    tokens: &[Token],
    name_offset: usize,
) -> (Text<'input>, Option<Text<'input>>) {
    if let Some(alias_sep) = bp
        .extension(Extensions::COMPONENT_ALIAS)
        .then(|| tokens.iter().position(|t| t.kind == T![|]))
        .flatten()
    {
        let (name_tokens, alias_tokens) = tokens.split_at(alias_sep);
        let (alias_sep, alias_text_tokens) = alias_tokens.split_first().unwrap();
        let alias_text = bp.text(alias_sep.span.end(), alias_text_tokens);
        let alias_text = if alias_text_tokens.iter().any(|t| t.kind == T![|]) {
            let bad_bit = Span::new(
                alias_sep.span.start(),
                alias_text_tokens.last().unwrap_or(alias_sep).span.end(),
            );
            bp.error(ParserError::ComponentPartInvalid {
                container,
                what: "alias",
                reason: "multiple aliases",
                labels: vec![label!(bad_bit, "more than one alias defined here")],
                help: Some("A component can only have one alias. Remove the extra '|'."),
            });
            None
        } else if alias_text.is_text_empty() {
            bp.error(ParserError::ComponentPartInvalid {
                container,
                what: "alias",
                reason: "is empty",
                labels: vec![
                    label!(alias_sep.span, "remove this"),
                    label!(alias_text.span(), "or add something here"),
                ],
                help: None,
            });
            None
        } else {
            Some(alias_text)
        };
        (bp.text(name_offset, name_tokens), alias_text)
    } else {
        (bp.text(name_offset, tokens), None)
    }
}

const INGREDIENT: &str = "ingredient";
const COOKWARE: &str = "cookware";
const TIMER: &str = "timer";

fn ingredient<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    // Parse
    let start = bp.current_offset();
    bp.consume(T![@])?;
    let modifiers_pos = bp.current_offset();
    let modifiers_tokens = modifiers(bp);
    let name_offset = bp.current_offset();
    let body = comp_body(bp)?;
    let note = note(bp);
    let end = bp.current_offset();

    // Build text(s) and checks
    let (name, alias) = parse_alias(INGREDIENT, bp, body.name, name_offset);

    if name.is_text_empty() {
        bp.error(ParserError::ComponentPartInvalid {
            container: INGREDIENT,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name.span(), "add a name here")],
            help: None,
        });
    }

    let ParsedModifiers {
        flags: modifiers,
        intermediate_data,
    } = parse_modifiers(bp, modifiers_tokens, modifiers_pos);

    let quantity = body
        .quantity
        .map(|tokens| parse_quantity(bp, tokens).quantity);

    Some(Event::Ingredient(Located::new(
        ast::Ingredient {
            modifiers,
            intermediate_data,
            name,
            alias,
            quantity,
            note,
        },
        start..end,
    )))
}

fn cookware<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    // Parse
    let start = bp.current_offset();
    bp.consume(T![#])?;
    let modifiers_pos = bp.current_offset();
    let modifiers_tokens = modifiers(bp);
    let name_offset = bp.current_offset();
    let body = comp_body(bp)?;
    let note = note(bp);
    let end = bp.current_offset();

    // Errors
    let (name, alias) = parse_alias(COOKWARE, bp, body.name, name_offset);
    if name.is_text_empty() {
        bp.error(ParserError::ComponentPartInvalid {
            container: COOKWARE,
            what: "name",
            reason: "is empty",
            labels: vec![label!(name, "add a name here")],
            help: None,
        });
    }
    let quantity = body.quantity.map(|tokens| {
        let q = parse_quantity(bp, tokens);
        if let Some(unit) = &q.quantity.unit {
            let span = if let Some(sep) = q.unit_separator {
                Span::new(sep.start(), unit.span().end())
            } else {
                unit.span()
            };
            bp.error(ParserError::ComponentPartNotAllowed {
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
            bp.error(ParserError::ComponentPartNotAllowed {
                container: COOKWARE,
                what: "auto scale marker",
                to_remove: *auto_scale,
                help: Some("Cookware quantity can't be auto scaled."),
            });
        }
        q.quantity.map(|q| q.value)
    });
    let modifiers = parse_modifiers(bp, modifiers_tokens, modifiers_pos);
    let modifiers = check_intermediate_data(bp, modifiers, COOKWARE);

    if modifiers.contains(Modifiers::RECIPE) {
        let pos = modifiers_tokens
            .iter()
            .find(|t| t.kind == T![@])
            .map(|t| t.span)
            .expect("no recipe token in modifiers with recipe");

        bp.error(ParserError::ComponentPartInvalid {
            container: COOKWARE,
            what: "modifiers",
            reason: "recipe modifier not allowed in cookware",
            labels: vec![(pos, Some("remove this".into()))],
            help: None,
        });
    }

    Some(Event::Cookware(Located::new(
        ast::Cookware {
            name,
            alias,
            quantity,
            modifiers,
            note,
        },
        start..end,
    )))
}

fn timer<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    // Parse
    let start = bp.current_offset();
    bp.consume(T![~])?;
    let modifiers_tokens = modifiers(bp);
    let name_offset = bp.current_offset();
    let body = comp_body(bp)?;
    let end = bp.current_offset();

    // Errors
    check_modifiers(bp, modifiers_tokens, TIMER);
    check_alias(bp, body.name, TIMER);
    check_note(bp, TIMER);

    let name = bp.text(name_offset, body.name);

    let mut quantity = body.quantity.map(|tokens| {
        let q = parse_quantity(bp, tokens);
        if let ast::QuantityValue::Single {
            auto_scale: Some(auto_scale),
            ..
        } = &q.quantity.value
        {
            bp.error(ParserError::ComponentPartNotAllowed {
                container: TIMER,
                what: "auto scale marker",
                to_remove: *auto_scale,
                help: Some("Timer quantity can't be auto scaled."),
            });
        }
        if q.quantity.unit.is_none() {
            bp.error(ParserError::ComponentPartMissing {
                container: TIMER,
                what: "quantity unit",
                expected_pos: Span::pos(q.quantity.value.span().end()),
            });
        }
        q.quantity
    });

    if quantity.is_none() && bp.extension(Extensions::TIMER_REQUIRES_TIME) {
        let span = body.close.unwrap_or_else(|| Span::pos(name.span().end()));
        bp.error(ParserError::ComponentPartMissing {
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
        bp.error(ParserError::ComponentPartMissing {
            container: TIMER,
            what: "quantity OR name",
            expected_pos: span,
        });
        quantity = Some(Recover::recover()); // could be also name, but whatever
    }

    Some(Event::Timer(Located::new(
        ast::Timer { name, quantity },
        start..end,
    )))
}

fn check_modifiers(bp: &mut BlockParser, modifiers_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    assert_ne!(container, COOKWARE);
    if !modifiers_tokens.is_empty() {
        bp.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "modifiers",
            to_remove: tokens_span(modifiers_tokens),
            help: Some("Modifiers are only available in ingredients and cookware"),
        });
    }
}

fn check_intermediate_data(
    bp: &mut BlockParser,
    parsed_modifiers: ParsedModifiers,
    container: &'static str,
) -> Located<Modifiers> {
    assert_ne!(container, INGREDIENT);
    if let Some(inter_data) = parsed_modifiers.intermediate_data {
        bp.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "intermediate reference modifier",
            to_remove: inter_data.span(),
            help: Some("Intermediate references are only available in ingredients"),
        })
    }
    parsed_modifiers.flags
}

fn check_alias(bp: &mut BlockParser, name_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if let Some(sep) = name_tokens.iter().position(|t| t.kind == T![|]) {
        let to_remove = Span::new(
            name_tokens[sep].span.start(),
            name_tokens.last().unwrap().span.end(),
        );
        bp.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "alias",
            to_remove,
            help: Some("Aliases are only available in ingredients"),
        });
    }
}

fn check_note(bp: &mut BlockParser, container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if !bp.extension(Extensions::COMPONENT_NOTE) {
        return;
    }

    assert!(bp
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
    use crate::{context::Context, parser::token_stream::TokenStream};
    use test_case::test_case;

    fn t(input: &str) -> (Vec<Event>, Context<ParserError, ParserWarning>) {
        let tokens = TokenStream::new(input).collect::<Vec<_>>();
        let mut bp = BlockParser::new(0, &tokens, input, Extensions::all());
        step(&mut bp);
        let mut events = Vec::new();
        let mut ctx = Context::default();

        for ev in bp.finish() {
            match ev {
                Event::Error(err) => ctx.error(err),
                Event::Warning(warn) => ctx.warn(warn),
                _ => events.push(ev),
            }
        }
        let [Event::StartStep {..}, items @ .., Event::EndStep { .. }] = events.as_slice() else { panic!() };
        (Vec::from(items), ctx)
    }

    macro_rules! igr {
        ($item:expr) => {
            match $item {
                Event::Ingredient(igr) => igr,
                _ => panic!(),
            }
        };
    }

    #[test_case("@&(~1)one step back{}" => (
        Located::new(Modifiers::REF, 1..6),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Relative,
            target_kind: IntermediateTargetKind::Step,
            val: 1
            }, 2..6)
    ); "step relative")]
    #[test_case("@&(1)step index 1{}" => (
        Located::new(Modifiers::REF, 1..5),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Index,
            target_kind: IntermediateTargetKind::Step,
            val: 1
        }, 2..5)
    ); "step index")]
    #[test_case("@&(=~1)one section back{}" => (
        Located::new(Modifiers::REF, 1..7),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Relative,
            target_kind: IntermediateTargetKind::Section,
            val: 1
        }, 2..7)
    ); "section relative")]
    #[test_case("@&(=1)section index 1{}" => (
        Located::new(Modifiers::REF, 1..6),
        Located::new(IntermediateData {
            ref_mode: IntermediateRefMode::Index,
            target_kind: IntermediateTargetKind::Section,
            val: 1
        }, 2..6)
    ); "section index")]
    fn intermediate_ref(input: &str) -> (Located<Modifiers>, Located<IntermediateData>) {
        let (s, ctx) = t(input);
        let igr = igr!(&s[0]);
        assert!(ctx.is_empty());
        (igr.modifiers, igr.intermediate_data.unwrap())
    }

    #[test_case("@&(~=1)name{}"; "swap ~ =")]
    #[test_case("@&(9999999999999999999999999999999999999999)name{}"; "number too big")]
    #[test_case("@&(awebo)name{}"; "unexpected syntax")]
    #[test_case("#&(1)name"; "cookware")]
    #[test_case("~&(1){1%min}"; "timer")]
    fn intermediate_ref_errors(input: &str) {
        let (_, ctx) = t(input);
        assert_eq!(ctx.errors.len(), 1);
    }
}
