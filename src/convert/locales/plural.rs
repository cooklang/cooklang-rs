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

    /// Category, condition as the locale's TOML carries it, samples.
    type Rule = (&'static str, &'static str, &'static [&'static str]);

    /// One locale per distinct rule set among the vendored locales, with the samples CLDR 48.2.3 attaches to each
    /// rule in `cldr-core/supplemental/plurals.json`: every listed value up to 1000, both ends and a middle of every
    /// range, and 1000000, which the Romance `many` rules turn on. Compact samples (`1c6`) are never printed and are
    /// left out, as are decimals past 100.0, which repeat the same digits.
    #[rustfmt::skip]
    const CLDR_RULES: &[(&str, &[Rule])] = &[
    ("en", &[  // and en-001, en-GB, en-AU, de, nl, sv, fi
        ("one", "i = 1 and v = 0", &["1"]),
        ("other", "",             &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
    ]),
    ("ja", &[  // and zh, zh-Hant, ko, th, vi, id, ms
        ("other", "", &["0", "7", "15", "100", "1000", "1000000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
    ]),
    ("es", &[  // and es-419, es-US, es-MX, es-CO
        ("one", "n = 1",                                                       &["1", "1.0", "1.00", "1.000", "1.0000"]),
        ("many", "e = 0 and i != 0 and i % 1000000 = 0 and v = 0 or e != 0..5", &["1000000"]),
        ("other", "",                                                          &["0", "2", "9", "16", "100", "1000", "0.0", "0.4", "0.9", "1.1", "1.3", "1.6", "10.0", "100.0"]),
    ]),
    ("no", &[  // and hu, tr, el
        ("one", "n = 1", &["1", "1.0", "1.00", "1.000", "1.0000"]),
        ("other", "",   &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "0.4", "0.9", "1.1", "1.3", "1.6", "10.0", "100.0"]),
    ]),
    ("it", &[  // and pt-PT
        ("one", "i = 1 and v = 0",                                             &["1"]),
        ("many", "e = 0 and i != 0 and i % 1000000 = 0 and v = 0 or e != 0..5", &["1000000"]),
        ("other", "",                                                          &["0", "2", "9", "16", "100", "1000", "0.0", "0.7", "1.5", "10.0", "100.0"]),
    ]),
    ("ru", &[  // and uk
        ("one", "v = 0 and i % 10 = 1 and i % 100 != 11",                                        &["1", "21", "31", "41", "51", "61", "71", "81", "101"]),
        ("few", "v = 0 and i % 10 = 2..4 and i % 100 != 12..14",                                 &["2", "3", "4", "22", "23", "24", "32", "33", "34", "42", "43", "44", "52", "53", "54", "62", "102"]),
        ("many", "v = 0 and i % 10 = 0 or v = 0 and i % 10 = 5..9 or v = 0 and i % 100 = 11..14", &["0", "5", "12", "19", "100", "1000", "1000000"]),
        ("other", "",                                                                            &["0.0", "0.7", "1.5", "10.0", "100.0"]),
    ]),
    ("fr", &[
        ("one", "i = 0,1",                                                     &["0", "1", "0.0", "0.7", "1.5"]),
        ("many", "e = 0 and i != 0 and i % 1000000 = 0 and v = 0 or e != 0..5", &["1000000"]),
        ("other", "",                                                          &["2", "9", "17", "100", "1000", "2.0", "2.7", "3.5", "10.0", "100.0"]),
    ]),
    ("pt", &[
        ("one", "i = 0..1",                                                    &["0", "1", "0.0", "0.7", "1.5"]),
        ("many", "e = 0 and i != 0 and i % 1000000 = 0 and v = 0 or e != 0..5", &["1000000"]),
        ("other", "",                                                          &["2", "9", "17", "100", "1000", "2.0", "2.7", "3.5", "10.0", "100.0"]),
    ]),
    ("pl", &[
        ("one", "i = 1 and v = 0",                                                                             &["1"]),
        ("few", "v = 0 and i % 10 = 2..4 and i % 100 != 12..14",                                               &["2", "3", "4", "22", "23", "24", "32", "33", "34", "42", "43", "44", "52", "53", "54", "62", "102"]),
        ("many", "v = 0 and i != 1 and i % 10 = 0..1 or v = 0 and i % 10 = 5..9 or v = 0 and i % 100 = 12..14", &["0", "5", "12", "19", "100", "1000", "1000000"]),
        ("other", "",                                                                                          &["0.0", "0.7", "1.5", "10.0", "100.0"]),
    ]),
    ("da", &[
        ("one", "n = 1 or t != 0 and i = 0,1", &["1", "0.1", "0.8", "1.6"]),
        ("other", "",                         &["0", "2", "9", "16", "100", "1000", "1000000", "0.0", "2.0", "2.7", "3.4", "10.0", "100.0"]),
    ]),
    ("cs", &[
        ("one", "i = 1 and v = 0",   &["1"]),
        ("few", "i = 2..4 and v = 0", &["2", "3", "4"]),
        ("many", "v != 0",           &["0.0", "0.7", "1.5", "10.0", "100.0"]),
        ("other", "",                &["0", "5", "12", "19", "100", "1000", "1000000"]),
    ]),
    ("ar", &[
        ("zero", "n = 0",           &["0", "0.0", "0.00", "0.000", "0.0000"]),
        ("one", "n = 1",            &["1", "1.0", "1.00", "1.000", "1.0000"]),
        ("two", "n = 2",            &["2", "2.0", "2.00", "2.000", "2.0000"]),
        ("few", "n % 100 = 3..10",  &["3", "6", "10", "103", "106", "110", "3.0", "4.0", "5.0", "6.0", "7.0", "8.0", "9.0", "10.0"]),
        ("many", "n % 100 = 11..99", &["11", "18", "26", "111", "11.0", "12.0", "13.0", "14.0", "15.0", "16.0", "17.0", "18.0"]),
        ("other", "",               &["100", "101", "102", "200", "201", "202", "300", "301", "302", "400", "401", "402", "500", "501", "502", "600", "1000", "1000000", "0.1", "0.5", "0.9", "1.1", "1.4", "1.7", "10.1", "100.0"]),
    ]),
    ("hi", &[
        ("one", "i = 0 or n = 1", &["0", "1", "0.0", "0.5", "1.0", "0.00", "0.02", "0.04"]),
        ("other", "",            &["2", "9", "17", "100", "1000", "1000000", "1.1", "1.8", "2.6", "10.0", "100.0"]),
    ]),
    ("he", &[
        ("one", "i = 1 and v = 0 or i = 0 and v != 0", &["1", "0.0", "0.4", "0.9", "0.00", "0.02", "0.05"]),
        ("two", "i = 2 and v = 0",                    &["2"]),
        ("other", "",                                 &["0", "3", "10", "17", "100", "1000", "1000000", "1.0", "1.7", "2.5", "10.0", "100.0"]),
    ]),
    ("ro", &[
        ("one", "i = 1 and v = 0",                              &["1"]),
        ("few", "v != 0 or n = 0 or n != 1 and n % 100 = 1..19", &["0", "2", "9", "16", "101", "0.0", "0.7", "1.5", "10.0", "100.0"]),
        ("other", "",                                           &["20", "27", "35", "100", "1000", "1000000"]),
    ]),
    ];

    /// Every sample CLDR attaches to a rule lands in that rule's category.
    #[test]
    fn every_cldr_sample_lands_in_its_own_category() {
        let mut checked = 0;
        for (locale, rules) in CLDR_RULES {
            let conditions: Vec<(&str, &str)> = rules.iter().map(|(c, r, _)| (*c, *r)).collect();
            for (expected, _, samples) in *rules {
                for sample in *samples {
                    assert_eq!(
                        category(&conditions, sample),
                        *expected,
                        "{locale}: {sample}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 250, "only {checked} samples checked");
    }

    /// The table and the vendored locales agree: every locale's `plural_rules` are one entry's conditions, and every
    /// entry is some locale's, so the oracle can neither drift from the TOML nor carry a rule nobody ships.
    #[test]
    fn the_table_is_exactly_the_vendored_rule_sets() {
        let table: Vec<(&str, BTreeSet<(&str, &str)>)> = CLDR_RULES
            .iter()
            .map(|(tag, rules)| (*tag, rules.iter().map(|(c, r, _)| (*c, *r)).collect()))
            .collect();
        let mut used = vec![false; table.len()];
        for tag in vendored_tags() {
            let own: BTreeSet<(&str, &str)> = vendored_locale(tag)
                .unwrap()
                .plural_rules()
                .iter()
                .copied()
                .collect();
            let i = table
                .iter()
                .position(|(_, conditions)| *conditions == own)
                .unwrap_or_else(|| panic!("{tag}: its plural rules match no entry in CLDR_RULES"));
            used[i] = true;
        }
        for ((tag, _), used) in table.iter().zip(used) {
            assert!(used, "{tag} in CLDR_RULES matches no vendored locale");
        }
    }
}
