# Change Log

## Unreleased - ReleaseDate
### Breaking changes
- Reworked intermediate references. Index is gone, now you reference the step
  or section number directly. Text steps can't be referenced now.
- Rename `INTERMEDIATE_INGREDIENTS` extension to `INTERMEDIATE_PREPARATIONS`.
- Sections now holds content: steps and text blocks. This makes a clear
  distinction between the old regular steps and text steps which have been
  removed.
- Remove name from `Recipe`. The name in cooklang is external to the recipe and
  up to the library user to handle it.
- Remove `analysis::RecipeContent`. Now `analysis::parse_events` returns a
  `ScalableRecipe` directly.
- Change the return type of the recipe ref checker.

### Features
- New warning for bad single word names. It could be confusing not getting
  any result because of a unsoported symbol there.
- Improve redundant modifiers warnings.
- Recipe not found warning is now customizable from the result of the recipe
  ref checker.
- Unknown special metadata keys are now added to the metadata.

### Fixed
- Text steps were ignored in `components` mode.

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
