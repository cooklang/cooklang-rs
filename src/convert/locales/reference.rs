//! One locale as a human-readable Markdown page: what `show-locale-as-md` prints, which is what a review refusal
//! points at. The pages of a few tags are pinned in `tests/snapshots/<tag>.md`.

use std::fmt::Write;

use super::locale::Locale;
use super::preference::{best_amount, official_system, rows_for, Row, Units, REGION_PREFERENCES};
use super::tag::LocaleTag;
use crate::convert::units::{PhysicalQuantity, Unit};

/// Preference rows as `` `liter` ≥ 1 → `deciliter` ≥ 0.5 ``, largest first.
pub(crate) fn order(rows: &[Row], with_thresholds: bool) -> String {
    rows.iter()
        .map(|row| {
            let mut s = format!("`{}`", row.unit.id());
            if with_thresholds {
                write!(s, " ≥ {}", row.at_least).unwrap();
            }
            if let Some(den) = row.fractions {
                write!(s, " (/{den})").unwrap();
            }
            if row.or_decimal {
                s.push_str(" or decimal");
            }
            s
        })
        .collect::<Vec<_>>()
        .join(" → ")
}

/// The numbers the plural evaluation table shows: enough to exercise
/// `one`, `two`, `few`, `many` and fractional `other` in most languages.
const PLURAL_SAMPLES: [f64; 9] = [0.0, 1.0, 2.0, 5.0, 11.0, 21.0, 22.0, 100.0, 1.5];

/// One locale as Markdown: every unit with its spaced identifier, symbol,
/// aliases and long forms, the plural rules, and what the long form writes
/// for a few numbers. `tag` is any BCP 47 tag; names come from the vendored
/// locale its fallback chain reaches.
pub(crate) fn render_locale(tag: &str) -> Result<String, String> {
    // `legacy` is a naming, not a tag (ADR 7); the example takes the bare word for it
    let (locale, parsed, reached) = if tag == "legacy" {
        (super::legacy::legacy_locale(), None, "legacy")
    } else {
        let parsed = LocaleTag::parse(tag).map_err(|e| e.to_string())?;
        let reached = super::vendored::tag_for(&parsed)
            .ok_or_else(|| format!("no vendored locale covers '{tag}'"))?;
        let locale = super::vendored::vendored_locale(reached).expect("tag_for found it");
        (locale, Some(parsed), reached)
    };

    let mut out = String::new();
    write_header(&mut out, tag, reached, parsed.as_ref());
    write_plural_rules(&mut out, &locale);
    write_units(&mut out, &locale);
    write_plural_evaluation(&mut out, &locale);
    out.push_str("\n## Units chosen for the reader\n\n");
    match &parsed {
        None => {
            out.push_str("Legacy has no region and no system; it names units and nothing more.\n")
        }
        Some(parsed) => write_units_for_reader(&mut out, &locale, parsed),
    }
    Ok(out)
}

/// The title, and where the names come from. `parsed` is `None` for legacy.
fn write_header(out: &mut String, tag: &str, reached: &str, parsed: Option<&LocaleTag>) {
    writeln!(out, "# Locale `{reached}`\n").unwrap();
    match parsed {
        None => {
            out.push_str("The frozen compatibility locale: exactly the names `units.toml` gives\n");
            out.push_str("the bundled units, and nothing for any other unit. It is not a BCP 47\n");
            out.push_str("tag: a recipe with no `locale` reads through it (ADR 7).\n\n");
        }
        Some(parsed) => {
            if reached != parsed.language_identifier() {
                writeln!(out, "Requested `{tag}`; names come from `{reached}`.\n").unwrap();
            }
            writeln!(
                out,
                "Unit names were seeded from CLDR {}.\n",
                super::vendored::cldr_version(reached).unwrap_or("unknown")
            )
            .unwrap();
        }
    }
}

fn write_plural_rules(out: &mut String, locale: &Locale) {
    out.push_str("## Plural rules\n\n");
    if locale.plural_rules().is_empty() {
        out.push_str("This locale has no plural rules: every number is `other`, and it has\n");
        out.push_str("no long forms to apply them to.\n");
        return;
    }
    out.push_str("| Category | Condition |\n|---|---|\n");
    for (category, condition) in locale.plural_rules() {
        let condition = if condition.is_empty() {
            "*everything else*".to_string()
        } else {
            format!("`{condition}`")
        };
        writeln!(out, "| {category} | {condition} |").unwrap();
    }
}

fn write_units(out: &mut String, locale: &Locale) {
    out.push_str("\n## Units\n\n");
    out.push_str("The identifier resolves in every locale and is what a conversion writes\n");
    out.push_str("when the locale has no symbol. So do `℃`, `℉`, `ºC`, `ºF` and the\n");
    out.push_str("bare `C` and `F`, whatever the locale names. The bare letters are safe\n");
    out.push_str("only because lookup is case-sensitive: `c` is a cup in `en`, `en-001`,\n");
    out.push_str("`en-AU` and `en-GB`. Long forms are CLDR patterns by plural category,\n");
    out.push_str("with `{0}` where the number goes.\n\n");
    out.push_str("| Identifier | Symbol | Aliases | Long forms |\n");
    out.push_str("|---|---|---|---|\n");
    for unit in Unit::all() {
        let names = locale.names(unit);
        let symbol = match locale.symbol(unit) {
            Some(s) => format!("`{s}`"),
            None => "—".to_string(),
        };
        let aliases = names
            .map(|n| n.aliases.as_slice())
            .filter(|a| !a.is_empty())
            .map(|a| {
                a.iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "—".to_string());
        let long = names
            .map(|n| n.long.as_slice())
            .filter(|l| !l.is_empty())
            .map(|l| {
                l.iter()
                    .map(|(c, p)| format!("{c} `{p}`"))
                    .collect::<Vec<_>>()
                    .join("; ")
            })
            .unwrap_or_else(|| "—".to_string());
        writeln!(out, "| `{}` | {symbol} | {aliases} | {long} |", unit.id()).unwrap();
    }
}

/// What `written_long` writes for [`PLURAL_SAMPLES`], per unit with a long form.
fn write_plural_evaluation(out: &mut String, locale: &Locale) {
    out.push_str("\n## Plural evaluation\n\n");
    out.push_str("What `written_long` writes, for the units that have a long form.\n\n");
    out.push_str("| Identifier |");
    for value in PLURAL_SAMPLES {
        write!(out, " {value} |").unwrap();
    }
    out.push_str("\n|---|");
    for _ in PLURAL_SAMPLES {
        out.push_str("---|");
    }
    out.push('\n');
    for unit in Unit::all() {
        if locale.names(unit).is_none_or(|n| n.long.is_empty()) {
            continue;
        }
        write!(out, "| `{}` |", unit.id()).unwrap();
        for value in PLURAL_SAMPLES {
            write!(out, " {} |", locale.written_long(value, unit)).unwrap();
        }
        out.push('\n');
    }
}

/// The rows and best-unit choices for the tag's audience, or what the tag
/// lacks to have one.
fn write_units_for_reader(out: &mut String, locale: &Locale, parsed: &LocaleTag) {
    let audience = match parsed.audience() {
        Ok(audience) => audience,
        Err(error) => {
            writeln!(out, "{error}.\n").unwrap();
            out.push_str("Names come from the language alone; choosing units needs a region\n");
            writeln!(
                out,
                "(`{}-XX`) or a measurement system (`{}-u-ms-metric`, `ussystem`, `uksystem`).\n",
                parsed.language_identifier(),
                parsed.language_identifier()
            )
            .unwrap();
            return;
        }
    };
    let system = match audience.units {
        Units::Region(region) => {
            let system = official_system(region);
            writeln!(
                out,
                "Region `{region}`, whose official system is **{system:?}**.\n"
            )
            .unwrap();
            system
        }
        Units::System(system) => {
            writeln!(out, "`-u-ms-` asks for **{system:?}** outright.\n").unwrap();
            system
        }
        Units::Legacy(system) => {
            writeln!(
                out,
                "The legacy locale's frozen rows for **{system:?}**, the system the quantity was written in.\n"
            )
            .unwrap();
            system
        }
    };
    if let Some(unit) = audience.temperature {
        writeln!(out, "`-u-mu-` asks for **{unit:?}** for temperature.\n").unwrap();
    }

    out.push_str("### Preference rows\n\n");
    out.push_str("Largest first; a value is shown in the first unit it reaches the\n");
    out.push_str("threshold of, falling back to the last. A region's own rows override\n");
    out.push_str("its system's for that quantity; time is the same everywhere.\n\n");
    out.push_str("| Quantity | From | Order |\n|---|---|---|\n");
    for quantity in [
        PhysicalQuantity::Volume,
        PhysicalQuantity::Mass,
        PhysicalQuantity::Length,
        PhysicalQuantity::Temperature,
        PhysicalQuantity::Time,
    ] {
        let from = match audience.units {
            _ if quantity == PhysicalQuantity::Temperature && audience.temperature.is_some() => {
                "`-u-mu-`".to_string()
            }
            Units::Region(region)
                if REGION_PREFERENCES
                    .iter()
                    .any(|(r, q, _)| *r == region && *q == quantity) =>
            {
                format!("region `{region}`")
            }
            _ if quantity == PhysicalQuantity::Time => "shared".to_string(),
            _ => format!("{system:?}"),
        };
        let order = order(rows_for(audience, quantity), true);
        writeln!(out, "| {quantity:?} | {from} | {order} |").unwrap();
    }

    out.push_str("\n### Best unit for sample amounts\n\n");
    out.push_str("| Given | Chosen unit | Written |\n|---|---|---|\n");
    for (value, from) in BEST_UNIT_SAMPLES {
        let from = Unit::from_id(from).expect("a sample unit");
        let (amount, chosen) = best_amount(value, from, audience);
        writeln!(
            out,
            "| {value} {} | `{}` | {amount} {} |",
            locale.written(from),
            chosen.id(),
            locale.written(chosen)
        )
        .unwrap();
    }
}

/// Amounts that show where each system's rows switch units.
const BEST_UNIT_SAMPLES: [(f64, &str); 16] = [
    (5.0, "milliliter"),
    (20.0, "milliliter"),
    (100.0, "milliliter"),
    (500.0, "milliliter"),
    (1500.0, "milliliter"),
    (4.0, "us fluid ounce"),
    (1.0, "imperial pint"),
    (10.0, "gram"),
    (500.0, "gram"),
    (2000.0, "gram"),
    (5.0, "millimeter"),
    (30.0, "centimeter"),
    (90.0, "minute"),
    (48.0, "hour"),
    (180.0, "celsius"),
    (-18.0, "celsius"),
];
