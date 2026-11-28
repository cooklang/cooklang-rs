use smallvec::SmallVec;

use crate::{
    error::Recover,
    error::{label, SourceDiag},
    lexer::T,
    located::Located,
    quantity::{Number, Value},
    span::Span,
    Extensions,
};

use super::{error, model::*, mt, token_stream::Token, tokens_span, warning, BlockParser};

pub struct ParsedQuantity<'a> {
    pub quantity: Located<Quantity<'a>>,
    pub unit_separator: Option<Span>,
}

/// "parent" block parser. This is just to emit error/warnings and get the text. No tokens will be consumed
/// `tokens` inside '{' '}'. must not be empty
pub(crate) fn parse_quantity<'i>(
    bp: &mut BlockParser<'_, 'i>,
    tokens: &[Token],
) -> ParsedQuantity<'i> {
    assert!(!tokens.is_empty(), "empty quantity tokens. this is a bug.");

    // create an insolated sub-block for the quantity tokens
    let mut bp2 = BlockParser::new(tokens, bp.input, bp.events, bp.extensions);

    let advanced = bp2
        .extension(Extensions::ADVANCED_UNITS)
        .then(|| bp2.with_recover(parse_advanced_quantity))
        .flatten();

    advanced.unwrap_or_else(|| parse_regular_quantity(&mut bp2))
}

fn parse_regular_quantity<'i>(bp: &mut BlockParser<'_, 'i>) -> ParsedQuantity<'i> {
    let mut value = value(bp);
    let unit = match bp.peek() {
        // values parsed correctly and unit
        T![%] => {
            let sep = bp.bump_any();
            let unit = bp.consume_rest();
            Some((sep.span, bp.text(sep.span.end(), unit)))
        }
        // values parsed correctly but no unit
        T![eof] => None,
        // fallback
        _ => {
            bp.consume_while(|t| t != T![%]);
            let text = bp.text(bp.span().start(), bp.parsed());
            let text_val = Value::Text(text.text_trimmed().into_owned());
            value = QuantityValue {
                value: Located::new(text_val, text.span()),
                scaling_lock: None,
            };

            if let Some(sep) = bp.consume(T![%]) {
                let unit = bp.consume_rest();
                Some((sep.span, bp.text(sep.span.end(), unit)))
            } else {
                None
            }
        }
    };

    let (unit_separator, mut unit) = unit.unzip();
    if let Some(unit_text) = &unit {
        if unit_text.is_text_empty() {
            bp.warn(
                warning!(
                    "Empty quantity unit",
                    label!(unit_separator.unwrap(), "remove this")
                )
                .hint("Add a unit or remove the separator"),
            );
            unit = None;
        }
    }

    ParsedQuantity {
        quantity: Located::new(Quantity { value, unit }, tokens_span(bp.tokens())),
        unit_separator,
    }
}

fn parse_advanced_quantity<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<ParsedQuantity<'i>> {
    if bp.tokens().iter().any(|t| matches!(t.kind, T![%])) {
        return None;
    }

    let scaling_lock = scaling_lock(bp);

    bp.ws_comments();

    let value_tokens = bp.consume_while(|t| !matches!(t, T![word]));

    if value_tokens.is_empty() || value_tokens.last().unwrap().kind != T![ws] {
        return None;
    }
    let value_tokens = {
        // beginning already trimmed
        let end_pos = value_tokens
            .iter()
            .rposition(|t| !matches!(t.kind, T![ws] | T![block comment]))
            .unwrap(); // ws_comments were already cosumed and then checked non empty
        &value_tokens[..=end_pos]
    };

    let unit_tokens = bp.consume_rest();
    if unit_tokens.is_empty() {
        return None;
    }

    let value_span = {
        let start = value_tokens.first().unwrap().span.start();
        let end = value_tokens.last().unwrap().span.end();
        Span::new(start, end)
    };

    let result = range_value(value_tokens, bp).or_else(|| numeric_value(value_tokens, bp))?;
    let value = match result {
        Ok(value) => value,
        Err(err) => {
            bp.error(err);
            Value::recover()
        }
    };
    warn_reversed_range(&value, value_span, bp);
    let value = Located::new(value, value_span);

    let unit = bp.text(unit_tokens.first().unwrap().span.start(), unit_tokens);
    Some(ParsedQuantity {
        quantity: Located::new(
            Quantity {
                value: QuantityValue {
                    value,
                    scaling_lock,
                },
                unit: Some(unit),
            },
            tokens_span(bp.tokens()),
        ),
        unit_separator: None,
    })
}

fn value(bp: &mut BlockParser) -> QuantityValue {
    let scaling_lock = scaling_lock(bp);
    let value_tokens = bp.consume_while(|t| !matches!(t, T![%]));
    let value = parse_value(value_tokens, bp);

    QuantityValue {
        value,
        scaling_lock,
    }
}

fn scaling_lock(bp: &mut BlockParser) -> Option<Span> {
    bp.ws_comments();

    match bp.peek() {
        T![=] => {
            let tok = bp.bump_any();
            Some(tok.span)
        }
        _ => None,
    }
}

fn parse_value(tokens: &[Token], bp: &mut BlockParser) -> Located<Value> {
    let start = tokens
        .first()
        .map(|t| t.span.start())
        .unwrap_or(bp.current_offset()); // if empty, use the current offset
    let end = bp.current_offset();
    let span = Span::new(start, end);

    let result = range_value(tokens, bp)
        .or_else(|| numeric_value(tokens, bp))
        .unwrap_or_else(|| Ok(text_value(tokens, start, bp)));

    let val = match result {
        Ok(value) => value,
        Err(err) => {
            bp.error(err);
            Value::recover()
        }
    };
    warn_reversed_range(&val, span, bp);

    Located::new(val, span)
}

/// A range is kept as written, but `3-2` is most likely a typo.
fn warn_reversed_range(value: &Value, span: Span, bp: &mut BlockParser) {
    if let Value::Range { start, end } = value {
        if start.value() > end.value() {
            bp.warn(
                warning!(
                    "Range starts higher than it ends",
                    label!(span, "the start is larger than the end"),
                )
                .hint("Write the lower number first, as in `2-3`"),
            );
        }
    }
}

fn text_value(tokens: &[Token], offset: usize, bp: &mut BlockParser) -> Value {
    let text = bp.text(offset, tokens);
    if text.is_text_empty() {
        bp.error(error!(
            "Empty quantity value",
            label!(text.span(), "add value here"),
        ));
    }
    Value::Text(text.text_trimmed().into_owned())
}

/// A range separator: the hyphen-minus `-`, the en dash `–` (U+2013) or the
/// em dash `—` (U+2014). Other dash-like characters, such as the minus sign
/// (U+2212), are not.
fn is_range_separator(token: &Token, bp: &BlockParser) -> bool {
    match token.kind {
        T![-] => true,
        T![punctuation] => matches!(bp.token_str(*token), "\u{2013}" | "\u{2014}"),
        _ => false,
    }
}

/// Two numbers joined by a [range separator](is_range_separator), part of the
/// Cooklang specification (proposal 0011) and parsed with or without
/// [`Extensions::RANGE_VALUES`].
///
/// `None` if the tokens are not a range: no separator, either end missing or
/// not a number, or more than one separator (`1-2-3`).
fn range_value(tokens: &[Token], bp: &BlockParser) -> Option<Result<Value, SourceDiag>> {
    let mid = tokens.iter().position(|t| is_range_separator(t, bp))?;
    let (start, end) = tokens.split_at(mid);
    let (_mid, end) = end.split_first().unwrap();

    macro_rules! unwrap_numeric {
        ($r:expr) => {
            match $r {
                Ok(Value::Number(value)) => value,
                Err(err) => return Some(Err(err)),
                _ => unreachable!("numeric_value not number"),
            }
        };
    }

    let start = unwrap_numeric!(numeric_value(start, bp)?);
    let end = unwrap_numeric!(numeric_value(end, bp)?);
    Some(Ok(Value::Range { start, end }))
}

fn not_ws_comment(t: &Token) -> bool {
    !matches!(t.kind, T![ws] | T![line comment] | T![block comment])
}

fn trim_tokens(s: &[Token]) -> &[Token] {
    let from = match s.iter().position(not_ws_comment) {
        Some(i) => i,
        None => return &s[0..0],
    };
    let to = s.iter().rposition(not_ws_comment).unwrap();
    &s[from..=to]
}

fn numeric_value(tokens: &[Token], bp: &BlockParser) -> Option<Result<Value, SourceDiag>> {
    // remove spaces and comments from start to end
    let trimmed_tokens = trim_tokens(tokens);
    if trimmed_tokens.is_empty() {
        return None;
    }

    // check simple numbers

    // int or float
    // at the end, bare ints are converted to floats, so parse them as floats
    // to allow unnecesary large values for recipes :)
    let r = match trimmed_tokens {
        &[mt![int]] => Some(float(trimmed_tokens, bp)),
        &[mt![int], mt![.], mt![int | zeroint]] | &[mt![.], mt![int | zeroint]] => {
            Some(float(trimmed_tokens, bp))
        }
        _ => None,
    };
    if r.is_some() {
        return r.map(|r| r.map(Value::from));
    }

    // remove spaces and comments in between other tokens
    // numeric values will be at most 4 tokens
    let filtered_tokens: SmallVec<[Token; 4]> = trimmed_tokens
        .iter()
        .copied()
        .filter(not_ws_comment)
        .collect();

    // check complex values
    let r = match *filtered_tokens.as_slice() {
        // mixed number
        [i @ mt![int], a @ mt![int], mt![/], b @ mt![int]] => mixed_num(i, a, b, bp),
        // frac
        [a @ mt![int], mt![/], b @ mt![int]] => frac(a, b, bp),
        // other => not numeric
        _ => return None,
    };
    Some(r.map(Value::Number))
}

fn mixed_num(i: Token, a: Token, b: Token, bp: &BlockParser) -> Result<Number, SourceDiag> {
    let i = int(i, bp)?;
    let Number::Fraction { num, den, .. } = frac(a, b, bp)? else {
        unreachable!()
    };
    Ok(Number::Fraction {
        whole: i,
        num,
        den,
        err: 0.0,
    })
}

fn frac(a: Token, b: Token, line: &BlockParser) -> Result<Number, SourceDiag> {
    let span = Span::new(a.span.start(), b.span.end());
    let a = int(a, line)?;
    let b = int(b, line)?;

    if b == 0 {
        Err(error!("Division by zero", label!(span))
            .hint("Change this please, we don't want an infinite amount of anything"))
    } else {
        Ok(Number::Fraction {
            whole: 0,
            num: a,
            den: b,
            err: 0.0,
        })
    }
}

fn int(tok: Token, block: &BlockParser) -> Result<u32, SourceDiag> {
    assert_eq!(tok.kind, T![int]);
    block
        .token_str(tok)
        .parse()
        .map_err(|e| error!("Error parsing integer number", label!(tok.span)).set_source(e))
}

fn float(tokens: &[Token], bp: &BlockParser) -> Result<f64, SourceDiag> {
    bp.slice_str(tokens).parse::<f64>().map_err(|e| {
        error!("Error parsing decimal number", label!(tokens_span(tokens))).set_source(e)
    })
}

/// Parse the text between an ingredient's braces on its own, with no
/// extensions: the quantity of a shopping list line.
///
/// Never fails: whatever the grammar makes of the text (text values for
/// anything that isn't a number) is the quantity. Where the grammar reports
/// an error (`1/0`, a number too big for a fraction) the value is kept as the
/// text written instead of the grammar's placeholder, and so is a number too
/// large for an `f64`. Text ending in an unpaired `\` (which escapes nothing)
/// is kept whole as a text value. `None` for text with neither a value nor a
/// unit: empty, `%` or `=`.
pub(crate) fn parse_quantity_text(text: &str) -> Option<crate::quantity::Quantity> {
    if text.trim().is_empty() {
        return None;
    }
    let trailing_backslashes = text.chars().rev().take_while(|&c| c == '\\').count();
    if trailing_backslashes % 2 == 1 {
        return Some(crate::quantity::Quantity::new(
            Value::Text(text.trim().to_string()),
            None,
        ));
    }
    let tokens = super::token_stream::TokenStream::new(text).collect::<Vec<_>>();
    let mut events = std::collections::VecDeque::new();
    let mut bp = BlockParser::new(&tokens, text, &mut events, Extensions::empty());
    let parsed = parse_quantity(&mut bp, &tokens);
    bp.consume_rest();
    bp.finish();
    let failed = events
        .iter()
        .any(|event| matches!(event, super::Event::Error(_)));
    let quantity = parsed.quantity.into_inner();
    let (value, value_span) = quantity.value.value.take_pair();
    // A number too large for an f64 is infinite and would be written as
    // `inf`: keep it as written, like a grammar error.
    let not_finite = match &value {
        Value::Number(n) => !n.value().is_finite(),
        Value::Range { start, end } => !start.value().is_finite() || !end.value().is_finite(),
        Value::Text(_) => false,
    };
    let value = if failed || not_finite {
        Value::Text(text[value_span.range()].trim().to_string())
    } else {
        value
    };
    let unit = quantity
        .unit
        .map(|unit| unit.text_trimmed().into_owned())
        .filter(|unit| !unit.is_empty());
    if unit.is_none() && matches!(&value, Value::Text(t) if t.is_empty()) {
        return None;
    }
    Some(crate::quantity::Quantity::new(value, unit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::TokenStream;
    use test_case::test_case;

    macro_rules! t {
        ($input:expr) => {
            t!($input, $crate::Extensions::all())
        };
        ($input:expr, $extensions:expr) => {{
            let input = $input;
            let tokens = TokenStream::new(input).collect::<Vec<_>>();
            let mut events = std::collections::VecDeque::new();
            let mut bp = BlockParser::new(&tokens, input, &mut events, $extensions);
            let q = parse_quantity(&mut bp, &tokens);
            bp.consume_rest();
            bp.finish();
            let mut ctx = $crate::error::SourceReport::empty();
            events.into_iter().for_each(|ev| match ev {
                $crate::parser::Event::Error(e) | $crate::parser::Event::Warning(e) => ctx.push(e),
                _ => {}
            });
            (q.quantity.into_inner(), q.unit_separator, ctx)
        }};
    }

    macro_rules! num {
        ($value:expr) => {
            Value::Number(Number::Regular($value))
        };
    }

    macro_rules! range {
        ($start:expr, $end:expr) => {
            Value::Range {
                start: Number::Regular($start),
                end: Number::Regular($end),
            }
        };
    }

    #[test]
    fn basic_quantity() {
        let (q, s, _) = t!("100%ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(num!(100.0), 0..3),
                scaling_lock: None,
            }
        );
        assert_eq!(s, Some(Span::new(3, 4)));
        assert_eq!(q.unit.unwrap().text(), "ml");
    }

    #[test]
    fn fixed_quantity() {
        let (q, s, _) = t!("=100%ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(num!(100.0), 1..4),
                scaling_lock: Some(Span::new(0, 1)),
            }
        );
        assert_eq!(s, Some(Span::new(4, 5)));
        assert_eq!(q.unit.unwrap().text(), "ml");
    }

    #[test]
    fn fixed_quantity_with_spaces() {
        let (q, s, _) = t!(" = 100%ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(num!(100.0), 2..6),
                scaling_lock: Some(Span::new(1, 2)),
            }
        );
        assert_eq!(s, Some(Span::new(6, 7)));
        assert_eq!(q.unit.unwrap().text(), "ml");
    }

    #[test]
    fn no_separator_ext() {
        let (q, s, ctx) = t!("100 ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(num!(100.0), 0..3),
                scaling_lock: None,
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit.unwrap().text(), "ml");
        assert!(ctx.is_empty());

        let (q, s, ctx) = t!("100 ml", Extensions::all() ^ Extensions::ADVANCED_UNITS);
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(Value::Text("100 ml".into()), 0..6),
                scaling_lock: None,
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit, None);
        assert!(ctx.is_empty());
    }

    #[test]
    fn no_separator_range() {
        let (q, s, ctx) = t!("100-200 ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(100.0, 200.0), 0..7),
                scaling_lock: None,
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit.unwrap().text(), "ml");
        assert!(ctx.is_empty());

        let (q, s, ctx) = t!("1 - 2 1 / 2 ml");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(
                    Value::Range {
                        start: 1.0.into(),
                        end: Number::Fraction {
                            whole: 2,
                            num: 1,
                            den: 2,
                            err: 0.0
                        }
                    },
                    0..11
                ),
                scaling_lock: None,
            }
        );
        assert_eq!(s, None);
        assert_eq!(q.unit.unwrap().text(), "ml");
        assert!(ctx.is_empty());
    }

    #[test]
    fn escaped_multibyte_char() {
        let (q, _, r) = t!("\\½%\\🧂", Extensions::empty());
        assert_eq!(q.value.value.into_inner(), Value::Text("½".into()));
        assert_eq!(q.unit.unwrap().text_trimmed(), "🧂");
        assert!(r.is_empty());
    }

    #[test]
    fn range_value() {
        let (q, _, _) = t!("2-3");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(2.0, 3.0), 0..3),
                scaling_lock: None,
            }
        );
        assert_eq!(q.unit, None);
    }

    #[test]
    fn range_value_without_extensions() {
        // ranges are core syntax (spec 0011), not an extension
        let (q, _, r) = t!("2-3", Extensions::empty());
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(2.0, 3.0), 0..3),
                scaling_lock: None,
            }
        );
        assert_eq!(q.unit, None);
        assert!(r.is_empty());
    }

    #[test_case("2-3" ; "hyphen-minus")]
    #[test_case("2–3" ; "en dash")]
    #[test_case("2—3" ; "em dash")]
    #[test_case("2 – 3" ; "en dash with spaces")]
    #[test_case(" 2 - 3 " ; "surrounding spaces")]
    fn range_separators(s: &str) {
        let (q, _, r) = t!(s, Extensions::empty());
        assert_eq!(q.value.value.into_inner(), range!(2.0, 3.0));
        assert!(r.is_empty());
    }

    #[test_case("2\u{2212}3" ; "minus sign")]
    #[test_case("2\u{2010}3" ; "hyphen")]
    #[test_case("2\u{2012}3" ; "figure dash")]
    #[test_case("2\u{2015}3" ; "horizontal bar")]
    #[test_case("2-" ; "open end")]
    #[test_case("-3" ; "open start")]
    #[test_case("1-2-3" ; "three numbers")]
    #[test_case("1–2-3" ; "three numbers mixed dashes")]
    #[test_case("2-few" ; "text end")]
    #[test_case("1-2 handfuls" ; "unit without separator")]
    fn not_a_range(s: &str) {
        let (q, _, r) = t!(s, Extensions::empty());
        assert_eq!(q.value.value.into_inner(), Value::Text(s.trim().into()));
        assert!(r.is_empty());
    }

    #[test]
    fn range_with_unit_and_scaling_lock() {
        let (q, s, r) = t!("=1-2%tsp", Extensions::empty());
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(1.0, 2.0), 1..4),
                scaling_lock: Some(Span::new(0, 1)),
            }
        );
        assert_eq!(s, Some(Span::new(4, 5)));
        assert_eq!(q.unit.unwrap().text(), "tsp");
        assert!(r.is_empty());
    }

    #[test]
    fn range_fraction_ends() {
        let (q, _, r) = t!("1/2-1%tsp", Extensions::empty());
        assert_eq!(
            q.value.value.into_inner(),
            Value::Range {
                start: Number::Fraction {
                    whole: 0,
                    num: 1,
                    den: 2,
                    err: 0.0
                },
                end: Number::Regular(1.0),
            }
        );
        assert!(r.is_empty());
    }

    #[test]
    fn reversed_range_is_kept_with_a_warning() {
        let (q, _, r) = t!("3-2", Extensions::empty());
        assert_eq!(q.value.value.into_inner(), range!(3.0, 2.0));
        assert!(r.errors().next().is_none());
        assert_eq!(r.warnings().count(), 1);

        let (q, _, r) = t!("2-2", Extensions::empty());
        assert_eq!(q.value.value.into_inner(), range!(2.0, 2.0));
        assert!(r.is_empty());
    }

    #[test]
    fn range_with_zero_denominator_is_an_error() {
        let (_, _, r) = t!("1/0-2", Extensions::empty());
        assert_eq!(r.errors().count(), 1);
        let (_, _, r) = t!("1-2/0", Extensions::empty());
        assert_eq!(r.errors().count(), 1);
    }

    #[test]
    fn range_mixed_value() {
        let (q, _, _) = t!("2 1/2-3");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(2.5, 3.0), 0..7),
                scaling_lock: None,
            }
        );
        assert_eq!(q.unit, None);

        let (q, _, _) = t!("2-3 1/2");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(2.0, 3.5), 0..7),
                scaling_lock: None,
            }
        );
        assert_eq!(q.unit, None);

        let (q, _, _) = t!("2 1/2-3 1/2");
        assert_eq!(
            q.value,
            QuantityValue {
                value: Located::new(range!(2.5, 3.5), 0..11),
                scaling_lock: None,
            }
        );
        assert_eq!(q.unit, None);
    }

    #[test_case("1/2" => (0, 1, 2); "fraction")]
    #[test_case("0 1/2" => (0, 1, 2); "zero whole")]
    #[test_case("01/2" => panics "not number"; "bad fraction")]
    #[test_case("2 1/2" => (2, 1, 2); "mixed value")]
    fn fractional_val(s: &str) -> (u32, u32, u32) {
        let (q, _, _) = t!(s);
        let value = q.value.value.into_inner();
        let Value::Number(num) = value else {
            panic!("not number")
        };
        let Number::Fraction {
            whole,
            num,
            den,
            err,
        } = num
        else {
            panic!("not fraction")
        };
        assert_eq!(err, 0.0);
        (whole, num, den)
    }

    #[test_case("1" => 1.0)]
    #[test_case("1.0" => 1.0)]
    #[test_case("10" => 10.0)]
    #[test_case("10.0000000" => 10.0)]
    #[test_case("10.05" => 10.05)]
    #[test_case("01" => panics "not number")]
    #[test_case("01.0" => panics "not number")]
    fn simple_numbers(s: &str) -> f64 {
        let (q, _, r) = t!(s);
        let value = q.value.value.into_inner();
        let Value::Number(num) = value else {
            panic!("not number")
        };
        let Number::Regular(n) = num else {
            panic!("not regular number")
        };
        assert!(r.is_empty(), "source error");
        n
    }

    mod text {
        use crate::quantity::{Number as QNumber, Quantity as Q, Value as QValue};

        fn text_q(input: &str) -> Option<Q> {
            super::super::parse_quantity_text(input)
        }

        fn num_q(n: f64, unit: Option<&str>) -> Option<Q> {
            Some(Q::new(
                QValue::Number(QNumber::Regular(n)),
                unit.map(str::to_string),
            ))
        }

        #[test]
        fn quantity_text_table() {
            assert_eq!(text_q("2%l"), num_q(2.0, Some("l")));
            assert_eq!(text_q(" 2 % l "), num_q(2.0, Some("l")));
            assert_eq!(text_q("3"), num_q(3.0, None));
            assert_eq!(text_q("0.50%kg"), num_q(0.5, Some("kg")));
            assert!(matches!(
                text_q("1/2%cup").unwrap().value(),
                QValue::Number(QNumber::Fraction {
                    whole: 0,
                    num: 1,
                    den: 2,
                    err: 0.0
                })
            ));
            assert!(matches!(
                text_q("1 1/2%cup").unwrap().value(),
                QValue::Number(QNumber::Fraction {
                    whole: 1,
                    num: 1,
                    den: 2,
                    err: 0.0
                })
            ));
            assert_eq!(
                text_q("a pinch"),
                Some(Q::new(QValue::Text("a pinch".into()), None))
            );
            assert_eq!(
                text_q("1-2%cup"),
                Some(Q::new(
                    QValue::Range {
                        start: QNumber::Regular(1.0),
                        end: QNumber::Regular(2.0)
                    },
                    Some("cup".into())
                ))
            );
            assert_eq!(text_q("1–2%cup"), text_q("1-2%cup"));
            assert_eq!(
                text_q("1-2 handfuls"),
                Some(Q::new(QValue::Text("1-2 handfuls".into()), None))
            );
            assert_eq!(
                text_q("1,5%kg").unwrap().value(),
                &QValue::Text("1,5".into())
            );
            assert_eq!(text_q("2%"), num_q(2.0, None));
            assert_eq!(text_q(""), None);
            assert_eq!(text_q("   "), None);
        }

        #[test]
        fn quantity_text_with_only_a_unit_is_text_or_empty_but_never_panics() {
            // "%l": no value. Whatever the grammar makes of it, it must not panic
            // and must keep the unit if it keeps anything.
            assert_eq!(
                text_q("%l"),
                Some(Q::new(QValue::Text("".into()), Some("l".into())))
            );
        }

        #[test]
        fn quantity_text_without_value_or_unit_is_none() {
            assert_eq!(text_q("%"), None);
            assert_eq!(text_q("="), None);
            assert_eq!(text_q(" % "), None);
        }

        #[test]
        fn quantity_text_keeps_what_the_grammar_rejects_as_text() {
            let text = |s: &str| Some(Q::new(QValue::Text(s.into()), None));
            assert_eq!(text_q("1/0"), text("1/0"));
            assert_eq!(
                text_q("1/0%cup"),
                Some(Q::new(QValue::Text("1/0".into()), Some("cup".into())))
            );
            assert_eq!(text_q("4294967296/2"), text("4294967296/2"));
            assert_eq!(text_q("01"), text("01"));
            assert_eq!(text_q("1.5.5"), text("1.5.5"));
        }

        #[test]
        fn quantity_text_range_is_written_with_a_hyphen() {
            for input in ["1-2%tbsp", "1–2%tbsp", "1—2%tbsp", "1 - 2%tbsp"] {
                assert_eq!(text_q(input).unwrap().to_cooklang(), "1-2%tbsp", "{input}");
            }
            assert_eq!(text_q("1 1/2-2%cup").unwrap().to_cooklang(), "1 1/2-2%cup");
        }

        #[test]
        fn quantity_text_range_like_text_stays_text() {
            // a text value that looks like a range is escaped so it reads back as text
            for t in ["2-3", "2–3", "1/2-1"] {
                let q = Q::new(QValue::Text(t.into()), None);
                let written = q.to_cooklang();
                assert_eq!(text_q(&written), Some(q), "{t} written as {written}");
            }
        }

        #[test]
        fn quantity_text_scaling_lock_is_dropped() {
            assert_eq!(text_q("=2%l"), num_q(2.0, Some("l")));
        }

        #[test]
        fn quantity_text_written_back_reads_the_same() {
            for input in [
                "2%l",
                "1/2%cup",
                "1 1/2%cup",
                "2 0/3",
                ".5",
                "0.0004%kg",
                "a pinch",
                "1-2%cup",
                "1–2%cup",
                "1 1/2-2%cup",
                "1,5%kg",
                "%l",
                "1/0",
                "1/0%cup",
                "4294967296/2",
                "01",
                "1.5.5",
                "2%a%b",
                "100000000000000000000000",
            ] {
                let once = text_q(input).unwrap();
                let written = once.to_cooklang();
                let twice = text_q(&written).unwrap();
                assert_eq!(once, twice, "{input}");
                assert_eq!(written, twice.to_cooklang(), "{input}");
            }
        }
    }
}
