# Locale-aware units

The decisions behind the unit, locale and conversion work, in the order they were taken. Each keeps
the number it was recorded under, because the code cites them that way: a comment saying `ADR 0005`
means decision 5 below. A decision that was later replaced is kept rather than deleted, marked as
superseded and left in place, because the decision that replaced it is argued against it.


## Contents

1. [Unit conversion uses a closed set of units defined in Rust](#1-unit-conversion-uses-a-closed-set-of-units-defined-in-rust)
2. [Unit identifiers are hyphenated lowercase words with the variant first](#2-unit-identifiers-are-hyphenated-lowercase-words-with-the-variant-first) *(superseded by 7)*
3. [CLDR is a source of unit names only](#3-cldr-is-a-source-of-unit-names-only) *(superseded by 6)*
4. [Locales are partial, and the identifier's spaced form is universal input](#4-locales-are-partial-and-the-identifiers-spaced-form-is-universal-input)
5. [Unit preferences follow a region's official measurement system](#5-unit-preferences-follow-a-regions-official-measurement-system)
6. [Locales are seeded from CLDR once, not vendored](#6-locales-are-seeded-from-cldr-once-not-vendored) *(supersedes 3)*
7. [Unit identifiers are lowercase words separated by spaces](#7-unit-identifiers-are-lowercase-words-separated-by-spaces) *(supersedes 2)*
8. [Fractions are a property of the preference row](#8-fractions-are-a-property-of-the-preference-row)
9. [The old converter is replaced in one step](#9-the-old-converter-is-replaced-in-one-step)
10. [A quantity carries its resolved unit](#10-a-quantity-carries-its-resolved-unit)


## 1. Unit conversion uses a closed set of units defined in Rust

Unit conversion used to be configured from `units.toml`, a file that also carried names, aliases, translations, fraction display, best-unit selection and unit systems. We are replacing its conversion core with `convert::units`: one Rust enum per physical quantity, holding the exact legal definitions, with no configuration. Conversion is the part that must be correct and that users have no reason to change, and keeping it out of the config makes the config small enough to expose to users.

Adding a unit is now a code change. Users can still choose what a recipe's text means, because aliases map words to unit identifiers. A locale can map `cup` to a Japanese cup, but only once that unit exists in the enum. Units that a units file adds at runtime will not convert once the converter is switched over to `convert::units`.

### Consequences

- Ambiguous units are named for what they are (`UsCup`, `UsTeaspoon`), and the alias layer decides what `cup` or `tsp` means.
- `convert::units` has no notion of a unit system. Metric versus imperial is a display preference handled above conversion, and some units, such as a Japanese cup, are neither.
- Temperature conversion handles temperature points only.

### Follow-up work

> Written before the locale-seeding series. Items 1–3 reference ADRs 0002 and 0003, both since
> superseded — by [0007](#7-unit-identifiers-are-lowercase-words-separated-by-spaces) and
> [0006](#6-locales-are-seeded-from-cldr-once-not-vendored). The list itself is current.

`convert::units` replaces only the arithmetic (`convert_f64` in `src/convert_legacy/mod.rs`) and the mixed-quantities check. Before the converter can switch over to it, everything else `src/convert_legacy/` does has to be rebuilt on top of `convert::units::Unit`, mostly in a layer keyed by unit id:

1. **Naming and resolving text to a unit.** *Largely done:* `convert::locale` has `Locale` (exact, case-sensitive lookup, at most one symbol per unit, no string naming two units; ADR 0004), `legacy_locale()`, the 38 seeded locales from `locales/<tag>.toml` (ADR 0006) and name fallback along CLDR's parent chain (`for_tag`); `convert::tag` parses BCP 47 tags. `ConverterV2::convert` reads and writes unit text through the locale.

   Still to do: **a locale has to be reviewed before it can be served at all** — `reviewed_for_tag` refuses every locale today, because none has a `reviewed` key (ADR 0006), so this item is not finishable by code alone; loading a locale from a user file at runtime (the `extend` layering that `apply_extend_groups`/`Precedence` do today — the shipped locales are compiled in by `build.rs` instead); per-locale SI prefix words (`join_prefixes`; the Spanish file keeps only `mili`); and moving `find_unit`, `Quantity::unit_info` and the timer error's `Display` of a unit onto it.
2. **Unit systems.** In v2 a system is a *preference key*, not a property of a unit (ADR 0005): `System::{Metric, UsCustomary, Imperial}` selects display rows, and a region maps to its official system. Still to do: `Recipe::convert(to: System)`, which converts a whole recipe and collects the errors, and whatever replaces `default_system`.
3. **Best-unit selection.** *Done for v2:* `convert::preference::best_unit` picks by region or system with CLDR's threshold schema and our rows (ADR 0005). Still to do: `ConvertTo::Best`/`SameSystem`/`Base`, `is_best_unit`, `best_units`, `base_unit`, `Quantity::to_base_unit` and `Quantity::fit` (used by scaling), all of which still sit on `BestConversionsStore`.
4. **Fraction display.** The layered `Fractions` config (all, metric/imperial, quantity, unit), `FractionsConfig`, `should_fit_fraction`, `Quantity::try_fraction` and `Quantity::fit_fraction`. `Quantity::convert` also rewrites the quantity after converting: it sets the unit text to the new unit's symbol, which resets `scalable`, and then fits a fraction depending on the `ConvertTo` mode.
5. **Equality tolerance.** `float_tolerance`/`float_eq` use the fraction `accuracy` as the equality tolerance for quantities (`SemanticEq`), so display configuration also decides when two quantities count as equal.
6. **Value and quantity plumbing.** `ConvertValue` (number or range, converted end by end; text values fail), its conversions to and from `Value`, and `ConvertError`'s `NoUnit`, `TextValue`, `UnknownUnit` and `BestUnitNotFound`.
7. **Loading and building.** `UnitsFile`, `ConverterBuilder` stacking several files, `build.rs` generating the bundled units from `units.toml`, and `Converter::bundled()`/`empty()`/`Default`.

Outside `src/convert_legacy/`, callers only resolve a unit's text, read its `physical_quantity`, and hand it back to the `Converter`, through `Quantity::unit_info`, `Quantity::compatible_unit` and `Converter::find_unit`. Items 4 and 5 are tied to the old `Unit` in ways that make them harder to move. The timer validation error also prints a unit, which `convert::units::Unit` cannot do.

### Differences from the legacy converter

Places where `convert_v2` deliberately behaves differently from `src/convert_legacy/`, and how the comparison tests allow for them:

- **`scalable` is kept.** `ConverterV2::convert` keeps a quantity's `scalable` flag. The legacy `Quantity::convert` rebuilds the quantity with `Quantity::new`, which resets it to `false`. The legacy behaviour looks accidental, and copying it would bake it in. The comparison ignores the flag, and `convert_preserves_scalable` pins the new behaviour.
- **Exact definitions.** Conversion uses the legal definitions rather than `units.toml`'s rounded ratios (largest difference about 1.2e-7 relative, for the teaspoon), so comparisons use a tolerance.
- **A US fluid ounce row, and metric spoons.** `units.toml`'s imperial `best` list for volume is cup, tablespoon, teaspoon, so bundled writes 4 fl oz of milk as `8 tbsp`; its metric list is litre and millilitre, so it writes two tablespoons of oil as `30 ml`. The v2 US customary rows add the fluid ounce between the cup and the tablespoon, and the metric rows add the metric tablespoon and teaspoon between 50 ml and a quarter teaspoon (ADR 0005). The US cup row also starts at a quarter (2026-10-08): American recipes write ¼, ⅓ and ½ cup, which bundled reached only through its fractions config capping tablespoons at four, so between a quarter and a whole cup bundled wrote tablespoons. `the_rows_reproduce_the_bundled_best_lists_except_the_recorded_divergences` pins every other choice to bundled's and asserts that wherever v2 chooses one of those units and bundled disagrees, bundled chose the tablespoon or millilitres respectively.

## 2. Unit identifiers are hyphenated lowercase words with the variant first

> **Superseded by [ADR 0007](#7-unit-identifiers-are-lowercase-words-separated-by-spaces).** Two of its
> decisions stand and carry the safety: the variant is always marked, so there is no bare
> `cup`, and the variant comes first. Only the separator changed, from `-` to a space, so
> every `us-cup` below now reads `us cup`. Its *Why hyphen* section is the argument 0007
> answers, and its *Alternatives considered* table — uom, icu4x, UCUM, QUDT, Wikidata —
> is untouched and still the reason not to re-research them.


Unit identifiers are the stable key that aliases, locale data and any future user configuration bind
to, so they are expensive to change once released. They are lowercase words joined by `-`, with any
regional or system variant **first**: `us-cup`, `us-fluid-ounce`, `metric-cup`, `imperial-pint`,
`japanese-cup`. The variant is **always** marked — there is no bare `cup`.

### Why the variant is always marked

Silently resolving the wrong unit is worse than refusing to resolve at all. The maintainer had
already been bitten by assuming a metric teaspoon in a metric recipe when the bundled table was US,
and `docs/research/cooking-units.md` records that a "cup" ranges from 180 ml to 284 ml and a "pint"
from 473 ml to 568 ml depending on region. An unmarked identifier makes one of those the silent
default.

Both standards surveyed do the opposite: CLDR's bare `cup` and `pint` are the US ones, with
`cup-metric` and `pint-imperial` as the marked variants, and UCUM marks every variant
(`[cup_us]`, `[cup_m]`, `[foz_br]`). We follow UCUM's completeness, not CLDR's defaulting.

### Why hyphen

A single unquoted token: no TOML key quoting (`"fl oz"` already needs quotes in
`units/spanish.toml`), no whitespace-normalisation hazards — double, trailing and non-breaking
spaces are invisible in a diff — and usable as-is in CLI arguments, URLs and log fields. It also
composes, since CLDR uses `-` structurally (`-per-`, `square-`), leaving room for densities or
per-unit prices. `id.replace('-', " ")` recovers a readable form when one is wanted.

### Why the variant comes first, and what that costs

Variant-first reads in natural English order and puts the disambiguating part where a reader looks
first. The cost, accepted knowingly: it is the reverse of both CLDR (`cup-metric`) and UCUM
(`[cup_us]`), so the mapping to those standards is a reversal rather than a copy, and our
identifiers do **not** sort with all cups adjacent — `us-cup`, `metric-cup` and `imperial-cup` land
under u, m and i, so a listing or a "did you mean" suggestion will not group them.

### Consequences

- `serialize_all = "snake_case"` stays the strum default even though no identifier contains an
  underscore. A multi-word variant added *without* an explicit `#[strum(serialize = "…")]` then
  yields `japanese_cup`, which the `ids_are_hyphenated_words` test rejects. Switching the default to
  `kebab-case` would instead yield `japanese-cup` — plausible, wrongly ordered, and silent.
- The identifier is not purely internal: its spaced form (`imperial pint`) is recipe-facing — it
  resolves as input in every locale and is what a conversion writes when a locale has no symbol
  for the unit. See [ADR 0004](#4-locales-are-partial-and-the-identifiers-spaced-form-is-universal-input).
- CLDR identifiers never appear in the public API. They are reachable only through explicitly named
  accessors, so swapping or dropping CLDR is not a breaking change. Locale identifiers are the
  opposite call: those are standard BCP 47 tags and are meant to be exposed. We own the unit
  vocabulary; we do not own the locale vocabulary.

### Alternatives considered

Recorded so they are not re-researched. Details and measurements in
[`../research/cldr-unit-data.md`](../research/cldr-unit-data.md) and
[`../research/cooking-units.md`](../research/cooking-units.md).

| Option | Verdict | Reason |
|---|---|---|
| **uom** | rejected for the core | The unit is a *type*, fixed at compile time; cooklang resolves units from recipe text at runtime, so a uom unit cannot be a `HashMap` key or an `all()` item. It would impose its programming model on `Quantity`, scaling and serde. Worth stealing later: its `thermodynamic_temperature` / `temperature_interval` split, which is the right shape for temperature deltas. |
| **icu4x** units | rejected for now | `icu::experimental::units` is experimental; it needs a compiled data provider across the Rust lib, the WASM package and two UniFFI targets; and conversions would then move with CLDR releases. It is also the wrong direction of travel: our primary job is input resolution (`cucharadas` → a unit), while a formatter turns a unit plus a number into a string and cannot hand back an alias list. |
| **UCUM** | interop target only | The best cooking coverage of any standard, and the only source for `[cup_m]` 240 mL, `[tsp_m]` 5 mL and `[tbs_m]` 15 mL. But its licence forbids derivative works and adding, deleting or modifying content, so no subset may be embedded. Map to its codes; never copy its tables. |
| **QUDT** | codes and cross-walks only | CC-BY 4.0 and embeddable, carrying `ucumCode`, `udunitsCode` and `uneceCommonCode` per unit — the licence-safe bridge to UCUM. But its labels are English-only outside SI base units (metre 24 languages, litre 2, `CUP_US` 1), and its `CUP_US` multiplier of 236.58825 mL is *less* precise than our exact 236.5882365 mL, so it is not a factor oracle. It carries **no** CLDR identifier, so there is no own-id → QUDT → CLDR path. |
| **CLDR identifiers as ours** | rejected | Adopting them would mean inheriting the bare-`cup`-means-US default this ADR exists to prevent, so a translation layer would be needed anyway. CLDR is still adopted for locale *names*; see `docs/outstanding.md`. |
| **Wikidata** | input aliases only | CC0 with no attribution required, and the only broad source of native-script spellings for units CLDR lacks. But translations and conversion factors sit on disjoint items: `shō` (Q995076) has ja/zh/ko labels and no conversion property at all, while `market catty` (Q108934447) has an exact 0.5 kg and no `zh` label. Never a source of factors or of unit identity. |

## 3. CLDR is a source of unit names only

> **Superseded by [ADR 0006](#6-locales-are-seeded-from-cldr-once-not-vendored).** Its core
> decision stands — CLDR is a source of unit *names* only, never of conversion factors —
> and its licence analysis is carried forward there verbatim in substance. What it
> describes below and 0006 replaces is the *delivery*: the vendored JSON subset, the
> four overlay layers and the default-reject mechanism. Kept for the reasoning and the
> alternatives it rules out.


Unit conversion needs, for every supported language, the strings a recipe may use for a unit and the
one string a conversion writes back. Unicode CLDR carries those for most of our units under the
permissive Unicode licence, and it also carries conversion factors and per-region unit preferences.
We vendor CLDR for the **names only**: conversion factors stay the statutory definitions in Rust
(ADR 0001) and unit preferences are a table we own (ADR 0005). The property this buys is that a
CLDR bump can change only how a unit is *written*, never which unit is chosen or what a conversion
returns.

### How CLDR is used

- **Vendored, pinned, offline.** `locales/cldr/vendor.sh` fetches a faithful JSON subset of
  `cldr-units-full` at the version in `locales/cldr/VERSION`; building never needs the network. The subset
  is selected by `locales/cldr/units.json`, which maps each CLDR key (`volume-cup`) to our identifier
  (`us-cup`) and is checked against the unit enum by a test. CLDR identifiers appear nowhere else
  (ADR 0002).
- **Symbol from the short pattern, aliases from the long ones.** CLDR's `displayName` is not a
  symbol (in `en` it is `grams`, `cups`), so the symbol is the short `unitPattern-count-other` with
  the placeholder stripped, and the long one/other patterns and display name become aliases. A
  pattern whose placeholder sits inside the string (`摂氏 {0} 度`) yields nothing. Narrow forms are
  never used: CLDR itself warns they are not unique. A short form that is prose rather than a
  symbol (`sec`, `hr`, `days`) is corrected in `locales/cldr/symbols.toml`; a test fails on an override
  that is dead or redundant.
- **Adjudication.** CLDR answers "what is the avoirdupois pound called in German"; we need "what
  does a German cook mean by *Pfund*". For a unit whose amount varies by region — cups, spoons,
  pints, quarts, gallons, fluid ounces, pounds, ounces, and the metric-marked cup and pint — every
  string CLDR offers needs a verdict in `locales/cldr/adjudicated.toml`, with a citation into
  `docs/research/cooking-units.md`. Units that mean the same everywhere (the SI family) are accepted
  without an entry. **A string with no verdict is rejected**, so a freshly vendored locale resolves
  SI and unambiguous names only, and the gaps are rendered in `docs/units.md` for later
  adjudication rather than failing the build.
- **Retargeting.** CLDR's unmarked keys name the US units, but in most languages the strings under
  them mean the metric ones: `tsk`, `EL`, `cucharada` are 5 and 15 ml spoons, and `en-GB`'s plain
  `pint` is the 568 ml imperial pint. An adjudication entry may therefore redirect a key to a
  different unit of ours (`unit = "metric-tablespoon"`); `locales/cldr/units.json` is only the default. A
  retarget needs a citation like a reject does.
- **An unmarked US name is refused outside US customary locales.** CLDR marks only the imperial
  side of each pair — `brittiska quarts`, `Imp. Gallonen`, `英制加仑` — and leaves the US side as the
  bare word in every non-English locale. That is CLDR's data layout, not evidence that a Swedish or
  German kitchen means 3.79 l by `gallon`: in those languages the bare word is the generic unit and
  "British" the qualifier, and the two readings differ by 20% (4% for the fluid ounce; 5.7% for the
  cup against the metric cup). So in a locale whose official system is not US customary, the US
  cup, pint, quart, gallon, fluid ounce and spoons resolve only from a name that carries a `US`
  marker (`en-GB`'s `US qt`, Finnish `am. gallona`) or from the spaced identifier; `en` and
  `es-US` keep the bare words. The avoirdupois pound and ounce have no imperial twin and are
  unaffected. A test holds this for every fully adjudicated locale.
- **Imperial names are stripped from metric-only locales too.** CLDR gives every locale a name
  for the imperial pint, fluid ounce, quart and gallon, and a scan of all 33 non-English locales
  found them to be root English in every one for the pint (`pt Imp.`), whole English in Swedish,
  Danish, Dutch and Indonesian (`Imp. fluid ounces`), translated definitions in Finnish, Arabic and
  Greek (*British quarter-gallon*), or correct words for a unit no kitchen there has measured with.
  So the imperial keys need a verdict like the regionally variable ones: English and US Spanish
  accept theirs; a locale whose region has no US/UK-system past rejects every string and writes
  the units by identifier; locales with such a past — `hi`, `ms`, `he`, `ar`, `zh-Hant` — keep
  their native names when they are adjudicated. The unit set is unchanged; only what resolves is.
- **Declining beats inventing.** A rejected string cannot be the symbol either, or we would write
  output we refuse to parse. English authors a marked symbol (`US c`, `US pt`, following CLDR's own
  `en-GB` `US qt`/`US gal`); other languages decline to name the US cup rather than coin a form, and
  the unit is written by its identifier (ADR 0004).
- **Hand-authored names live outside `locales/cldr/`.** Names CLDR cannot supply — the 1 ml Nordic spice
  measure, the Austrian `dag`, the marked English symbols — go in `locales/<tag>.toml`, merged
  after CLDR at build time under the same collision checks and the same citation rule. `locales/cldr/`
  stays a faithful subset plus verdicts *about* that subset.
- **Name fallback follows CLDR's `parentLocales`**, so `en-NZ` reaches `en-001` (Commonwealth
  English) rather than truncating to US English, and `nb` reaches `no`. This chain is independent
  of the preference fallback in ADR 0005.

### Licence

CLDR data is under the Unicode License V3 (SPDX `Unicode-3.0`): MIT-like, no copyleft, and its one
condition is that the notice appear with copies or in associated documentation. `locales/cldr/LICENSE`
holds the notice next to the data it covers, the crate is declared `MIT AND Unicode-3.0`, and the
README states the scope: the files under `locales/cldr/*.json` and the code generated from them carry the
Unicode licence; everything else, including every hand-authored verdict and supplement, is MIT.

### Alternatives considered

| Option | Verdict | Reason |
|---|---|---|
| ICU4X unit formatting | rejected | It runs the other way: a formatter turns a unit and a number into a string and cannot hand back an alias list, while our primary job is input resolution (`cucharadas` → a unit). `icu::experimental::units` is also experimental and needs a data provider across the Rust library, the WASM package and two UniFFI targets. |
| CLDR conversion factors | rejected | Already decided in ADR 0001; and CLDR's `cup-jp` is 100 ml, which is wrong. |
| CLDR unit preferences | seed only | There is no cooking usage; the nearest list gives `GB` only the imperial gallon and fluid ounce. See ADR 0005. |
| Build fails on an unadjudicated alias | superseded | Right for six locales, a multi-day blocker for thirty. Default-reject keeps the safety property (never resolve the wrong unit) without making vendoring a human task. |
| Wikidata for aliases | not now | CC0 and broad in native-script spellings for units CLDR lacks, but its factors are unreliable and it conflates regional variants. Possible later source for supplements. |

## 4. Locales are partial, and the identifier's spaced form is universal input

A locale names only the units it has a safe name for. CLDR lacks some of our units, and adjudication
makes a locale decline others (German has no name for the 236.588 ml US cup). Previously every
locale was built as an overlay on the legacy locale so that each unit had a symbol; that overlay
made legacy's English `c` resolve to the US cup in German, Spanish and Japanese recipes — a string
nobody adjudicated. Now a locale's symbol is optional, and a unit the locale does not name is
written as its identifier with the hyphens replaced by spaces: `2 imperial pint`, `1 nordic spice
measure`. That spaced form resolves as input in **every** locale, so what we write we can always
read back, and the kebab-case identifier itself never appears in recipe text.

### Consequences

- `Locale::new` no longer requires every unit to be named. It still rejects a string that names two
  units, and additionally rejects an alias equal to another unit's spaced identifier.
- The legacy locale stays exactly the 38 bundled units under their `units.toml` names. It is frozen;
  it does not learn new units, and nothing is layered on it.
- Best-unit selection (ADR 0005) must not consult names: if it skipped unnamed units, a CLDR bump
  could change which unit is chosen. An unnamed choice is written by identifier instead.
- **No derived SI symbols.** CLDR lacks `kiloliter`, `decaliter`, `hectogram`, `decagram`,
  `decigram`, `centigram`, `hectometer` and `decameter`. They are not synthesised from prefix and
  base symbol; a language that uses one hand-authors it (`hg` in the Nordic locales, `dag`/`dkg`
  in German). Likewise `kilometer`, `hectoliter`, `decimeter` and `milligram`, which CLDR does
  have, are deliberately not vendored: no recipe uses them, and every resolvable string is also a
  string the inline-quantities extension will match in prose, so short symbols are not spent on
  units nobody writes. Legacy keeps all of them.
- In US English the spaced identifier is already the natural name (`metric teaspoon`), so `en`
  needs no hand-authored name for the metric units.

### Alternatives considered

| Option | Reason rejected |
|---|---|
| Legacy's symbol for a non-bundled unit is its identifier, by rule | Keeps the overlay and with it the leak above; makes legacy the carrier of a rule that is not about legacy. |
| Legacy grows hand-authored English names for new units | Legacy stops being "exactly `units.toml`", and those English names leak into every locale's fallback. |
| Converting to an unnamed unit is an error | Output would depend on names after the unit was chosen without them; a preference row could select a unit the locale then refuses to write. |
| Derive the eight SI symbols from prefix + base | Correct in principle (the prefix symbols are international) but it spends `kl`, `dag`, `cg` in thirty locales where nobody writes them. |

## 5. Unit preferences follow a region's official measurement system

Which unit a converted value is *written in* is a display preference, separate from how units are
*named*. Names belong to the language: `en-GB` names `pint` as the 568 ml imperial pint because
that is what a British recipe means. Preferences belong to the region, and a region gets the units
of its **official** measurement system: `GB` is metric, so a value converted for a British reader is
written in millilitres and grams even though the recipe's own `pint` parsed as imperial. Preference
rows are keyed by region *or* by system, a region without rows of its own uses its official
system's rows, and `-u-ms-metric`, `-u-ms-ussystem` and `-u-ms-uksystem` select a system directly.

### The table

The rows are data, in `locales/preferences.toml`: one `[[system]]` or `[[region]]` entry per
quantity, each with its rows, thresholds and citation, like the verdicts and supplements. `build.rs`
emits them as enum paths, so a unit that is not in `definitions.rs` fails to compile. Only the two
rules stay in code: a region's official system, and time falling back to the metric list.

- `official_system(region)`: `US` → US customary; every other region → metric. CLDR's own data
  also lists `LR` as US customary and `GB MM` as UK/imperial; we do not copy it, because Liberia,
  Myanmar and the UK are officially metric, and a reader there who wants otherwise can say so with
  `-u-ms-`. An unrecognised `-u-ms-` value is an error, not a guess.
- Rows, largest first, each with the amount a value must reach to be shown in it:
  - **Metric**: `liter → milliliter ≥ 50 → metric-tablespoon → metric-teaspoon ≥ ¼ → milliliter`;
    `kilogram → gram → milligram`; `meter → centimeter → millimeter`; `celsius`. The spoons are
    in the *system* row, not repeated per region, because every metric region's recipes print
    them for 5–45 ml — the exceptions are mainland China, South Africa's all-millilitre register
    and a few publishers (`research/best-units-by-region.md` §7.3). Decided 2026-10-04; it changes
    the output of every metric reader and diverges from the bundled converter, which wrote
    `30 ml`.
  - **US customary**: `us-cup → us-fluid-ounce → us-tablespoon → us-teaspoon`; `pound → ounce`;
    `foot → inch`; `fahrenheit`.
  - **Imperial**: `imperial-pint → imperial-fluid-ounce → metric-tablespoon → metric-teaspoon`;
    `pound → ounce`; `foot → inch`; `celsius`. The spoons are metric because a British teaspoon is
    5 ml; the chain stops at the pint for the same reason the US chain stops at the cup (`3 pints`,
    not `0.4 gal`); ovens are Celsius.
  - **Time** is one list for every system: `day → hour → minute → second`.
- Regional variation *within* metric is a region row, overriding the official system's row for
  that quantity only. Authored from `research/best-units-by-region.md` §7.1 (2026-10-04): volume in
  decilitres for `SE NO DK FI CH HU` and Flemish `BE`, centilitres for `FR`, the 250 ml cup for
  `NZ`, `CA` and `AU` (with the 20 ml Australian tablespoon), the 200 ml cup for `JP` and `KR`,
  the 240 ml cup for `BR` and `MX`, Hungarian mass in decagrams and Colombian mass in the 500 g
  libra. No source states a switching threshold, so each
  is inferred from how recipes there are written and is a judgement to revisit, not a fact. An
  explicit `-u-ms-` ignores region rows: `sv-SE-u-ms-metric` asks for the system, and gets
  millilitres. Austrian decagrams, this ADR's original example, turned out to be contested by
  every edited Austrian publisher and have no row.

### Consequences

- `GB` has no rows. The naming half of the UK problem is solved by adjudication (ADR 0003), the
  preference half by `GB` being metric.
- Decision 6 of the earlier plan — "preferences keyed `(quantity, region)`" — becomes "keyed by
  region, falling back to the region's official system". The property that an unlisted region is
  metric, correct rather than a guess, survives.
- The US rows diverge from the bundled converter's `best` list by the `us fluid ounce` row:
  bundled writes `4 fl oz` of milk as `8 tbsp`. ADR 0001's list of differences records it, and the
  parity test pins the new rows rather than bundled's.

### Alternatives considered

| Option | Reason rejected |
|---|---|
| `GB` rows are imperial, `-u-ms-uksystem` ≡ `GB` (CLDR's equivalence) | Writes imperial output to every British reader when the country is metric; the only region with imperial units would be one where they are no longer official. |
| `-u-ms-uksystem` is unsupported | Nobody could ask for imperial output; a tag CLDR considers valid would be rejected. |
| CLDR's `unitPreferenceData` | No cooking usage; its `GB` fluid list has only the imperial gallon and fluid ounce and could never select the 568 ml pint. |
| Rows keyed by system only | Loses the ability to express Nordic `dl` or Austrian `dag` later without inventing a system per country. |

### Amendment 2026-10-08: `-u-mu-` overrides the temperature unit

BCP 47's `mu` key (`celsius`, `fahrenhe`; CLDR also defines `kelvin`, which has no unit here and is
refused) names a temperature unit outright and overrides whatever `-u-ms-` or the region would
choose, for temperature only. It is the standard answer to the one quantity a reader's system and
habit commonly disagree on: `en-CA-u-mu-fahrenhe` keeps Canada's metric cups and gets Fahrenheit
ovens. CLDR models the same split as `measurementSystem category="temperature"`; its data does not
list Canada there, so the override is the reader's to ask for, not inferred from the tag.

## 6. Locales are seeded from CLDR once, not vendored

Supersedes [ADR 0003](#3-cldr-is-a-source-of-unit-names-only), whose core decision this keeps: CLDR is a
source of unit **names** only, never of conversion factors. What changes is the delivery.

A locale is now one hand-editable file, `locales/<tag>.toml`, holding its plural rules, every name
it resolves, and every name it refuses with the reason. It is seeded once from CLDR by
`xtask/new-locale.sh` and hand-edited from then on. CLDR's JSON is not vendored, not committed, and
not read by the build.

### Why

ADR 0003 vendored a faithful subset of CLDR's JSON and layered our decisions on top of it in four
places: `adjudicated.toml` for verdicts, `symbols.toml` for unusable short forms, `locales/<tag>.toml`
for names CLDR lacks, and `units.json` as a key bridge, merged by 331 lines of `build.rs` in a
precedence order you had to know to read any of it.

That is cldr-json's architecture, and it is right for cldr-json: a living, multi-contributor
database covering every locale and every unit, where contributors learn the layout because the
layout is the project. Cooklang needs a small frozen subset of it. Importing the structure imported
the shape of someone else's problem — answering "what does `sv` name?" meant reading six files and
simulating a merge.

The usual defence of the layering is that it keeps upstream pristine so a CLDR bump is reviewable.
It does not deliver that. The vendored files were single-line minified JSON, so a bump produced a
38-file diff in which every changed file was one unreadable line; the 862-row unadjudicated list
existed as a proxy for a review the format made impossible.

ADR 0003's own reason for default-reject was to avoid "making vendoring a human task" across thirty
locales. Since then the review gate refuses any locale no speaker has confirmed, so a human task per
locale is mandatory before anyone can be served. That benefit is no longer received, and cooking
units change slowly enough that a once-per-locale conversion is cheap.

### What this costs

- **Adding a locale is a human task.** `xtask/new-locale.sh` makes starting one cheap, but nothing
  arrives automatically.
- **A CLDR fix does not propagate.** If Unicode corrects a Polish plural we do not get it; someone
  re-seeds or edits by hand. Deliberate: a name change is something to review, not absorb.
- **More lines, all of them readable.** 38 files of about 230 lines replace 1610 lines of overlay
  *plus* 736 KB of opaque JSON. The reduction is in code, vocabulary and the number of places to
  look, not in bytes.
- **`reason` is not required on a name.** Serde no longer enforces that a hand-authored name is
  cited; the review gate is what asks about an uncited one. A weaker guarantee, stated plainly.

### Consequences

- **Refusal is a comment, because that is all it ever was.** Names a locale must not resolve are
  listed in prose at the foot of its file, with the reason. They were briefly `[[refused]]` TOML
  entries, which was a mistake: deleting all 453 of them changed not one of the 40 snapshots,
  because absence of a name is what stops it resolving. Dressing prose as data invites a reader to
  think the build enforces it. What the comment is genuinely for is the audit trail
  `adjudicated.toml` used to hold, and the list a reviewer works through — without it, dropping the
  CLDR JSON would have destroyed 857 candidate names.
- **Refusing a string for one unit is how it is given to another.** Finnish refuses CLDR's `mm` for
  the millimetre because in a Finnish recipe `mm` is the 1 ml *maustemitta*. The two stanzas sit
  next to each other in `locales/fi.toml`; four overlay layers never showed this.
- **A spaced identifier cannot be refused.** It resolves in every locale (ADR 0004), so a refusal
  naming one would be false and is dropped. Three such strings existed, all untranslated English in
  CLDR coinciding with our identifiers: `nl`'s `ounce` and `pound`, and `id`'s `metric cup`. The old
  unadjudicated list asserted they were unresolvable when they never were.
- **Provenance is per locale.** `cldr_version` sits in each file, because locales are seeded when
  someone comes to review them and will not share a release.
- **The review marker lives with the locale.** `reviewed = { date, by, sources }` is a key in the
  locale's own file; there is no separate list. The question asked is "is `sv` ready?", which the
  file answers.
- **`Unit::needs_a_verdict` replaces the `needs_verdict` list.** Whether a unit's amount varies by
  region is a property of the unit, not of a config file.
- **The build reads only TOML.** `serde_json` left `[build-dependencies]`; it is now an optional
  dependency used solely by `cargo xtask new-locale`.
- **Two files stay global, for stated reasons.** `parent-locales.toml`, because 193 of its 199
  children are tags we have no file for, and it is consulted for whatever tag a recipe asks for.
  And `plural-samples.json`, which is a *test fixture*, not build input: it carries CLDR's own
  `@integer`/`@decimal` samples, the oracle for more than a thousand assertions about the plural
  evaluator.

### Licence

Carried forward from ADR 0003, unchanged in substance.

CLDR data is under the Unicode License V3 (SPDX `Unicode-3.0`): MIT-like, with **no copyleft**. Its
grant explicitly includes the right to *modify*, so converting the JSON to TOML and then editing or
deleting strings is within it. Its single condition is that the copyright and permission notice
appear with all copies of the data or in associated documentation.

`locales/UNICODE-LICENSE` holds that notice beside the data it covers, and the crate is declared
`MIT AND Unicode-3.0` — which means the package *contains* material under both licences, not that
everything is dual-licensed. Because there is no copyleft, mixing CLDR-derived and hand-authored
names in one file relicenses nothing: our refusal reasons, hand-authored names and `reviewed` blocks
stay MIT.

Each locale file says so in its header, and records which CLDR release it was seeded from. Provenance
is marked per file rather than per entry: a `source = "cldr"` field on every name would be noise for
a question nobody asks per entry, and relying on `reason` being present only on hand-authored names
would be a coincidence rather than a rule — someone will eventually write a reason for keeping a
CLDR string.

One further restriction in the licence: Unicode's name may not be used to promote cooklang. It is
not.

## 7. Unit identifiers are lowercase words separated by spaces

Supersedes [ADR 0002](#2-unit-identifiers-are-hyphenated-lowercase-words-with-the-variant-first), and keeps the two decisions in it that carry the
safety: the variant is **always** marked, so there is no bare `cup`; and the variant comes **first**,
so `us cup` rather than `cup us`. Only the separator changes, from `-` to a single space:
`us cup`, `us fluid ounce`, `metric cup`, `imperial pint`, `japanese cup`.

There is now **one** form. `Unit::spaced_id` is gone, along with the `id().replace('-', " ")` that
produced it.

### Why

ADR 0002 gave four reasons for the hyphen. Three do not survive contact with the code as it stands:

- **"No TOML key quoting."** No identifier is ever used as a TOML key. Every one of the ~900 places
  an identifier appears is a *value* — `id = "us cup"`, `unit = "metric tablespoon"` — and those are
  quoted either way. ADR 0002's own example, `"fl oz"` needing quotes in `units/spanish.toml`, is
  about an *alias* in a legacy units file, not an identifier.
- **"Usable as-is in CLI arguments, URLs and log fields."** No such use exists. `Unit::from_id` is
  called only from internal code and tests; nothing takes an identifier on a command line.
- **"It composes, since CLDR uses `-` structurally, leaving room for densities or per-unit prices."**
  Speculative. Neither exists, and `grams per liter` reads better as recipe text than
  `grams-per-liter` would.

The fourth is real and survives: **whitespace normalisation hazards.** A double, leading, trailing
or non-breaking space is invisible in a diff where a hyphen could not hide one. That is now
*asserted* rather than avoided — `ids_are_single_spaced_words` rejects all four, including
whitespace that is not U+0020. The hazard is converted from invisible to caught, which is the
trade this ADR makes.

### What it buys

The two-form scheme earned its keep on 20 of 51 units and was pure overhead on the other 31, whose
single-word identifiers were already identical in both forms. Against that:

- `Unit::spaced_id` and its eleven call sites are gone, as is the idea that a unit has an internal
  name and a recipe-facing one that must be kept in step.
- `Locale::written` stops allocating. Both arms borrow now, so it returns `&str` instead of
  `Cow<str>`; it used to build a `String` on every call that fell through to the identifier.
- The reference tables stop printing the same value twice. `docs/units.md` and
  `show-locale-as-md` carried `Identifier` and `Spaced identifier` columns which were **identical
  for 31 of 51 rows**.
- What a config file says is what a recipe says. `preferences.toml` wrote `metric-tablespoon` while
  a reader was written `metric tablespoon`; there is no longer a translation to do in your head.

### What it costs

- **ADR 0002 is superseded**, though nothing was published: `convert_v2` is `pub(crate)`, so no
  identifier has ever been in the public API, and the spaced form already resolved as input, so
  this breaks no recipe.
- **The whitespace guard has to exist**, and someone must not delete it.
- **Composition headroom is gone.** If a `grams per liter` class of unit ever arrives, the
  separator inside it will be ambiguous with the separator between words. ADR 0002 reserved `-` for
  that; this spends it. Judged worth it: the units in front of us do not compose, and designing for
  the ones we are imagining is how the overlay layers happened.

### Consequences

- `serialize_all = "snake_case"` stays the strum default, and all 20 multi-word variants keep an
  explicit `#[strum(serialize = "…")]` — no case style writes lowercase words separated by spaces
  either, so the attributes do not go away, they only change content. A multi-word variant added
  without one still yields `japanese_cup`, which the test rejects.
- `build.rs`'s `variant()` splits on `' '` rather than `'-'` to derive `MetricTablespoon` from
  `metric tablespoon`.
- A test helper parsing `"{value} {id}"` must use `split_once(' ')`, not `split(' ')`.
- The mapping to CLDR and UCUM is unaffected: it was already a reversal rather than a copy
  (`volume-cup-metric`, `[cup_m]`), and `cldr_id` is an explicit `match`.

### Evidence

The rename is **behaviour-preserving, and this was checked rather than assumed**. Resolution never
depended on the hyphen: the locale index only ever contained the spaced form, so `resolve("us cup")`
worked before and after and `resolve("us-cup")` worked in neither.

Applying the 20 substitutions mechanically to the 40 snapshot files and to `docs/units.md` captured
before the change reproduces the committed ones **exactly** — zero lines differ beyond the
relabelling. 899 replacements across 55 files.

## 8. Fractions are a property of the preference row

A converted amount is written as a fraction — `1/2 cup`, `3 1/2 fl oz` — when the preference row
that chose its unit says so: `{ unit = "us cup", at_least = 0.25, fractions = 4 }` in
`locales/preferences.toml`. The row's `fractions` is the largest denominator allowed. A row without
it writes decimals. Decided 2026-10-08.

### The rule

`best_amount` walks the rows largest first, as `best_unit` does. The first row the value reaches
the threshold of takes it — unless that row writes fractions and no fraction within 5 % of the
amount exists with a denominator up to `fractions`, in which case the next row is tried. The last
row takes whatever is left, as a decimal. So `100 ml` for a US reader is no quarter or third of a
cup, and becomes `3 1/2 fl oz`; `0.5 cup` stays `1/2 cup`; `2 cups` stays `2 cups`.

A fraction is a display form, not a rounding. `Number::Fraction` carries the residual, so the stored
amount is exact and a conversion back drifts by nothing. Only what is printed is approximate, and by
at most 5 % — the legacy converter's imperial `accuracy`, kept until the precision tiers in
`docs/research/measurement-precision.md` are decided.

### Why on the row

- **Which units take fractions is regional, not a property of the unit.** Sweden writes `½ tsk`,
  Norway `0,25 ts`, for the same 5 ml spoon (`research/best-units-by-region.md` §2). The row is
  already where a region says how it writes a quantity.
- **"Odd amounts in the smaller unit" is a row transition.** Australia and New Zealand print clean
  cup fractions and otherwise millilitres (§7.1 AU, NZ). Falling through to the next row when no
  fraction fits expresses that without a second mechanism.
- **The legacy converter's unit-and-fraction search is not ported.** It ranked candidates across
  every unit by denominator first, so `1.5 lb` became `24 oz` and `0.26 cup` became `4 tbsp`. Its
  purpose — a quarter cup rather than four tablespoons — is what the thresholds now say directly
  (ADR 0005, `us cup ≥ 0.25`). Choosing the unit, then the fraction in it, keeps the two decisions
  reviewable separately in the snapshot.

### What it costs

- A fraction that fits the first unit always wins, even when the next unit would be whole:
  `1.1 lb` is `1 1/8 lb` if pounds allow eighths, `18 oz` if they allow only quarters. The row's
  denominator is therefore a choice about idiom, made per unit with a citation.
- Zero and negative amounts are never fractions; they are written as they are in the first unit
  they reach, or the last row.
- A range is judged by its start; the end is written in the start's unit, as a fraction if the
  row has one that fits, else as a decimal.

## 9. The old converter is replaced in one step

The old converter, built from `units.toml` and user-supplied unit files, is replaced by the
locale-aware converter in a single pull request. During review both pipelines exist and a
differential test proves the new one reproduces the old one; once the approach is approved, the
old converter, its unit files and the differential test are deleted before merge, and nothing is
ever released with both. Decided 2026-10-11. Vocabulary is in `GLOSSARY.md`.

### What a recipe reads its units through

- **An untagged recipe reads through the legacy locale**, a frozen locale that names exactly the
  units `units.toml` knew and carries its own preference rows transcribed from `units.toml`, so
  that what an untagged recipe does is byte-for-byte what it did. The legacy locale is a distinct
  naming, not a tag: `legacy` is not a language, and a private-use tag such as `und-x-legacy`
  would let `und-x-legacy-u-ms-metric` parse into nonsense. The naming is derived from metadata on
  demand; `Recipe` gains no field.
- **An untagged recipe gets a parse warning** telling the author to add `locale:`. The canonical
  parser warns too, since the key is standard metadata, not an extension.
- **A tag must determine an audience.** `sv-SE` and `sv-u-ms-metric` parse; `sv` alone is a parse
  error naming both fixes. Fitting units after scaling needs an audience, and a parser that
  guessed one, or silently skipped the fit, would be behaviour people come to rely on and
  maintainers can never change. The crate's optional behaviours are where that has happened before.
- **The locale is read from front matter only.** A `>> locale:` line after the first quantity warns
  instead of being honoured; `>>` metadata is deprecated and a token pre-scan would exist only to
  serve it.

### What the converter is

- **There is no unit-free converter.** `Converter::empty()` goes. Locales are part of the crate,
  not an optional feature, and the canonical spec suite passes with unit knowledge on.
- **User-defined units go with it**: `ConverterBuilder`, `UnitsFile`, `Extend`, `units.toml`,
  `units/spanish.toml`, the `bundled_units` feature and the `build.rs` that generated code from
  it. ADR 0001 closed the unit set; an open units file mapped onto a closed set is the complexity
  that decision removed. Adding names for an existing unit can come back as a feature if asked for.
- **A converter holds both names and audience**, built from a tag or from the legacy locale. The
  parser holds one, legacy by default, as a convenience for apps that keep one parser object; the
  parser never reads it when parsing, since a recipe is always parsed through its own naming. It
  is the rendering target for shopping lists and pantry subtraction, which take it explicitly.
- **Aggregation converts into the target first, then adds**, so a Swedish `msk` and an English
  `tbsp` land in one unit written once. A quantity that cannot be converted, because its unit is
  text or unknown, stays as written and groups by its text, as before.
- **Scaling stays in the recipe's own naming.** A scaled `sv-SE` recipe is still in Swedish units.
- **A quantity's unit stays text.** The converter writes the locale's symbol back as text; there is
  no `Unit` id on `Quantity`, so text units stay representable and serialised recipes keep their
  shape. `Unit`, `PhysicalQuantity`, `System`, `Audience`, `LocaleTag` and the converter's error
  are public; `ConvertTo` and `ConvertValue` have no replacement.

### How the swap is reviewed

- **Siblings, not replacements, during review.** Every operation that takes the old converter
  is kept beside the new one, because Rust has no overloading and keeping the old pipeline whole
  is what lets it be run. The new one has the plain name and the old one a `_legacy` suffix:
  `scale` and `scale_legacy`, `Converter` and `LegacyConverter`, `SemanticEq` and
  `LegacySemanticEq`, `convert` and `convert_legacy`. So the reviewer reads the API that merges,
  and the removal commits only delete. The default constructors, `CooklangParser::new`,
  `default`, `extended` and `canonical`, are the new pipeline already; the old one is reached
  through `new_legacy` alone. (Amended 2026-10-11: the suffix was first put on the new names,
  as `_v2`, and flipped once the swap was complete.)
- **One call site per commit**, in the order scaling and fit, grouped lists, quantity arithmetic,
  `Metadata::time`, semantic equality, each green with both pipelines present.
- **The differential test is written first.** A core-crate integration test runs the canonical
  cases, every `.cook` fixture and hand-written scale and grouping cases through both pipelines
  end to end and compares the results with `PartialEq`, because the promise is about what the user
  sees. A divergence is either ported or recorded here; there is no tolerance to widen. The one
  divergence known in advance is the old converter's denominator-first unit search, not ported
  (ADR 0008).
- **The last commits delete.** After approval: the old converter and everything listed above,
  every `_legacy` item, and the differential test. The TypeScript `load_units` option and the
  bindings' empty converter go in the same commits, as breaking changes to those packages. The
  pull request says so in its description.

### Considered and rejected

- A release with both converters and `#[deprecated]` on the old one, letting users migrate at
  their pace. Rejected: one more configuration to keep green, and a deprecation period for an
  API whose replacement changes the type in every signature anyway.
- The new converter as the receiver (`converter.scale(&mut recipe)`), which needs no suffix and no
  rename. Rejected in favour of keeping the recipe-side shape users already have.
- Suffixing the new names (`_v2`) and renaming at removal. Done first, then rejected: the
  reviewer would read an API that is not the one that merges, and the removal would be a rename
  on top of a deletion.
- Silent fallback to the legacy locale with no warning. Rejected: the legacy locale is frozen, and
  without a nudge untagged recipes never move off it.

## 10. A quantity carries its resolved unit

A parsed `Quantity` records both the unit as the recipe wrote it and the [`Unit`](0002-unit-identifiers.md)
that spelling names in the recipe's locale, resolved once, by the parser. Operations that only need to
know what a unit *is* — adding, subtracting, grouping, comparing by meaning — take no converter. A
converter is needed only where there is a reader to write for: fitting after scaling, and writing a
list in its target locale. Decided 2026-10-12. This reverses one line of ADR 0009, "there is no `Unit`
id on `Quantity`"; the rest of that decision stands.

### Why

ADR 0009 kept the unit as text so that serialised recipes keep their shape and text units stay
representable. The price was that every operation on a quantity had to be handed the converter that
could read its text, and the caller had to know which one: a Swedish recipe's `5 dl` and `2 msk`
did not add through the parser's converter, and its `prep time: 30 minuter` read as nothing. The
operations on a recipe were made to derive the converter from the recipe's own `locale` key, which
fixed the recipe-level calls, but a `Quantity` compared or added on its own still needs a converter
it cannot name. The converter parameter was the price of the "text only" line, and it was being paid
in every signature.

Units are now a closed set with stable identifiers, and locales are part of the crate rather than a
units file a caller may or may not have loaded. So there is exactly one correct resolution of a unit
string, fixed at parse time by the recipe's naming. Storing it is recording a fact the parser already
established, not adding configuration.

### The type

```rust
pub struct Quantity {
    value: Value,
    unit: Option<WrittenUnit>,
    scalable: bool,
}

/// A unit as the recipe wrote it, and what it names when the recipe's
/// locale knows that spelling.
pub struct WrittenUnit {
    text: String,
    unit: Option<Unit>,
}
```

Text and resolution live in one value so there is no invariant between two sibling fields. `Unit` and
`PhysicalQuantity` become public, as ADR 0009 planned.

- `Quantity::unit()` is still the text. `Quantity::unit_id()` is what it names, if known.
- `Quantity::new(value, unit)` still builds an unresolved quantity. An unresolved unit behaves exactly
  as an unknown unit does today: it adds only to the same text, compares as text, and is never
  fitted. `Quantity::read_with(&Converter)` resolves it through that converter's locale, for the few
  callers that build quantities by hand.
- `PartialEq` stays "as written" and compares the text only. Two parses that resolved differently are
  the same text, which is what as-written means. The resolved unit is what `SemanticEq` reads.
- `Recipe::read_units()` resolves every quantity through the recipe's own naming, for a recipe
  deserialised from a document written before this change.

### The serialised shape

One additive field, `unit_id`, holding the spaced identifier of ADR 0002 and omitted when the unit is
unknown:

```json
{ "value": { "type": "number", "value": { "type": "regular", "value": 2 } },
  "unit": "msk", "unit_id": "metric tablespoon", "scalable": true }
```

Deserialising applies two rules: a `unit_id` with no `unit` text is an error, and a `unit_id` that
names no unit is an error, since the identifier set is closed and versioned with the crate. A
document without `unit_id` deserialises unresolved. The TypeScript `Quantity` type gains
`unit_id?: string`.

### What changes around it

- The parser resolves at the one place that already knows the locale, where it builds a quantity.
  The collector's own unit lookups go; the timer check and reference compatibility read the
  resolved unit.
- `Quantity::try_add`, `try_sub` and `compatible_unit`, and `GroupedQuantity::add`, `merge`,
  `try_saturating_sub` and `try_saturating_sub_group`, take no converter. Converting between two
  known units is `Unit::convert`; the result keeps the left operand's text, as before.
- `SemanticEq::equals(&self, other)` takes no converter on any type. A `Quantity` converts both
  known units to the metric base and compares there, else compares as text.
- `Converter` keeps `fit` and `convert_for`, which need an audience, and gains `rewrite`, which
  writes a resolved quantity in the converter's locale; `rewrite_from` goes, since the quantity now
  says what unit it is. A quantity with no resolved unit is an `UnknownUnit` error to all three.
- `Metadata::yield_quantity` resolves through the metadata's own naming, so scaling to a yield
  needs no lookup of its own.

### Costs

- A custom serde representation on `Quantity` where a derive did the job, and a change to the
  TypeScript type.
- `Unit`, `PhysicalQuantity` and `WrittenUnit` enter the public API, so adding a unit becomes a
  visible change. ADR 0001 already accepts that.
- Every `Quantity` built outside the parser must be resolved explicitly or it stays unknown. That is
  today's behaviour for an unknown unit, but it is a step a caller can forget, so `read_with`
  appears in every example that builds one.

### Considered and rejected

- Folding `SemanticEq` into `PartialEq`. `PartialEq` has to stay as-written: serde round trips and
  diffing two parses of one file depend on it, and Rust's `PartialEq` requires transitivity, which a
  tolerance-based comparison does not have.
- Two sibling fields, `unit: Option<String>` and `unit_id: Option<Unit>`. They carry an invariant
  nothing enforces; one value that holds both cannot disagree with itself.
- Resolving lazily, on first use, through the recipe's naming. A `Quantity` on its own has no
  recipe to ask, which is the problem being solved.
- Pantry subtraction matching by resolved unit instead of by text, so `1%l` in the pantry covers
  `500%ml` in the list. Follows from this decision but is a behaviour change of its own; not done
  here.
