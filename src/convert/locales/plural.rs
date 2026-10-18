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
    use crate::convert::locales::locale::PluralRules;
    use crate::convert::locales::vendored::{vendored_locale, vendored_tags};
    use std::collections::BTreeSet;

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

    /// Each `(number, category)` holds under `rules`.
    #[track_caller]
    fn assert_plural_eval(rules: &[(&str, &str)], expected: &[(&str, &str)]) {
        for (number, expected) in expected {
            assert_eq!(
                category(rules, number),
                *expected,
                "{number} under {rules:?}"
            );
        }
    }

    /// The rules a vendored locale ships, from its TOML.
    fn rules_of(tag: &str) -> PluralRules {
        vendored_locale(tag)
            .unwrap_or_else(|| panic!("{tag} is not vendored"))
            .plural_rules()
    }

    /// The samples CLDR 48.2.3 attaches to each rule in `cldr-core/supplemental/plurals.json`, for one locale per
    /// distinct rule set among the vendored locales: every listed value up to 1000, both ends and a middle of every
    /// range, and 1000000, which the Romance `many` rules turn on. Compact samples (`1c6`) are never printed and are
    /// left out, as are decimals past 100.0, which repeat the same digits.
    #[rustfmt::skip]
    mod cldr_samples {
        use super::*;

        /// Also en-001, en-GB, en-AU, de, nl, sv, fi.
        #[test]
        fn en() {
            assert_plural_eval(rules_of("en"), &[
                ("1", "one"),
                ("0", "other"), ("2", "other"), ("9", "other"), ("16", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("0.0", "other"), ("0.7", "other"), ("1.5", "other"), ("10.0", "other"),
                ("100.0", "other"),
            ]);
        }

        /// Also zh, zh-Hant, ko, th, vi, id, ms.
        #[test]
        fn ja() {
            assert_plural_eval(rules_of("ja"), &[
                ("0", "other"), ("7", "other"), ("15", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("0.0", "other"), ("0.7", "other"), ("1.5", "other"), ("10.0", "other"),
                ("100.0", "other"),
            ]);
        }

        /// Also es-419, es-US, es-MX, es-CO.
        #[test]
        fn es() {
            assert_plural_eval(rules_of("es"), &[
                ("1", "one"), ("1.0", "one"), ("1.00", "one"), ("1.000", "one"), ("1.0000", "one"),
                ("1000000", "many"),
                ("0", "other"), ("2", "other"), ("9", "other"), ("16", "other"), ("100", "other"), ("1000", "other"),
                ("0.0", "other"), ("0.4", "other"), ("0.9", "other"), ("1.1", "other"), ("1.3", "other"),
                ("1.6", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        /// Also hu, tr, el.
        #[test]
        fn no() {
            assert_plural_eval(rules_of("no"), &[
                ("1", "one"), ("1.0", "one"), ("1.00", "one"), ("1.000", "one"), ("1.0000", "one"),
                ("0", "other"), ("2", "other"), ("9", "other"), ("16", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("0.0", "other"), ("0.4", "other"), ("0.9", "other"), ("1.1", "other"),
                ("1.3", "other"), ("1.6", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        /// Also pt-PT.
        #[test]
        fn it() {
            assert_plural_eval(rules_of("it"), &[
                ("1", "one"),
                ("1000000", "many"),
                ("0", "other"), ("2", "other"), ("9", "other"), ("16", "other"), ("100", "other"), ("1000", "other"),
                ("0.0", "other"), ("0.7", "other"), ("1.5", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        /// Also uk.
        #[test]
        fn ru() {
            assert_plural_eval(rules_of("ru"), &[
                ("1", "one"), ("21", "one"), ("31", "one"), ("41", "one"), ("51", "one"), ("61", "one"), ("71", "one"),
                ("81", "one"), ("101", "one"),
                ("2", "few"), ("3", "few"), ("4", "few"), ("22", "few"), ("23", "few"), ("24", "few"), ("32", "few"),
                ("33", "few"), ("34", "few"), ("42", "few"), ("43", "few"), ("44", "few"), ("52", "few"), ("53", "few"),
                ("54", "few"), ("62", "few"), ("102", "few"),
                ("0", "many"), ("5", "many"), ("12", "many"), ("19", "many"), ("100", "many"), ("1000", "many"),
                ("1000000", "many"),
                ("0.0", "other"), ("0.7", "other"), ("1.5", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        #[test]
        fn fr() {
            assert_plural_eval(rules_of("fr"), &[
                ("0", "one"), ("1", "one"), ("0.0", "one"), ("0.7", "one"), ("1.5", "one"),
                ("1000000", "many"),
                ("2", "other"), ("9", "other"), ("17", "other"), ("100", "other"), ("1000", "other"), ("2.0", "other"),
                ("2.7", "other"), ("3.5", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        #[test]
        fn pt() {
            assert_plural_eval(rules_of("pt"), &[
                ("0", "one"), ("1", "one"), ("0.0", "one"), ("0.7", "one"), ("1.5", "one"),
                ("1000000", "many"),
                ("2", "other"), ("9", "other"), ("17", "other"), ("100", "other"), ("1000", "other"), ("2.0", "other"),
                ("2.7", "other"), ("3.5", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        #[test]
        fn pl() {
            assert_plural_eval(rules_of("pl"), &[
                ("1", "one"),
                ("2", "few"), ("3", "few"), ("4", "few"), ("22", "few"), ("23", "few"), ("24", "few"), ("32", "few"),
                ("33", "few"), ("34", "few"), ("42", "few"), ("43", "few"), ("44", "few"), ("52", "few"), ("53", "few"),
                ("54", "few"), ("62", "few"), ("102", "few"),
                ("0", "many"), ("5", "many"), ("12", "many"), ("19", "many"), ("100", "many"), ("1000", "many"),
                ("1000000", "many"),
                ("0.0", "other"), ("0.7", "other"), ("1.5", "other"), ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        #[test]
        fn da() {
            assert_plural_eval(rules_of("da"), &[
                ("1", "one"), ("0.1", "one"), ("0.8", "one"), ("1.6", "one"),
                ("0", "other"), ("2", "other"), ("9", "other"), ("16", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("0.0", "other"), ("2.0", "other"), ("2.7", "other"), ("3.4", "other"),
                ("10.0", "other"), ("100.0", "other"),
            ]);
        }

        #[test]
        fn cs() {
            assert_plural_eval(rules_of("cs"), &[
                ("1", "one"),
                ("2", "few"), ("3", "few"), ("4", "few"),
                ("0.0", "many"), ("0.7", "many"), ("1.5", "many"), ("10.0", "many"), ("100.0", "many"),
                ("0", "other"), ("5", "other"), ("12", "other"), ("19", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"),
            ]);
        }

        #[test]
        fn ar() {
            assert_plural_eval(rules_of("ar"), &[
                ("0", "zero"), ("0.0", "zero"), ("0.00", "zero"), ("0.000", "zero"), ("0.0000", "zero"),
                ("1", "one"), ("1.0", "one"), ("1.00", "one"), ("1.000", "one"), ("1.0000", "one"),
                ("2", "two"), ("2.0", "two"), ("2.00", "two"), ("2.000", "two"), ("2.0000", "two"),
                ("3", "few"), ("6", "few"), ("10", "few"), ("103", "few"), ("106", "few"), ("110", "few"),
                ("3.0", "few"), ("4.0", "few"), ("5.0", "few"), ("6.0", "few"), ("7.0", "few"), ("8.0", "few"),
                ("9.0", "few"), ("10.0", "few"),
                ("11", "many"), ("18", "many"), ("26", "many"), ("111", "many"), ("11.0", "many"), ("12.0", "many"),
                ("13.0", "many"), ("14.0", "many"), ("15.0", "many"), ("16.0", "many"), ("17.0", "many"),
                ("18.0", "many"),
                ("100", "other"), ("101", "other"), ("102", "other"), ("200", "other"), ("201", "other"),
                ("202", "other"), ("300", "other"), ("301", "other"), ("302", "other"), ("400", "other"),
                ("401", "other"), ("402", "other"), ("500", "other"), ("501", "other"), ("502", "other"),
                ("600", "other"), ("1000", "other"), ("1000000", "other"), ("0.1", "other"), ("0.5", "other"),
                ("0.9", "other"), ("1.1", "other"), ("1.4", "other"), ("1.7", "other"), ("10.1", "other"),
                ("100.0", "other"),
            ]);
        }

        #[test]
        fn hi() {
            assert_plural_eval(rules_of("hi"), &[
                ("0", "one"), ("1", "one"), ("0.0", "one"), ("0.5", "one"), ("1.0", "one"), ("0.00", "one"),
                ("0.02", "one"), ("0.04", "one"),
                ("2", "other"), ("9", "other"), ("17", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("1.1", "other"), ("1.8", "other"), ("2.6", "other"), ("10.0", "other"),
                ("100.0", "other"),
            ]);
        }

        #[test]
        fn he() {
            assert_plural_eval(rules_of("he"), &[
                ("1", "one"), ("0.0", "one"), ("0.4", "one"), ("0.9", "one"), ("0.00", "one"), ("0.02", "one"),
                ("0.05", "one"),
                ("2", "two"),
                ("0", "other"), ("3", "other"), ("10", "other"), ("17", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"), ("1.0", "other"), ("1.7", "other"), ("2.5", "other"), ("10.0", "other"),
                ("100.0", "other"),
            ]);
        }

        #[test]
        fn ro() {
            assert_plural_eval(rules_of("ro"), &[
                ("1", "one"),
                ("0", "few"), ("2", "few"), ("9", "few"), ("16", "few"), ("101", "few"), ("0.0", "few"), ("0.7", "few"),
                ("1.5", "few"), ("10.0", "few"), ("100.0", "few"),
                ("20", "other"), ("27", "other"), ("35", "other"), ("100", "other"), ("1000", "other"),
                ("1000000", "other"),
            ]);
        }
    }

    /// Every vendored locale's rule set is one of those with samples above, so a new rule set cannot arrive
    /// without them.
    #[test]
    fn every_vendored_rule_set_has_samples() {
        const SAMPLED: [&str; 15] = [
            "en", "ja", "es", "no", "it", "ru", "fr", "pt", "pl", "da", "cs", "ar", "hi", "he",
            "ro",
        ];
        let as_set =
            |rules: PluralRules| -> BTreeSet<(&str, &str)> { rules.iter().copied().collect() };
        let sampled: Vec<BTreeSet<(&str, &str)>> =
            SAMPLED.iter().map(|t| as_set(rules_of(t))).collect();
        for tag in vendored_tags() {
            let own = as_set(rules_of(tag));
            assert!(
                sampled.contains(&own),
                "{tag}: its plural rules are none of the sampled sets {SAMPLED:?}"
            );
        }
    }
}
