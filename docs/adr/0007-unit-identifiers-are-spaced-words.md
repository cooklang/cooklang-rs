# Unit identifiers are lowercase words separated by spaces

Supersedes [ADR 0002](./0002-unit-identifiers.md), and keeps the two decisions in it that carry the
safety: the variant is **always** marked, so there is no bare `cup`; and the variant comes **first**,
so `us cup` rather than `cup us`. Only the separator changes, from `-` to a single space:
`us cup`, `us fluid ounce`, `metric cup`, `imperial pint`, `japanese cup`.

There is now **one** form. `Unit::spaced_id` is gone, along with the `id().replace('-', " ")` that
produced it.

## Why

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

## What it buys

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

## What it costs

- **ADR 0002 is superseded**, though nothing was published: `convert_v2` is `pub(crate)`, so no
  identifier has ever been in the public API, and the spaced form already resolved as input, so
  this breaks no recipe.
- **The whitespace guard has to exist**, and someone must not delete it.
- **Composition headroom is gone.** If a `grams per liter` class of unit ever arrives, the
  separator inside it will be ambiguous with the separator between words. ADR 0002 reserved `-` for
  that; this spends it. Judged worth it: the units in front of us do not compose, and designing for
  the ones we are imagining is how the overlay layers happened.

## Consequences

- `serialize_all = "snake_case"` stays the strum default, and all 20 multi-word variants keep an
  explicit `#[strum(serialize = "…")]` — no case style writes lowercase words separated by spaces
  either, so the attributes do not go away, they only change content. A multi-word variant added
  without one still yields `japanese_cup`, which the test rejects.
- `build.rs`'s `variant()` splits on `' '` rather than `'-'` to derive `MetricTablespoon` from
  `metric tablespoon`.
- A test helper parsing `"{value} {id}"` must use `split_once(' ')`, not `split(' ')`.
- The mapping to CLDR and UCUM is unaffected: it was already a reversal rather than a copy
  (`volume-cup-metric`, `[cup_m]`), and `cldr_id` is an explicit `match`.

## Evidence

The rename is **behaviour-preserving, and this was checked rather than assumed**. Resolution never
depended on the hyphen: the locale index only ever contained the spaced form, so `resolve("us cup")`
worked before and after and `resolve("us-cup")` worked in neither.

Applying the 20 substitutions mechanically to the 40 snapshot files and to `docs/units.md` captured
before the change reproduces the committed ones **exactly** — zero lines differ beyond the
relabelling. 899 replacements across 55 files.
