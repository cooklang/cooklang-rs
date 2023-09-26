# Change Log

## Unreleased - ReleaseDate
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
