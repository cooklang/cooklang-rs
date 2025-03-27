# Change Log

## Unreleased - ReleaseDate

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
