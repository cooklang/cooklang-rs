# Unit conversion uses a closed set of units defined in Rust

Unit conversion used to be configured from `units.toml`, a file that also carried names, aliases, translations, fraction display, best-unit selection and unit systems. We are replacing its conversion core with `convert::units`: one Rust enum per physical quantity, holding the exact legal definitions, with no configuration. Conversion is the part that must be correct and that users have no reason to change, and keeping it out of the config makes the config small enough to expose to users.

Adding a unit is now a code change. Users can still choose what a recipe's text means, because aliases map words to unit identifiers. A locale can map `cup` to a Japanese cup, but only once that unit exists in the enum. Units that a units file adds at runtime will not convert once the converter is switched over to `convert::units`.

## Consequences

- Ambiguous units are named for what they are (`UsCup`, `UsTeaspoon`), and the alias layer decides what `cup` or `tsp` means.
- `convert::units` has no notion of a unit system. Metric versus imperial is a display preference handled above conversion, and some units, such as a Japanese cup, are neither.
- Temperature conversion handles temperature points only.

## Follow-up work

> Written before the locale-seeding series. Items 1–3 reference ADRs 0002 and 0003, both since
> superseded — by [0007](./0007-unit-identifiers-are-spaced-words.md) and
> [0006](./0006-locales-are-seeded-not-vendored.md). The list itself is current.

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

## Differences from the legacy converter

Places where `convert_v2` deliberately behaves differently from `src/convert_legacy/`, and how the comparison tests allow for them:

- **`scalable` is kept.** `ConverterV2::convert` keeps a quantity's `scalable` flag. The legacy `Quantity::convert` rebuilds the quantity with `Quantity::new`, which resets it to `false`. The legacy behaviour looks accidental, and copying it would bake it in. The comparison ignores the flag, and `convert_preserves_scalable` pins the new behaviour.
- **Exact definitions.** Conversion uses the legal definitions rather than `units.toml`'s rounded ratios (largest difference about 1.2e-7 relative, for the teaspoon), so comparisons use a tolerance.
- **A US fluid ounce row, and metric spoons.** `units.toml`'s imperial `best` list for volume is cup, tablespoon, teaspoon, so bundled writes 4 fl oz of milk as `8 tbsp`; its metric list is litre and millilitre, so it writes two tablespoons of oil as `30 ml`. The v2 US customary rows add the fluid ounce between the cup and the tablespoon, and the metric rows add the metric tablespoon and teaspoon between 50 ml and a quarter teaspoon (ADR 0005). The US cup row also starts at a quarter (2026-10-08): American recipes write ¼, ⅓ and ½ cup, which bundled reached only through its fractions config capping tablespoons at four, so between a quarter and a whole cup bundled wrote tablespoons. `the_rows_reproduce_the_bundled_best_lists_except_the_recorded_divergences` pins every other choice to bundled's and asserts that wherever v2 chooses one of those units and bundled disagrees, bundled chose the tablespoon or millilitres respectively.
