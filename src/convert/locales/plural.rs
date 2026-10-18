//! CLDR plural categories, so a long form can pick between `1 łyżeczka`, `2 łyżeczki` and `5 łyżeczek`.

/// The operands a rule is written against, taken from the number *as it will be printed*: `v` and `f` depend on the
/// digits shown, which is why this takes text and not an `f64`.
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
        // `n` may be fractional; a range or value relation on it is true only for an integral value, as CLDR's `in`
        // semantics require
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

/// The CLDR category for `number` (the digits as they will be printed) under `rules`, a list of `(category, rule)`
/// pairs. Categories other than `other` are mutually exclusive by construction, so the first match wins; nothing
/// matching is `other`.
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

    struct Eval<'a> {
        rules: &'a [(&'a str, &'a str)],
        expected: &'a [(&'a str, &'a [&'a str])],
    }

    #[track_caller]
    fn assert_plural_eval(Eval { rules, expected }: Eval) {
        for (expected, numbers) in expected {
            for number in *numbers {
                assert_eq!(
                    category(rules, number),
                    *expected,
                    "{number} under {rules:?}"
                );
            }
        }
    }

    #[test]
    #[rustfmt::skip]
    fn french() {
        assert_plural_eval(Eval {
            rules: &[
                ("one",   "i = 0,1"),
                ("many",  "e = 0 and i != 0 and i % 1000000 = 0 and v = 0 or e != 0..5"),
                ("other", ""),
            ],
            expected: &[
                ("one",   &["0", "1", "0.0", "0.7", "1.5"]),
                ("many",  &["1000000"]),
                ("other", &["2", "9", "17", "100", "1000", "2.0", "2.7", "3.5", "10.0", "100.0"]),
            ],
        });
    }

    #[test]
    #[rustfmt::skip]
    fn polish() {
        assert_plural_eval(Eval {
            rules: &[
                ("one",   "i = 1 and v = 0"),
                ("few",   "v = 0 and i % 10 = 2..4 and i % 100 != 12..14"),
                ("many",  "v = 0 and i != 1 and i % 10 = 0..1 or v = 0 and i % 10 = 5..9 or v = 0 and i % 100 = 12..14"),
                ("other", ""),
            ],
            expected: &[
                ("one",   &["1"]),
                ("few",   &["2", "3", "4", "22", "23", "24", "32", "34", "42", "44", "52", "54", "62", "102"]),
                ("many",  &["0", "5", "9", "11", "12", "14", "15", "19", "100", "1000", "1000000"]),
                ("other", &["0.0", "0.7", "1.0", "1.5", "10.0", "100.0"]),
            ],
        });
    }

    #[test]
    #[rustfmt::skip]
    fn danish() {
        assert_plural_eval(Eval {
            rules: &[
                ("one",   "n = 1 or t != 0 and i = 0,1"),
                ("other", ""),
            ],
            expected: &[
                ("one",   &["1", "0.1", "0.8", "1.6"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "0.0", "2.0", "2.7", "3.4", "10.0", "100.0"]),
            ],
        });
    }

    #[test]
    #[rustfmt::skip]
    fn arabic() {
        assert_plural_eval(Eval {
            rules: &[
                ("zero",  "n = 0"),
                ("one",   "n = 1"),
                ("two",   "n = 2"),
                ("few",   "n % 100 = 3..10"),
                ("many",  "n % 100 = 11..99"),
                ("other", ""),
            ],
            expected: &[
                ("zero",  &["0"]),
                ("one",   &["1"]),
                ("two",   &["2"]),
                ("few",   &["3", "6", "10", "103", "110", "1003", "3.0", "6.0", "10.0", "103.0"]),
                ("many",  &["11", "18", "26", "111", "1011", "11.0", "18.0", "26.0", "111.0"]),
                ("other", &["100", "101", "102", "1000", "1000000", "0.1", "0.5", "0.9", "1.1", "10.1", "100.0"]),
            ],
        });
    }
}
