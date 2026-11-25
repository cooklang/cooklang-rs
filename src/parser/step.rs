use smallvec::SmallVec;

use crate::{
    error::label, error::Recover, lexer::T, located::Located, parser::model::*, span::Span,
    text::Text, Extensions,
};

use super::{
    error, mt,
    quantity::parse_quantity,
    spec2::{find_tail_arrow, is_backtick, parse_tail, take_label, take_reason},
    token_stream::Token,
    tokens_span, warning, BlockKind, BlockParser, Event,
};

pub(crate) fn parse_step(bp: &mut BlockParser<'_, '_>) {
    bp.event(Event::Start(BlockKind::Step));

    // `?label:` at the very start of the step (spec proposal 0025)
    if bp.extension(Extensions::ALTERNATIVES) {
        if let Some(label) = bp.with_recover(|bp| {
            let label = take_label(bp)?;
            bp.consume(T![:])?;
            bp.consume_while(|t| t == T![ws]);
            Some(label)
        }) {
            bp.event(Event::StepLabel(label));
        }
    }

    // `-> name` at the end of the step (spec proposal 0027)
    let tail_at = if bp.extension(Extensions::PRODUCTS) {
        find_tail_arrow(bp, bp.rest()).map(|i| bp.current + i)
    } else {
        None
    };

    match tail_at {
        None => step_items(bp),
        Some(arrow) => {
            let head = &bp.tokens()[bp.current..arrow];
            if !head.is_empty() {
                let mut sub = BlockParser::new(head, bp.input, &mut *bp.events, bp.extensions);
                step_items(&mut sub);
                sub.finish();
            }
            bp.current = arrow + 2;
            let tail = bp.consume_rest();
            let products = parse_tail(bp, tail);
            if !products.is_empty() {
                bp.event(Event::Products(products));
            }
        }
    }

    bp.event(Event::End(BlockKind::Step));
}

fn step_items(bp: &mut BlockParser<'_, '_>) {
    let actions = bp.extension(Extensions::STEP_ACTIONS);
    let alternatives = bp.extension(Extensions::ALTERNATIVES);

    // a backtick that closed a pair which turned out to be text opens nothing
    let mut not_opener: Option<usize> = None;
    while !bp.rest().is_empty() {
        let next = bp.rest()[0];
        if actions && is_backtick(bp, next) && not_opener != Some(bp.current) {
            if let Some(ev) = bp.with_recover(action) {
                bp.event(ev);
                continue;
            }
            not_opener = closing_backtick(bp);
        }
        if alternatives && matches!(next.kind, T![@] | T![#] | T![~] | T!['(']) {
            if component_or_group(bp) {
                continue;
            }
        } else {
            let component = match bp.peek() {
                T![@] => bp.with_recover(ingredient),
                T![#] => bp.with_recover(cookware),
                T![~] => bp.with_recover(timer),
                _ => None,
            };
            if let Some(ev) = component {
                bp.event(ev);
                continue;
            }
        }
        let start = bp.current_offset();
        let tokens = bp.capture_slice(|bp| {
            bp.bump_any(); // consume the first token, this avoids entering an infinite loop
            while let Some(t) = bp.rest().first().copied() {
                if matches!(t.kind, T![@] | T![#] | T![~])
                    || (alternatives && t.kind == T!['('])
                    || (actions && is_backtick(bp, t))
                {
                    break;
                }
                bp.current += 1;
            }
        });
        let text = bp.text(start, tokens);
        if !text.fragments().is_empty() {
            bp.event(Event::Text(text));
        }
    }
}

/// Position of the backtick that closes the one at the parser's position, on
/// the same line.
fn closing_backtick(bp: &BlockParser) -> Option<usize> {
    let rest = bp.rest();
    for (i, t) in rest.iter().enumerate().skip(1) {
        if t.kind == T![newline] {
            return None;
        }
        if is_backtick(bp, *t) {
            return Some(bp.current + i);
        }
    }
    None
}

/// `` `Fold in` `` (spec proposal 0029). The pair closes on its line, holds
/// no component and is not empty; otherwise the backticks are text.
fn action<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Event<'i>> {
    let open = bp.bump_any();
    let rest = bp.rest();
    let mut i = 0;
    loop {
        let t = *rest.get(i)?;
        if is_backtick(bp, t) {
            break;
        }
        if matches!(t.kind, T![newline] | T![@] | T![#] | T![~]) {
            return None;
        }
        i += 1;
    }
    let inner = &rest[..i];
    if inner.iter().all(|t| t.kind == T![ws]) {
        return None;
    }
    bp.current += i + 1;
    Some(Event::Action(bp.text(open.span.end(), inner)))
}

/// One component, a labelled component, or a group of options
/// `@a{}|@b{}|(@c{} + @d{})` (spec proposal 0025). Returns false, consuming
/// nothing, when there is no component here.
fn component_or_group(bp: &mut BlockParser<'_, '_>) -> bool {
    let start = bp.current;
    let Some(first) = parse_option(bp, None) else {
        bp.current = start;
        return false;
    };
    let mut options = vec![first];
    let label = take_label(bp);
    let before_reason = bp.current;
    let reason = take_reason(bp);
    let mut meta = vec![(label, reason)];

    while let Some(skip) = pipe_ahead(bp) {
        let save = bp.current;
        bp.current += skip;
        match parse_option(bp, options[0].first()) {
            Some(opt) => {
                options.push(opt);
                let label = take_label(bp);
                let reason = take_reason(bp);
                meta.push((label, reason));
            }
            None => {
                bp.current = save;
                break;
            }
        }
    }

    let first_is_bracket = options[0].len() > 1;
    if options.len() == 1 && meta[0].0.is_none() {
        if first_is_bracket {
            // round brackets are a group only when a `|` touches them
            bp.current = start;
            return false;
        }
        // a reason is only read after an option or a labelled component
        if meta[0].1.is_some() {
            bp.current = before_reason;
        }
        for ev in options.pop().unwrap() {
            bp.event(ev);
        }
        return true;
    }

    bp.event(Event::GroupStart);
    for (opt, (label, reason)) in options.into_iter().zip(meta) {
        for ev in opt {
            bp.event(ev);
        }
        bp.event(Event::GroupOption { label, reason });
    }
    bp.event(Event::GroupEnd);
    true
}

/// If a `|` follows (spaces and one line break allowed on either side) and
/// then something that can be an option, the number of tokens up to it.
fn pipe_ahead(bp: &BlockParser) -> Option<usize> {
    let rest = bp.rest();
    let mut i = 0;
    let mut newlines = 0;
    let skip_ws = |i: &mut usize, newlines: &mut usize| {
        while let Some(t) = rest.get(*i) {
            match t.kind {
                T![ws] => *i += 1,
                T![newline] if *newlines == 0 => {
                    *newlines += 1;
                    *i += 1
                }
                _ => break,
            }
        }
    };
    skip_ws(&mut i, &mut newlines);
    if rest.get(i)?.kind != T![|] {
        return None;
    }
    i += 1;
    newlines = 0;
    skip_ws(&mut i, &mut newlines);
    matches!(rest.get(i)?.kind, T![@] | T![#] | T![~] | T!['{'] | T!['(']).then_some(i)
}

/// One option: a component, `{qty}` taking the first option's name, or
/// `(@a{} + @b{})`. Returns its events.
fn parse_option<'i>(
    bp: &mut BlockParser<'_, 'i>,
    first: Option<&Event<'i>>,
) -> Option<Vec<Event<'i>>> {
    match bp.peek() {
        T![@] => bp.with_recover(ingredient).map(|e| vec![e]),
        T![#] => bp.with_recover(cookware).map(|e| vec![e]),
        T![~] => bp.with_recover(timer).map(|e| vec![e]),
        T!['{'] => {
            let first = first?;
            bp.with_recover(|bp| {
                let open = bp.consume(T!['{'])?;
                let q = bp.until(|t| t == T!['}'])?;
                let close = bp.bump(T!['}']);
                let q_not_empty = q.iter().any(|t| t.kind != T![ws]);
                let quantity = q_not_empty.then(|| parse_quantity(bp, q).quantity);
                let span = open.span.start()..close.span.end();
                match first {
                    Event::Ingredient(i) => {
                        let mut i = i.clone().into_inner();
                        i.quantity = quantity;
                        i.note = None;
                        Some(vec![Event::Ingredient(Located::new(i, span))])
                    }
                    Event::Cookware(c) => {
                        let mut c = c.clone().into_inner();
                        c.quantity = quantity;
                        c.note = None;
                        Some(vec![Event::Cookware(Located::new(c, span))])
                    }
                    _ => None,
                }
            })
        }
        T!['('] => bp.with_recover(|bp| {
            bp.consume(T!['('])?;
            let mut events = Vec::new();
            loop {
                bp.consume_while(|t| t == T![ws]);
                let ev = match bp.peek() {
                    T![@] => bp.with_recover(ingredient)?,
                    T![#] => bp.with_recover(cookware)?,
                    _ => return None,
                };
                events.push(ev);
                bp.consume_while(|t| t == T![ws]);
                if bp.consume(T![+]).is_some() {
                    continue;
                }
                bp.consume(T![')'])?;
                break;
            }
            (events.len() >= 2).then_some(events)
        }),
        _ => None,
    }
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
        let quantity_not_empty = quantity
            .iter()
            .any(|t| !matches!(t.kind, T![ws] | T![block comment]));
        Some(Body {
            name,
            close: Some(close_span),
            quantity: quantity_not_empty.then_some(quantity),
        })
    })
    .or_else(|| {
        bp.with_recover(|bp| {
            let tokens = bp.consume_while(|t| matches!(t, T![word] | T![int] | T![zeroint]));
            if tokens.is_empty() {
                if !bp.rest().is_empty() && !bp.at(T![ws]) {
                    bp.warn(
                        warning!(
                            "Invalid single word name, the component will be ignored",
                            label!(
                                Span::pos(bp.current_offset()),
                                "expected single word name here"
                            ),
                        )
                        .hint("Add `{}` at the end of the name to use it, or change the name"),
                    );
                }
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

/// Consumes the modifier tokens after a component sigil.
///
/// The optional marker `?` is part of the core syntax for ingredients and
/// cookware (`allow_optional`), so it is recognised even without the
/// [`Extensions::COMPONENT_MODIFIERS`] extension. At most one `?` is taken:
/// in `@??thyme{}` the name is `?thyme`.
fn modifiers<'t>(bp: &mut BlockParser<'t, '_>, allow_optional: bool) -> &'t [Token] {
    let start = bp.current;

    if !bp.extension(Extensions::COMPONENT_MODIFIERS) {
        if allow_optional && bp.at(T![?]) {
            bp.bump_any();
        }
        return &bp.tokens()[start..bp.current];
    }

    let mut seen_optional = false;
    loop {
        match bp.peek() {
            T![?] if seen_optional => break,
            T![?] => {
                seen_optional = true;
                bp.bump_any();
            }
            T![@] | T![+] | T![-] => {
                bp.bump_any();
            }
            T![&] => {
                bp.bump_any();
                if bp.extension(Extensions::INTERMEDIATE_PREPARATIONS) {
                    bp.with_recover(|bp| {
                        bp.consume(T!['('])?;
                        let _intermediate = bp.until(|t| t == T![')'])?;
                        bp.bump(T![')']);
                        Some(())
                    });
                }
            }
            _ => break,
        }
    }
    &bp.tokens()[start..bp.current]
}

fn note<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Text<'i>> {
    bp.with_recover(|line| {
        line.consume(T!['('])?;
        let offset = line.current_offset();
        let note = line.until(|t| t == T![')'])?;
        line.bump(T![')']);
        Some(line.text(offset, note))
    })
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
                    if bp.extension(Extensions::INTERMEDIATE_PREPARATIONS) {
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
                bp.error(
                    error!(
                        format!("Duplicate modifier: {}", bp.token_str(*tok)),
                        label!(modifiers_span, "only leave one {}", bp.token_str(*tok)),
                    )
                    .hint("Order does not matter, but duplicates are not allowed"),
                );
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
    const INTER_PREP_HELP: &str = "The target is something like: `1`, `~1`, `=1` or `=~1`";
    const INVALID: &str = "Invalid intermediate preparation reference";

    // if '(' has been taken as a modifier token, it has taken until
    // a closing ')'

    if !matches!(tokens.clone().next(), Some(mt!['('])) {
        return None;
    }

    let slice = {
        let slice = tokens.as_slice();
        let end_pos = tokens
            .position(|t| t.kind == T![')']) // consumes until and including ')'
            .expect("No closing paren in intermediate preparation reference");
        &slice[..=end_pos]
    };
    let inner_slice = &slice[1..slice.len() - 1];

    let filtered_tokens: SmallVec<[Token; 3]> = inner_slice
        .iter()
        .filter(|t| !matches!(t.kind, T![ws] | T![block comment]))
        .copied()
        .collect();

    let (i, ref_mode, target_kind) = match *filtered_tokens.as_slice() {
        [i @ mt![int]] => (i, Number, Step),
        [mt![~], i @ mt![int]] => (i, Relative, Step),
        [mt![=], i @ mt![int]] => (i, Number, Section),
        [mt![=], mt![~], i @ mt![int]] => (i, Relative, Section),

        // common errors
        [] => {
            bp.error(
                error!(
                    format!("{INVALID}: empty"),
                    label!(tokens_span(slice), "add the target preparation here"),
                )
                .hint(INTER_PREP_HELP),
            );
            return None;
        }
        [rel @ mt![~], sec @ mt![=], mt![int]] => {
            bp.error(
                error!(
                    format!("{INVALID}: wrong relative section order"),
                    label!(rel.span, "the relative marker"),
                )
                .label(label!(sec.span, "goes after the section marker"))
                .hint("Swap the `~` and the `=`"),
            );
            return None;
        }
        [.., s @ mt![- | +], mt![int]] => {
            bp.error(
                error!(
                    format!("{INVALID}: value sign"),
                    label!(s.span, "remove this"),
                )
                .hint("The value cannot have a sign. It is absolute or relative always backwards"),
            );
            return None;
        }
        _ => {
            bp.error(error!(INVALID, label!(tokens_span(inner_slice))).hint(INTER_PREP_HELP));
            return None;
        }
    };

    let val = match bp.token_str(i).parse::<i16>() {
        Ok(val) => val,
        Err(err) => {
            bp.error(error!("Error parsing integer number", label!(i.span)).set_source(err));
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

fn parse_alias<'i>(
    container: &'static str,
    bp: &mut BlockParser<'_, 'i>,
    tokens: &[Token],
    name_offset: usize,
) -> (Text<'i>, Option<Text<'i>>) {
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
            bp.error(
                error!(
                    format!("Invalid {container}: multiple aliases"),
                    label!(bad_bit, "more than one alias defined here"),
                )
                .hint("A component can only have one alias"),
            );
            None
        } else if alias_text.is_text_empty() {
            bp.error(
                error!(
                    format!("Invalid {container}: empty alias"),
                    label!(alias_sep.span, "remove this"),
                )
                .hint("Either remove the `|` or add an alias"),
            );
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
    let modifiers_tokens = modifiers(bp, true);
    let name_offset = bp.current_offset();
    let body = comp_body(bp)?;
    let note = note(bp);
    let end = bp.current_offset();

    // Build text(s) and checks
    let (name, alias) = parse_alias(INGREDIENT, bp, body.name, name_offset);
    // `@{4}`: a quantity with no name (spec proposal 0030)
    let nameless = bp.extension(Extensions::NAMELESS_QUANTITIES)
        && name.is_text_empty()
        && body.quantity.is_some();
    if !nameless {
        check_empty_name(INGREDIENT, bp, &name);
    }

    let ParsedModifiers {
        flags: mut modifiers,
        intermediate_data,
    } = parse_modifiers(bp, modifiers_tokens, modifiers_pos);
    if nameless {
        // never listed
        *modifiers |= Modifiers::HIDDEN;
    }

    let quantity = body
        .quantity
        .map(|tokens| parse_quantity(bp, tokens).quantity);

    Some(Event::Ingredient(Located::new(
        Ingredient {
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
    let modifiers_tokens = modifiers(bp, true);
    let name_offset = bp.current_offset();
    let body = comp_body(bp)?;
    let note = note(bp);
    let end = bp.current_offset();

    // Errors
    let (name, alias) = parse_alias(COOKWARE, bp, body.name, name_offset);
    check_empty_name(COOKWARE, bp, &name);

    let quantity = body
        .quantity
        .map(|tokens| parse_quantity(bp, tokens).quantity);
    let modifiers = parse_modifiers(bp, modifiers_tokens, modifiers_pos);
    let modifiers = check_intermediate_data(bp, modifiers, COOKWARE);

    if modifiers.contains(Modifiers::RECIPE) {
        let pos = modifiers_tokens
            .iter()
            .find(|t| t.kind == T![@])
            .map(|t| t.span)
            .expect("no recipe token in modifiers with recipe");
        bp.error(
            error!(
                "Invalid cookware modifiers: recipe modifier not allowed",
                label!(pos, "remove this"),
            )
            .hint("Only ingredients can have the recipe modifier"),
        );
    }

    Some(Event::Cookware(Located::new(
        Cookware {
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
    let modifiers_tokens = modifiers(bp, false);
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
        let text_condition = bp.extension(Extensions::TEXT_TIMERS)
            && matches!(*q.quantity.value.value, crate::quantity::Value::Text(_));
        if q.quantity.unit.is_none() && !text_condition {
            bp.warn(
                warning!(
                    "Invalid timer quantity: missing unit",
                    label!(
                        Span::pos(q.quantity.value.span().end()),
                        "expected unit here"
                    ),
                )
                .hint("A timer needs a unit to know the duration"),
            )
        }
        q.quantity
    });

    if quantity.is_none() && bp.extension(Extensions::TIMER_REQUIRES_TIME) {
        let span = body.close.unwrap_or_else(|| Span::pos(name.span().end()));
        bp.error(error!(
            "Invalid timer: missing quantity",
            label!(span, "expected timer duration here"),
        ));
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
        bp.error(error!(
            "Invalid timer: neither quantity nor name",
            label!(span, "expected duration or name"),
        ));
        quantity = Some(Recover::recover()); // could be also name, but whatever
    }

    Some(Event::Timer(Located::new(
        Timer { name, quantity },
        start..end,
    )))
}

fn check_modifiers(bp: &mut BlockParser, modifiers_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    assert_ne!(container, COOKWARE);
    if !modifiers_tokens.is_empty() {
        bp.error(
            error!(
                format!("Invalid {container}: modifiers not allowed"),
                label!(tokens_span(modifiers_tokens), "remove this"),
            )
            .hint("Modifiers are only available in ingredients and cookware items"),
        );
    }
}

fn check_intermediate_data(
    bp: &mut BlockParser,
    parsed_modifiers: ParsedModifiers,
    container: &'static str,
) -> Located<Modifiers> {
    assert_ne!(container, INGREDIENT);
    if let Some(inter_data) = parsed_modifiers.intermediate_data {
        bp.error(
            error!(
                format!("Invalid {container}: intermediate preparation reference not allowed"),
                label!(inter_data.span(), "remove this"),
            )
            .hint("Intermediate preparation references are only available in ingredients"),
        );
    }
    parsed_modifiers.flags
}

fn check_alias(bp: &mut BlockParser, name_tokens: &[Token], container: &'static str) {
    assert_ne!(container, INGREDIENT);
    assert_ne!(container, COOKWARE);
    if !bp.extension(Extensions::COMPONENT_ALIAS) {
        return;
    }
    if let Some(sep) = name_tokens.iter().position(|t| t.kind == T![|]) {
        let to_remove = Span::new(
            name_tokens[sep].span.start(),
            name_tokens.last().unwrap().span.end(),
        );
        bp.error(
            error!(
                format!("Invalid {container}: alias not allowed"),
                label!(to_remove, "remove this"),
            )
            .hint("Aliases are only available in ingredients and cookware items"),
        );
    }
}

fn check_note(bp: &mut BlockParser, container: &'static str) {
    assert_ne!(container, INGREDIENT);
    assert_ne!(container, COOKWARE);

    assert!(bp
        .with_recover(|bp| {
            let start = bp.consume(T!['('])?.span.start();
            let _ = bp.until(|t| t == T![')'])?;
            let end = bp.bump(T![')']).span.end();
            bp.warn(
                warning!(
                    format!("A {container} cannot have a note, it will be text"),
                    label!(Span::new(start, end)),
                )
                .label(label!(Span::pos(start - 1), "add a space here")) // this at least will be the marker character
                .hint("Notes are only available in ingredients and cookware items"),
            );
            None::<()> // always backtrack
        })
        .is_none());
}

fn check_empty_name(container: &'static str, bp: &mut BlockParser, name: &Text) {
    if name.is_text_empty() {
        bp.error(error!(
            format!("Invalid {container} name: is empty"),
            label!(name.span(), "add a name here"),
        ));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{error::SourceReport, parser::token_stream::TokenStream};
    use test_case::test_case;

    fn t(input: &str) -> (Vec<Event<'_>>, SourceReport) {
        let mut tokens = TokenStream::new(input).collect::<Vec<_>>();
        // trim trailing newlines, block splitting should make sure this never
        // reaches the step function
        while let Some(mt![newline]) = tokens.last() {
            tokens.pop();
        }
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::all());
        parse_step(&mut bp);
        bp.finish();
        let mut other = Vec::new();
        let mut ctx = SourceReport::empty();

        for ev in events {
            match ev {
                Event::Error(err) | Event::Warning(err) => ctx.push(err),
                _ => other.push(ev),
            }
        }
        let [Event::Start(BlockKind::Step), items @ .., Event::End(BlockKind::Step)] =
            other.as_slice()
        else {
            panic!()
        };
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
            ref_mode: IntermediateRefMode::Number,
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
            ref_mode: IntermediateRefMode::Number,
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
        assert_eq!(ctx.errors().count(), 1);
    }

    #[test_case("bread" => "bread")]
    #[test_case("bread1" => "bread1")]
    #[test_case("bread01" => "bread01")]
    #[test_case("01bread" => "01bread")]
    #[test_case("1bread" => "1bread")]
    #[test_case("1" => "1")]
    #[test_case("01" => "01")]
    #[test_case("1.1" => "1")]
    #[test_case("01.1" => "01")]
    fn single_word_component(input: &str) -> String {
        let tokens = TokenStream::new(input).collect::<Vec<_>>();
        let mut events = VecDeque::new();
        let mut bp = BlockParser::new(&tokens, input, &mut events, Extensions::empty());
        let body = comp_body(&mut bp).expect("not parsed");
        bp.text(0, body.name).text_trimmed().into_owned()
    }
}
