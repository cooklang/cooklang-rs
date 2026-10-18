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

    /// Every number falls in the category it is listed under.
    #[track_caller]
    fn assert_plural_eval(rules: &[(&str, &str)], expected: &[(&str, &[&str])]) {
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
                ("one",   &["1"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
            ]);
        }

        /// Also zh, zh-Hant, ko, th, vi, id, ms.
        #[test]
        fn ja() {
            assert_plural_eval(rules_of("ja"), &[
                ("other", &["0", "7", "15", "100", "1000", "1000000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
            ]);
        }

        /// Also es-419, es-US, es-MX, es-CO.
        #[test]
        fn es() {
            assert_plural_eval(rules_of("es"), &[
                ("one",   &["1", "1.0", "1.00", "1.000", "1.0000"]),
                ("many",  &["1000000"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "0.0", "0.4", "0.9", "1.1", "1.3", "1.6", "10.0",
                            "100.0"]),
            ]);
        }

        /// Also hu, tr, el.
        #[test]
        fn no() {
            assert_plural_eval(rules_of("no"), &[
                ("one",   &["1", "1.0", "1.00", "1.000", "1.0000"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "0.4", "0.9", "1.1", "1.3", "1.6",
                            "10.0", "100.0"]),
            ]);
        }

        /// Also pt-PT.
        #[test]
        fn it() {
            assert_plural_eval(rules_of("it"), &[
                ("one",   &["1"]),
                ("many",  &["1000000"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
            ]);
        }

        /// Also uk.
        #[test]
        fn ru() {
            assert_plural_eval(rules_of("ru"), &[
                ("one",   &["1", "21", "31", "41", "51", "61", "71", "81", "101"]),
                ("few",   &["2", "3", "4", "22", "23", "24", "32", "33", "34", "42", "43", "44", "52", "53", "54",
                            "62", "102"]),
                ("many",  &["0", "5", "12", "19", "100", "1000", "1000000"]),
                ("other", &["0.0", "0.7", "1.5", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn fr() {
            assert_plural_eval(rules_of("fr"), &[
                ("one",   &["0", "1", "0.0", "0.7", "1.5"]),
                ("many",  &["1000000"]),
                ("other", &["2", "9", "17", "100", "1000", "2.0", "2.7", "3.5", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn pt() {
            assert_plural_eval(rules_of("pt"), &[
                ("one",   &["0", "1", "0.0", "0.7", "1.5"]),
                ("many",  &["1000000"]),
                ("other", &["2", "9", "17", "100", "1000", "2.0", "2.7", "3.5", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn pl() {
            assert_plural_eval(rules_of("pl"), &[
                ("one",   &["1"]),
                ("few",   &["2", "3", "4", "22", "23", "24", "32", "33", "34", "42", "43", "44", "52", "53", "54",
                            "62", "102"]),
                ("many",  &["0", "5", "12", "19", "100", "1000", "1000000"]),
                ("other", &["0.0", "0.7", "1.5", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn da() {
            assert_plural_eval(rules_of("da"), &[
                ("one",   &["1", "0.1", "0.8", "1.6"]),
                ("other", &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "2.0", "2.7", "3.4", "10.0",
                            "100.0"]),
            ]);
        }

        #[test]
        fn cs() {
            assert_plural_eval(rules_of("cs"), &[
                ("one",   &["1"]),
                ("few",   &["2", "3", "4"]),
                ("many",  &["0.0", "0.7", "1.5", "10.0", "100.0"]),
                ("other", &["0", "5", "12", "19", "100", "1000", "1000000"]),
            ]);
        }

        #[test]
        fn ar() {
            assert_plural_eval(rules_of("ar"), &[
                ("zero",  &["0", "0.0", "0.00", "0.000", "0.0000"]),
                ("one",   &["1", "1.0", "1.00", "1.000", "1.0000"]),
                ("two",   &["2", "2.0", "2.00", "2.000", "2.0000"]),
                ("few",   &["3", "6", "10", "103", "106", "110", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0",
                            "10.0"]),
                ("many",  &["11", "18", "26", "111", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0"]),
                ("other", &["100", "101", "102", "200", "201", "202", "300", "301", "302", "400", "401", "402", "500",
                            "501", "502", "600", "1000", "1000000", "0.1", "0.5", "0.9", "1.1", "1.4", "1.7", "10.1",
                            "100.0"]),
            ]);
        }

        #[test]
        fn hi() {
            assert_plural_eval(rules_of("hi"), &[
                ("one",   &["0", "1", "0.0", "0.5", "1.0", "0.00", "0.02", "0.04"]),
                ("other", &["2", "9", "17", "100", "1000", "1000000", "1.1", "1.8", "2.6", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn he() {
            assert_plural_eval(rules_of("he"), &[
                ("one",   &["1", "0.0", "0.4", "0.9", "0.00", "0.02", "0.05"]),
                ("two",   &["2"]),
                ("other", &["0", "3", "10", "17", "100", "1000", "1000000", "1.0", "1.7", "2.5", "10.0", "100.0"]),
            ]);
        }

        #[test]
        fn ro() {
            assert_plural_eval(rules_of("ro"), &[
                ("one",   &["1"]),
                ("few",   &["0", "2", "9", "16", "101", "0.0", "0.7", "1.5", "10.0", "100.0"]),
                ("other", &["20", "27", "35", "100", "1000", "1000000"]),
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
