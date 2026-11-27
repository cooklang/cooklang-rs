# `cooklang-ingredient-parser`: one typed ingredient line to a name and an amount

**Date:** 2026-11-27
**Status:** Approved in conversation
**First user:** CookCLI's shopping list add field (`CookCLI/docs/superpowers/specs/2026-11-27-shopping-list-manual-items-design.md`)
**Origin:** the iOS recogniser (`mobile-app-ios`, `Packages/ShoppingList/Sources/ShoppingList/Core/IngredientLineRecognizer.swift`, `ManualItemUnits.swift`, `ManualItemQuantity.swift`; spec `docs/superpowers/specs/2026-11-26-shopping-list-manual-items-design.md`)
**Model:** `cook-md/db`, `crates/ingredient-tagger` (ONNX bert-small BIO tagger)

## Problem

People type or paste ingredients as plain text: "Milk 2 l", "2l milk", "2 cans chopped tomatoes", or a whole list copied from a message. Every Cooklang app that takes such text has to turn each line into a name and an amount that a `.shopping-list` or a recipe can hold. The iOS app has done this in Swift since 2026-11-26. CookCLI needs it now, and Android, the editor and recipe import will need it later. With one implementation, every app reads the same line the same way and writes the same stored quantity, which matters because they share synced files and merge lines by comparing stored text.

The crate is called an ingredient parser, not a shopping item parser. The same API should parse a recipe's ingredient list too, and an ML model plugs in from the start.

### Rules and the model

`ingredient-tagger` was measured on 2026-11-27 against shopping-style lines:

| Line | Tagger | Confidence | Rules |
|---|---|---|---|
| `2 l milk` | 2 l, milk | 0.96 | same |
| `1/2 kg flour` | 1/2 kg, flour | 0.98 | same |
| `2 cans chopped tomatoes` | 2 cans, tomatoes, prep "chopped" | 0.91 | a count of 2, name "cans chopped tomatoes" |
| `Milk 2 l` | whole line is the name | 0.45 | 2 l, Milk |
| `Sugar 1.5kg` | whole line is the name | 0.96 | 1.5 kg, Sugar |
| `Сахар 200 гр` | whole line is the name | 0.32 | 200 g, Сахар |
| `Flour 1,5 kg` | 5, name "Flour 1" | 0.34 | 1.5 kg, Flour |

The model is trained on English recipe lines, where the amount comes first. It is strong there and adds what rules cannot do: unknown units, sizes, prep and comments. It is weak on what shopping input looks like (an amount at the end, glued units, six languages), and can be confidently wrong (`Sugar 1.5kg`, 0.96). So the rules decide the lines they recognise, and the model is asked only about lines that start with a number.

The model cannot be a dependency of this crate: it is 29 MB, needs the ONNX Runtime shared library at run time, does not run in wasm, and lives in a private repository. Instead this crate defines a trait, and `ingredient-tagger` implements it, so the dependency points from `db` to `cooklang-rs`.

## Crate

- Directory `ingredient-parser/` in the `cooklang-rs` workspace, next to `bindings` and `typescript`. Package `cooklang-ingredient-parser`, versioned with `cooklang`.
- Depends on `cooklang` (`default-features = false`, `features = ["aisle"]`) for `AisleConf`, plus `serde` and `thiserror`.
- No `unsafe`, no I/O, no global state, no ML dependencies.

## API

```rust
pub fn parse_ingredient(
    line: &str,
    aisle: Option<&AisleConf>,
    model: Option<&dyn IngredientModel>,
) -> ParsedIngredient;

pub fn parse_ingredient_list(
    text: &str,
    aisle: Option<&AisleConf>,
    model: Option<&dyn IngredientModel>,
) -> Vec<ParsedIngredient>;

pub struct ParsedIngredient {
    /// The name to write: cleaned (see "Name cleaning"), and in the aisle
    /// configuration's spelling when `aisle_match` is set. "" only for a
    /// blank line.
    pub name: String,
    /// `None` when the line has no amount.
    pub amount: Option<Amount>,
    /// "large", "small" — only a model fills these three.
    pub size: Option<String>,
    pub prep: Option<String>,
    pub comment: Option<String>,
    /// 1.0 for rules; the model's own score otherwise.
    pub confidence: f32,
    pub source: ParseSource,
    pub aisle_match: Option<AisleMatch>,
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
pub enum ParseSource { Rules, Model }

/// A line-level ingredient model. `ingredient-tagger` implements it.
pub trait IngredientModel {
    fn parse_line(&self, line: &str) -> Result<ModelParse, ModelError>;
}

/// What a model returns, field for field `ingredient_tagger::ParsedIngredient`.
pub struct ModelParse {
    pub quantity: Option<String>,  // "2", "1 1/2", "1-2"
    pub unit: Option<String>,
    pub size: Option<String>,
    pub name: String,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
}

pub struct ModelError(pub String);

/// The lowest model confidence `parse_ingredient` accepts.
pub const MODEL_CONFIDENCE_THRESHOLD: f32 = 0.80;

impl Amount {
    /// Cooklang quantity syntax: "2%l", "3", "2%cans". `None` when the
    /// value rounds to "0" at six decimals.
    pub fn to_quantity(&self) -> Option<String>;
    /// "2 l", "x3" for a count.
    pub fn chip_text(&self) -> String;
    /// "2 l", "3" for a count.
    pub fn display(&self) -> String;
    /// Reads a stored quantity back. `None` for text quantities ("a pinch",
    /// "1-2%cup") and for a comma inside the number ("1,000%g").
    pub fn from_quantity(quantity: &str) -> Option<Amount>;
}

/// Sum two stored quantities with the same unit (compared exactly), or
/// `NotAddable` when they cannot be added (different units, text
/// quantities, one with a quantity and one without). Two `None`s give `None`.
pub fn add_quantities(a: Option<&str>, b: Option<&str>) -> Result<Option<String>, NotAddable>;

/// The line to put back in an input field so that the rules parse it to the
/// same item: "Milk 2 l", "3 lemons"... see "Edit text".
pub fn edit_text(name: &str, quantity: Option<&str>) -> String;

/// A name safe to write as a `.shopping-list` line or a `@name{}` ingredient.
pub fn cleaned_name(raw: &str) -> String;
```

`parse_ingredient_list` splits on line breaks, drops a leading bullet (`-`, `*`, `–` when followed by whitespace or ending the line; `•` always) and then a checkbox (`[ ]`, `[x]`, `[X]`), skips lines that are blank after that and Markdown heading lines (one to six `#`, alone or followed by whitespace), and parses each remaining line. One result per kept line, in order.

All public data types derive `Debug`, `Clone`, `PartialEq`, `serde::Serialize` and `serde::Deserialize`.

## Parsing one line

Whitespace (any Unicode whitespace, including no-break spaces) splits words.

### 1. Rules

Ported unchanged from the iOS recogniser at `mobile-app-ios` commit `a6f8a1d1`:

- A `,` or `;` ending the last word is dropped ("bread,", "Milk 2 l,").
- Punctuation that only separates the name from the amount goes with the amount: a word made only of `, ; : - – —` is dropped, and `, ; :` glued to the name's edge next to the amount are trimmed ("potato, 2 kg", "Milk - 2 l", "2 kg - potatoes"). Punctuation inside a name stays ("garlic, 1 head", "Mix 50:50").

- **End first:** the last word as a number glued to a unit ("2l", "1.5kg", "1/2kg", "3шт"); else the second-to-last word a number and the last word a unit ("2 l"); else the last word a number (a count).
- **Then start:** the same, mirrored ("2l milk", "2 l milk", "3 lemons").
- A unit is only read from the unit table. "Milk 2 packs" has no amount. A table unit is stored in its short form ("grams" gives `g`); a piece word makes the number a count.
- Typed numbers are whole, decimal with `.` or `,`, or a fraction **below one** ("1/2"; "24/7 pizza" and "50/50 mix" stay names). Zero is no amount.
- A number glued to a word stays in the name: "Milk2l", "7up", "Sugar500 grams".
- If taking the amount would leave no name, or nothing matches, the whole line (words joined by single spaces) is the name with no amount.

### 2. Model

Only when `model` is given, and only when the line's **first word is a typed number or a number glued to a table unit**, and the rules did **not** take their amount from the end of the line. "2 cans chopped tomatoes", "1 1/2 cups flour, sifted" and "3 lemons" ask the model. "Milk 2 l", "7up" and "Bread" do not.

The model's answer replaces the rules' answer when all of these hold:

- `parse_line` returned `Ok`;
- `confidence >= MODEL_CONFIDENCE_THRESHOLD`;
- `name` is not empty after `cleaned_name`;
- `quantity` is absent, or reads as one positive number: whole, decimal, any fraction, or mixed ("1 1/2"). A range ("1-2") or text fails this check.

A unit the model gives is looked up in the unit table and stored in its short form when found ("teaspoons" gives `tsp`), otherwise kept as given, with `%`, `{` and `}` removed. `size`, `prep` and `comment` are copied over, and `source` is `Model`. Otherwise the rules' answer stands, and a model error is ignored.

### 3. Aisle match

After the name is found, and only when `aisle` is given, it is compared with every name and alias in the aisle configuration:

- ignoring case (Unicode lowercase);
- and ignoring simple English plurals. Each side's forms are the lowercase word plus, where they apply, the word with a final `ies` turned into `y`, with a final `es` dropped after `s`, `x`, `z`, `ch`, `sh` or `o`, and with a final `s` dropped when not after another `s`. Two names match when their forms share one. "Tomato" matches `tomatoes`, "Berry" matches `berries`, "cookie" matches `cookies`, "Lemon" matches `lemons`; "glass" does not match `glas`.

An exact case-insensitive match wins over a plural match. Among matches of the same kind, the first in file order wins. On a match, `name` becomes the configuration's spelling and `aisle_match` is filled.

## Unit table

One file, `units.rs`: each stored short form with its aliases. Copied word for word from the iOS `ManualItemUnits.swift` (en, ru, de, es, fr, pt-BR): `g`, `kg`, `ml`, `l`, `tsp`, `tbsp`, `cup`, `oz`, `lb`, and piece words that make a count. Matching ignores case and any trailing full stops. A unit test fails if one alias is listed under two units.

## Stored quantity and numbers

`to_quantity` gives `<number>%<unit>`, or `<number>` for a count. Numbers are rounded to six decimals with trailing zeros removed and a `.` decimal point: "2", "1.5", "0.0004". An amount that rounds to "0" has no quantity. This matches `ManualItemQuantity.stored` and `format` in the iOS app exactly, so lines written by either app merge.

## Edit text

`edit_text(name, quantity)` gives "name display" ("Milk 2 l", "Lemons 3") when the quantity is a count or has a table unit in its short form, and the name alone otherwise ("2%cans", "a pinch", "2%L"). The rules then read the line back to the same name and quantity, except when the name itself ends or starts with something that reads as an amount. Callers editing an item whose quantity `edit_text` left out keep that quantity unless the edited line has an amount.

## Name cleaning

`cleaned_name`, ported from `ManualItemQuantity.cleanedName`, works on Unicode scalars:

- removes `{`, `}`, U+200B, U+2060 and U+FEFF;
- turns any run of whitespace, line breaks included, into one space, and trims both ends;
- drops a `-` that follows `-` or `[` (no `--` or `[-` comments);
- repeatedly strips a leading `/`, `./`, or a `?` that is alone or followed by a space.

The result can be empty; callers must not write an empty name. `parse_ingredient` cleans the name before the aisle match.

## `ingredient-tagger` (in `cook-md/db`)

A follow-up in that repository, not part of this crate: `ingredient-tagger` depends on `cooklang-ingredient-parser` and implements `IngredientModel` for `Tagger`, with the tagger's own threshold set to 0 so that `parse_ingredient` applies `MODEL_CONFIDENCE_THRESHOLD`. Its golden tests then also run through `parse_ingredient`.

## Bindings

- `bindings/` (uniffi): `parse_ingredient(line, aisle_text: Option<String>)`, `parse_ingredient_list`, `add_quantities`, `edit_text`, `cleaned_name`, and the records above. The bindings' own `AisleConf` record is a flattened cache, not `cooklang::aisle::AisleConf`, so they take the aisle file's text and parse it with `cooklang::aisle::parse_lenient`, as `parse_aisle_config` does. A model is not exposed yet; the iOS and Android apps would pass one through a uniffi callback interface when they ship one.
- `typescript/` (wasm): the same functions, taking the aisle text the same way, through the existing `tsify` setup. No model.
- The release workflow publishes `cooklang-ingredient-parser` to crates.io before `cooklang-bindings` and alongside `cooklang`.

## Testing

`test-case` tables in the crate:

- **Every case in the iOS `IngredientLineRecognizerTests.swift` and `ManualItemQuantityTests.swift`**, ported with the same expected name, value and unit, with no model. The two must agree on every line.
- **Model combination**, with a fake `IngredientModel` returning fixed answers: not asked for "Milk 2 l", "Bread", "7up"; asked for "2 cans chopped tomatoes" and its answer used at 0.91; its answer ignored at 0.79, with an empty name, with a range quantity, or on `Err`; a model unit "teaspoons" stored as `tsp`, "cans" kept; `size`, `prep`, `comment` and `source` carried over.
- **Aisle:** exact, case and plural matches against a small aisle.conf; alias versus common name; no match without `aisle`; exact beats plural; aisle match after a model answer.
- **Lists:** bullets, checkboxes, blank lines.
- **Quantities:** `to_quantity` / `from_quantity` round trips; `add_quantities` (same unit sums, different unit or text is `NotAddable`); `edit_text` for counts, table units, other units and text quantities; every `edit_text` result with a quantity parses back to the same name and quantity.
- **Round trip through the parser:** for hostile names (braces, `--`, `[-`, `./`, `? `, zero-width characters), `cleaned_name` output written as a `.shopping-list` free-hand line and as `@name{}` reads back through `cooklang` as a single ingredient with that name.

## Out of scope

- Using the crate in the iOS app (a follow-up; what both write is compatible).
- Implementing `IngredientModel` in `ingredient-tagger` (a follow-up in `cook-md/db`, described above).
- Text amounts ("a pinch"), units of more than one word, plurals other than English, fuzzy matching.
