# Change Log

## Unreleased - ReleaseDate
- Semantic equality: a `SemanticEq` trait whose `equals` takes a `Converter`, implemented for `Recipe`, `Ingredient`, `Cookware`, `Timer`, `Quantity`, `GroupedQuantity`, `Metadata` and `Servings`, plus `unordered_equals` for collections whose order carries no meaning. The relation is not transitive, because two values are equal within the error their units tolerate
- Scaling rounds a servings count to one decimal, instead of to a whole number, so `servings: 4` scaled by a third is `1.3` rather than `1`; `scale_to_servings` writes its target the same way. `Servings` owns both that precision and the finer one its comparison uses
- `Ingredient::resolved_reference` resolves a recipe reference against the file the ingredient was parsed from, and `equals` compares references resolved, so two recipes that reach the same target from different directories compare equal
- Quantity subtraction: `Quantity::try_sub` and a `TrySub` trait mirroring `TryAdd`, with `GroupedQuantity::try_saturating_sub` and `try_saturating_sub_group`
- Quantities are compared in the metric base unit within a single absolute tolerance, which absorbs the error of converting between units but not a difference a recipe actually writes down: `3 tsp` equals `1 tbsp`, while `2.3` does not equal `2.301` and `240 ml` does not equal `1 cup`. Infinity equals only itself and `NaN` equals nothing. A servings count is compared rounded to two decimals
- Scaling a recipe a second time scales the ingredients again. Converting a quantity, which scaling does to fit the unit, used to drop its scalable flag, so after one `scale` the ingredients stayed put while the servings kept moving
- `StdKey` lookups (`Metadata::get`, `get_mut` and the readers built on them) resolve the key's aliases, so `serves:` and `yields:` are read and scaled like `servings:` and `yield:`. Only the canonical spelling used to be found, so a recipe written with `serves:` had no servings as far as `Metadata::servings()` and scaling were concerned
- `Metadata::equals` sees every spelling of a standard key, in any order, so a recipe with both `servings: 4` and `serves: 99` is not equal to one with only `servings: 4`. A value it cannot parse is compared verbatim
- `GroupedQuantity::try_saturating_sub` empties the group when the amount subtracted equals what it holds, even where the unit ratios keep the subtraction from landing on a clean zero. A text quantity, having no amount, takes its equal out of the group. `try_saturating_sub_group` subtracts the other group's unmergeable quantities once
- (breaking) `ParseOptions::recipe_ref_check` receives a `RecipeRefTarget` instead of a `&str` holding only the referenced file's name. `Resolved` carries the reference resolved against the recipe being parsed, so `../spices/x.cook` written in `./pickles/sour.cook` arrives as `./spices/x.cook`; `Unresolved` says the recipe was parsed with no `path`, so a checker need not report a missing recipe when it was never told where to look. The checker is only called for a reference: it used to run for every `@@` ingredient, bare names included, which are not references and never were parsed as one
- (breaking) `CooklangParser::parse_with_options` takes a third argument, `path: Option<RecipeReference>`, with `parse_with_path` as a shorthand. `Recipe` gains a `path` field and `Ingredient` a `recipe_path` field recording the file they were parsed from
- (breaking) Servings are `f64` rather than `u32`: `Servings::Number(f64)`, `Servings::as_number() -> Option<f64>` and `scale_to_servings(f64)`. `servings: 4.5` validates now, `Servings` no longer derives `Eq`, and scaling writes the count back as a float, so `servings: 4` scaled by 2 serializes as `8.0`
- (breaking) `servings` is validated as a positive, finite number, so `0`, `-4`, `.nan` and `.inf` are rejected: reported as invalid metadata and read as `None` by `Metadata::servings()`. `0` used to be accepted, and scaling by it produced infinite amounts
- (breaking) `yield` is validated the same way: `yield: -3%g`, `yield: 0` and `yield: .nan` stay regular metadata entries with a warning, and `Metadata::yield_quantity()` / `CooklangValueExt::as_yield()` read them as `None`. `scale_to_yield` used to scale by a negative yield
- (breaking) `scale_to_servings` and `scale_to_yield` refuse a target that is not a positive, finite amount, because the target is written back as the recipe's own count or yield. `ScaleError::InvalidServings` and `InvalidYield` now cover the target as well as the metadata, and their messages say so
- `scale_to_yield` takes a target in any unit the yield's converts to, so a `500%g` yield scales to `2%kg` by 4 and is written back as `2%kg`; `UnitMismatch` is kept for units that cannot be converted. It used to require the exact same unit
- (breaking) `yield`/`yields` is its own `StdKey::Yield` holding a `value%unit` quantity instead of resolving to `StdKey::Servings`, so anything reading servings from `yield` no longer finds it. `StdKey` is not `#[non_exhaustive]`, so an exhaustive match needs the new variant
- (breaking) `RecipeReference` wraps a `RelativePathBuf` instead of `{ components, name }`: those public fields and `path()` are gone, construction goes through `FromStr` (`"./b/stew".parse()`) and is fallible, with a `RecipeReferenceParseError`, because a reference must have a file name, `PartialEq` compares the normalized path so `./a/../b/stew` equals `./b/stew`, and serde reads and writes the path string `"./b/stew"` rather than `{"name": "stew", "components": [".", "b"]}`, which breaks stored JSON and changes the `ts` type
- (breaking) `QuantityAddError` is renamed `QuantityOpError`, because it is no longer only about adding; the old name stays as a deprecated alias. Adds `GroupedQuantityOpError`
- (breaking) `relative-path` is a new unconditional dependency, so it is built even with `default-features = false`
- (breaking) bindings: `RecipeReference` is `{ path, name }` instead of `{ name, components }`. `path` is the normalized relative path, e.g. `./pasta/spaghetti`, which is what the core type and the `ts` type carry too. The `components` vector and the `path(separator)` helper that rebuilt a path from it are gone

## 0.19.0 - 2026-09-30
- (breaking) Optional ingredients and cookware (`@?name`, `#?name`) are core syntax (spec proposal 0018): the `?` marker is parsed without `Extensions::COMPONENT_MODIFIERS`. The other modifiers still need the extension
- In `duplicate: ref` mode, optional and required components with the same name are no longer implicitly linked, instead of reporting a modifier conflict
- (breaking) Shopping lists: `RecipeItem` and `IngredientItem` gain an `optional` field for selection lines (`? name{quantity}`, `? ./path{n}`), parsed under a recipe reference and written before nested references. A selection line at the top level is a `TopLevelSelection` error
- (breaking) bindings: `Ingredient`, `Cookware` and shopping list items gain an `optional` field
- typescript: `ingredient_is_optional` and `cookware_is_optional` helpers; the HTML renderer marks optional items. Component `modifiers` are now kept when serializing to JS, so the `*_should_be_listed` helpers also work
- (breaking) `parser::Block` gains a `FrontMatter(Text)` variant; `build_ast` no longer panics (`todo!()`) on recipes with YAML front matter
- Playground redesign (#106)

## 0.17.3
- Fixes references components by @mawo66 in https://github.com/cooklang/cooklang-rs/pull/81

## 0.17.0
- Softens YAML frontmatter parsing by @dubadub in https://github.com/cooklang/cooklang-rs/pull/57
- Adds pantry config

## 0.16.6 - 2025/08/11

- (breaking) Remove generics from `Recipe`. Now recipes are scalable multiple times.
- (breaking) Remove `Recipe::default_scale`.
- (breaking) Scaling is now infallible, text values are ignored. Removed
  `ScaleOutcome`, if this functionality was needed, it can easily
  be replaced by checking if the quatity is text or not scalable.
- Cookware now can have full quantities with units, not just values.
- Servings value in metadata now isn't a vector

## 0.16.3 - 2025/07/28

- Fixes scaling behavior in metadata servings by @dubadub in https://github.com/cooklang/cooklang-rs/pull/43
- Adds lenient aisle parsing to allow more flexible formatting by @dubadub in https://github.com/cooklang/cooklang-rs/pull/44
- Softens canonical parser to reduce strictness on edge cases by @dubadub in https://github.com/cooklang/cooklang-rs/pull/45

## 0.16.1 - 2025/05/27

- Adds references support into IngredientList by @dubadub in https://github.com/cooklang/cooklang-rs/pull/36

## 0.16.0 - 2025/03/27

- Correct spelling `ouput` -> `output` and `bunlded` -> `bundled` by @melusc in https://github.com/cooklang/cooklang-rs/pull/31
- Don't hide servings for input by default in playground by @melusc in https://github.com/cooklang/cooklang-rs/pull/32
- Enable scaling according to new spec changes by @dubadub in https://github.com/cooklang/cooklang-rs/pull/30
- Allow referencing other recipes by @dubadub in https://github.com/cooklang/cooklang-rs/pull/34
- (breaking) Use floating value for scaling factor instead of base and target
  servings by @dubadub in https://github.com/cooklang/cooklang-rs/pull/35

## 0.15.0 - 2025/01/14

- Add support in `cooklang::metadata` for [canonical
  metadata](https://cooklang.org/docs/spec/#canonical-metadata), making it
  easier to query these keys and expected values.
- Add warnings for missused canonical metadata keys.
- Improve custom checks for metadata keys. Now they can choose to skip the
  included checks too.
- Fix ingredients aliases from aisle configuration not being merged in
  `IngredientList`. (#24 @kaylee-kiako)
- Remove many dependencies, binaries should be smaller.
- (breaking) Change `Quantity` API.
  - `value` is not a getter.
  - `unit` returns the unit text
  - `unit_info` (new method) returns the `Unit` value with a runtime lookup.
- (breaking) Rename `TEMPERATURE` extension with `INLINE_QUANTITIES`. Now all
  inline quantities are found, not only temperatures. You may need to update
  your application to handle this.

## 0.14.0 - 2024/12/11

- Add YAML frontmatter for metadata. Deprecate old style metadata keys with the
  `>>` syntax. This also comes with changes in the `cooklang::metadata` module.
- Add deprecation and how to fix warnings when using old style metadata.
- Remove `MULTILINE_STEPS`, `COMPONENT_NOTE`, `SECTIONS` and `TEXT_STEPS`
  **extensions** as they are now part of the cooklang specification and are
  always enabled.

## 0.13.3 - 2024/08/12
- Replace `ariadne` dependency with `codesnake`. Because of this, errors may
  have some minor differences.

## 0.13.2 - 2024/04/07
- Fixed name and url parsing in `author` and `source` special metadata keys.
  Before, the name was too restrictive and some names could be miss interpreted
  as URLs. (thanks to @Someone0nEarth)

## 0.13.1
### Fixed
- Panic when parsing just metadata.

## 0.13.0
### Features
- The parser now has the option to check every metadata entry with a custom
  function. See `ParseOptions`.

### Breaking
- Replace recipe ref checks API with `ParseOptions`. This now also holds the
  metadata validator.
- Tags are no longer check. Use a custom entry validator if you need it.

## 0.12.0 - 2024-01-13
### Features
- Special metadata keys are now an extension.
- Improve `Metadata` memory layout and interface.
- Emoji can now also be a shortcode like `:taco:`.

### Breaking
- (De)Serializing `ScaleOutcome` was not camel case, so (de)serialization has changed
  from previous versions.
- (De)Serializing format change of `Metadata` special values. Now all special
  key values whose parsed values have some different representation are under
  the `special` field.
- Removed all fields except `map` from `Metadata`, now they are methods.

## 0.11.1 - 2023-12-28
### Fixed
- Add missing auto traits to `SourceReport` and all of it's dependent structs.
  Notably, it was missing `Send` and `Sync`, which were implemented in
  previous releases.

## 0.11.0 - 2023-12-26
### Breaking changes
- Remove `PassResult::take_output`.
- `Metadata::map_filtered` now returns an iterator instead of a copy of the map.

### Fixed
- Implement `Clone` for `PassResult`.

## 0.10.0 - 2023-12-17
### Breaking changes
- Reworked intermediate references. Index is gone, now you reference the step or
  section number directly. Text steps can't be referenced now.
- Rename `INTERMEDIATE_INGREDIENTS` extension to `INTERMEDIATE_PREPARATIONS`.
- Sections now holds content: steps and text blocks. This makes a clear
  distinction between the old regular steps and text steps which have been
  removed.
- Remove name from `Recipe`. The name in cooklang is external to the recipe and
  up to the library user to handle it.
- Remove `analysis::RecipeContent`. Now `analysis::parse_events` returns a
  `ScalableRecipe` directly.
- Change the return type of the recipe ref checker.
- Reworked error model.
- Removed `Ingredient::total_quantity`.
- Change `Cookware::group_amounts` return type.
- Several changes in UnitsFile:
  - System is no longer set when declaring a unit with an unspecified system as best of a specific system.
  - `extend.names`, `extend.aliases` and `extend.symbols` are now combined in `extend.units`.
- Removed `UnitCount` and `Converter::unit_count_detailed`.
- Removed `hide_warnings` arg from `SourceReport` `write`, `print` and `eprint` methods.
  Use `SourceReport::zip` or `SourceReport::remove_warnings`.

### Features
- New warning for bad single word names. It could be confusing not getting any
  result because of a unsoported symbol there.
- Improve redundant modifiers warnings.
- Recipe not found warning is now customizable from the result of the recipe ref
  checker.
- Unknown special metadata keys are now added to the metadata.
- Advanced units removal of `%` now supports range values too.
- New error for text value in a timer with the advanced units extension.
- Special metadata keys for time, now use the configured time units. When no
  units are loaded, fallback unit times are used just for this.
- Bundled units now includes `secs` and `mins` as aliases to seconds and
  minutes.
- New warning for overriding special recipe total time with composed time and
  vice versa.
- Added `ScaledRecipe::group_cookware`.
- Rework `GroupedQuantity` API and add `GroupedValue`.
- Ignored ingredients in text mode are now added as text.
- Several features in UnitsFile to make it more intuitive:
  - The best unit of a system can now be from any system. It's up to the user if
    they want to mix them.
  - New `extend.units`, which allows to edit the conversions.
  - Improve and actually make usable the fractions configuration. Now with an
    `all` and `quantity.<physical_quantity>` options.
- An empty unit after the separator (%) is now a warning and it counts as there
  is no unit.
- Added `SourceReport::remove_warnings`.

### Fixed
- Text steps were ignored in `components` mode.
- Scale text value error was firing for all errors marked with `*`.
- Even though number values for quantities were decimal, a big integer would
  fail to parse. That's no more the case. If it's too big, it will only fail in
  a fraction.
- Incorrect behaviour with single word components that started with a decimal
  number.

## 0.9.0 - 2023-10-07
### Features
- Better support for fractions in the parser.
- `Quantity` `convert`/`fit` now tries to use a fractional value when needed.

### Changes
- Use US customary units for imperial units in the bundled `units.toml` file.
- Expose more `Converter` methods.

### Breaking changes
- Several model changes from struct enums to tuple enums and renames.

## 0.8.0 - 2023-09-26
### Features
- New warnings for metadata and sections blocks that failed to parse and are
  treated as text.
### Breaking changes
- The `servings` metadata value now rejects when a duplicate amount is given
  ```
  >> servings: 14 | 14   -- this rejects and raise a warning
  ```
- `CooklangError`, `CooklangWarning`, `ParserError`, `ParserWarning`,
  `AnalysisError`, `AnalysisWarning`, `MetadataError` and `Metadata` are now
  `non_exhaustive`.
### Fixed
- `Metadata::map_filtered` was filtering `slug`, an old special key.

## 0.7.1 - 2023-08-28
### Fixed
- Only the first temperature in a parser `Text` event was being parsed
