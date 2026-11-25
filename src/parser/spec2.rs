//! Token helpers for the spec 2.0 draft extensions: products (0027), step
//! actions (0029), alternatives and labels (0025).

use crate::{error::label, lexer::T, text::Text};

use super::{
    quantity::parse_quantity, token_stream::Token, tokens_span, warning, BlockParser, ProductDecl,
};

/// Is `token` a punctuation token whose text is `s`?
pub(crate) fn is_punct(bp: &BlockParser, token: Token, s: &str) -> bool {
    token.kind == T![punctuation] && bp.token_str(token) == s
}

/// Is `token` a backtick?
pub(crate) fn is_backtick(bp: &BlockParser, token: Token) -> bool {
    is_punct(bp, token, "`")
}

fn is_label_char(kind: crate::lexer::TokenKind) -> bool {
    matches!(kind, T![word] | T![int] | T![zeroint])
}

/// Length in tokens of a label at the start of `tokens`, `?` included.
///
/// A label is `?` followed directly by letters and digits of any script, and
/// hyphens after the first character: `?vegetarian`, `?air-fryer`. The
/// qualified form `?choice=option` has one `=`.
pub(crate) fn label_len(_bp: &BlockParser, tokens: &[Token]) -> Option<usize> {
    if tokens.first()?.kind != T![?] || !is_label_char(tokens.get(1)?.kind) {
        return None;
    }
    let mut i = 2;
    let mut seen_eq = false;
    while let Some(t) = tokens.get(i) {
        match t.kind {
            k if is_label_char(k) => i += 1,
            T![-] if tokens.get(i + 1).is_some_and(|n| is_label_char(n.kind)) => i += 1,
            T![=] if !seen_eq && tokens.get(i + 1).is_some_and(|n| is_label_char(n.kind)) => {
                seen_eq = true;
                i += 1
            }
            _ => break,
        }
    }
    Some(i)
}

/// Reads a label at the parser's position. Returns the label without `?`.
pub(crate) fn take_label<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Text<'i>> {
    let n = label_len(bp, bp.rest())?;
    let toks = &bp.rest()[..n];
    let text = bp.text(toks[1].span.start(), &toks[1..]);
    bp.current += n;
    Some(text)
}

/// Reads `[reason]` at the parser's position. The reason may wrap a line.
pub(crate) fn take_reason<'i>(bp: &mut BlockParser<'_, 'i>) -> Option<Text<'i>> {
    let rest = bp.rest();
    let first = *rest.first()?;
    if !is_punct(bp, first, "[") {
        return None;
    }
    let end = rest.iter().position(|t| is_punct(bp, *t, "]"))?;
    let inner = &rest[1..end];
    let offset = first.span.end();
    let text = bp.text(offset, inner);
    bp.current += end + 1;
    Some(text)
}

/// Index of the `-` of the last `->` that has white space before it.
///
/// "Everything from that arrow to the end of the step is the tail"
/// (0027 rule 1). An arrow with no white space before it is text.
pub(crate) fn find_tail_arrow(_bp: &BlockParser, tokens: &[Token]) -> Option<usize> {
    (1..tokens.len().saturating_sub(1)).rev().find(|&i| {
        tokens[i].kind == T![-]
            && tokens[i + 1].kind == T![>]
            && matches!(tokens[i - 1].kind, T![ws] | T![newline])
    })
}

/// Parses the tokens after `->`: `yolks{4} + whites`, `pasta water{250%ml}`.
///
/// A tail that starts with `#` names no product (kept for locations).
pub(crate) fn parse_tail<'i>(bp: &mut BlockParser<'_, 'i>, tail: &[Token]) -> Vec<ProductDecl<'i>> {
    let trimmed: Vec<Token> = tail.to_vec();
    let first = trimmed
        .iter()
        .find(|t| !matches!(t.kind, T![ws] | T![newline] | T![line comment] | T![block comment]));
    match first {
        None => return Vec::new(),
        Some(t) if t.kind == T![#] => return Vec::new(),
        _ => {}
    }

    // split on ` + ` (white space on both sides)
    let mut parts: Vec<&[Token]> = Vec::new();
    let mut start = 0;
    for i in 0..tail.len() {
        if tail[i].kind == T![+]
            && i > 0
            && tail[i - 1].kind == T![ws]
            && tail.get(i + 1).is_some_and(|t| t.kind == T![ws])
        {
            parts.push(&tail[start..i]);
            start = i + 1;
        }
    }
    parts.push(&tail[start..]);

    let mut products = Vec::new();
    for part in parts {
        let brace = part.iter().position(|t| t.kind == T!['{']);
        let (name_toks, quantity) = match brace {
            Some(b) => {
                let close = part[b..].iter().position(|t| t.kind == T!['}']).map(|c| b + c);
                match close {
                    Some(c) => {
                        let q = &part[b + 1..c];
                        let q_not_empty = q.iter().any(|t| t.kind != T![ws]);
                        let quantity = q_not_empty.then(|| parse_quantity(bp, q).quantity);
                        (&part[..b], quantity)
                    }
                    None => (part, None),
                }
            }
            None => (part, None),
        };
        if name_toks.is_empty() {
            continue;
        }
        let name = bp.text(name_toks[0].span.start(), name_toks);
        if name.is_text_empty() {
            continue;
        }
        if name.text_trimmed().starts_with(['@', '#', '~', '&', '?']) {
            bp.warn(warning!(
                "A product name cannot start with @, #, ~, & or ?",
                label!(tokens_span(name_toks))
            ));
            continue;
        }
        products.push(ProductDecl { name, quantity });
    }
    products
}
