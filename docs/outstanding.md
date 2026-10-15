# Outstanding items

Things deliberately deferred, and findings that would otherwise have to be rediscovered. Each entry
says what it is, why it is parked, and what finishing it would take.

This file does not duplicate:

- [`adr/locale-aware-units.md`](./adr/locale-aware-units.md) — the decisions themselves, and the
  alternatives each one rejected.
- [`research/cooking-units.md`](./research/cooking-units.md) — the survey of units worldwide, with
  the recipe-scale cutoff.
- [`research/measurement-precision.md`](./research/measurement-precision.md) — what rounding error
  cooking tolerates, and the prior art for marking a quantity exempt from rounding (the precision
  tiers [decision 6](./adr/locale-aware-units.md#6-fractions-are-a-property-of-the-preference-row) leaves open).
- [`locale-aware-units.md`, decision 3](./adr/locale-aware-units.md#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale) —
  why a locale is one hand-editable file rather than vendored CLDR plus four overlays, and what
  that costs.

## Parked features

### Inline quantities

`Extensions::INLINE_QUANTITIES`, implemented by `find_inline_quantity` in
`src/analysis/event_consumer.rs`. Matching arbitrary units in prose is considered unsupportable long
term; the intent is to keep at most the documented temperature-symbol case (`180 ºC`).

Facts worth not rediscovering:

- `extensions.md` documents this as **Temperature** only, and says "in the future this may be
  extended to any unit". The code has no temperature check — it accepts any unit the converter
  knows, by any name, symbol or alias. So `2 c` (cups), `8 in` (inches) and `3 d` (days) already
  become quantities today. The documentation is stale, not the code.
- The format is `<number><unit>` or `<number> <unit>`, with trailing punctuation tolerated and
  leading `-` accepted as a negative. A unit containing a space can never match, because the scanner
  tokenises on whitespace — so `fl oz` and `fluid ounce`, both real bundled keys, are unreachable.
- It is in `Extensions::default()` (which is `all()`) **and** in `Extensions::COMPAT`. Narrowing or
  removing it is therefore user-visible: it needs a `CHANGELOG.md` entry marked `(breaking)`, and
  probably its own ADR. Only `CooklangParser::canonical()` turns it off today.
- Stacking a units file makes it worse. The Spanish file's
  `kilogram = { aliases = ["kilo", "kilos"] }` makes `2 kilos` match, and `taza`, `pie` and `día`
  become live too.
- Inline quantities are built with `Quantity::new`, so `scalable` is `false` — they never scale with
  servings. Correct for `20 min`, questionable for `250 ml`.
- **The call site is untested.** The only coverage is a four-case unit test on
  `find_inline_quantity` itself (`1ml`, `1 ml`, `1ml.`, `1 ml.`). Nothing asserts how the step text
  is split into `Item::Text` and `Item::InlineQuantity`, nor what ends up in
  `Recipe::inline_quantities`. If the feature is narrowed, that is the moment to add those tests.
- **Every resolvable unit string widens this feature's reach.** Every name in every
  `locales/<tag>.toml` adds a string the scanner will match in prose, in
  whichever locale is active. That is the reason ADR 4 declines to name `km`, `hl`, `dm` and
  `mg` in CLDR locales, and it is one more reason to narrow this feature to the documented
  temperature case.

## Known-stale documentation in the legacy converter

- **The conversion formula is documented backwards** in two places: the doc comment on `Unit`
  (`src/convert_legacy/mod.rs`) and on `UnitEntry` (`src/convert_legacy/units_file.rs`) both say conversion is
  `val * ratio + difference`. The code (`convert_f64` in `src/convert_legacy/mod.rs`) does
  `(val + from.difference) * from.ratio`, then `/ to.ratio - to.difference` — offset first, then
  scale. This matters for temperature, where it is the difference between going through Kelvin and
  getting a wrong answer.
- **`equal_f64` has no doc comment**, and is an absolute comparison (`|a - b| < epsilon`). Its tests
  in `src/float.rs` still use helpers named `eq_1pct` and `ne_1pct` with an epsilon of `0.01`, which
  reads as "1 percent" and no longer describes what the function does. Residue of commit `5c0b5f0`,
  "remove the relative float comparison".
- **`cargo doc` is already failing.** With `RUSTDOCFLAGS="--deny warnings"` there are 17 unresolved
  intra-doc links (`ScaledRecipe`, `ScalableRecipe`, `ScalableValue`, `crate::pantry` and others) in
  `src/lib.rs`, `src/model.rs`, `src/ingredient_list.rs` and `src/convert_legacy/mod.rs`. These predate the
  `convert_v2` work and none are in it, but `nix flake check` runs this check, so it is red.

## Facts about CLDR worth not rediscovering

Collected while seeding locale names from CLDR. The mechanism is in
[ADR 3](./adr/locale-aware-units.md#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale) and in `locales/<tag>.toml`; these are
the things that would otherwise be rediscovered the hard way. CLDR's JSON is no longer in the
repository, so re-checking any of these means re-fetching with `xtask/new-locale.sh`.

- **CLDR's `cup-jp` is 100 ml** (`factor='1*100*0.000001'`), not the 200 ml Japanese measuring cup
  in `research/cooking-units.md`; the factor syntax reading is confirmed against units that
  cross-check. Looks like a CLDR bug. Never import this value.
- **CLDR has no metric teaspoon or tablespoon unit** and no spice measure. Every language's words
  for the 5 and 15 ml spoons sit under the US keys, which is why seeding sends them to our own
  `metric teaspoon` and `metric tablespoon`.
- **`cup-imperial` and `pint-imperial` are untranslated in every non-English locale**: root's
  `{0} cup Imp.` and `{0} pt Imp.` inherited unchanged. The cldr-json files are
  inheritance-resolved, so a missing key never means "untranslated"; only comparison with root's
  string finds these. They are rejected locale by locale for now; a general "a string equal to
  root's is not a name in a non-English locale" rule would need `und` vendored. Parked below.
- **Danish calls the US quart and fluid ounce "engelsk"** (English), which to a Danish reader is the
  imperial unit; its imperial ones are "britiske". Both strings are refused in `da`.
- **Finnish files the metric cup under `teekuppi`**, a teacup. Refused.
- **Commonwealth English marks CLDR's dessert spoon as US** (`US dessertspoon`, `US dstspn`); the
  10 ml British `dsp` is hand-authored.
- **Korean short forms** for the imperial and cocktail units are root's strings minus a space
  (`{0}pinch`), so a byte-equality check misses them. Harmless under default-reject.
- One form per width, not a synonym set, so punctuation variants such as `tbs.` and `fl. oz.`
  stay hand-authored.
- **Strange CLDR strings, from an audit of all 35 locales (2026-10-04).** Live in adjudicated
  locales: Polish names the US cup `ćwierćkwarta amerykańska` (*American quarter-quart*) and the
  pint `półkwarta amerykańska`, with the short form `ćwierćkwarty am.` as the **symbol**, and its
  display name carries CLDR's typo `amerykańske` — all currently accepted; Danish accepts the
  English passthrough `ounces`; `en` has `qt-Imp.` beside `pt Imp.` and `gal Imp.`. (The
  imperial strings — Japanese and Danish root-English symbols, Finnish *British quarter-gallon*,
  German `Imp.fl.oz.` — are gone from the metric locales since the imperial keys need a verdict,
  ADR 3.) Candidate fixes: reject the Polish short form and typo, reject Danish `ounces`.
  Waiting on adjudication: Romanian's long `other` patterns are `{0} de X`
  (`20 de litri`, the partitive), so extraction yields 31 aliases like `de litri` that already
  resolve under auto-accept; Arabic and Hebrew dual forms carry no placeholder (`يومان`,
  `שעתיים`), which is correct but means `written_long(2, …)` prints no number; `zh-Hant` (27 of
  32 keys), `zh` (24), `ar` (20), `hi`, `th`, `vi` have no abbreviations, so their symbols are
  full words (`毫升`, `克`) and a symbol decision is due per key; English passthroughs in
  `fr` (`gallons`, `quarts`, `minutes`), `nl` (`inches`), `pt`/`pt-PT` (`pints`), `vi` (`feet`),
  and whole root-English imperial units in `ms`. No locale uses one string for two units.
- **A unit's spaced identifier resolves in every locale** (ADR 4), so `ounce`, `pound` and
  `liter` resolve whatever a locale lists. Listing one as refused is therefore meaningless, and
  `new-locale` drops it rather than let the file say something untrue. Three existed:
  `nl`'s `ounce` and `pound`, and `id`'s `metric cup` — all untranslated English in CLDR that
  happens to match our identifiers. Any test for "this string does not resolve" must use a string
  that is not an identifier, such as the short form `oz`.

## Parked after the 2026-10-04 series

- **20 locales have words nobody has judged**: `ar cs el fr he hi hu id it ms nl pt-PT ro ru th
  tr uk vi zh zh-Hant`; `pl`, `ko` and `pt` only for their spoons and cup. Their SI and marked
  names resolve; their cup, spoon, pint and pound words are listed under "No verdict yet" in the
  comment at the foot of their own `locales/<tag>.toml`, which is the worklist — accepting one
  means adding a `[[unit]]` entry for it. Each needs citations from `research/cooking-units.md`,
  which covers most of these languages already. `hu` is worth doing early: it has region rows
  (dl, dkg) but its words are still refused, so a Hungarian reader gets `25 dag` written and
  cannot yet write `25 dkg` back. Note that review is a separate act: a locale is refused
  entirely until it has a `reviewed` key, however well adjudicated it is.
- **Root-equal strings are accepted when a new locale is seeded, unless the unit needs a
  verdict.** `NEEDS_A_VERDICT` in `xtask/src/seed.rs` covers the units where the problem
  showed (`pt Imp.` in every locale), so the live cases are gone. The general rule — a string
  equal to root's is not a name in a non-English locale — would need `und` fetched alongside
  the locale, and would matter again for any unit that leaves that set, or any future unit
  CLDR leaves untranslated.
- **Dessert spoons and ounces refused pending research.** German `Dessertlöffel`, Spanish
  `cucharada de postre`, Japanese 中さじ, Swedish `dessertsked`, Danish `dessertske` and Finnish
  `jälkiruokalusikka` are refused because the research file gives no value for them (it has 10 ml
  for the UK, NO, RU, PT and NL only). Swedish `uns` and Norwegian `unse` likewise.
- **The Nordic `hg` is uncited.** `locales/{sv,no,da,fi}.toml` name the hectogram on the
  maintainer's statement that it is the everyday retail unit; the research file cites the
  hectogram only as the Italian etto (line 495) and the Dutch ons (line 471). Worth a line of
  research.
- **Still-missing units**: the 500 g `Pfund`-class units other than Colombia's (`Pfund`, `pond`,
  `livre`, `pund`) — the research found none of them written as a recipe amount, so no unit yet;
  the 180 ml 合 (rice only, ingredient-specific); the imperial cup and gill; the 7.5 ml Portuguese
  dessertspoon reading; the Hungarian 2 ml mokkáskanál and Brazilian 2.5 ml colher de café
  (single-source). CLDR 48 also carries `kosaji`, `osaji` and `cup-jp` under a `jpsystem`, so
  小さじ and 大さじ could be vendored once they have units.
- **Unmarked US names outside US customary locales** are now refused everywhere (ADR 3), so
  `es`, `es-419`, `de`, `ja` and the Nordic locales no longer resolve `galones`, `Gallonen`,
  `ガロン`, `gallon` or the symbols `pt`/`qt`/`gal`/`fl oz` to the US units; `es-US` is the home
  of the US units in Spanish. Open: whether any metric locale wants a *marked* US name authored
  as a supplement (`amerikansk gallon`, `US-Gallone`) so translated American recipes can be
  written without the spaced identifier. None has been asked for.
- **Region rows still open**, from
  [`research/best-units-by-region.md`](./research/best-units-by-region.md) after the 17 rows
  landed (`SE NO DK FI CH HU BE FR NZ CA AU JP KR BR MX CO`; ADR 5). Every threshold in those
  rows is inferred from exhibits — no source states one — so they are judgements to revisit, not
  facts. Fractions are per row as well (ADR 6): the system rows have them, the region rows do
  not yet, and whether a region writes `½ dl` and `½ tsk` (SE, FI) or `0,25 ts` (NO) is its own
  line, with the exhibits in §2. Needing a decision first (§7.2): **AT** decagrams (user recipes write `25 dag`, every
  edited Austrian publisher writes grams); **PT** decilitres (Teleculinária and Vaqueiro dl, DGS
  and Pingo Doce ml); **ZA** cups versus the all-millilitre register; **IN** volume (which cup:
  NIN 200, Archana's 240, IDA 250) — only the thin `inch` length row has evidence; **PL**
  szklanka for flour and sugar but `650 ml` for water; **TW** 240 ml cup versus cc; **CO** volume
  (Resolución 810 allows 200 or 240 ml, Nestlé exhibits 250); **IS** as SE on one source.
  **Canada** splits by *language*, not region — en-CA prints °F and imperial first, fr-CA metric
  and °C — which a region row cannot express. `-u-mu-fahrenhe` (ADR 5 amendment, 2026-10-08)
  covers the °F half; the imperial-first half is a secondary display, below, not a row. No dataset
  or library keys units by language, and CLDR's spec lists language keys as a future extension of
  `unitPreference`. **Secondary displays** are a separate
  feature, not best-unit rows: UK/IE imperial in brackets with its rounding table (25 g = 1 oz,
  450 g = 1 lb, 568 ml = 1 pint, 150 ml = ¼ pint), gas mark and thermostat beside °C (point
  tables, cooking-units.md lines 109–125), fan-oven second figures, Brazilian and Argentine band
  names. `-u-ms-uksystem` could take `imperial-pint ≥ 0.5` and a compound `1 lb 2 oz` (§7.4).
  Australia and New Zealand print odd amounts in ml rather than fractional cups (`100ml
  buttermilk`), which a threshold table cannot express; it needs a "clean fraction, else ml" rule.
- **Package-level Unicode notices.** The npm WASM package and the Swift/Kotlin bindings will embed
  CLDR strings once `convert_v2` is reachable from them; each then needs the notice in its own
  README or a copy of `locales/UNICODE-LICENSE`.
- **Finnish millimetre has no symbol.** `fi` gives `mm` to the spice measure, so the millimetre is
  written as `millimeter` and resolves only from `millimetri`/`millimetriä`/`millimetrit`. A
  hand-authored symbol can be added if anyone needs one; pan sizes are prose.
- **`en-150`** (European English: en-DE, en-SE, en-NL and 21 more) is its own CLDR parent under
  `en-001`. It is not vendored; those tags reach `en-001` today, which is right until European
  English needs different names.
- **Inflected short forms.** CLDR's short `other` form is the symbol, and in inflecting languages
  that is often a plural word: Polish's teaspoon was `łyżeczki` until it was corrected by hand to
  `łyż.` in `locales/pl.toml`. The same check is due for every other inflecting locale as it is
  reviewed
  (Czech, Russian, Ukrainian, Lithuanian-class), key by key; `written_long` is always grammatical.
- **Most hand-authored names have no long patterns yet.** A `[[unit]]` entry may carry
  `long = { one = …, other = … }` (the Finnish spice measure does: `1 maustemitta`,
  `2 maustemittaa`); `szklanka`,
  `dag`, `hg`, `etto`, `ons`, カップ and the English marked symbols still write as number plus
  symbol in long form. Mechanical to add, one unit at a time, with the plural forms of each.
- **Number formatting in long forms** is Rust's `{}` for `f64`: no thousands separators, no
  locale decimal comma, and no trailing zeros — so the plural operand `v` (visible fraction
  digits) is always the minimal one. Fine for recipes; wrong for `1,5 Teelöffel` in German text.
- **Bare `C` and `F` resolve in every locale** since 6d67c28, which brought the CLDR locales into
  line with legacy — `units.toml` had always given them. So do `℃`, `℉`, `ºC` and `ºF`
  (`UNIVERSAL_ALIASES`): the degree symbols in another encoding or with the ordinal indicator, the
  same symbol rather than a language's word. **This makes the case decision below harder, not
  easier**: `c` is a cup in `en`, `en-001`, `en-AU` and `en-GB`, so folding case would turn
  `180 C` into 180 cups — a wrong unit rather than an error, off by a factor no tolerance catches.
  `Locale::with_plurals` refuses one string naming two units, so the collision cannot be
  introduced by accident, and `resolve_matches_symbols_and_aliases_exactly` pins both halves.
- **Unit names are case-sensitive, and that is confusing.** `Locale::resolve` is an exact lookup,
  as the legacy `find_unit` is, so `2 Cups` and `1 teelöffel` fail while `2 cups` and
  `1 Teelöffel` resolve; an author has to remember which units a locale capitalises (German nouns,
  `TL`/`EL` against Finnish `tl`). To investigate. One possible solution: match words and
  multi-word names case-insensitively while keeping short symbols exact, since the collisions all
  live among the symbols — `c` is the US cup and `C` Celsius, `F` Fahrenheit, `US c`, `Imp.fl.oz.`
  — and the words never collide by case. The folding would happen at lookup, after the exact match
  fails, with `Locale::new` rejecting two names that differ only by case and mean different units.
  Costs to weigh: every lenient step widens what the inline-quantities scanner matches in prose,
  and a rule such as "longer than N characters" or "contains a space" needs a defensible cut-off
  (Unicode case folding, not ASCII — `İ`, `ß`, Greek final sigma).
- **Unaccented input for accented names.** Same argument as case: `galon`, `Flussigunze`,
  `kryddmatt`, `lyzeczka` are what a phone or a non-native keyboard produces, and each maps to one
  unit. But it is not one rule. Stripping combining marks (NFD) covers `á é ñ ü ö å` and misses
  the letters that have no decomposition — Polish `ł`, Danish and Norwegian `ø æ`, German `ß`,
  Vietnamese `đ` — so three adjudicated locales need transliteration, and transliteration is per
  language and many-to-one (`ü` → `u` or `ue`, `å` → `a` or `aa`, `ø` → `o` or `oe`, `ß` → `ss`).
  Proposed shape: **derived aliases generated at build time** from a per-locale folding table in
  `locales/<tag>.toml` (`fold = ["ü→u", "ü→ue", "ß→ss"]`), applied to accepted words and
  multi-word names and never to symbols; lookup stays exact, every derived spelling shows in
  `docs/units.md` and the snapshot for review, and `Locale::new` rejects collisions statically.
  Not a lookup-time fold with a generic transliteration crate: invisible, guesses the same
  mapping for every language, and accepts spellings nobody adjudicated. Design together with the
  case item as one "lenient input spellings" change; the inline-quantities cost applies to both.

## Closed by the seeding conversion (2026-10-06)

Kept briefly so that anyone holding the old mental model can see what happened; delete once the
dust settles. [ADR 3](./adr/locale-aware-units.md#3-cldr-is-a-source-of-unit-names-only-seeded-once-into-a-file-per-locale) is the decision.

- The four overlay layers, the 331-line merge in `build.rs`, the `needs_verdict` /
  `accept` / `reject` / `accept_all` / `Unadjudicated` vocabulary and the ten tests policing the
  seam are all gone. A locale is `locales/<tag>.toml` and nothing is merged over it.
- **The "Unadjudicated" table in `docs/units.md` no longer exists.** Those strings are
  listed in the comment at the foot of the locale they concern, beside the reason.
- **`docs/units.md` lost its Locales section** as well: a seeded file is better documentation than
  a generated table of the same names. The file is now 115 lines of genuinely derived content —
  the 51 units with factors, and the preference rows.
- **`cargo package` no longer needs the CLDR JSON**, which also retires the worry about shipping
  736 KB of it. The snapshots are excluded instead.

## Blocked on a decision elsewhere

- **The precision tiers and the exactness marker.**
  [`research/measurement-precision.md`](./research/measurement-precision.md) researched ADR 1
  items 4 and 5 and found **four** tiers rather than one tolerance: a quantity written in a
  volumetric container (`cup`, `tbsp`, `tsp`) is self-rounding and may go to a usable fraction; an
  ordinary quantity gets about **1% relative**; an additive dosed as a fraction of batch mass gets
  **0.5%** or tighter; a quantity the author marks **exact** is not rounded at all. The 0.5% tier is
  measurement-grounded, not a guess — a 0.01 g scale (€29 at ordinary kitchen retail) gives 0.2% at
  the 2.5 g a cook actually weighs, and a €20 milligram scale gives 0.02%, so at these masses *our
  rounding is the dominant error term*, not the scale. The exempt tier exists because curing salt is
  a one-sided *legal ceiling* (156.25 ppm nitrite in US comminuted products, 80 mg/kg as nitrite ion
  in the EU since 2025-10-09), where rounding `5.66 g` up to `5.7 g` is a compliance failure — a
  failure of direction, which no relative tolerance addresses.

  Six things need deciding before any of this can be built. **(a)** Whether the marker sits on the
  ingredient (`@*cure{…}`; `*` is the only unused single-char token, verified) or on the quantity
  (`@cure{*5.66%g}`); only the quantity form reaches timers and inline temperatures, and metrology
  marks exactness on the number, not the substance. **(b)** Whether the budget should instead be
  **half a unit in the last digit the author wrote** — which derives all four tiers from the text
  (`1 cup` → 50%, `250 ml` → 0.2%, `5.66 g` → 0.088%), already protects the nitrite case, and
  removes the fact that *the parser cannot tell an additive from a seasoning*; its price is making
  `Number` carry the written digit count, which `Number::Regular(f64)` does not. **(c)** Whether the
  self-rounding tier is wanted deliberately — it is today's behaviour by accident, via `units.toml`'s
  5% imperial fraction accuracy. **(d)** Temperature *points* must not get a relative tolerance at
  all; an absolute band (0.1 K) is the defensible replacement. **(e)** Whether an exact quantity
  scales (it must: it is a ratio to batch mass) and then refuses to round, or refuses both — scaling
  is the cook's own precision technique, the way a 0.2 g dose becomes a weighable 2.5 g, so rounding
  a scaled exact quantity defeats the step taken to get precision. **(f)** Whether an exact quantity
  refuses conversion into a container that cannot express it, which "explicit over implicit" would
  favour.

  Independent of all five: **the stored value is already never rounded.** `Number::Fraction`
  keeps its residual (ADR 6), so rounding is a display matter, and item 5's complaint reduces to
  `main`'s `float_tolerance` borrowing the display budget for equality. The marker only has to
  reach the display layer.

- **How the `legacy` locale is addressed.** Decided in
  [ADR 7](./adr/locale-aware-units.md#7-a-recipe-reads-its-units-through-its-own-locale): a distinct naming, not a
  tag; untagged recipes read through it and get a parse warning. The ADR also settles the rest of
  the converter swap.

- **`>> locale:` already exists and its syntax is incompatible.** `StdKey::Locale` is a standard
  metadata key, and `value_as_locale` in `src/metadata.rs` accepts only `xx` or `xx_YY`:
  underscore-separated, exactly two ASCII letters each. So `en_GB` parses but **`en-GB` does not**,
  and neither does `en-001` or any `-u-ms-metric` extension — all of which this work has since
  settled on. Its own documentation concedes it "can return non existent/assigned locales" and that
  `en_gb` is accepted though it should be `en_GB`. `Metadata::locale()` returns
  `(&str, Option<&str>)`, so widening it to BCP 47 changes a public accessor's type and accepted
  syntax: **breaking**, needing a CHANGELOG entry and its own ADR. The alternative, a second key
  such as `units-locale`, avoids the break but leaves two locale keys meaning different things.

  Related: warning on every untagged recipe changes parse output for all existing recipes, which
  will touch `tests/canonical.rs` and the generated spec cases. Whether the warning is wanted at
  all, or whether silently defaulting to `legacy` suffices until the converter swap, is undecided.
