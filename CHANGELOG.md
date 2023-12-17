# Change Log

## Unreleased - ReleaseDate

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
