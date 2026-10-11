# Unit identifiers are hyphenated lowercase words with the variant first

> **Superseded by [ADR 0007](./0007-unit-identifiers-are-spaced-words.md).** Two of its
> decisions stand and carry the safety: the variant is always marked, so there is no bare
> `cup`, and the variant comes first. Only the separator changed, from `-` to a space, so
> every `us-cup` below now reads `us cup`. Its *Why hyphen* section is the argument 0007
> answers, and its *Alternatives considered* table — uom, icu4x, UCUM, QUDT, Wikidata —
> is untouched and still the reason not to re-research them.


Unit identifiers are the stable key that aliases, locale data and any future user configuration bind
to, so they are expensive to change once released. They are lowercase words joined by `-`, with any
regional or system variant **first**: `us-cup`, `us-fluid-ounce`, `metric-cup`, `imperial-pint`,
`japanese-cup`. The variant is **always** marked — there is no bare `cup`.

## Why the variant is always marked

Silently resolving the wrong unit is worse than refusing to resolve at all. The maintainer had
already been bitten by assuming a metric teaspoon in a metric recipe when the bundled table was US,
and `docs/research/cooking-units.md` records that a "cup" ranges from 180 ml to 284 ml and a "pint"
from 473 ml to 568 ml depending on region. An unmarked identifier makes one of those the silent
default.

Both standards surveyed do the opposite: CLDR's bare `cup` and `pint` are the US ones, with
`cup-metric` and `pint-imperial` as the marked variants, and UCUM marks every variant
(`[cup_us]`, `[cup_m]`, `[foz_br]`). We follow UCUM's completeness, not CLDR's defaulting.

## Why hyphen

A single unquoted token: no TOML key quoting (`"fl oz"` already needs quotes in
`units/spanish.toml`), no whitespace-normalisation hazards — double, trailing and non-breaking
spaces are invisible in a diff — and usable as-is in CLI arguments, URLs and log fields. It also
composes, since CLDR uses `-` structurally (`-per-`, `square-`), leaving room for densities or
per-unit prices. `id.replace('-', " ")` recovers a readable form when one is wanted.

## Why the variant comes first, and what that costs

Variant-first reads in natural English order and puts the disambiguating part where a reader looks
first. The cost, accepted knowingly: it is the reverse of both CLDR (`cup-metric`) and UCUM
(`[cup_us]`), so the mapping to those standards is a reversal rather than a copy, and our
identifiers do **not** sort with all cups adjacent — `us-cup`, `metric-cup` and `imperial-cup` land
under u, m and i, so a listing or a "did you mean" suggestion will not group them.

## Consequences

- `serialize_all = "snake_case"` stays the strum default even though no identifier contains an
  underscore. A multi-word variant added *without* an explicit `#[strum(serialize = "…")]` then
  yields `japanese_cup`, which the `ids_are_hyphenated_words` test rejects. Switching the default to
  `kebab-case` would instead yield `japanese-cup` — plausible, wrongly ordered, and silent.
- The identifier is not purely internal: its spaced form (`imperial pint`) is recipe-facing — it
  resolves as input in every locale and is what a conversion writes when a locale has no symbol
  for the unit. See [ADR 0004](./0004-partial-locales.md).
- CLDR identifiers never appear in the public API. They are reachable only through explicitly named
  accessors, so swapping or dropping CLDR is not a breaking change. Locale identifiers are the
  opposite call: those are standard BCP 47 tags and are meant to be exposed. We own the unit
  vocabulary; we do not own the locale vocabulary.

## Alternatives considered

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
