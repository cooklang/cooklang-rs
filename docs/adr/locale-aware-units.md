# Locale-aware units

The decisions behind unit conversion, locales and the converter, as they stand. Decisions were
taken between 2026-10-02 and 2026-10-12 and several were revised along the way; this records the
result, not the sequence. Code comments cite a decision as `ADR 3`, meaning section 3 below.
Vocabulary is in `GLOSSARY.md`; evidence is in `docs/research/`.

## Contents

1. [Conversion uses a closed set of units defined in Rust](#1-conversion-uses-a-closed-set-of-units-defined-in-rust)
2. [Unit identifiers are lowercase words separated by spaces, with the variant first](#2-unit-identifiers-are-lowercase-words-separated-by-spaces-with-the-variant-first)
3. [CLDR is a source of unit names only, seeded once into a file per locale](#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale)
4. [A locale names only the units it is sure of, and the identifier is universal input](#4-a-locale-names-only-the-units-it-is-sure-of-and-the-identifier-is-universal-input)
5. [Unit preferences follow a region's official measurement system](#5-unit-preferences-follow-a-regions-official-measurement-system)
6. [Fractions are a property of the preference row](#6-fractions-are-a-property-of-the-preference-row)
7. [A recipe reads its units through its own locale](#7-a-recipe-reads-its-units-through-its-own-locale)
8. [A quantity carries its resolved unit](#8-a-quantity-carries-its-resolved-unit)
9. [The old converter is replaced in one step](#9-the-old-converter-is-replaced-in-one-step)

## 1. Conversion uses a closed set of units defined in Rust

Conversion is `convert::units`: one Rust enum per physical quantity, holding the exact legal
definitions, with no configuration. Conversion is the part that must be correct and that users have
no reason to change. Keeping it out of configuration is what makes the rest small enough to expose:
names, preferences and fractions are data, and all of it binds to these units.

Adding a unit is a code change. What a recipe's *text* means is still a choice, because a locale
maps words to units: a locale can map `cup` to a Japanese cup, but only once that unit exists in
the enum.

### Consequences

- Ambiguous units are named for what they are (`UsCup`, `MetricTeaspoon`), and the locale decides
  what `cup` or `tsp` means.
- `convert::units` has no notion of a unit system. Metric versus US customary is a display
  preference handled above conversion ([decision 5](#5-unit-preferences-follow-a-regions-official-measurement-system)),
  and some units, such as a Japanese cup, belong to no system.
- Factors are the statutory definitions, not rounded ratios. The largest difference from the
  ratios `units.toml` carried is about 1.2e-7 relative, for the teaspoon, so comparisons against
  the old converter use a tolerance.
- Temperature conversion handles temperature points only, such as an oven setting, not intervals.
- `Unit` and `PhysicalQuantity` are public, so adding a unit is a visible change to the crate.

## 2. Unit identifiers are lowercase words separated by spaces, with the variant first

Every unit has one stable identifier, the key that locales, preference rows and the serialised
form bind to: `us cup`, `us fluid ounce`, `metric cup`, `imperial pint`, `japanese cup`. Three
rules:

- **The variant is always marked.** There is no bare `cup`, `pint`, `teaspoon` or `gallon`.
- **The variant comes first.** `us cup`, not `cup us`.
- **Words are separated by a single space, and there is one form.** The identifier is
  recipe-facing: it resolves as input in every locale and is what a conversion writes when a
  locale has no symbol for the unit ([decision 4](#4-a-locale-names-only-the-units-it-is-sure-of-and-the-identifier-is-universal-input)).

### Why the variant is always marked

Silently resolving the wrong unit is worse than refusing to resolve at all.
`docs/research/cooking-units.md` records that a "cup" ranges from 180 ml to 284 ml and a "pint"
from 473 ml to 568 ml depending on region, and the maintainer had already been bitten by a metric
recipe resolving a US teaspoon. An unmarked identifier makes one of those the silent default.

The two standards surveyed differ: CLDR's bare `cup` and `pint` are the US ones, with `cup-metric`
and `pint-imperial` as the marked variants, while UCUM marks every variant (`[cup_us]`, `[cup_m]`,
`[foz_br]`). We follow UCUM's completeness, not CLDR's defaulting.

### Why the variant comes first, and what it costs

Variant first reads in natural English order and puts the disambiguating part where a reader looks.
The cost, accepted knowingly: it is the reverse of both CLDR (`cup-metric`) and UCUM (`[cup_us]`),
so the mapping to those standards is a reversal rather than a copy, and the identifiers do not sort
with all cups adjacent. The mapping is an explicit `match` either way.

### Why spaces

No identifier is ever a TOML key, a command-line argument or a URL segment; every one of the
several hundred places an identifier appears is a quoted value. So the arguments for a
machine-friendly separator do not apply, and a space makes a configuration file say what a recipe
says: `preferences.toml` writes `metric tablespoon` and a reader is shown `metric tablespoon`.

The one real hazard of a space is that a double, leading, trailing or non-breaking one is invisible
in a diff. It is asserted rather than avoided: `ids_are_single_spaced_words` rejects all four,
including whitespace that is not U+0020. The guard has to exist, and nobody may delete it.

The cost is composition headroom. If a `grams per liter` class of unit ever arrives, the separator
inside it is ambiguous with the separator between words. Judged worth it: the units in front of us
do not compose, and designing for the ones we are imagining is how over-layered designs happen.

### Consequences

- `serialize_all = "snake_case"` stays the strum default, and every multi-word variant carries an
  explicit `#[strum(serialize = "…")]`, because no case style writes lowercase words separated by
  spaces. A multi-word variant added without one yields `japanese_cup`, which the test rejects.
- `build.rs` derives `MetricTablespoon` from `metric tablespoon` by splitting on the space, so a
  preference row naming a unit that is not in `definitions.rs` fails to compile.
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
| **QUDT** | codes and cross-walks only | CC-BY 4.0 and embeddable, carrying `ucumCode`, `udunitsCode` and `uneceCommonCode` per unit, the licence-safe bridge to UCUM. But its labels are English-only outside SI base units (metre 24 languages, litre 2, `CUP_US` 1), and its `CUP_US` multiplier of 236.58825 mL is *less* precise than our exact 236.5882365 mL, so it is not a factor oracle. It carries **no** CLDR identifier, so there is no own-id → QUDT → CLDR path. |
| **CLDR identifiers as ours** | rejected | Adopting them would mean inheriting the bare-`cup`-means-US default this decision exists to prevent, so a translation layer would be needed anyway. CLDR is still the source of locale *names* ([decision 3](#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale)). |
| **Hyphenated identifiers** (`us-cup`) | rejected | A second, recipe-facing spaced form then has to exist beside it and be kept in step; it was pure overhead on the 31 single-word units and a translation to do in your head for the other 20. |
| **Wikidata** | input aliases only | CC0 with no attribution required, and the only broad source of native-script spellings for units CLDR lacks. But translations and conversion factors sit on disjoint items: `shō` (Q995076) has ja/zh/ko labels and no conversion property at all, while `market catty` (Q108934447) has an exact 0.5 kg and no `zh` label. Never a source of factors or of unit identity. |

## 3. CLDR is a source of unit names only, seeded once into a file per locale

Unit conversion needs, for every supported language, the strings a recipe may use for a unit and
the one string a conversion writes back. Unicode CLDR carries those for most of our units under the
permissive Unicode licence. It also carries conversion factors and per-region unit preferences,
and we use neither: factors are the statutory definitions in Rust
([decision 1](#1-conversion-uses-a-closed-set-of-units-defined-in-rust)) and preferences are a
table we own ([decision 5](#5-unit-preferences-follow-a-regions-official-measurement-system)). The
property this buys is that CLDR can change only how a unit is *written*, never which unit is chosen
or what a conversion returns.

A locale is one hand-editable file, `locales/<tag>.toml`, holding its plural rules, every name it
resolves, and every name it refuses with the reason. It is seeded once from CLDR by
`xtask/new-locale.sh`, records the release it was seeded from in `cldr_version`, and is edited by
hand from then on. CLDR's JSON is not vendored, not committed and not read by the build; the build
reads only TOML.

### What a seed takes from CLDR

- **The symbol from the short pattern, aliases from the long ones.** CLDR's `displayName` is not a
  symbol (in `en` it is `grams`, `cups`), so the symbol is the short `unitPattern-count-other` with
  the placeholder stripped, and the long one/other patterns and display name become aliases. A
  pattern whose placeholder sits inside the string (`摂氏 {0} 度`) yields nothing. Narrow forms are
  never used: CLDR itself warns they are not unique. A short form that is prose rather than a
  symbol (`sec`, `hr`, `days`) is corrected by hand.
- **Plural rules and long forms**, CLDR's own, so that a long form can pick between `1 łyżeczka`,
  `2 łyżeczki` and `5 łyżeczek`. They are evaluated in the crate rather than taken from a
  plural-rules dependency, because they are part of how a unit is *written*, on the same footing as
  the names. `locales/plural-samples.json` is a test fixture, not build input: CLDR's own
  `@integer` and `@decimal` samples, the oracle for the evaluator.
- **The parent chain.** Name fallback follows CLDR's `parentLocales`, so `en-NZ` reaches `en-001`
  (Commonwealth English) rather than truncating to US English, and `nb` reaches `no`.
  `locales/parent-locales.toml` is global because it is consulted for whatever tag a recipe asks
  for, and most of its children are tags we have no file for. This chain is independent of the
  preference fallback in decision 5.

### Adjudication

CLDR answers "what is the avoirdupois pound called in German"; we need "what does a German cook mean
by *Pfund*". For a unit whose amount varies by region, every string CLDR offers needs a verdict,
with a citation into `docs/research/cooking-units.md`. Units that mean the same everywhere, the SI
family, need none. The rules the verdicts follow:

- **Retargeting.** CLDR's unmarked keys name the US units, but in most languages the strings under
  them mean the metric ones: `tsk`, `EL`, `cucharada` are 5 and 15 ml spoons, and `en-GB`'s plain
  `pint` is the 568 ml imperial pint. A seeded name may therefore be moved to a different unit of
  ours, with a citation like a refusal.
- **An unmarked US name is refused outside US customary locales.** CLDR marks only the imperial
  side of each pair (`brittiska quarts`, `Imp. Gallonen`, `英制加仑`) and leaves the US side as the
  bare word in every non-English locale. That is CLDR's data layout, not evidence that a Swedish or
  German kitchen means 3.79 l by `gallon`: in those languages the bare word is the generic unit and
  "British" the qualifier, and the two readings differ by 20% (4% for the fluid ounce; 5.7% for the
  cup against the metric cup). So in a locale whose official system is not US customary, the US
  cup, pint, quart, gallon, fluid ounce and spoons resolve only from a name that carries a `US`
  marker (`en-GB`'s `US qt`, Finnish `am. gallona`) or from the identifier; `en` and `es-US` keep
  the bare words. The avoirdupois pound and ounce have no imperial twin and are unaffected. A test
  holds this for every adjudicated locale.
- **Imperial names are stripped from metric-only locales.** CLDR gives every locale a name for the
  imperial pint, fluid ounce, quart and gallon, and across the 33 non-English locales those are
  root English for the pint (`pt Imp.`), whole English in Swedish, Danish, Dutch and Indonesian
  (`Imp. fluid ounces`), translated definitions in Finnish, Arabic and Greek (*British
  quarter-gallon*), or correct words for a unit no kitchen there has measured with. English and US
  Spanish keep theirs; a locale whose region has no US/UK-system past refuses every string and
  writes the units by identifier; locales with such a past (`hi`, `ms`, `he`, `ar`, `zh-Hant`) keep
  their native names. The unit set is unchanged; only what resolves is.
- **Declining beats inventing.** A refused string cannot be the symbol either, or we would write
  output we refuse to parse. English authors a marked symbol (`US c`, `US pt`, following CLDR's own
  `en-GB` `US qt`); other languages decline to name the US cup rather than coin a form, and the unit
  is written by its identifier (decision 4).
- **Refusing a string for one unit is how it is given to another.** Finnish refuses CLDR's `mm` for
  the millimetre because in a Finnish recipe `mm` is the 1 ml *maustemitta*. The two stanzas sit
  next to each other in `locales/fi.toml`.
- **A refusal is a comment, not data.** Names a locale must not resolve are listed in prose at the
  foot of its file, with the reason. Absence of a name is what stops it resolving, so dressing the
  list as TOML would invite a reader to think the build enforces it. The comment is the audit trail
  and the list a reviewer works through.
- **An identifier cannot be refused.** It resolves in every locale (decision 4), so a refusal
  naming one would be false.
- **Hand-authored names live in the same file** as the seeded ones: the 1 ml Nordic spice measure,
  the Austrian `dag`, the marked English symbols. A `reason` on a name marks it as hand-authored
  rather than seeded; it is not required, and the review is what asks about an uncited name.

### The review gate

A locale may be named by a recipe only once a speaker has confirmed that what it writes is what
recipes in the language write. `reviewed = { date, by, sources }` in the locale's own file records
that; there is no separate list, and the question "is `sv` ready?" is answered by the file. A tag
that reaches an unreviewed locale is refused, not degraded: falling back to identifiers would ship
`2 us cup` to a Swede as though it were a translation, and falling back to the parent would hand a
Ukrainian reader Russian. Adjudication is not review: a locale can have a verdict for every
ambiguous string and still be unreviewed. `locales/README.md` is what a reviewer is asked, and
`cargo run --example show-locale-as-md -- <tag>` prints what a locale would write, reviewed or not.

### What this costs

- **Adding a locale is a human task.** `xtask/new-locale.sh` makes starting one cheap, but nothing
  arrives automatically.
- **A CLDR fix does not propagate.** If Unicode corrects a Polish plural we do not get it; someone
  re-seeds or edits by hand. Deliberate: a name change is something to review, not absorb.
- **Provenance is per file, not per name.** Each file records its `cldr_version`; a `source` field
  on every name would be noise for a question nobody asks per entry.

### Licence

CLDR data is under the Unicode License V3 (SPDX `Unicode-3.0`): MIT-like, with **no copyleft**. Its
grant explicitly includes the right to *modify*, so converting the JSON to TOML and then editing or
deleting strings is within it. Its single condition is that the copyright and permission notice
appear with all copies of the data or in associated documentation.

`locales/UNICODE-LICENSE` holds that notice beside the data it covers, and the crate is declared
`MIT AND Unicode-3.0`, which means the package *contains* material under both licences, not that
everything is dual-licensed. Because there is no copyleft, mixing CLDR-derived and hand-authored
names in one file relicenses nothing: refusal reasons, hand-authored names and `reviewed` blocks
stay MIT. Each locale file says so in its header. Unicode's name may not be used to promote
cooklang, and is not.

### Alternatives considered

| Option | Verdict | Reason |
|---|---|---|
| ICU4X unit formatting | rejected | It runs the other way: a formatter turns a unit and a number into a string and cannot hand back an alias list, while our primary job is input resolution (`cucharadas` → a unit). `icu::experimental::units` is also experimental and needs a data provider across the Rust library, the WASM package and two UniFFI targets. |
| CLDR conversion factors | rejected | Already decided in decision 1; and CLDR's `cup-jp` is 100 ml, which is wrong. |
| CLDR unit preferences | seed only | There is no cooking usage; the nearest list gives `GB` only the imperial gallon and fluid ounce. See decision 5. |
| Vendored CLDR JSON with overlay files for verdicts, symbol fixes and supplements | rejected | That is cldr-json's architecture, right for a living multi-contributor database and wrong for a small frozen subset: answering "what does `sv` name?" meant reading six files and simulating a merge, and a CLDR bump was a 38-file diff of single-line minified JSON that nobody could review. The review gate already makes a human task per locale mandatory, so the automation bought nothing. |
| Build fails on an unadjudicated name | rejected | Right for six locales, a multi-day blocker for thirty. |
| Wikidata for aliases | not now | CC0 and broad in native-script spellings for units CLDR lacks, but its factors are unreliable and it conflates regional variants. Possible later source for supplements. |

## 4. A locale names only the units it is sure of, and the identifier is universal input

A locale names only the units it has a safe name for. CLDR lacks some of our units, and adjudication
makes a locale decline others: German has no name for the 236.588 ml US cup. A locale's symbol is
therefore optional, and a unit the locale does not name is written as its identifier: `2 imperial
pint`, `1 nordic spice measure`. The identifier resolves as input in **every** locale, so what we
write we can always read back. Nothing is inherited from any other locale: inheriting the legacy
locale's English names would reinstate strings adjudication refused.

### Consequences

- `Locale::new` does not require every unit to be named. It rejects a string that names two units,
  and an alias equal to another unit's identifier.
- The legacy locale is exactly the 38 bundled units under their `units.toml` names. It is frozen;
  it does not learn new units, and nothing is layered on it.
- Best-unit selection (decision 5) must not consult names: if it skipped unnamed units, a change to
  a locale could change which unit is chosen. An unnamed choice is written by identifier instead.
- **No derived SI symbols.** CLDR lacks `kiloliter`, `decaliter`, `hectogram`, `decagram`,
  `decigram`, `centigram`, `hectometer` and `decameter`. They are not synthesised from prefix and
  base symbol; a language that uses one hand-authors it (`hg` in the Nordic locales, `dag`/`dkg`
  in German). Likewise `kilometer`, `hectoliter`, `decimeter` and `milligram`, which CLDR does
  have, are deliberately not seeded: no recipe uses them, and every resolvable string is also a
  string the inline-quantities extension will match in prose, so short symbols are not spent on
  units nobody writes. The legacy locale keeps all of them.
- In US English the identifier is already the natural name (`metric teaspoon`), so `en` needs no
  hand-authored name for the metric units.

### Alternatives considered

| Option | Reason rejected |
|---|---|
| Every locale is an overlay on the legacy locale, so every unit has a symbol | Makes legacy's English `c` resolve to the US cup in German, Spanish and Japanese recipes, a string nobody adjudicated. |
| The legacy locale grows hand-authored English names for new units | Legacy stops being "exactly `units.toml`", and those names leak into every locale that inherits from it. |
| Converting to an unnamed unit is an error | Output would depend on names after the unit was chosen without them; a preference row could select a unit the locale then refuses to write. |
| Derive the eight SI symbols from prefix + base | Correct in principle (the prefix symbols are international) but it spends `kl`, `dag`, `cg` in thirty locales where nobody writes them. |

## 5. Unit preferences follow a region's official measurement system

Which unit a converted value is *written in* is a display preference, separate from how units are
*named*. Names belong to the language: `en-GB` names `pint` as the 568 ml imperial pint because
that is what a British recipe means. Preferences belong to the region, and a region gets the units
of its **official** measurement system: `GB` is metric, so a value converted for a British reader
is written in millilitres and grams even though the recipe's own `pint` parsed as imperial.

An audience is a region or a system, plus an optional temperature unit, and comes from the locale
tag: the tag's region, or `-u-rg-` overriding it, or `-u-ms-metric`, `-u-ms-ussystem` and
`-u-ms-uksystem` naming a system outright, which takes precedence. `-u-mu-celsius` and
`-u-mu-fahrenhe` name a temperature unit outright, for temperature only: the one quantity a
reader's system and habit commonly disagree on, so `en-CA-u-mu-fahrenhe` keeps Canada's metric
cups and gets Fahrenheit ovens. CLDR's data does not list Canada as a Fahrenheit region, so the
override is the reader's to ask for, not inferred. A `-u-ms-` or `-u-mu-` value we have no rows
for is an error, not a guess.

### The table

The rows are data, in `locales/preferences.toml`: one `[[system]]` or `[[region]]` entry per
quantity, each with its rows, thresholds, fraction denominators
([decision 6](#6-fractions-are-a-property-of-the-preference-row)) and a `reason` citing
`docs/research/best-units-by-region.md`. Only two rules stay in code: a region's official system,
and time falling back to the metric list.

- `official_system(region)`: `US` is US customary; every other region is metric. CLDR's own data
  also lists Liberia as US customary and Myanmar, with the UK, as imperial; we do not copy it,
  because all three are officially metric or metricating, and a reader there who wants otherwise
  can say so with `-u-ms-`.
- Rows, largest first, each with the amount a value must reach to be shown in it:
  - **Metric**: `liter → milliliter ≥ 50 → metric tablespoon → metric teaspoon ≥ ¼ → milliliter`;
    `kilogram → gram → milligram`; `meter → centimeter → millimeter`; `celsius`. The spoons are in
    the *system* row, not repeated per region, because every metric region's recipes print them
    for 5–45 ml; the exceptions are mainland China, South Africa's all-millilitre register and a
    few publishers (`research/best-units-by-region.md` §7.3).
  - **US customary**: `us cup ≥ ¼ → us fluid ounce → us tablespoon → us teaspoon`; `pound → ounce`;
    `foot → inch`; `fahrenheit`. The chain stops at the cup (`4 cups`, not `1 quart`). The cup
    starts at a quarter because American recipes write ¼, ⅓ and ½ cup on every page.
  - **Imperial**: `imperial pint → imperial fluid ounce → metric tablespoon → metric teaspoon`;
    `pound → ounce`; `foot → inch`; `celsius`. The spoons are metric because a British teaspoon is
    5 ml; the chain stops at the pint for the same reason the US chain stops at the cup; ovens
    are Celsius.
  - **Time** is one list for every system: `day → hour → minute → second`.
- Regional variation *within* metric is a region row, overriding the official system's row for
  that quantity only, authored from `research/best-units-by-region.md` §7.1: volume in
  decilitres for `SE NO DK FI CH HU` and Flemish `BE`, centilitres for `FR`, the 250 ml cup for
  `NZ`, `CA` and `AU` (with the 20 ml Australian tablespoon), the 200 ml cup for `JP` and `KR`,
  the 240 ml cup for `BR` and `MX`, Hungarian mass in decagrams and Colombian mass in the 500 g
  libra. Austrian decagrams, though real, are contested by every edited Austrian publisher and
  have no row. No source states a switching threshold, so each is inferred from how recipes
  there are written and is a judgement to revisit, not a fact. An explicit `-u-ms-` ignores
  region rows: `sv-SE-u-ms-metric` asks for the system, and gets millilitres.

### Consequences

- `GB` has no rows. The naming half of the UK problem is solved by adjudication
  ([decision 3](#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale)), the
  preference half by `GB` being metric.
- An unlisted region is metric, which is correct rather than a guess.
- Three choices differ from the old converter's `best` lists, each recorded and pinned by a test:
  the US rows have a fluid ounce row, so `4 fl oz` of milk is no longer `8 tbsp`; the metric rows
  have the metric spoons, so two tablespoons of oil are `2 tbsp` rather than `30 ml`; and the US
  cup starts at a quarter, which the old converter reached only sideways, by capping tablespoons at
  four in its fractions config.

### Alternatives considered

| Option | Reason rejected |
|---|---|
| `GB` rows are imperial, `-u-ms-uksystem` ≡ `GB` (CLDR's equivalence) | Writes imperial output to every British reader when the country is metric; the only region with imperial units would be one where they are no longer official. |
| `-u-ms-uksystem` is unsupported | Nobody could ask for imperial output; a tag CLDR considers valid would be rejected. |
| CLDR's `unitPreferenceData` | No cooking usage; its `GB` fluid list has only the imperial gallon and fluid ounce and could never select the 568 ml pint. |
| Rows keyed by system only | Cannot express Nordic `dl` or Hungarian `dkg` without inventing a system per country. |
| Rows keyed by `(quantity, region)` only | Every region needs its own rows, and an unlisted one has none; keying by region *with* a fallback to its official system keeps the property that an unlisted region is correct. |

## 6. Fractions are a property of the preference row

A converted amount is written as a fraction (`1/2 cup`, `3 1/2 fl oz`) when the preference row that
chose its unit says so: `{ unit = "us cup", at_least = 0.25, fractions = 4 }` in
`locales/preferences.toml`. The row's `fractions` is the largest denominator allowed, and the build
refuses one larger than `Number` can write. A row without it writes decimals.

### The rule

`best_amount` walks the rows largest first. The first row the value reaches the threshold of takes
it, unless that row writes fractions and no fraction within 5 % of the amount exists with a
denominator up to `fractions`, in which case the next row is tried. The last row takes whatever is
left, as a decimal. So `100 ml` for a US reader is no quarter or third of a cup, and becomes
`3 1/2 fl oz`; `0.5 cup` stays `1/2 cup`; `2 cups` stays `2 cups`.

A fraction is a display form, not a rounding. `Number::Fraction` carries the residual, so the stored
amount is exact and a conversion back drifts by nothing. Only what is printed is approximate, and by
at most 5 %, the old converter's imperial `accuracy`, kept until the precision tiers in
`docs/research/measurement-precision.md` are decided.

### Why on the row

- **Which units take fractions is regional, not a property of the unit.** Sweden writes `½ tsk`,
  Norway `0,25 ts`, for the same 5 ml spoon (`research/best-units-by-region.md` §2). The row is
  already where a region says how it writes a quantity.
- **"Odd amounts in the smaller unit" is a row transition.** Australia and New Zealand print clean
  cup fractions and otherwise millilitres (§7.1 AU, NZ). Falling through to the next row when no
  fraction fits expresses that without a second mechanism.
- **Choosing the unit, then the fraction in it, keeps the two decisions reviewable separately** in
  the snapshot. The old converter ranked candidates across every unit by denominator first, so
  `1.5 lb` became `24 oz` and `0.26 cup` became `4 tbsp`; its purpose, a quarter cup rather than
  four tablespoons, is what the thresholds now say directly (`us cup ≥ ¼`). That search is not
  ported, and the eleven renderings it changes for untagged recipes are listed in
  `tests/differential.rs`.

### What it costs

- A fraction that fits the first unit always wins, even when the next unit would be whole:
  `1.1 lb` is `1 1/8 lb` if pounds allow eighths, `18 oz` if they allow only quarters. The row's
  denominator is therefore a choice about idiom, made per unit with a citation.
- Zero and negative amounts are never fractions; they are written as they are in the first unit
  they reach, or the last row.
- A range is judged by its start; the end is written in the start's unit, as a fraction if the
  row has one that fits, else as a decimal.

## 7. A recipe reads its units through its own locale

Which locale a recipe's units are read and written through comes from the recipe, never from the
caller. This is its *naming*: the legacy locale when the recipe has no `locale` key, else the
reviewed locale its tag names. The parser is configured by its extensions alone and holds no
converter; `Recipe::converter()` is the converter the naming calls for, and scaling, grouping,
`Metadata::time`, `Recipe::convert_for` and semantic equality all go through it.

### The locale key

- **It is a BCP 47 tag** (`sv-SE`, `en-GB-u-ms-uksystem`), not `xx_YY`. The tag's language picks
  the names, its region or extensions the audience (decision 5).
- **A tag must determine an audience.** `sv-SE` and `sv-u-ms-metric` parse; `sv` alone is a parse
  error naming both fixes. Fitting units after scaling needs an audience, and a parser that
  guessed one, or silently skipped the fit, would be behaviour people come to rely on and
  maintainers can never change.
- **A tag no locale covers, or one nobody has reviewed, is a parse error** that says how to get it
  reviewed. The recipe is then read through the legacy locale, which is also what every operation
  on it will use.
- **The key is read before the rest of the front matter**, whatever its position in the block, so
  a `prep time: 30 minuter` above it is read through Swedish. A `>> locale:` line is honoured only
  while nothing has been read through a locale yet; after a quantity with a unit, an inline
  quantity or a time key it is refused with a warning, since the units above it have already been
  read. There is no token pre-scan: `>>` metadata is deprecated.
- **A recipe with no `locale` gets a warning** telling the author to add one, from every parser
  including the canonical one, since the key is standard metadata. The legacy locale is frozen,
  and without a nudge untagged recipes never move off it.

### The legacy locale

An untagged recipe reads through a frozen locale that names exactly the units `units.toml` knew,
with preference rows transcribed from `units.toml`'s `best` lists and fraction config, so that what
an untagged recipe does is what it always did: a quantity is fitted within the system it was
written in, metric staying metric and US staying US, and a row that wrote a decimal where no
fraction fitted still does. It is a distinct naming, not a tag: `legacy` is not a language, and a
private-use tag such as `und-x-legacy` would let `und-x-legacy-u-ms-metric` parse into nonsense.
The naming is derived from metadata on demand; `Recipe` gains no field.

### What the converter is

A `Converter` holds a locale and an audience, built from a tag (`Converter::for_tag`), from a
recipe (`Converter::for_recipe`) or as the legacy one (`Converter::legacy`). It fits a quantity for
its audience after scaling, writes a quantity for any audience (`convert_for`), and rewrites a
quantity's unit in its own locale's spelling (`rewrite`). There is no unit-free converter:
locales are part of the crate, not an optional feature, and the canonical spec suite passes with
unit knowledge on. User-defined units go with the old converter: `ConverterBuilder`, `UnitsFile`,
`units.toml`, `units/spanish.toml` and the `bundled_units` feature. Adding names for an existing
unit can come back as a feature if asked for.

- **Scaling stays in the recipe's own naming.** A scaled `sv-SE` recipe is still in Swedish units.
- **A list is written in a target.** `IngredientList::add_recipe` takes the converter the list is
  written in, rewrites each quantity into the target's spelling, then adds, so a Swedish `msk` and
  an English `tbsp` land in one unit written once. A quantity that cannot be rewritten, because its
  unit is text or unknown, stays as written and groups by its text.
- **`Recipe::convert_for(audience)`** writes every quantity for another audience while keeping the
  recipe's own names: a `sv-SE` recipe for an American reader is in US units spelled the Swedish
  way, and the US units Swedish declines to name are written by identifier.
- `Unit`, `PhysicalQuantity`, `System`, `Audience`, `LocaleTag`, `Naming` and the converter's
  error are public. `ConvertTo` and `ConvertValue` have no replacement.

### Considered and rejected

- The parser holds the list target as a convenience for apps that keep one parser object. The
  parser never read it, no caller used the accessor, and a constructor argument the type ignores
  misleads every reader.
- Silent fallback to the legacy locale with no warning: without a nudge untagged recipes never
  move off it.
- Guessing an audience for a tag with no region, or skipping the fit: behaviour nobody could later
  change.

## 8. A quantity carries its resolved unit

A parsed `Quantity` records both the unit as the recipe wrote it and the `Unit` that spelling names
in the recipe's locale, resolved once, by the parser. Operations that only need to know what a unit
*is* (adding, subtracting, grouping, comparing by meaning) take no converter. A converter is needed
only where there is a reader to write for.

### Why

Units are a closed set with stable identifiers, and locales are part of the crate rather than a
file a caller may or may not have loaded. So there is exactly one correct resolution of a unit
string, fixed at parse time by the recipe's naming, and storing it records a fact the parser already
established. Keeping the unit as text instead would mean every operation on a quantity has to be
handed the converter that can read it, and a `Quantity` compared or added on its own has no recipe
to ask which one that is.

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

Text and resolution live in one value so there is no invariant between two sibling fields.

- `Quantity::unit()` is the text. `Quantity::unit_id()` is what it names, if known.
- `Quantity::new(value, unit)` builds an unread quantity, which behaves as an unknown unit: it adds
  only to the same text, compares as text, and is never fitted. `Quantity::read_with(&Converter)`
  reads it through that converter's locale, for the few callers that build quantities by hand.
- `PartialEq` is "as written" and compares the text only. The resolved unit is what `SemanticEq`
  reads: both known units go to the metric base and are compared there, else as text.
- `Recipe::read_units()` reads every quantity through the recipe's own naming, for a recipe
  deserialised from a document written before units were recorded.

### The serialised shape

One additive field, `unit_id`, holding the identifier and omitted when the unit is unknown:

```json
{ "value": { "type": "number", "value": { "type": "regular", "value": 2 } },
  "unit": "msk", "unit_id": "metric tablespoon", "scalable": true }
```

A `unit_id` with no `unit` text is an error, and a `unit_id` that names no unit is an error, since
the identifier set is closed and versioned with the crate. A document without `unit_id`
deserialises unread. The TypeScript `Quantity` type gains `unit_id?: string`.

### Consequences

- The parser reads the unit at the one place that already knows the locale, where it builds the
  quantity; the timer check and reference compatibility read the result.
- `Quantity::try_add`, `try_sub` and `compatible_unit`, and `GroupedQuantity::add`, `merge`,
  `try_saturating_sub` and `try_saturating_sub_group`, take no converter. Converting between two
  known units is `Unit::convert`; the result keeps the left operand's text.
- `SemanticEq::equals(&self, other)` takes no converter on any type.
- `Converter::fit`, `convert_for` and `rewrite` require a read unit; one that was not read, or is
  not known, is an `UnknownUnit` error.
- `Metadata::yield_quantity` reads through the metadata's own naming, so scaling to a yield needs
  no lookup of its own.

### What it costs

- A custom serde representation on `Quantity` where a derive did the job, and a change to the
  TypeScript type.
- `WrittenUnit` enters the public API beside `Unit` and `PhysicalQuantity`.
- Every `Quantity` built outside the parser must be read explicitly or it stays unknown. That is
  the behaviour an unknown unit has anyway, but it is a step a caller can forget, so `read_with`
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
  `500%ml` in the list. Follows from this decision but is a behaviour change of its own; not done.

## 9. The old converter is replaced in one step

The old converter, built from `units.toml` and user-supplied unit files, is replaced by the
locale-aware converter in a single pull request. During review both pipelines exist and a
differential test proves the new one reproduces the old one on untagged recipes; once the approach
is approved, the old converter, its unit files and the differential test are deleted before merge,
and nothing is ever released with both.

### How the swap is reviewed

- **Siblings, not replacements, during review.** Every operation that takes the old converter is
  kept beside the new one, because Rust has no overloading and keeping the old pipeline whole is
  what lets it be run. The new one has the plain name and the old one a `_legacy` suffix: `scale`
  and `scale_legacy`, `Converter` and `LegacyConverter`, `SemanticEq` and `LegacySemanticEq`,
  `convert` and `convert_legacy`. So the reviewer reads the API that merges, and the removal
  commits only delete. `CooklangParser::new`, `default`, `extended` and `canonical` are the new
  pipeline; the old one is reached through `new_legacy` alone.
- **The differential test is the evidence.** `tests/differential.rs` runs the canonical cases,
  every `.cook` fixture and hand-written scale and grouping cases through both pipelines end to end
  and compares parsing, scaling, lists, metadata time and equality verdicts. A divergence is either
  ported or listed there with its cause, and a listed divergence must still occur or the test
  fails, so the list cannot outlive what it explains. The one class of divergence is the old
  converter's denominator-first unit search, not ported (decision 6).
- **The last commits delete.** After approval: the old converter and everything built on it, every
  `_legacy` item, and the differential test. The TypeScript `load_units` option and the bindings'
  empty converter go in the same commits, as breaking changes to those packages, and the pull
  request says so.

### Considered and rejected

- A release with both converters and `#[deprecated]` on the old one, letting users migrate at
  their pace. One more configuration to keep green, and a deprecation period for an API whose
  replacement changes the type in every signature anyway.
- The new converter as the receiver (`converter.scale(&mut recipe)`), which needs no suffix. Keeping
  the recipe-side shape users already have was worth more.
- Suffixing the *new* names (`_v2`) and renaming at removal. The reviewer would read an API that is
  not the one that merges, and the removal would be a rename on top of a deletion.
