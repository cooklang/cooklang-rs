# `cooklang-ingredient-parser`: one typed ingredient line to a name and an amount

**Date:** 2026-11-27
**Status:** Approved in conversation
**First user:** CookCLI's shopping list add field (`CookCLI/docs/superpowers/specs/2026-11-27-shopping-list-manual-items-design.md`)
**Origin:** the iOS recogniser (`mobile-app-ios`, `Packages/ShoppingList/Sources/ShoppingList/Core/IngredientLineRecognizer.swift`, `ManualItemUnits.swift`, `ManualItemQuantity.swift`; spec `docs/superpowers/specs/2026-11-26-shopping-list-manual-items-design.md`)

## Problem

People type or paste ingredients as plain text: "Milk 2 l", "2l milk", "Parsley, 2 bunches", or a whole list copied from a message. Every Cooklang app that takes such text has to turn each line into a name and an amount that a `.shopping-list` or a recipe can hold. The iOS app has done this in Swift since 2026-11-26. CookCLI needs it now, Android and the editor will need it later, and so will recipe import. With one implementation, every app reads the same line the same way and writes the same stored quantity, which matters because they share synced files and merge lines by comparing stored text.

The crate is called an ingredient parser, not a shopping item parser: the same API should later parse a recipe's ingredient list, and may later use an ML model behind the same functions.

## Crate

- Directory `ingredient-parser/` in the `cooklang-rs` workspace, next to `bindings` and `typescript`. Package `cooklang-ingredient-parser`, versioned with `cooklang`.
- Depends on `cooklang` (`default-features = false`, `features = ["aisle"]`) for `AisleConf`, and nothing else beyond `serde` and `thiserror`, if needed.
- No `unsafe`, no I/O, no global state.

## API

```rust
pub fn parse_ingredient(line: &str, aisle: Option<&AisleConf>) -> ParsedIngredient;
pub fn parse_ingredient_list(text: &str, aisle: Option<&AisleConf>) -> Vec<ParsedIngredient>;

pub struct ParsedIngredient {
    /// The name to write: cleaned (see "Name cleaning"), and in the aisle
    /// configuration's spelling when `aisle_match` is set. Never empty for a
    /// line that had any text; `parse_ingredient` of a blank line gives "".
    pub name: String,
    /// `None` when the line has no amount.
    pub amount: Option<Amount>,
    pub aisle_match: Option<AisleMatch>,
    /// How this result was produced. `Rules` for now; a later model-based
    /// parser adds variants without breaking callers.
    pub source: ParseSource,
}

pub struct Amount {
    pub value: f64,            // > 0, finite
    pub unit: Option<String>,  // None = a count
}

pub struct AisleMatch {
    /// The aisle configuration's spelling of the matched name or alias.
    pub name: String,
    /// The common name (first name of the line), which shopping lists show.
    pub common_name: String,
    /// The category it is listed under.
    pub category: String,
}

#[non_exhaustive]
pub enum ParseSource { Rules }

impl Amount {
    /// Cooklang quantity syntax: "2%l", "3", "2%bunches".
    pub fn to_quantity(&self) -> String;
    /// "2 l", "x3" for a count.
    pub fn chip_text(&self) -> String;
    /// "2 l", "3" for a count.
    pub fn display(&self) -> String;
    /// Reads a stored quantity back. `None` for text quantities ("a pinch",
    /// "1-2%cup") and for a comma inside the number ("1,000%g").
    pub fn from_quantity(quantity: &str) -> Option<Amount>;
}

/// Sum two stored quantities with the same unit (compared exactly), or
/// `None` when they cannot be added (different units, text quantities).
/// A `None` quantity is only added to another `None`, giving `None`.
pub fn add_quantities(a: Option<&str>, b: Option<&str>) -> Result<Option<String>, NotAddable>;

/// The line to put back in an input field so that it parses to the same
/// item: "Milk 2 l"; "Parsley, 2 bunches" when the unit is not in the unit
/// table; the name alone when the quantity is text.
pub fn edit_text(name: &str, quantity: Option<&str>) -> String;

/// A name safe to write as a `.shopping-list` line or a `@name{}` ingredient.
pub fn cleaned_name(raw: &str) -> String;
```

`parse_ingredient_list` splits on line breaks, drops a leading bullet (`-`, `*`, `–`, `•`, when followed by whitespace or ending the line) and then a checkbox (`[ ]`, `[x]`, `[X]`), skips lines that are blank after that, and parses each remaining line. One result per kept line, in order.

All public types derive `Debug`, `Clone`, `PartialEq`, `serde::Serialize` and `serde::Deserialize`.

## Parsing one line

Whitespace (any Unicode whitespace, including no-break spaces) splits words. The rules are tried in order; the first that gives an amount and leaves a non-empty name wins. If none does, the whole line (words joined by single spaces) is the name and there is no amount.

### 1. Comma: an explicit amount

A **separating comma** is a `,` that is not between two ASCII digits, so "1,5" is a decimal and not a separator. If the line has one, take the text after the last separating comma:

- If it starts with a number, it is the amount: the number, then every remaining word as the unit. "Parsley, 2 bunches" gives `parsley` with 2 `bunches`; "Flour, 1 1/2 cup" gives 1.5 `cup`; "Eggs, 6" gives a count of 6.
- The number here may be whole, a decimal (`.` or `,`), any fraction `a/b` with `b ≠ 0`, or mixed (`1 1/2`). It must be above zero.
- A unit that is in the unit table is stored in its short form ("Milk, 2 liters" gives `l`); a piece word makes it a count. Any other unit is kept as typed, words joined by single spaces, with `%`, `{` and `}` removed. If nothing is left, it is a count.
- The name is the text before that comma, trimmed, with no trailing commas.
- If the text after the comma does not start with a number ("Tomatoes, cherry", "Salt, a pinch"), rule 1 gives nothing and rule 2 runs on the **whole** line.

Text amounts after a comma ("a pinch", "to taste") are not recognised: nothing reliable tells them from "Tomatoes, cherry" in every language.

### 2. Known units at the end or start

Ported unchanged from the iOS recogniser:

- **End first:** the last word as a number glued to a unit ("2l", "1.5kg", "1/2kg", "3шт"); else the second-to-last word a number and the last word a unit ("2 l"); else the last word a number (a count).
- **Then start:** the same, mirrored ("2l milk", "2 l milk", "3 lemons").
- A unit is only read from the unit table. "Milk 2 packs" has no amount.
- Typed numbers are whole, decimal with `.` or `,`, or a fraction **below one** ("1/2"; "24/7 pizza" and "50/50 mix" stay names). Zero is no amount.
- A number glued to a word stays in the name: "Milk2l", "7up", "Sugar500 grams".

### 3. Aisle match

After the name is found, and only when `aisle` is given, it is compared with every name and alias in the aisle configuration:

- ignoring case (Unicode lowercase);
- and ignoring simple English plurals, by comparing both sides after reducing them: a final `ies` becomes `y`; a final `es` after `s`, `x`, `z`, `ch`, `sh` or `o` is dropped; otherwise a final `s` not preceded by `s` is dropped. "Tomato" matches `tomatoes`, "Berry" matches `berries`, "Lemon" matches `lemons`. "Glass" does not lose its `s`.

An exact case-insensitive match wins over a plural match. Among matches of the same kind, the first in file order wins. On a match, `name` becomes the configuration's spelling and `aisle_match` is filled.

## Unit table

One file, `units.rs`: each stored short form with its aliases. Copied word for word from the iOS `ManualItemUnits.swift` (en, ru, de, es, fr, pt-BR): `g`, `kg`, `ml`, `l`, `tsp`, `tbsp`, `cup`, `oz`, `lb`, and piece words that make a count. Matching ignores case and any trailing full stops. A unit test fails if one alias is listed under two units.

## Stored quantity and numbers

`to_quantity` gives `<number>%<unit>`, or `<number>` for a count. Numbers are rounded to six decimals with trailing zeros removed and a `.` decimal point: "2", "1.5", "0.0004". An amount that rounds to "0" has no quantity. This matches `ManualItemQuantity.stored` and `format` in the iOS app exactly, so lines written by either app merge.

## Name cleaning

`cleaned_name`, ported from `ManualItemQuantity.cleanedName`, works on Unicode scalars:

- removes `{`, `}`, U+200B, U+2060 and U+FEFF;
- turns any run of whitespace, line breaks included, into one space, and trims both ends;
- drops a `-` that follows `-` or `[` (no `--` or `[-` comments);
- repeatedly strips a leading `/`, `./`, or a `?` that is alone or followed by a space.

The result can be empty; callers must not write an empty name. `parse_ingredient` cleans the name before the aisle match.

## Bindings

- `bindings/` (uniffi): `parse_ingredient(line, aisle_text: Option<String>)`, `parse_ingredient_list`, `add_quantities`, `edit_text`, `cleaned_name`, and the records above. The bindings' own `AisleConf` record is a flattened cache, not `cooklang::aisle::AisleConf`, so they take the aisle file's text and parse it with `cooklang::aisle::parse_lenient`, as `parse_aisle_config` does.
- `typescript/` (wasm): the same functions, taking the aisle text the same way, through the existing `tsify` setup.
- The release workflow publishes `cooklang-ingredient-parser` to crates.io before `cooklang-bindings` and alongside `cooklang`.

## Testing

`test-case` tables in the crate:

- **Every case in the iOS `IngredientLineRecognizerTests.swift` and `ManualItemQuantityTests.swift`**, ported with the same expected name, value and unit. The two rule sets must agree on every line without a separating comma.
- **Comma rule:** "Parsley, 2 bunches", "Salt, a pinch" (whole line is the name), "Tomatoes, cherry" (whole line), "Flour 1,5 kg" (decimal comma, not a separator, gives 1.5 `kg`), "Flour, 1,5 kg", "Milk, 2 liters" (`l`), "Eggs, 6 pcs" (count), "Flour, 1 1/2 cup", "Butter, 100{g}" (braces stripped), ", 2 kg" (no name: whole line).
- **Aisle:** exact, case and plural matches against a small aisle.conf; alias versus common name; no match without `aisle`; exact beats plural.
- **Lists:** bullets, checkboxes, blank lines.
- **Quantities:** `to_quantity` / `from_quantity` round trips, `add_quantities` (same unit sums, different unit or text is `NotAddable`), `edit_text` for table units, other units and text quantities. Every `edit_text` result parses back to the same name and quantity.
- **Round trip through the parser:** for hostile names (braces, `--`, `[-`, `./`, `? `, zero-width characters), `cleaned_name` output written as a `.shopping-list` free-hand line and as `@name{}` reads back through `cooklang` as a single ingredient with that name.

## Out of scope

- Using the crate in the iOS app (a follow-up; until then iOS does not apply the comma rule or plural matching, but what it writes is compatible).
- Text amounts after a comma, units of more than one word without a comma, plurals other than English, fuzzy or edit-distance matching.
- A model-based parser. `ParseSource` and the two entry points are where it will plug in.
