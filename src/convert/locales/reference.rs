//! One locale as a human-readable Markdown page: what `show-locale-as-md` prints, which is what a reviewer
//! reads (`locales/README.md`).

use std::fmt::{self, Write};
use std::sync::Arc;

use super::legacy::legacy_locale;
use super::locale::Locale;
use super::preference::{best_amount, sourced_rows_for, Row, Units};
use super::tag::LocaleTag;
use super::vendored::{cldr_version, tag_for, vendored_locale};
use crate::convert::units::{PhysicalQuantity, Unit};

/// The numbers the plural evaluation table shows: enough to exercise
/// `one`, `two`, `few`, `many` and fractional `other` in most languages.
const PLURAL_SAMPLES: [f64; 9] = [0.0, 1.0, 2.0, 5.0, 11.0, 21.0, 22.0, 100.0, 1.5];

/// The quantities the preference-rows table lists, in its row order.
const QUANTITIES: [PhysicalQuantity; 5] = [
    PhysicalQuantity::Volume,
    PhysicalQuantity::Mass,
    PhysicalQuantity::Length,
    PhysicalQuantity::Temperature,
    PhysicalQuantity::Time,
];

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

/// What a page is about: the frozen locale, or a vendored one reached from a tag.
enum Page {
    Legacy(Arc<Locale>),
    Vendored {
        requested: LocaleTag,
        /// The vendored tag the fallback chain reached, which is where the names come from.
        reached: &'static str,
        locale: Arc<Locale>,
    },
}

impl Page {
    /// `tag` is any BCP 47 tag, or the bare word `legacy`: a naming, not a tag (ADR 7).
    fn resolve(tag: &str) -> Result<Self, String> {
        if tag == "legacy" {
            return Ok(Page::Legacy(legacy_locale()));
        }
        let requested = LocaleTag::parse(tag).map_err(|e| e.to_string())?;
        let reached =
            tag_for(&requested).ok_or_else(|| format!("no vendored locale covers '{tag}'"))?;
        let locale = vendored_locale(reached).expect("tag_for found it");
        Ok(Page::Vendored {
            requested,
            reached,
            locale,
        })
    }

    fn title(&self) -> &str {
        match self {
            Page::Legacy(_) => "legacy",
            Page::Vendored { reached, .. } => reached,
        }
    }

    fn locale(&self) -> &Locale {
        match self {
            Page::Legacy(locale) | Page::Vendored { locale, .. } => locale,
        }
    }
}

/// One locale as Markdown: every unit with its spaced identifier, symbol,
/// aliases and long forms, the plural rules, and what the long form writes
/// for a few numbers.
pub(crate) fn render_locale(tag: &str) -> Result<String, String> {
    let page = Page::resolve(tag)?;
    let mut out = String::new();
    render(&mut out, &page).expect("writing to a String cannot fail");
    Ok(out)
}

fn render(out: &mut impl Write, page: &Page) -> fmt::Result {
    write_header(out, page)?;
    write_plural_rules(out, page.locale())?;
    write_units(out, page.locale())?;
    write_plural_evaluation(out, page.locale())?;
    out.write_str("\n## Units chosen for the reader\n\n")?;
    match page {
        Page::Legacy(_) => {
            out.write_str("Legacy has no region and no system; it names units and nothing more.\n")
        }
        Page::Vendored { requested, .. } => write_units_for_reader(out, page.locale(), requested),
    }
}

const LEGACY_PREAMBLE: &str = "\
The frozen compatibility locale: exactly the names `units.toml` gives
the bundled units, and nothing for any other unit. It is not a BCP 47
tag: a recipe with no `locale` reads through it (ADR 7).

";

/// The title, and where the names come from.
fn write_header(out: &mut impl Write, page: &Page) -> fmt::Result {
    writeln!(out, "# Locale `{}`\n", page.title())?;
    match page {
        Page::Legacy(_) => out.write_str(LEGACY_PREAMBLE),
        Page::Vendored {
            requested, reached, ..
        } => {
            if *reached != requested.language_identifier() {
                writeln!(
                    out,
                    "Requested `{requested}`; names come from `{reached}`.\n"
                )?;
            }
            let cldr = cldr_version(reached).unwrap_or("unknown");
            writeln!(out, "Unit names were seeded from CLDR {cldr}.\n")
        }
    }
}

fn write_plural_rules(out: &mut impl Write, locale: &Locale) -> fmt::Result {
    out.write_str("## Plural rules\n\n")?;
    if locale.plural_rules().is_empty() {
        out.write_str("This locale has no plural rules: every number is `other`, and it has\n")?;
        return out.write_str("no long forms to apply them to.\n");
    }
    out.write_str("| Category | Condition |\n|---|---|\n")?;
    for (category, condition) in locale.plural_rules() {
        let condition = if condition.is_empty() {
            "*everything else*".to_string()
        } else {
            format!("`{condition}`")
        };
        writeln!(out, "| {category} | {condition} |")?;
    }
    Ok(())
}

const UNITS_PREAMBLE: &str = "
## Units

The identifier resolves in every locale and is what a conversion writes
when the locale has no symbol. Lookup is case-sensitive: `C` is Celsius
where `c` is a cup. Long forms are CLDR patterns by plural category,
with `{0}` where the number goes.

| Identifier | Symbol | Aliases | Long forms |
|---|---|---|---|
";

fn write_units(out: &mut impl Write, locale: &Locale) -> fmt::Result {
    out.write_str(UNITS_PREAMBLE)?;
    for unit in Unit::all() {
        let names = locale.names(unit);
        let symbol = cell(", ", locale.symbol(unit).map(code).into_iter());
        let aliases = cell(
            ", ",
            names.into_iter().flat_map(|n| &n.aliases).map(|s| code(s)),
        );
        let long = cell(
            "; ",
            names
                .into_iter()
                .flat_map(|n| &n.long)
                .map(|(c, p)| format!("{c} `{p}`")),
        );
        writeln!(out, "| `{}` | {symbol} | {aliases} | {long} |", unit.id())?;
    }
    Ok(())
}

/// What `written_long` writes for [`PLURAL_SAMPLES`], per unit with a long form.
fn write_plural_evaluation(out: &mut impl Write, locale: &Locale) -> fmt::Result {
    out.write_str("\n## Plural evaluation\n\n")?;
    out.write_str("What `written_long` writes, for the units that have a long form.\n\n")?;
    out.write_str("| Identifier |")?;
    for value in PLURAL_SAMPLES {
        write!(out, " {value} |")?;
    }
    out.write_str("\n|---|")?;
    for _ in PLURAL_SAMPLES {
        out.write_str("---|")?;
    }
    out.write_char('\n')?;
    for unit in Unit::all() {
        if locale.names(unit).is_none_or(|n| n.long.is_empty()) {
            continue;
        }
        write!(out, "| `{}` |", unit.id())?;
        for value in PLURAL_SAMPLES {
            write!(out, " {} |", locale.written_long(value, unit))?;
        }
        out.write_char('\n')?;
    }
    Ok(())
}

const PREFERENCE_ROWS_PREAMBLE: &str = "\
### Preference rows

Largest first; a value is shown in the first unit it reaches the
threshold of, falling back to the last. A region's own rows override
its system's for that quantity; time is the same everywhere.

| Quantity | From | Order |
|---|---|---|
";

/// The rows and best-unit choices for the tag's audience, or what the tag
/// lacks to have one.
fn write_units_for_reader(
    out: &mut impl Write,
    locale: &Locale,
    requested: &LocaleTag,
) -> fmt::Result {
    let audience = match requested.audience() {
        Ok(audience) => audience,
        Err(error) => {
            writeln!(out, "{error}.\n")?;
            out.write_str("Names come from the language alone; choosing units needs a region\n")?;
            return writeln!(
                out,
                "(`{}-XX`) or a measurement system (`{}-u-ms-metric`, `ussystem`, `uksystem`).\n",
                requested.language_identifier(),
                requested.language_identifier()
            );
        }
    };
    let system = audience.official();
    match audience.units {
        Units::Region(region) => writeln!(
            out,
            "Region `{region}`, whose official system is **{system:?}**.\n"
        )?,
        Units::System(_) => writeln!(out, "`-u-ms-` asks for **{system:?}** outright.\n")?,
        // `LocaleTag::audience` builds only the two above
        Units::Legacy(_) => unreachable!("a parsed tag has no legacy audience"),
    }
    if let Some(unit) = audience.temperature {
        writeln!(out, "`-u-mu-` asks for **{unit:?}** for temperature.\n")?;
    }

    out.write_str(PREFERENCE_ROWS_PREAMBLE)?;
    for quantity in QUANTITIES {
        let (source, rows) = sourced_rows_for(audience, quantity);
        writeln!(out, "| {quantity:?} | {source} | {} |", order(rows))?;
    }

    out.write_str("\n### Best unit for sample amounts\n\n")?;
    out.write_str("| Given | Chosen unit | Written |\n|---|---|---|\n")?;
    for (value, from) in BEST_UNIT_SAMPLES {
        let from = Unit::from_id(from).expect("a sample unit");
        let (amount, chosen) = best_amount(value, from, audience);
        writeln!(
            out,
            "| {value} {} | `{}` | {amount} {} |",
            locale.written(from),
            chosen.id(),
            locale.written(chosen)
        )?;
    }
    Ok(())
}

/// A table cell listing `items`, or a dash when there are none.
fn cell(separator: &str, items: impl Iterator<Item = String>) -> String {
    let items: Vec<String> = items.collect();
    if items.is_empty() {
        "—".to_string()
    } else {
        items.join(separator)
    }
}

fn code(s: &str) -> String {
    format!("`{s}`")
}

/// Preference rows as `` `liter` ≥ 1 → `deciliter` ≥ 0.5 ``, largest first.
fn order(rows: &[Row]) -> String {
    let row = |row: &Row| {
        let mut s = format!("`{}` ≥ {}", row.unit.id(), row.at_least);
        if let Some(den) = row.fractions {
            write!(s, " (/{den})").expect("writing to a String cannot fail");
        }
        if row.or_decimal {
            s.push_str(" or decimal");
        }
        s
    };
    rows.iter().map(row).collect::<Vec<_>>().join(" → ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_renders(tag: &str, saying: &str) {
        let page = render_locale(tag).unwrap_or_else(|e| panic!("{tag}: {e}"));
        assert!(page.contains(saying), "{tag}: no {saying:?} in\n{page}");
    }

    /// One tag per branch of the page: the frozen locale, a language with no region, names reached through the parent
    /// chain, a region with rows of its own, a US region, a system asked for outright, a temperature unit asked for
    /// outright. `en-NZ` reaches `en-001`, which is unreviewed: the page a refusal points at must render.
    #[test]
    fn every_kind_of_page_renders() {
        assert_renders("legacy", "The frozen compatibility locale");
        assert_renders("pl", "locale 'pl' has no region");
        assert_renders("en-NZ", "names come from `en-001`");
        assert_renders("sv-SE", "Region `SE`, whose official system is **Metric**");
        assert_renders(
            "es-US",
            "Region `US`, whose official system is **UsCustomary**",
        );
        assert_renders(
            "en-GB-u-ms-uksystem",
            "`-u-ms-` asks for **Imperial** outright",
        );
        assert_renders(
            "en-CA-u-mu-fahrenhe",
            "`-u-mu-` asks for **Fahrenheit** for temperature",
        );
    }

    #[test]
    fn bad_input_is_an_error_not_a_panic() {
        assert!(render_locale("not a tag")
            .unwrap_err()
            .contains("not a valid locale tag"));
        assert!(render_locale("sw-KE")
            .unwrap_err()
            .contains("no vendored locale"));
    }
}
