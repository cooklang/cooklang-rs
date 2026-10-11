# CLDR is a source of unit names only

> **Superseded by [ADR 0006](./0006-locales-are-seeded-not-vendored.md).** Its core
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

## How CLDR is used

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

## Licence

CLDR data is under the Unicode License V3 (SPDX `Unicode-3.0`): MIT-like, no copyleft, and its one
condition is that the notice appear with copies or in associated documentation. `locales/cldr/LICENSE`
holds the notice next to the data it covers, the crate is declared `MIT AND Unicode-3.0`, and the
README states the scope: the files under `locales/cldr/*.json` and the code generated from them carry the
Unicode licence; everything else, including every hand-authored verdict and supplement, is MIT.

## Alternatives considered

| Option | Verdict | Reason |
|---|---|---|
| ICU4X unit formatting | rejected | It runs the other way: a formatter turns a unit and a number into a string and cannot hand back an alias list, while our primary job is input resolution (`cucharadas` → a unit). `icu::experimental::units` is also experimental and needs a data provider across the Rust library, the WASM package and two UniFFI targets. |
| CLDR conversion factors | rejected | Already decided in ADR 0001; and CLDR's `cup-jp` is 100 ml, which is wrong. |
| CLDR unit preferences | seed only | There is no cooking usage; the nearest list gives `GB` only the imperial gallon and fluid ounce. See ADR 0005. |
| Build fails on an unadjudicated alias | superseded | Right for six locales, a multi-day blocker for thirty. Default-reject keeps the safety property (never resolve the wrong unit) without making vendoring a human task. |
| Wikidata for aliases | not now | CC0 and broad in native-script spellings for units CLDR lacks, but its factors are unreliable and it conflates regional variants. Possible later source for supplements. |
