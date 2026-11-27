//! The iOS recogniser's rules: an amount at the end or the start of a line.

use crate::number::{is_digit, typed_number};
use crate::units::{match_unit, UnitMatch};
use crate::Amount;

pub(crate) struct RulesResult {
    /// Words joined by single spaces; not yet cleaned.
    pub name: String,
    pub amount: Option<Amount>,
    /// The amount was taken from the end of the line.
    pub from_end: bool,
}

struct Found {
    amount: Amount,
    name_tokens: Vec<String>,
    from_end: bool,
}

/// Dropped from the end of the last word: "bread,", "Milk 2 l,".
const LINE_ENDERS: [char; 2] = [',', ';'];
/// A word made only of these separates name and amount: "Milk - 2 l".
const SEPARATORS: [char; 6] = [',', ';', ':', '-', '\u{2013}', '\u{2014}'];
/// Trimmed from the name's edge next to the amount: "potato, 2 kg".
const EDGE_TRIMMED: [char; 3] = [',', ';', ':'];

pub(crate) fn recognize(line: &str) -> RulesResult {
    let mut tokens: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    if let Some(last) = tokens.last() {
        let trimmed = last.trim_end_matches(LINE_ENDERS).to_owned();
        if trimmed.is_empty() {
            tokens.pop();
        } else if let Some(slot) = tokens.last_mut() {
            *slot = trimmed;
        }
    }
    let whole = RulesResult {
        name: tokens.join(" "),
        amount: None,
        from_end: false,
    };
    if tokens.len() < 2 {
        return whole;
    }
    let Some(found) = amount_at_end(&tokens).or_else(|| amount_at_start(&tokens)) else {
        return whole;
    };
    let name = without_separators(found.name_tokens, found.from_end);
    if name.is_empty() {
        return whole;
    }
    RulesResult {
        name: name.join(" "),
        amount: Some(found.amount),
        from_end: found.from_end,
    }
}

fn amount_of(value: f64, unit: UnitMatch) -> Amount {
    match unit {
        UnitMatch::Unit(stored) => Amount {
            value,
            unit: Some(stored.to_owned()),
        },
        UnitMatch::Count => Amount { value, unit: None },
    }
}

fn amount_at_end(tokens: &[String]) -> Option<Found> {
    let n = tokens.len();
    let last = &tokens[n - 1];
    if let Some(amount) = glued_amount(last) {
        return Some(Found {
            amount,
            name_tokens: tokens[..n - 1].to_vec(),
            from_end: true,
        });
    }
    if let (Some(value), Some(unit)) = (typed_number(&tokens[n - 2]), match_unit(last)) {
        return Some(Found {
            amount: amount_of(value, unit),
            name_tokens: tokens[..n - 2].to_vec(),
            from_end: true,
        });
    }
    if let Some(value) = typed_number(last) {
        return Some(Found {
            amount: Amount { value, unit: None },
            name_tokens: tokens[..n - 1].to_vec(),
            from_end: true,
        });
    }
    None
}

fn amount_at_start(tokens: &[String]) -> Option<Found> {
    let first = &tokens[0];
    if let Some(amount) = glued_amount(first) {
        return Some(Found {
            amount,
            name_tokens: tokens[1..].to_vec(),
            from_end: false,
        });
    }
    let value = typed_number(first)?;
    if let Some(unit) = match_unit(&tokens[1]) {
        return Some(Found {
            amount: amount_of(value, unit),
            name_tokens: tokens[2..].to_vec(),
            from_end: false,
        });
    }
    Some(Found {
        amount: Amount { value, unit: None },
        name_tokens: tokens[1..].to_vec(),
        from_end: false,
    })
}

/// "2l", "1.5kg", "1/2kg", "3шт": a number with a table unit glued on.
/// Not "7up": "up" is no unit.
pub(crate) fn glued_amount(token: &str) -> Option<Amount> {
    let end = token
        .char_indices()
        .find(|(_, c)| !(is_digit(*c) || matches!(c, '.' | ',' | '/')))
        .map_or(token.len(), |(i, _)| i);
    let mut digits = &token[..end];
    while let Some(c) = digits.chars().last() {
        if is_digit(c) {
            break;
        }
        digits = &digits[..digits.len() - c.len_utf8()];
    }
    let rest = &token[digits.len()..];
    if rest.is_empty() || digits.matches('/').count() > 1 {
        return None;
    }
    let value = typed_number(digits)?;
    Some(amount_of(value, match_unit(rest)?))
}

/// Drops words that are only separators, and separators glued to the name's
/// edge next to the amount, so "potato," and "Milk -" read as names.
fn without_separators(mut name: Vec<String>, from_end: bool) -> Vec<String> {
    loop {
        let edge = if from_end { name.last() } else { name.first() };
        let Some(token) = edge.cloned() else { break };
        let rest = if from_end {
            token.trim_end_matches(EDGE_TRIMMED)
        } else {
            token.trim_start_matches(EDGE_TRIMMED)
        };
        if rest.is_empty() || token.chars().all(|c| SEPARATORS.contains(&c)) {
            if from_end {
                name.pop();
            } else {
                name.remove(0);
            }
            continue;
        }
        let rest = rest.to_owned();
        if from_end {
            if let Some(slot) = name.last_mut() {
                *slot = rest;
            }
        } else {
            name[0] = rest;
        }
        break;
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> (String, Option<f64>, Option<String>, bool) {
        let r = recognize(line);
        let value = r.amount.as_ref().map(|a| a.value);
        let unit = r.amount.and_then(|a| a.unit);
        (r.name, value, unit, r.from_end)
    }

    #[test]
    fn smoke() {
        assert_eq!(
            run("Milk 2 l"),
            ("Milk".into(), Some(2.0), Some("l".into()), true)
        );
        assert_eq!(
            run("2l milk"),
            ("milk".into(), Some(2.0), Some("l".into()), false)
        );
        assert_eq!(run("7up"), ("7up".into(), None, None, false));
        assert_eq!(
            run("Milk - 2 l"),
            ("Milk".into(), Some(2.0), Some("l".into()), true)
        );
        assert_eq!(
            run("2 kg - potatoes"),
            ("potatoes".into(), Some(2.0), Some("kg".into()), false)
        );
    }
}
