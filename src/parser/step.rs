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
    mt, quantity::parse_quantity, token_stream::Token, tokens_span, LineParser, ParserError,
    ParserWarning,
};

pub struct ParsedStep<'input> {
    pub is_text: bool,
    pub items: Vec<ast::Item<'input>>,
}

pub(crate) fn step<'input>(
    line: &mut LineParser<'_, 'input>,
    force_text: bool,
) -> ParsedStep<'input> {
    let is_text = line.consume(T![>]).is_some() || force_text;

    let mut items: Vec<ast::Item> = vec![];

    if is_text {
        let start = line.current_offset();
        let tokens = line.consume_rest();
        let text = line.text(start, tokens);
        if !text.is_text_empty() {
            items.push(ast::Item::Text(text));
        }
    } else {
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
    }

    ParsedStep { is_text, items }
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

fn modifiers<'t>(line: &mut LineParser<'t, '_>) -> &'t [Token] {
    if !line.extension(Extensions::COMPONENT_MODIFIERS) {
        return &[];
    }

    let start = line.current;
    loop {
        match line.peek() {
            T![@] | T![?] | T![+] | T![-] => {
                line.bump_any();
            }
            T![&] => {
                line.bump_any();
                if line.extension(Extensions::INTERMEDIATE_INGREDIENTS) {
                    line.with_recover(|line| {
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
    &line.tokens()[start..line.current]
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

struct ParsedModifiers {
    flags: Located<Modifiers>,
    intermediate_data: Option<Located<IntermediateData>>,
}

// Parsing is defered so there are no errors for components that doesn't support modifiers
fn parse_modifiers(
    line: &mut LineParser,
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
                    if line.extension(Extensions::INTERMEDIATE_INGREDIENTS) {
                        intermediate_data = parse_intermediate_ref_data(line, &mut tokens);
                    }

                    Modifiers::REF
                }
                T![?] => Modifiers::OPT,
                T![+] => Modifiers::NEW,
                T![-] => Modifiers::HIDDEN,
                _ => panic!("Bad modifiers token sequence. Unexpected token: {tok:?}"),
            };

            if modifiers.contains(new_m) {
                line.error(ParserError::DuplicateModifiers {
                    modifiers_span,
                    dup: line.as_str(*tok).to_string(),
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
    line: &mut LineParser,
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
        line.error(ParserError::ComponentPartInvalid {
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
            line.error(ParserError::ComponentPartInvalid {
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
            line.error(ParserError::ComponentPartNotAllowed {
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
            line.error(ParserError::ComponentPartInvalid {
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

    let val = match line.as_str(i).parse::<i16>() {
        Ok(val) => val,
        Err(err) => {
            line.error(ParserError::ParseInt {
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
    line: &mut LineParser<'_, 'input>,
    tokens: &[Token],
    name_offset: usize,
) -> (Text<'input>, Option<Text<'input>>) {
    if let Some(alias_sep) = line
        .extension(Extensions::COMPONENT_ALIAS)
        .then(|| tokens.iter().position(|t| t.kind == T![|]))
        .flatten()
    {
        let (name_tokens, alias_tokens) = tokens.split_at(alias_sep);
        let (alias_sep, alias_text_tokens) = alias_tokens.split_first().unwrap();
        let alias_text = line.text(alias_sep.span.end(), alias_text_tokens);
        let alias_text = if alias_text_tokens.iter().any(|t| t.kind == T![|]) {
            let bad_bit = Span::new(
                alias_sep.span.start(),
                alias_text_tokens.last().unwrap_or(alias_sep).span.end(),
            );
            line.error(ParserError::ComponentPartInvalid {
                container,
                what: "alias",
                reason: "multiple aliases",
                labels: vec![label!(bad_bit, "more than one alias defined here")],
                help: Some("A component can only have one alias. Remove the extra '|'."),
            });
            None
        } else if alias_text.is_text_empty() {
            line.error(ParserError::ComponentPartInvalid {
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
        (line.text(name_offset, name_tokens), alias_text)
    } else {
        (line.text(name_offset, tokens), None)
    }
}

const INGREDIENT: &str = "ingredient";
const COOKWARE: &str = "cookware";
const TIMER: &str = "timer";

fn ingredient<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Ingredient<'input>> {
    // Parse
    line.consume(T![@])?;
    let modifiers_pos = line.current_offset();
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let body = comp_body(line)?;
    let note = note(line);

    // Build text(s) and checks
    let (name, alias) = parse_alias(INGREDIENT, line, body.name, name_offset);

    if name.is_text_empty() {
        line.error(ParserError::ComponentPartInvalid {
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
    } = parse_modifiers(line, modifiers_tokens, modifiers_pos);

    let quantity = body.quantity.map(|tokens| {
        parse_quantity(tokens, line.input, line.extensions, &mut line.context).quantity
    });

    Some(ast::Ingredient {
        modifiers,
        intermediate_data,
        name,
        alias,
        quantity,
        note,
    })
}

fn cookware<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Cookware<'input>> {
    // Parse
    line.consume(T![#])?;
    let modifiers_pos = line.current_offset();
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let body = comp_body(line)?;
    let note = note(line);

    // Errors
    let (name, alias) = parse_alias(COOKWARE, line, body.name, name_offset);
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
        q.quantity.map(|q| q.value)
    });
    let modifiers = parse_modifiers(line, modifiers_tokens, modifiers_pos);
    let modifiers = check_intermediate_data(line, modifiers, COOKWARE);

    if modifiers.contains(Modifiers::RECIPE) {
        let pos = modifiers_tokens
            .iter()
            .find(|t| t.kind == T![@])
            .map(|t| t.span)
            .expect("no recipe token in modifiers with recipe");

        line.error(ParserError::ComponentPartInvalid {
            container: COOKWARE,
            what: "modifiers",
            reason: "recipe modifier not allowed in cookware",
            labels: vec![(pos, Some("remove this".into()))],
            help: None,
        });
    }

    Some(ast::Cookware {
        name,
        alias,
        quantity,
        modifiers,
        note,
    })
}

fn timer<'input>(line: &mut LineParser<'_, 'input>) -> Option<ast::Timer<'input>> {
    // Parse
    line.consume(T![~])?;
    let modifiers_tokens = modifiers(line);
    let name_offset = line.current_offset();
    let body = comp_body(line)?;

    // Errors
    check_modifiers(line, modifiers_tokens, TIMER);
    check_alias(line, body.name, TIMER);
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

fn check_modifiers(line: &mut LineParser, modifiers_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    assert_ne!(container, COOKWARE);
    if !modifiers_tokens.is_empty() {
        line.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "modifiers",
            to_remove: tokens_span(modifiers_tokens),
            help: Some("Modifiers are only available in ingredients and cookware"),
        });
    }
}

fn check_intermediate_data(
    line: &mut LineParser,
    parsed_modifiers: ParsedModifiers,
    container: &'static str,
) -> Located<Modifiers> {
    assert_ne!(container, INGREDIENT);
    if let Some(inter_data) = parsed_modifiers.intermediate_data {
        line.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "intermediate reference modifier",
            to_remove: inter_data.span(),
            help: Some("Intermediate references are only available in ingredients"),
        })
    }
    parsed_modifiers.flags
}

fn check_alias(line: &mut LineParser, name_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    if let Some(sep) = name_tokens.iter().position(|t| t.kind == T![|]) {
        let to_remove = Span::new(
            name_tokens[sep].span.start(),
            name_tokens.last().unwrap().span.end(),
        );
        line.error(ParserError::ComponentPartNotAllowed {
            container,
            what: "alias",
            to_remove,
            help: Some("Aliases are only available in ingredients"),
        });
    }
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
                Item::Component(comp) => comp.clone().map(|comp| match comp {
                    ast::Component::Ingredient(igr) => igr,
                    _ => panic!(),
                }),
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
        let (s, ctx) = t!(input);
        let igr = igr!(&s.items[0]);
        assert!(ctx.is_empty());
        (igr.modifiers, igr.intermediate_data.unwrap())
    }

    #[test_case("@&(~=1)name{}"; "swap ~ =")]
    #[test_case("@&(9999999999999999999999999999999999999999)name{}"; "number too big")]
    #[test_case("@&(awebo)name{}"; "unexpected syntax")]
    #[test_case("#&(1)name"; "cookware")]
    #[test_case("~&(1){1%min}"; "timer")]
    fn intermediate_ref_errors(input: &str) {
        let (_, ctx) = t!(input);
        assert_eq!(ctx.errors.len(), 1);
    }
}
