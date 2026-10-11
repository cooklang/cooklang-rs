//! Starts a locale from CLDR's own JSON for it, as `locales/<tag>.toml`.
//!
//! Only for a language we do not have. There is deliberately no command to
//! re-render a locale we *do* have: those files carry hand-written comments
//! and ordering, and a canonical re-render would flatten them. Once a file
//! exists it is edited by hand, which is the point of ADR 0006.
//!
//! This is the only code that knows CLDR's vocabulary. The library reads
//! `locales/*.toml` and never CLDR itself.

use std::collections::BTreeMap;
use std::fmt::Write;

/// CLDR's key for each of our units whose names are taken from it.
///
/// CLDR leaves the US variant unmarked -- its bare `cup` is the US cup -- so
/// this is not symmetric with our identifiers. A unit missing from here must
/// be declined in the tests' `NOT_FROM_CLDR`, so adding a unit to the library
/// fails them until someone decides whether CLDR has it.
#[rustfmt::skip]
const CLDR_KEYS: &[(&str, &str)] = &[
    ("liter",                "volume-liter"),
    ("deciliter",            "volume-deciliter"),
    ("centiliter",           "volume-centiliter"),
    ("milliliter",           "volume-milliliter"),
    ("us teaspoon",          "volume-teaspoon"),
    ("us tablespoon",        "volume-tablespoon"),
    ("us fluid ounce",       "volume-fluid-ounce"),
    ("us cup",               "volume-cup"),
    ("us pint",              "volume-pint"),
    ("us quart",             "volume-quart"),
    ("us gallon",            "volume-gallon"),
    ("imperial fluid ounce", "volume-fluid-ounce-imperial"),
    ("imperial pint",        "volume-pint-imperial"),
    ("imperial quart",       "volume-quart-imperial"),
    ("imperial gallon",      "volume-gallon-imperial"),
    // CLDR's dessert spoon is the US one (two US teaspoons); ours is the
    // 10 ml convention, 1.4% away
    ("metric dessert spoon", "volume-dessert-spoon"),
    ("metric cup",           "volume-cup-metric"),
    ("kilogram",             "mass-kilogram"),
    ("gram",                 "mass-gram"),
    ("ounce",                "mass-ounce"),
    ("pound",                "mass-pound"),
    ("meter",                "length-meter"),
    ("centimeter",           "length-centimeter"),
    ("millimeter",           "length-millimeter"),
    ("foot",                 "length-foot"),
    ("inch",                 "length-inch"),
    ("second",               "duration-second"),
    ("minute",               "duration-minute"),
    ("hour",                 "duration-hour"),
    ("day",                  "duration-day"),
    ("celsius",              "temperature-celsius"),
    ("fahrenheit",           "temperature-fahrenheit"),
];

/// Units whose names are listed for a reviewer instead of accepted unseen.
///
/// The units whose *amount* varies by region -- a cup is 180 to 284 ml and a
/// pint 473 to 568 ml depending on where you are -- and the US and imperial
/// units most kitchens never use. CLDR will happily give every locale a word
/// for the imperial gallon, and a metric kitchen should not resolve it.
/// Everything else is safe to take unseen: a millilitre cannot mean 0.9 ml
/// anywhere.
///
/// "Safe" means it cannot name the wrong amount, not that recipes in the
/// language write it: feet and inches are seeded into every locale, and
/// whether a kitchen uses them is left to its reviewer (locales/README.md).
/// Adding a name after review is harmless; removing one may break recipes.
const NEEDS_A_VERDICT: &[&str] = &[
    "us cup",
    "metric cup",
    "us pint",
    "us quart",
    "us gallon",
    "us fluid ounce",
    "us teaspoon",
    "us tablespoon",
    "metric dessert spoon",
    "imperial pint",
    "imperial quart",
    "imperial gallon",
    "imperial fluid ounce",
    "pound",
    "ounce",
];

fn cldr_key(id: &str) -> Option<&'static str> {
    CLDR_KEYS
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, key)| *key)
}

/// A TOML basic string. Names are CLDR text and can hold anything.
fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Starts a new locale from CLDR's own JSON for it.
///
/// For a language nobody has looked at: it takes CLDR's unit names and sorts
/// them into the two sections.
///
/// Names for units whose amount is the same everywhere land in `[[unit]]` --
/// a millilitre cannot mean 0.9 ml in any language. Names for the units whose
/// amount varies by region, and for the US and imperial units most kitchens
/// never use, are listed in a "no verdict yet" comment instead
/// ([`NEEDS_A_VERDICT`]), so a reviewer is asked only about the words
/// that could mean the wrong amount.
///
/// The locale is unreviewed either way, so `reviewed_for_tag` refuses to
/// serve it until someone adds a `reviewed` key.
pub(crate) fn render_new_locale(
    tag: &str,
    cldr_version: &str,
    units_json: &str,
    plural_rules: &[(String, String)],
) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(units_json).map_err(|e| format!("{tag} units.json: {e}"))?;
    // CLDR nests the data under `main.<tag>.units`; accept either that or an
    // already-unwrapped `{ long, short }`.
    let units = if json.get("long").is_some() {
        &json
    } else {
        &json["main"][tag]["units"]
    };

    let mut out = String::new();
    writeln!(out, "# Unit names for `{tag}`.").unwrap();
    out.push_str("#\n");
    writeln!(
        out,
        "# Seeded from CLDR {cldr_version}. NOT REVIEWED: no speaker has confirmed these"
    )
    .unwrap();
    writeln!(
        out,
        "# are what recipes in this language write, so cooklang refuses to serve `{tag}`"
    )
    .unwrap();
    out.push_str("# until a `reviewed` key is added. Run\n");
    writeln!(out, "#     cargo run --example show-locale-as-md -- {tag}").unwrap();
    out.push_str("# to see what it would write.\n");
    out.push_str("#\n");
    out.push_str("# A name with no `reason` came from CLDR and is covered by the Unicode\n");
    out.push_str("# notice in locales/UNICODE-LICENSE. Anything with a `reason` is\n");
    out.push_str("# hand-authored and MIT.\n\n");
    writeln!(out, "cldr_version = {}\n", quote(cldr_version)).unwrap();

    out.push_str("plural_rules = [\n");
    for (category, condition) in plural_rules {
        writeln!(
            out,
            "    {{ category = {}, condition = {} }},",
            quote(category),
            quote(condition)
        )
        .unwrap();
    }
    out.push_str("]\n");

    let spaced: Vec<&str> = cooklang::unit_ids().collect();
    let mut refused: Vec<(&str, Vec<String>)> = Vec::new();

    for &id in &spaced {
        let Some(key) = cldr_key(id) else { continue };
        let long = &units["long"][key];

        let mut patterns: BTreeMap<String, String> = BTreeMap::new();
        for (field, value) in long.as_object().into_iter().flat_map(|o| o.iter()) {
            if let (Some(category), Some(pattern)) =
                (field.strip_prefix("unitPattern-count-"), value.as_str())
            {
                patterns.insert(category.to_string(), pattern.to_string());
            }
        }
        let mut names: Vec<String> = patterns.values().filter_map(|p| alias_of(p)).collect();
        if let Some(display) = long["displayName"].as_str() {
            names.push(display.to_string());
        }
        let symbol = units["short"][key]["unitPattern-count-other"]
            .as_str()
            .and_then(alias_of);
        names.extend(symbol.clone());

        // a unit's spaced identifier resolves everywhere (ADR 0004), so it is
        // neither a name to add nor one that could be refused
        names.retain(|n| !spaced.iter().any(|i| i == n));
        names.sort();
        names.dedup();
        if names.is_empty() {
            continue;
        }

        if NEEDS_A_VERDICT.contains(&id) {
            refused.push((id, names));
            continue;
        }

        out.push_str("\n[[unit]]\n");
        writeln!(out, "id = {}", quote(id)).unwrap();
        let symbol = symbol.filter(|s| names.contains(s));
        if let Some(symbol) = &symbol {
            writeln!(out, "symbol = {}", quote(symbol)).unwrap();
        }
        let aliases: Vec<String> = names
            .iter()
            .filter(|n| Some(*n) != symbol.as_ref())
            .map(|a| quote(a))
            .collect();
        if !aliases.is_empty() {
            writeln!(out, "aliases = [{}]", aliases.join(", ")).unwrap();
        }
        if !patterns.is_empty() {
            let list: Vec<String> = patterns
                .iter()
                .map(|(c, p)| format!("{c} = {}", quote(p)))
                .collect();
            writeln!(out, "long = {{ {} }}", list.join(", ")).unwrap();
        }
    }

    if !refused.is_empty() {
        out.push_str(
            "\n# No verdict yet. These units mean different amounts in different places --\n\
             # a cup is 180 to 284 ml, a pint 473 to 568 ml -- or are US and imperial units\n\
             # most kitchens never use, so none of their names is accepted unseen. For each:\n\
             # does the word mean *this* unit in this language? If so, add a [[unit]] entry\n\
             # above. If not, say why here. A name resolves only by being listed above, so\n\
             # leaving one here is what refuses it.\n#\n",
        );
        for (id, names) in refused {
            writeln!(out, "#   {id} -- {}", names.join(", ")).unwrap();
        }
    }

    Ok(out)
}

/// The name inside a CLDR pattern: `{0} millilitres` is `millilitres`.
///
/// `None` when the placeholder sits inside the string, as in the Japanese
/// circumfix `摂氏 {0} 度`, which has no extractable name.
fn alias_of(pattern: &str) -> Option<String> {
    let at = pattern.find("{0}")?;
    if at != 0 && at + 3 != pattern.len() {
        return None;
    }
    Some(pattern.replace("{0}", "").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Our units that take no names from CLDR.
    ///
    /// Most are units CLDR does not define. Four it does define are declined
    /// because no recipe uses them: taking them would spend `km`, `hl`, `dm` and
    /// `mg` in every locale on units nobody writes, and every resolvable string
    /// is one more the inline-quantities scanner matches in prose (ADR 0004).
    #[rustfmt::skip]
    const NOT_FROM_CLDR: &[&str] = &[
        // CLDR has no kiloliter or decaliter
        "kiloliter", "decaliter",
        // CLDR has no metric teaspoon or tablespoon; the words for them live
        // under its US keys and are retargeted per locale
        "metric teaspoon", "metric tablespoon",
        // CLDR has no spice measure, knife point, Australian tablespoon or 240 ml
        // cup; its cup-jp is 100 ml and must not seed the 200 ml one
        "nordic spice measure", "danish knife point", "australian tablespoon",
        "japanese cup", "us legal cup",
        // CLDR carries only kilo, (base), milli and micro for mass
        "hectogram", "decagram", "decigram", "centigram",
        // CLDR's `pound` is the avoirdupois one
        "colombian libra",
        // CLDR has no hectometer or decameter
        "hectometer", "decameter",
        // declined
        "hectoliter", "milligram", "kilometer", "decimeter",
    ];

    /// Stands in for an exhaustive `match`: a unit added to the library must
    /// be placed in one list or the other before this passes.
    #[test]
    fn every_unit_is_either_taken_from_cldr_or_declined() {
        let units: HashSet<&str> = cooklang::unit_ids().collect();
        let keyed: HashSet<&str> = CLDR_KEYS.iter().map(|(id, _)| *id).collect();
        let declined: HashSet<&str> = NOT_FROM_CLDR.iter().copied().collect();

        let mut both: Vec<_> = keyed.intersection(&declined).collect();
        both.sort_unstable();
        assert!(both.is_empty(), "both taken and declined: {both:?}");

        let listed: HashSet<&str> = keyed.union(&declined).copied().collect();
        let mut unplaced: Vec<_> = units.difference(&listed).collect();
        unplaced.sort_unstable();
        assert!(unplaced.is_empty(), "placed in neither list: {unplaced:?}");
        let mut stale: Vec<_> = listed.difference(&units).collect();
        stale.sort_unstable();
        assert!(stale.is_empty(), "not a unit: {stale:?}");
    }

    #[test]
    fn no_cldr_key_seeds_two_units() {
        let keys: HashSet<&str> = CLDR_KEYS.iter().map(|(_, key)| *key).collect();
        assert_eq!(keys.len(), CLDR_KEYS.len());
    }

    /// A misspelt entry here would not fail anything else: the unit's names
    /// would be accepted unseen instead of held for a verdict.
    #[test]
    fn every_unit_needing_a_verdict_is_taken_from_cldr() {
        for id in NEEDS_A_VERDICT {
            assert!(cldr_key(id).is_some(), "'{id}' has no CLDR key");
        }
    }
}
