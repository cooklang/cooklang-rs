# Unit preferences follow a region's official measurement system

Which unit a converted value is *written in* is a display preference, separate from how units are
*named*. Names belong to the language: `en-GB` names `pint` as the 568 ml imperial pint because
that is what a British recipe means. Preferences belong to the region, and a region gets the units
of its **official** measurement system: `GB` is metric, so a value converted for a British reader is
written in millilitres and grams even though the recipe's own `pint` parsed as imperial. Preference
rows are keyed by region *or* by system, a region without rows of its own uses its official
system's rows, and `-u-ms-metric`, `-u-ms-ussystem` and `-u-ms-uksystem` select a system directly.

## The table

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

## Consequences

- `GB` has no rows. The naming half of the UK problem is solved by adjudication (ADR 0003), the
  preference half by `GB` being metric.
- Decision 6 of the earlier plan — "preferences keyed `(quantity, region)`" — becomes "keyed by
  region, falling back to the region's official system". The property that an unlisted region is
  metric, correct rather than a guess, survives.
- The US rows diverge from the bundled converter's `best` list by the `us fluid ounce` row:
  bundled writes `4 fl oz` of milk as `8 tbsp`. ADR 0001's list of differences records it, and the
  parity test pins the new rows rather than bundled's.

## Alternatives considered

| Option | Reason rejected |
|---|---|
| `GB` rows are imperial, `-u-ms-uksystem` ≡ `GB` (CLDR's equivalence) | Writes imperial output to every British reader when the country is metric; the only region with imperial units would be one where they are no longer official. |
| `-u-ms-uksystem` is unsupported | Nobody could ask for imperial output; a tag CLDR considers valid would be rejected. |
| CLDR's `unitPreferenceData` | No cooking usage; its `GB` fluid list has only the imperial gallon and fluid ounce and could never select the 568 ml pint. |
| Rows keyed by system only | Loses the ability to express Nordic `dl` or Austrian `dag` later without inventing a system per country. |

## Amendment 2026-10-08: `-u-mu-` overrides the temperature unit

BCP 47's `mu` key (`celsius`, `fahrenhe`; CLDR also defines `kelvin`, which has no unit here and is
refused) names a temperature unit outright and overrides whatever `-u-ms-` or the region would
choose, for temperature only. It is the standard answer to the one quantity a reader's system and
habit commonly disagree on: `en-CA-u-mu-fahrenhe` keeps Canada's metric cups and gets Fahrenheit
ovens. CLDR models the same split as `measurementSystem category="temperature"`; its data does not
list Canada there, so the override is the reader's to ask for, not inferred from the tag.
