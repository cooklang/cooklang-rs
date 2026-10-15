//! CLDR plural categories, so a long form can pick between `1 łyżeczka`,
//! `2 łyżeczki` and `5 łyżeczek`.
//!
//! The rules are CLDR's own, now carried in each `locales/<tag>.toml`. Evaluated
//! here rather than taken from a plural-rules crate: they are part of how a
//! unit is *written*, the same footing as the names (ADR 3), and a CLDR
//! bump then refreshes rules and forms together. The syntax is small —
//! operands `n i v w f t` (and `e`, always 0 here), `=` and `!=` against
//! values and `a..b` ranges, `%`, `and`, `or`. The tests use CLDR's own
//! sample numbers, kept in `locales/plural-samples.json`, as the oracle.

/// The operands a rule is written against, taken from the number *as it
/// will be printed*: `v` and `f` depend on the digits shown, which is why
/// this takes text and not an `f64`.
struct Operands {
    /// Absolute value.
    n: f64,
    /// Integer digits.
    i: u64,
    /// Count of visible fraction digits, with trailing zeros.
    v: u64,
    /// Count of visible fraction digits, without trailing zeros.
    w: u64,
    /// Visible fraction digits as an integer, with trailing zeros.
    f: u64,
    /// Visible fraction digits as an integer, without trailing zeros.
    t: u64,
}

fn operands(text: &str) -> Operands {
    let text = text.trim().trim_start_matches('-');
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    let frac_trimmed = frac.trim_end_matches('0');
    let digits = |s: &str| -> u64 {
        if s.is_empty() {
            0
        } else {
            s.parse().unwrap_or(u64::MAX)
        }
    };
    Operands {
        n: text.parse().unwrap_or(0.0),
        i: digits(int),
        v: frac.len() as u64,
        w: frac_trimmed.len() as u64,
        f: digits(frac),
        t: digits(frac_trimmed),
    }
}

/// `operand % modulus = ranges` or `!= ranges`.
struct Relation<'a> {
    operand: char,
    modulus: Option<u64>,
    negated: bool,
    ranges: Vec<(u64, u64)>,
    source: &'a str,
}

impl Relation<'_> {
    fn holds(&self, ops: &Operands) -> bool {
        // `n` may be fractional; a range or value relation on it is true
        // only for an integral value, as CLDR's `in` semantics require
        let (value, integral) = match self.operand {
            'n' => (ops.n, ops.n.fract() == 0.0),
            'i' => (ops.i as f64, true),
            'v' => (ops.v as f64, true),
            'w' => (ops.w as f64, true),
            'f' => (ops.f as f64, true),
            't' => (ops.t as f64, true),
            'e' | 'c' => (0.0, true),
            other => panic!("unknown plural operand '{other}' in '{}'", self.source),
        };
        let value = match self.modulus {
            Some(m) => value % m as f64,
            None => value,
        };
        let within = integral
            && self
                .ranges
                .iter()
                .any(|&(lo, hi)| value >= lo as f64 && value <= hi as f64);
        within != self.negated
    }
}

/// Parses one relation such as `i % 10 = 2..4` or `v != 0`.
fn relation(text: &str) -> Relation<'_> {
    let (lhs, negated, rhs) = match text.split_once("!=") {
        Some((l, r)) => (l, true, r),
        None => match text.split_once('=') {
            Some((l, r)) => (l, false, r),
            None => panic!("plural relation without '=': '{text}'"),
        },
    };
    let (operand, modulus) = match lhs.split_once('%') {
        Some((o, m)) => (o.trim(), Some(m.trim().parse().expect("modulus"))),
        None => (lhs.trim(), None),
    };
    let operand = {
        let mut chars = operand.chars();
        let c = chars
            .next()
            .unwrap_or_else(|| panic!("empty operand in '{text}'"));
        assert!(chars.next().is_none(), "operand is one letter: '{text}'");
        c
    };
    let ranges = rhs
        .split(',')
        .map(|item| {
            let item = item.trim();
            match item.split_once("..") {
                Some((lo, hi)) => (
                    lo.trim().parse().expect("range"),
                    hi.trim().parse().expect("range"),
                ),
                None => {
                    let v: u64 = item.parse().unwrap_or_else(|_| panic!("value in '{text}'"));
                    (v, v)
                }
            }
        })
        .collect();
    Relation {
        operand,
        modulus,
        negated,
        ranges,
        source: text,
    }
}

/// Whether a rule's condition holds. `or` binds loosest, then `and`.
fn holds(rule: &str, ops: &Operands) -> bool {
    let condition = rule.split('@').next().unwrap_or("").trim();
    if condition.is_empty() {
        // only the `other` rule is empty; it is the fallback, not a match
        return false;
    }
    condition.split(" or ").any(|and_condition| {
        and_condition
            .split(" and ")
            .all(|rel| relation(rel.trim()).holds(ops))
    })
}

/// The CLDR category for `number` (the digits as they will be printed)
/// under `rules`, a list of `(category, rule)` pairs. Categories other than
/// `other` are mutually exclusive by construction, so the first match wins;
/// nothing matching is `other`.
pub(crate) fn category<'r>(rules: &'r [(&'r str, &'r str)], number: &str) -> &'r str {
    let ops = operands(number);
    rules
        .iter()
        .filter(|(category, _)| *category != "other")
        .find(|(_, rule)| holds(rule, &ops))
        .map(|(category, _)| *category)
        .unwrap_or("other")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn operands_follow_the_printed_digits() {
        let o = operands("1");
        assert_eq!((o.i, o.v, o.w, o.f, o.t), (1, 0, 0, 0, 0));
        let o = operands("1.50");
        assert_eq!((o.i, o.v, o.w, o.f, o.t), (1, 2, 1, 50, 5));
        assert_eq!(o.n, 1.5);
        let o = operands("-18");
        assert_eq!((o.n, o.i), (18.0, 18));
    }

    #[test]
    fn polish_by_hand() {
        let rules = [
            ("one", "i = 1 and v = 0"),
            ("few", "v = 0 and i % 10 = 2..4 and i % 100 != 12..14"),
            ("many", "v = 0 and i != 1 and i % 10 = 0..1 or v = 0 and i % 10 = 5..9 or v = 0 and i % 100 = 12..14"),
            ("other", ""),
        ];
        assert_eq!(category(&rules, "1"), "one");
        assert_eq!(category(&rules, "2"), "few");
        assert_eq!(category(&rules, "12"), "many");
        assert_eq!(category(&rules, "22"), "few");
        assert_eq!(category(&rules, "25"), "many");
        assert_eq!(category(&rules, "1.5"), "other");
        assert_eq!(category(&rules, "0"), "many");
    }

    /// Every sample CLDR attaches to a rule must land in that rule's
    /// category: `@integer 2~4, 22~24, …` and `@decimal 0.0~1.5, 10.0, …`.
    /// Ranges are enumerated at the sample's precision; compact-notation
    /// samples (`1c6`) are skipped since we never print them.
    #[test]
    fn every_cldr_sample_lands_in_its_own_category() {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/locales/plural-samples.json"
        ))
        .unwrap();
        let locales: BTreeMap<String, BTreeMap<String, String>> =
            serde_json::from_str(&text).unwrap();
        let mut checked = 0;

        for (locale, raw) in &locales {
            let rules: Vec<(&str, &str)> = raw
                .iter()
                .map(|(k, v)| (k.trim_start_matches("pluralRule-count-"), v.as_str()))
                .collect();
            for (expected, rule) in &rules {
                for sample in samples(rule) {
                    assert_eq!(
                        category(&rules, &sample),
                        *expected,
                        "{locale}: sample {sample} of '{rule}'"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 1000, "only {checked} samples checked");
    }

    fn samples(rule: &str) -> Vec<String> {
        let mut out = Vec::new();
        for section in rule.split('@').skip(1) {
            let body = section
                .trim_start_matches("integer")
                .trim_start_matches("decimal");
            for item in body.split(',') {
                let item = item.trim();
                if item.is_empty() || item.contains('…') || item.contains('c') || item.contains('e')
                {
                    continue;
                }
                match item.split_once('~') {
                    Some((lo, hi)) => {
                        let decimals = lo.split_once('.').map_or(0, |(_, d)| d.len());
                        let scale = 10f64.powi(decimals as i32);
                        let lo_n = (lo.parse::<f64>().unwrap() * scale).round() as i64;
                        let hi_n = (hi.parse::<f64>().unwrap() * scale).round() as i64;
                        for k in lo_n..=hi_n {
                            out.push(format_scaled(k, decimals));
                        }
                    }
                    None => out.push(item.to_string()),
                }
            }
        }
        out
    }

    /// `k / 10^decimals` printed with exactly `decimals` digits.
    fn format_scaled(k: i64, decimals: usize) -> String {
        if decimals == 0 {
            return k.to_string();
        }
        let s = format!("{:0width$}", k, width = decimals + 1);
        let (int, frac) = s.split_at(s.len() - decimals);
        format!("{int}.{frac}")
    }
}
