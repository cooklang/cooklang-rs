# The old converter is replaced in one step

The old converter, built from `units.toml` and user-supplied unit files, is replaced by the
locale-aware converter in a single pull request. During review both pipelines exist and a
differential test proves the new one reproduces the old one; once the approach is approved, the
old converter, its unit files and the differential test are deleted before merge, and nothing is
ever released with both. Decided 2026-10-11. Vocabulary is in `GLOSSARY.md`.

## What a recipe reads its units through

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

## What the converter is

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

## How the swap is reviewed

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

## Considered and rejected

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
