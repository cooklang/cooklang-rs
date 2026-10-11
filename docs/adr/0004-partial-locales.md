# Locales are partial, and the identifier's spaced form is universal input

A locale names only the units it has a safe name for. CLDR lacks some of our units, and adjudication
makes a locale decline others (German has no name for the 236.588 ml US cup). Previously every
locale was built as an overlay on the legacy locale so that each unit had a symbol; that overlay
made legacy's English `c` resolve to the US cup in German, Spanish and Japanese recipes — a string
nobody adjudicated. Now a locale's symbol is optional, and a unit the locale does not name is
written as its identifier with the hyphens replaced by spaces: `2 imperial pint`, `1 nordic spice
measure`. That spaced form resolves as input in **every** locale, so what we write we can always
read back, and the kebab-case identifier itself never appears in recipe text.

## Consequences

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

## Alternatives considered

| Option | Reason rejected |
|---|---|
| Legacy's symbol for a non-bundled unit is its identifier, by rule | Keeps the overlay and with it the leak above; makes legacy the carrier of a rule that is not about legacy. |
| Legacy grows hand-authored English names for new units | Legacy stops being "exactly `units.toml`", and those English names leak into every locale's fallback. |
| Converting to an unnamed unit is an error | Output would depend on names after the unit was chosen without them; a preference row could select a unit the locale then refuses to write. |
| Derive the eight SI symbols from prefix + base | Correct in principle (the prefix symbols are international) but it spends `kl`, `dag`, `cg` in thirty locales where nobody writes them. |
