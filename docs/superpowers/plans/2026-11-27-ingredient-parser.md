# `cooklang-ingredient-parser` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A new workspace crate that turns one typed ingredient line ("Milk 2 l", "2 cans chopped tomatoes") into a cleaned name, an amount and an aisle match, with an optional ML model plugged in through a trait, exposed to Swift/Kotlin (uniffi) and JS (wasm).

**Architecture:** Pure-Rust rules ported from the iOS recogniser (`mobile-app-ios` commit `a6f8a1d1`, `Packages/ShoppingList/Sources/ShoppingList/Core/`) decide first; an `IngredientModel` trait object is consulted only for lines that start with a number the rules could not place from the end. An aisle match against `cooklang::aisle::AisleConf` (case and simple English plurals) replaces the name with the configuration's spelling. No I/O, no ML dependency.

**Tech Stack:** Rust 2021, `cooklang` (feature `aisle`), `serde`, `thiserror`, `unicode-normalization`, `test-case`; optional `tsify`/`wasm-bindgen` (feature `ts`); `uniffi` in `bindings/`.

**Spec:** `docs/superpowers/specs/2026-11-27-ingredient-parser-design.md`

**Branch:** `feat/ingredient-parser` (already created; the spec is committed on it).

---

## File structure

```
ingredient-parser/
  Cargo.toml
  src/lib.rs            public types, parse_ingredient, parse_ingredient_list, re-exports
  src/units.rs          unit alias table, match_unit
  src/number.rs         number reading (typed and stored) and formatting
  src/rules.rs          the iOS line rules
  src/list.rs           splitting pasted text into lines
  src/name.rs           cleaned_name
  src/quantity.rs       Amount methods, add_quantities, edit_text
  src/aisle_match.rs    matching a name against AisleConf
  src/model.rs          IngredientModel trait, when to ask, accepting an answer
  tests/ios_parity.rs   the iOS recogniser and quantity test tables
  tests/parse.rs        parse_ingredient / parse_ingredient_list end to end, model and aisle
  tests/roundtrip.rs    cleaned names through cooklang's shopping list and recipe parsers
bindings/src/ingredient_parser.rs   uniffi records and functions
typescript/src/lib.rs               wasm functions (append)
typescript/index.ts                 re-exports
typescript/test/ingredientParser.test.ts
Cargo.toml                          workspace member
.github/workflows/release.yml       publish step
```

Run every `cargo` command from the repository root `/Users/dubadub/Cooklang/cooklang-rs`.

---

### Task 1: Crate skeleton

**Files:**
- Create: `ingredient-parser/Cargo.toml`, `ingredient-parser/src/lib.rs`
- Modify: `Cargo.toml` (workspace `members`, line 67)

- [ ] **Step 1: Create `ingredient-parser/Cargo.toml`**

```toml
[package]
name = "cooklang-ingredient-parser"
version = "0.20.0"
edition = "2021"
authors = ["dubadub <dubovskoy.a@gmail.com>"]
description = "Turns a typed ingredient line into a name and an amount, for Cooklang apps"
license = "MIT"
repository = "https://github.com/cooklang/cooklang-rs"
keywords = ["cooklang", "ingredients", "parser", "shopping-list"]
categories = ["parser-implementations"]

[dependencies]
cooklang = { version = "0.20.0", path = "..", default-features = false, features = ["aisle"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
unicode-normalization = "0.1"
tsify = { version = "0.5", optional = true }
wasm-bindgen = { version = "0.2", optional = true }

[features]
ts = ["dep:tsify", "dep:wasm-bindgen"]

[dev-dependencies]
cooklang = { path = "..", default-features = false, features = ["aisle", "shopping_list"] }
test-case = "3.2.1"
```

- [ ] **Step 2: Create `ingredient-parser/src/lib.rs`** with only the crate doc for now

```rust
//! Turns one typed ingredient line into a name and an amount.
//!
//! "Milk 2 l", "2l milk", "Сахар 200 гр", "2 cans chopped tomatoes": every
//! Cooklang app that takes typed or pasted ingredients reads them through
//! [`parse_ingredient`], so they all agree on the name and write the same
//! stored quantity. See `docs/superpowers/specs/2026-11-27-ingredient-parser-design.md`.
```

- [ ] **Step 3: Add the crate to the workspace**

In `Cargo.toml` change

```toml
members = [".", "typescript", "bindings", "fuzz"]
```

to

```toml
members = [".", "typescript", "bindings", "fuzz", "ingredient-parser"]
```

- [ ] **Step 4: Build**

Run: `cargo build -p cooklang-ingredient-parser`
Expected: `Finished` with no errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock ingredient-parser
git commit -m "feat(ingredient-parser): add the crate to the workspace"
```

---

### Task 2: Numbers

**Files:**
- Create: `ingredient-parser/src/number.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write the module with its tests first (tests at the bottom, implementation as `todo!()`)**

Create `ingredient-parser/src/number.rs`:

```rust
//! Reading and writing the numbers of an amount.
//!
//! Ported from `IngredientLineRecognizer.number` / `typedNumber` and
//! `ManualItemQuantity.format` in the iOS app, so both write the same text.

pub(crate) fn is_digit(c: char) -> bool {
    todo!()
}

/// A whole number, a decimal with "." or ",", or any fraction "a/b", above
/// zero. Reads stored quantities.
pub(crate) fn number(token: &str) -> Option<f64> {
    todo!()
}

/// [`number`] for typed text: a fraction counts only below one, so
/// "24/7 pizza" stays a name.
pub(crate) fn typed_number(token: &str) -> Option<f64> {
    todo!()
}

/// Only ASCII digits, at least one.
pub(crate) fn whole(text: &str) -> Option<f64> {
    todo!()
}

/// Six decimals at most, no trailing zeros, "." whatever the locale.
pub(crate) fn format_number(value: f64) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("2" => Some(2.0))]
    #[test_case("1.5" => Some(1.5))]
    #[test_case("1,5" => Some(1.5))]
    #[test_case("1/2" => Some(0.5))]
    #[test_case("3/2" => Some(1.5) ; "an improper fraction is a stored number")]
    #[test_case("0" => None)]
    #[test_case("0/5" => None)]
    #[test_case("1/0" => None)]
    #[test_case("-1" => None)]
    #[test_case("0x10" => None)]
    #[test_case(".5" => None)]
    #[test_case("1." => None)]
    #[test_case("1.2.3" => None)]
    #[test_case("1/2/3" => None)]
    #[test_case("" => None)]
    fn reads_numbers(token: &str) -> Option<f64> {
        number(token)
    }

    #[test_case("1/2" => Some(0.5))]
    #[test_case("3/4" => Some(0.75))]
    #[test_case("24/7" => None)]
    #[test_case("50/50" => None)]
    #[test_case("2" => Some(2.0))]
    fn reads_typed_numbers(token: &str) -> Option<f64> {
        typed_number(token)
    }

    #[test]
    fn a_number_too_big_for_f64_is_none() {
        assert_eq!(number(&"9".repeat(400)), None);
    }

    #[test_case(2.0 => "2")]
    #[test_case(1.5 => "1.5")]
    #[test_case(0.0004 => "0.0004")]
    #[test_case(1.0 / 3.0 => "0.333333")]
    #[test_case(1234567.0 => "1234567")]
    #[test_case(0.0000001 => "0")]
    #[test_case(0.000001 => "0.000001")]
    fn formats(value: f64) -> String {
        format_number(value)
    }
}
```

Add to `ingredient-parser/src/lib.rs`:

```rust
mod number;
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test -p cooklang-ingredient-parser number`
Expected: FAIL, panics with `not yet implemented`.

- [ ] **Step 3: Implement**

Replace the four `todo!()` bodies:

```rust
pub(crate) fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

fn positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

fn decimal(text: &str) -> Option<f64> {
    let first = text.chars().next()?;
    let last = text.chars().last()?;
    if !is_digit(first) || !is_digit(last) {
        return None;
    }
    if text.chars().filter(|c| *c == '.' || *c == ',').count() > 1 {
        return None;
    }
    if !text.chars().all(|c| is_digit(c) || c == '.' || c == ',') {
        return None;
    }
    text.replace(',', ".").parse().ok()
}

pub(crate) fn whole(text: &str) -> Option<f64> {
    if text.is_empty() || !text.chars().all(is_digit) {
        return None;
    }
    text.parse().ok()
}

pub(crate) fn number(token: &str) -> Option<f64> {
    let parts: Vec<&str> = token.split('/').collect();
    if parts.len() == 2 {
        let top = whole(parts[0])?;
        let bottom = whole(parts[1])?;
        if bottom == 0.0 {
            return None;
        }
        return positive(top / bottom);
    }
    positive(decimal(token)?)
}

pub(crate) fn typed_number(token: &str) -> Option<f64> {
    let parts: Vec<&str> = token.split('/').collect();
    if parts.len() == 2 {
        if let (Some(top), Some(bottom)) = (whole(parts[0]), whole(parts[1])) {
            if top >= bottom {
                return None;
            }
        }
    }
    number(token)
}

pub(crate) fn format_number(value: f64) -> String {
    if value == value.round() && value.abs() < 1e15 {
        return format!("{}", value as i64);
    }
    let mut text = format!("{value:.6}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser number`
Expected: PASS (all `reads_numbers`, `reads_typed_numbers`, `formats`, `a_number_too_big_for_f64_is_none`).

- [ ] **Step 5: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): read and format amount numbers"
```

---

### Task 3: Unit table

**Files:**
- Create: `ingredient-parser/src/units.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write the module**

Create `ingredient-parser/src/units.rs`. The table is copied word for word from `ManualItemUnits.swift` at iOS commit `a6f8a1d1`. All words are lowercase and NFC (composed) in this source file.

```rust
//! The unit words a typed line may use, in six languages, each mapped to the
//! one short form that is stored. Copied from the iOS `ManualItemUnits.swift`.

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitMatch {
    /// A unit, in its stored short form.
    Unit(&'static str),
    /// A piece word ("pcs", "шт"): the number is a plain count.
    Count,
}

/// `""` as the stored form marks piece words.
const ALIASES: &[(&str, &[&str])] = &[
    ("g", &[
        "g", "gr", "gram", "grams", "gramm", "gramme", "grammes", "gramo", "gramos",
        "grama", "gramas", "г", "гр", "грамм", "грамма", "граммов",
    ]),
    ("kg", &[
        "kg", "kgs", "kilo", "kilos", "kilogram", "kilograms", "kilogramm", "kilogramme",
        "kilogrammes", "kilogramo", "kilogramos", "quilo", "quilos", "quilograma",
        "quilogramas", "кг", "кило", "килограмм", "килограмма", "килограммов",
    ]),
    ("ml", &[
        "ml", "milliliter", "milliliters", "millilitre", "millilitres", "mililitro",
        "mililitros", "мл", "миллилитр", "миллилитра", "миллилитров",
    ]),
    ("l", &[
        "l", "liter", "liters", "litre", "litres", "litro", "litros",
        "л", "литр", "литра", "литров",
    ]),
    ("tsp", &[
        "tsp", "tsps", "teaspoon", "teaspoons", "tl", "teelöffel", "cdta", "cdtas", "cucharadita",
        "cucharaditas", "ч.л", "чл",
    ]),
    ("tbsp", &[
        "tbsp", "tbsps", "tbs", "tablespoon", "tablespoons", "el", "esslöffel", "cda", "cdas", "cucharada",
        "cucharadas", "ст.л", "стл",
    ]),
    ("cup", &[
        "cup", "cups", "tasse", "tassen", "tasses", "taza", "tazas", "xícara", "xícaras",
        "xicara", "xicaras", "стакан", "стакана", "стаканов",
    ]),
    ("oz", &[
        "oz", "ounce", "ounces", "unze", "unzen", "onza", "onzas", "once", "onces", "onça",
        "onças", "унция", "унции", "унций",
    ]),
    ("lb", &[
        "lb", "lbs", "pound", "pounds", "pfund", "libra", "libras", "livre", "livres",
        "фунт", "фунта", "фунтов",
    ]),
    ("", &[
        "pc", "pcs", "piece", "pieces", "stück", "stk", "unidad", "unidades", "ud", "uds",
        "pièce", "pièces", "unidade", "шт", "штук", "штука", "штуки",
    ]),
];

/// The unit a word names, ignoring case, Unicode composition and trailing
/// full stops ("TSP.", "ч.л.", "xi\u{301}cara").
pub(crate) fn match_unit(word: &str) -> Option<UnitMatch> {
    let mut key: String = word.nfc().collect::<String>().to_lowercase();
    while key.ends_with('.') {
        key.pop();
    }
    ALIASES
        .iter()
        .find(|(_, words)| words.contains(&key.as_str()))
        .map(|(stored, _)| {
            if stored.is_empty() {
                UnitMatch::Count
            } else {
                UnitMatch::Unit(stored)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_word_is_listed_twice() {
        let mut seen = HashSet::new();
        for (_, words) in ALIASES {
            for word in *words {
                assert!(seen.insert(*word), "{word} is listed under two units");
            }
        }
    }

    #[test]
    fn every_word_is_lowercase_and_composed() {
        for (_, words) in ALIASES {
            for word in *words {
                assert_eq!(word.to_lowercase(), *word);
                assert_eq!(word.nfc().collect::<String>(), *word);
            }
        }
    }

    #[test]
    fn case_composition_and_full_stops_do_not_matter() {
        assert_eq!(match_unit("TSP."), Some(UnitMatch::Unit("tsp")));
        assert_eq!(match_unit("ч.л."), Some(UnitMatch::Unit("tsp")));
        assert_eq!(match_unit("xi\u{301}cara"), Some(UnitMatch::Unit("cup")));
        assert_eq!(match_unit("Stück"), Some(UnitMatch::Count));
        assert_eq!(match_unit("packs"), None);
        assert_eq!(match_unit(""), None);
    }
}
```

Add to `ingredient-parser/src/lib.rs`:

```rust
mod units;
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser units`
Expected: PASS (3 tests). If `no_word_is_listed_twice` fails, the table was mistyped: compare it with the Swift file at `a6f8a1d1` (`git -C ../mobile-app-ios show a6f8a1d1:Packages/ShoppingList/Sources/ShoppingList/Core/ManualItemUnits.swift`).

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): unit alias table in six languages"
```

---

### Task 4: Public types and quantities

**Files:**
- Create: `ingredient-parser/src/quantity.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Add the public types to `lib.rs`**

Append to `ingredient-parser/src/lib.rs`:

```rust
mod quantity;

pub use quantity::{add_quantities, edit_text, NotAddable};

use serde::{Deserialize, Serialize};

/// What [`parse_ingredient`] made of one line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct ParsedIngredient {
    /// The name to write: cleaned, and in the aisle configuration's spelling
    /// when `aisle_match` is set. `""` only for a blank line.
    pub name: String,
    /// `None` when the line has no amount.
    pub amount: Option<Amount>,
    /// "large", "small". Only a model fills `size`, `prep` and `comment`.
    pub size: Option<String>,
    pub prep: Option<String>,
    pub comment: Option<String>,
    /// 1.0 for rules; the model's own score otherwise.
    pub confidence: f32,
    pub source: ParseSource,
    pub aisle_match: Option<AisleMatch>,
}

/// A number and an optional unit. No unit means a count.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct Amount {
    /// Above zero and finite.
    pub value: f64,
    pub unit: Option<String>,
}

/// The aisle configuration entry a name matched.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
pub struct AisleMatch {
    /// The configuration's spelling of the matched name or alias.
    pub name: String,
    /// The first name of its line, which shopping lists show.
    pub common_name: String,
    /// The category it is listed under.
    pub category: String,
}

/// Who produced a [`ParsedIngredient`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(tsify::Tsify), tsify(into_wasm_abi))]
#[non_exhaustive]
pub enum ParseSource {
    Rules,
    Model,
}
```

- [ ] **Step 2: Write `quantity.rs` with tests and `todo!()` bodies**

Create `ingredient-parser/src/quantity.rs`:

```rust
//! The text forms of an amount: stored ("2%l"), shown ("2 l", chip "x3"),
//! and the line that edits an item ("Milk 2 l"). Ported from the iOS
//! `ManualItemQuantity.swift`.

use crate::number::{format_number, number};
use crate::units::{match_unit, UnitMatch};
use crate::Amount;

impl Amount {
    /// Cooklang quantity syntax, `"2%l"` or `"3"` for a count. `None` when the
    /// value is not above zero or rounds to `"0"` at six decimals.
    pub fn to_quantity(&self) -> Option<String> {
        todo!()
    }

    /// `"2 l"`, `"3"` for a count.
    pub fn display(&self) -> String {
        todo!()
    }

    /// `"2 l"`, `"x3"` for a count.
    pub fn chip_text(&self) -> String {
        todo!()
    }

    /// Reads a stored quantity back. `None` for text ("a pinch", "1-2%cup",
    /// "200 ml") and for a comma in the number ("1,000%g": a thousands
    /// separator in a file, not a decimal comma).
    pub fn from_quantity(quantity: &str) -> Option<Amount> {
        todo!()
    }
}

/// Two stored quantities cannot be added: different units, text, or one
/// with a quantity and one without.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the two quantities cannot be added")]
pub struct NotAddable;

/// The sum of two stored quantities with the same unit (compared exactly).
/// Two `None`s give `None`.
pub fn add_quantities(a: Option<&str>, b: Option<&str>) -> Result<Option<String>, NotAddable> {
    todo!()
}

/// The line to put in an input field so the rules read it back as the same
/// item: `"Milk 2 l"`, `"Lemons 3"`; the name alone when the quantity has a
/// unit outside the table ("2%bunches"), a unit in another spelling ("2%L"),
/// or is text ("a pinch").
pub fn edit_text(name: &str, quantity: Option<&str>) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amount(value: f64, unit: Option<&str>) -> Amount {
        Amount { value, unit: unit.map(str::to_owned) }
    }

    #[test]
    fn stored_form() {
        assert_eq!(amount(2.0, Some("l")).to_quantity().as_deref(), Some("2%l"));
        assert_eq!(amount(1.5, Some("kg")).to_quantity().as_deref(), Some("1.5%kg"));
        assert_eq!(amount(0.5, Some("kg")).to_quantity().as_deref(), Some("0.5%kg"));
        assert_eq!(amount(3.0, None).to_quantity().as_deref(), Some("3"));
        assert_eq!(amount(0.0, Some("g")).to_quantity(), None);
        assert_eq!(amount(-1.0, Some("l")).to_quantity(), None);
        assert_eq!(amount(f64::INFINITY, None).to_quantity(), None);
        assert_eq!(amount(0.0000001, Some("g")).to_quantity(), None);
        assert_eq!(amount(0.000001, Some("g")).to_quantity().as_deref(), Some("0.000001%g"));
    }

    #[test]
    fn reads_stored_form() {
        assert_eq!(Amount::from_quantity("2%l"), Some(amount(2.0, Some("l"))));
        assert_eq!(Amount::from_quantity("1.5%kg"), Some(amount(1.5, Some("kg"))));
        assert_eq!(Amount::from_quantity("3"), Some(amount(3.0, None)));
        assert_eq!(Amount::from_quantity("a pinch"), None);
        assert_eq!(Amount::from_quantity("200 ml"), None);
        assert_eq!(Amount::from_quantity(""), None);
        assert_eq!(Amount::from_quantity("1/2%cup"), Some(amount(0.5, Some("cup"))));
        assert_eq!(Amount::from_quantity("3/2%cup"), Some(amount(1.5, Some("cup"))));
        assert_eq!(Amount::from_quantity("0%g"), None);
        assert_eq!(Amount::from_quantity("-1%l"), None);
        assert_eq!(Amount::from_quantity("0x10"), None);
        assert_eq!(Amount::from_quantity(" 2 % l "), Some(amount(2.0, Some("l"))));
        assert_eq!(Amount::from_quantity("2%"), Some(amount(2.0, None)));
        assert_eq!(Amount::from_quantity("1,000%g"), None);
        assert_eq!(Amount::from_quantity("1,5%kg"), None);
    }

    #[test]
    fn a_small_amount_survives_storing() {
        let small = amount(0.0004, Some("kg"));
        let stored = small.to_quantity().unwrap();
        assert_eq!(Amount::from_quantity(&stored), Some(small));
    }

    #[test]
    fn display_and_chip() {
        assert_eq!(amount(500.0, Some("g")).chip_text(), "500 g");
        assert_eq!(amount(1.5, Some("kg")).chip_text(), "1.5 kg");
        assert_eq!(amount(3.0, None).chip_text(), "x3");
        assert_eq!(amount(3.0, None).display(), "3");
        assert_eq!(amount(2.0, Some("l")).display(), "2 l");
    }

    #[test]
    fn adds_quantities() {
        assert_eq!(add_quantities(Some("2%l"), Some("1%l")), Ok(Some("3%l".into())));
        assert_eq!(add_quantities(Some("1.5%kg"), Some("0.5%kg")), Ok(Some("2%kg".into())));
        assert_eq!(add_quantities(Some("3"), Some("2")), Ok(Some("5".into())));
        assert_eq!(add_quantities(None, None), Ok(None));
        assert_eq!(add_quantities(Some("2%l"), Some("500%ml")), Err(NotAddable));
        assert_eq!(add_quantities(Some("2%l"), Some("2")), Err(NotAddable));
        assert_eq!(add_quantities(Some("2%l"), None), Err(NotAddable));
        assert_eq!(add_quantities(None, Some("2")), Err(NotAddable));
        assert_eq!(add_quantities(Some("a pinch"), Some("a pinch")), Err(NotAddable));
        assert_eq!(add_quantities(Some("2%L"), Some("2%l")), Err(NotAddable));
    }

    #[test]
    fn edit_text_is_the_line_that_makes_the_item() {
        assert_eq!(edit_text("Milk", Some("2%l")), "Milk 2 l");
        assert_eq!(edit_text("Baking powder", Some("1")), "Baking powder 1");
        assert_eq!(edit_text("Bread", None), "Bread");
        assert_eq!(edit_text("olive oil", Some("a splash")), "olive oil");
        assert_eq!(edit_text("Oil", Some("1-2%cup")), "Oil");
        assert_eq!(edit_text("parsley", Some("2%bunches")), "parsley");
        assert_eq!(edit_text("Milk", Some("2%L")), "Milk");
    }
}
```

- [ ] **Step 3: Run the tests to see them fail**

Run: `cargo test -p cooklang-ingredient-parser quantity`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 4: Implement**

Replace the `todo!()` bodies in `quantity.rs`:

```rust
    pub fn to_quantity(&self) -> Option<String> {
        if !(self.value.is_finite() && self.value > 0.0) {
            return None;
        }
        let number = format_number(self.value);
        if number == "0" {
            return None;
        }
        Some(match &self.unit {
            Some(unit) => format!("{number}%{unit}"),
            None => number,
        })
    }

    pub fn display(&self) -> String {
        match &self.unit {
            Some(unit) => format!("{} {unit}", format_number(self.value)),
            None => format_number(self.value),
        }
    }

    pub fn chip_text(&self) -> String {
        match &self.unit {
            Some(_) => self.display(),
            None => format!("x{}", format_number(self.value)),
        }
    }

    pub fn from_quantity(quantity: &str) -> Option<Amount> {
        let mut parts = quantity.splitn(2, '%');
        let first = parts.next()?;
        if first.contains(',') {
            return None;
        }
        let value = number(first.trim())?;
        let unit = parts
            .next()
            .map(str::trim)
            .filter(|unit| !unit.is_empty())
            .map(str::to_owned);
        Some(Amount { value, unit })
    }
```

```rust
pub fn add_quantities(a: Option<&str>, b: Option<&str>) -> Result<Option<String>, NotAddable> {
    match (a, b) {
        (None, None) => Ok(None),
        (Some(a), Some(b)) => {
            let a = Amount::from_quantity(a).ok_or(NotAddable)?;
            let b = Amount::from_quantity(b).ok_or(NotAddable)?;
            if a.unit != b.unit {
                return Err(NotAddable);
            }
            let sum = Amount { value: a.value + b.value, unit: a.unit };
            sum.to_quantity().map(Some).ok_or(NotAddable)
        }
        _ => Err(NotAddable),
    }
}

/// The stored amount when the rules read its display form back the same.
fn editable_amount(quantity: &str) -> Option<Amount> {
    let amount = Amount::from_quantity(quantity)?;
    match &amount.unit {
        None => Some(amount),
        Some(unit) => {
            matches!(match_unit(unit), Some(UnitMatch::Unit(stored)) if stored == unit)
                .then_some(amount)
        }
    }
}

pub fn edit_text(name: &str, quantity: Option<&str>) -> String {
    match quantity.and_then(editable_amount) {
        Some(amount) => format!("{name} {}", amount.display()),
        None => name.to_owned(),
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser quantity`
Expected: PASS (6 tests).

- [ ] **Step 6: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): stored quantities, sums and edit text"
```

---

### Task 5: Name cleaning

**Files:**
- Create: `ingredient-parser/src/name.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write the module with tests and a `todo!()` body**

Create `ingredient-parser/src/name.rs`:

```rust
//! A name that reads back from `.shopping-list` and from `@name{}` as the
//! same single ingredient. Ported from `ManualItemQuantity.cleanedName`.

const INVISIBLE: [char; 3] = ['\u{200B}', '\u{2060}', '\u{FEFF}'];

/// Removes braces and zero-width characters, collapses whitespace (line
/// breaks included) to single spaces, drops a `-` after `-` or `[` (no
/// `--` or `[-` comments), and strips a leading `/`, `./` or lone `?`.
///
/// Works on Unicode scalars (`char`), as the Cooklang parser compares bytes.
/// The result can be empty: never write an empty name.
pub fn cleaned_name(raw: &str) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("Tomatoes {ripe}" => "Tomatoes ripe")]
    #[test_case("salt -- sea" => "salt - sea")]
    #[test_case("a [- b" => "a [ b")]
    #[test_case("./Milk" => "Milk")]
    #[test_case("/Milk" => "Milk")]
    #[test_case("  Olive   oil \n" => "Olive oil")]
    #[test_case("{}" => "")]
    #[test_case("Olive\u{00A0}oil" => "Olive oil")]
    #[test_case("? milk" => "milk")]
    #[test_case("?" => "")]
    #[test_case("?milk" => "?milk")]
    #[test_case("? ./ ? milk" => "milk")]
    #[test_case("Milk\u{200B}" => "Milk")]
    #[test_case("a\tb\u{2028}c" => "a b c")]
    #[test_case("[--x" => "[x")]
    fn cleans(raw: &str) -> String {
        cleaned_name(raw)
    }
}
```

Add to `ingredient-parser/src/lib.rs`:

```rust
mod name;

pub use name::cleaned_name;
```

- [ ] **Step 2: Run to see it fail**

Run: `cargo test -p cooklang-ingredient-parser name::`
Expected: FAIL with `not yet implemented`.

- [ ] **Step 3: Implement**

```rust
pub fn cleaned_name(raw: &str) -> String {
    let mut chars: Vec<char> = Vec::with_capacity(raw.len());
    for c in raw.chars() {
        if c == '{' || c == '}' || INVISIBLE.contains(&c) {
            continue;
        }
        if c.is_whitespace() {
            if matches!(chars.last(), Some(last) if *last != ' ') {
                chars.push(' ');
            }
            continue;
        }
        if c == '-' && matches!(chars.last(), Some('-') | Some('[')) {
            continue;
        }
        chars.push(c);
    }
    if chars.last() == Some(&' ') {
        chars.pop();
    }
    loop {
        if chars.first() == Some(&'/') {
            chars.remove(0);
        } else if chars.starts_with(&['.', '/']) {
            chars.drain(..2);
        } else if chars.first() == Some(&'?') && (chars.len() == 1 || chars[1] == ' ') {
            chars.remove(0);
        } else {
            break;
        }
        if chars.first() == Some(&' ') {
            chars.remove(0);
        }
    }
    chars.into_iter().collect()
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser name::`
Expected: PASS (15 cases).

- [ ] **Step 5: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): clean names so they read back as one ingredient"
```

---

### Task 6: Line rules

**Files:**
- Create: `ingredient-parser/src/rules.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write `rules.rs`**

Ported from `IngredientLineRecognizer.recognize` at iOS commit `a6f8a1d1`. The full iOS test table runs against it in Task 10; this task carries a handful of smoke tests.

```rust
//! The iOS recogniser's rules: an amount at the end or the start of a line.

use crate::number::{is_digit, typed_number};
use crate::units::{match_unit, UnitMatch};
use crate::Amount;

pub(crate) struct RulesResult {
    /// Words joined by single spaces; not yet cleaned.
    pub name: String,
    pub amount: Option<Amount>,
    /// The amount was taken from the end of the line.
    pub from_end: bool,
}

struct Found {
    amount: Amount,
    name_tokens: Vec<String>,
    from_end: bool,
}

/// Dropped from the end of the last word: "bread,", "Milk 2 l,".
const LINE_ENDERS: [char; 2] = [',', ';'];
/// A word made only of these separates name and amount: "Milk - 2 l".
const SEPARATORS: [char; 6] = [',', ';', ':', '-', '\u{2013}', '\u{2014}'];
/// Trimmed from the name's edge next to the amount: "potato, 2 kg".
const EDGE_TRIMMED: [char; 3] = [',', ';', ':'];

pub(crate) fn recognize(line: &str) -> RulesResult {
    let mut tokens: Vec<String> = line.split_whitespace().map(str::to_owned).collect();
    if let Some(last) = tokens.last() {
        let trimmed = last.trim_end_matches(LINE_ENDERS).to_owned();
        if trimmed.is_empty() {
            tokens.pop();
        } else if let Some(slot) = tokens.last_mut() {
            *slot = trimmed;
        }
    }
    let whole = RulesResult { name: tokens.join(" "), amount: None, from_end: false };
    if tokens.len() < 2 {
        return whole;
    }
    let Some(found) = amount_at_end(&tokens).or_else(|| amount_at_start(&tokens)) else {
        return whole;
    };
    let name = without_separators(found.name_tokens, found.from_end);
    if name.is_empty() {
        return whole;
    }
    RulesResult { name: name.join(" "), amount: Some(found.amount), from_end: found.from_end }
}

fn amount_of(value: f64, unit: UnitMatch) -> Amount {
    match unit {
        UnitMatch::Unit(stored) => Amount { value, unit: Some(stored.to_owned()) },
        UnitMatch::Count => Amount { value, unit: None },
    }
}

fn amount_at_end(tokens: &[String]) -> Option<Found> {
    let n = tokens.len();
    let last = &tokens[n - 1];
    if let Some(amount) = glued_amount(last) {
        return Some(Found { amount, name_tokens: tokens[..n - 1].to_vec(), from_end: true });
    }
    if let (Some(value), Some(unit)) = (typed_number(&tokens[n - 2]), match_unit(last)) {
        return Some(Found {
            amount: amount_of(value, unit),
            name_tokens: tokens[..n - 2].to_vec(),
            from_end: true,
        });
    }
    if let Some(value) = typed_number(last) {
        return Some(Found {
            amount: Amount { value, unit: None },
            name_tokens: tokens[..n - 1].to_vec(),
            from_end: true,
        });
    }
    None
}

fn amount_at_start(tokens: &[String]) -> Option<Found> {
    let first = &tokens[0];
    if let Some(amount) = glued_amount(first) {
        return Some(Found { amount, name_tokens: tokens[1..].to_vec(), from_end: false });
    }
    let value = typed_number(first)?;
    if let Some(unit) = match_unit(&tokens[1]) {
        return Some(Found {
            amount: amount_of(value, unit),
            name_tokens: tokens[2..].to_vec(),
            from_end: false,
        });
    }
    Some(Found {
        amount: Amount { value, unit: None },
        name_tokens: tokens[1..].to_vec(),
        from_end: false,
    })
}

/// "2l", "1.5kg", "1/2kg", "3шт": a number with a table unit glued on.
/// Not "7up": "up" is no unit.
pub(crate) fn glued_amount(token: &str) -> Option<Amount> {
    let end = token
        .char_indices()
        .find(|(_, c)| !(is_digit(*c) || matches!(c, '.' | ',' | '/')))
        .map_or(token.len(), |(i, _)| i);
    let mut digits = &token[..end];
    while let Some(c) = digits.chars().last() {
        if is_digit(c) {
            break;
        }
        digits = &digits[..digits.len() - c.len_utf8()];
    }
    let rest = &token[digits.len()..];
    if rest.is_empty() || digits.matches('/').count() > 1 {
        return None;
    }
    let value = typed_number(digits)?;
    Some(amount_of(value, match_unit(rest)?))
}

/// Drops words that are only separators, and separators glued to the name's
/// edge next to the amount, so "potato," and "Milk -" read as names.
fn without_separators(mut name: Vec<String>, from_end: bool) -> Vec<String> {
    loop {
        let edge = if from_end { name.last() } else { name.first() };
        let Some(token) = edge.cloned() else { break };
        let rest = if from_end {
            token.trim_end_matches(EDGE_TRIMMED)
        } else {
            token.trim_start_matches(EDGE_TRIMMED)
        };
        if rest.is_empty() || token.chars().all(|c| SEPARATORS.contains(&c)) {
            if from_end {
                name.pop();
            } else {
                name.remove(0);
            }
            continue;
        }
        let rest = rest.to_owned();
        if from_end {
            if let Some(slot) = name.last_mut() {
                *slot = rest;
            }
        } else {
            name[0] = rest;
        }
        break;
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> (String, Option<f64>, Option<String>, bool) {
        let r = recognize(line);
        let value = r.amount.as_ref().map(|a| a.value);
        let unit = r.amount.and_then(|a| a.unit);
        (r.name, value, unit, r.from_end)
    }

    #[test]
    fn smoke() {
        assert_eq!(run("Milk 2 l"), ("Milk".into(), Some(2.0), Some("l".into()), true));
        assert_eq!(run("2l milk"), ("milk".into(), Some(2.0), Some("l".into()), false));
        assert_eq!(run("7up"), ("7up".into(), None, None, false));
        assert_eq!(run("Milk - 2 l"), ("Milk".into(), Some(2.0), Some("l".into()), true));
        assert_eq!(run("2 kg - potatoes"), ("potatoes".into(), Some(2.0), Some("kg".into()), false));
    }
}
```

Add to `ingredient-parser/src/lib.rs`:

```rust
mod rules;
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser rules`
Expected: PASS (1 test, `smoke`).

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): port the iOS line rules"
```

---

### Task 7: Splitting pasted text

**Files:**
- Create: `ingredient-parser/src/list.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write `list.rs` with its tests**

Ported from `splitPastedText`, `withoutListMarker`, `isHeading` at `a6f8a1d1`.

```rust
//! Pasted text to one entry per line.

/// What Foundation's `CharacterSet.newlines` holds.
const NEWLINES: [char; 7] = ['\n', '\u{0B}', '\u{0C}', '\r', '\u{85}', '\u{2028}', '\u{2029}'];

/// One entry per non-blank line, without a leading bullet or checkbox, and
/// without Markdown heading lines ("## Dairy").
pub(crate) fn split_lines(text: &str) -> Vec<String> {
    text.split(NEWLINES)
        .map(without_list_marker)
        .filter(|line| !line.is_empty() && !is_heading(line))
        .collect()
}

fn without_list_marker(line: &str) -> String {
    let mut text = line.trim();
    if let Some(rest) = ["-", "*", "\u{2013}"].iter().find_map(|m| starts_marker(text, m)) {
        text = rest.trim();
    } else if let Some(rest) = text.strip_prefix('\u{2022}') {
        text = rest.trim();
    }
    if let Some(rest) = ["[ ]", "[x]", "[X]"].iter().find_map(|b| text.strip_prefix(b)) {
        text = rest.trim();
    }
    text.to_owned()
}

/// A marker counts when whitespace follows it or it is the whole line.
fn starts_marker<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(marker)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest)
}

/// One to six `#`, alone or followed by whitespace.
fn is_heading(text: &str) -> bool {
    let hashes = text.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return false;
    }
    let rest = &text[hashes..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_without_bullets_checkboxes_or_blank_lines() {
        let text = "Can you grab on the way home\n\n- Milk 2 l\n* Lemons 1\n• Avocado\n- [ ] Bread\n[x] Parmigiano\r\n   \n- \n*\n•\n";
        assert_eq!(
            split_lines(text),
            ["Can you grab on the way home", "Milk 2 l", "Lemons 1", "Avocado", "Bread", "Parmigiano"]
        );
    }

    #[test]
    fn drops_markdown_headings_but_not_hashes_in_names() {
        let text = "## Dairy\n\n- [ ] Milk, 2 l\n- [x] Butter\n#\n#1 flour\n#hashtag soup\n###### Deep\n####### seven";
        assert_eq!(
            split_lines(text),
            ["Milk, 2 l", "Butter", "#1 flour", "#hashtag soup", "####### seven"]
        );
    }

    #[test]
    fn keeps_a_hyphen_inside_a_name() {
        assert_eq!(split_lines("Coca-Cola\n-Zero"), ["Coca-Cola", "-Zero"]);
    }
}
```

Add to `ingredient-parser/src/lib.rs`:

```rust
mod list;
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser list`
Expected: PASS (3 tests).

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): split pasted text into lines"
```

---

### Task 8: Aisle match

**Files:**
- Create: `ingredient-parser/src/aisle_match.rs`
- Modify: `ingredient-parser/src/lib.rs`

Plural rule, as implemented: each side's *forms* are the lowercase word plus, when it applies, the word with `ies`→`y`, with `es` dropped after `s`/`x`/`z`/`ch`/`sh`/`o`, and with a final `s` dropped when not after another `s`. Two names match when their forms share one. This is what the spec's "comparing both sides after reducing them" means; using sets rather than one reduction keeps "cookies"/"cookie" and "shoes"/"shoe" matching.

- [ ] **Step 1: Write the module with tests**

```rust
//! Matching a typed name against an aisle configuration.

use crate::AisleMatch;
use cooklang::aisle::AisleConf;

/// The lowercase word and the singulars it may be the plural of.
fn forms(word: &str) -> Vec<String> {
    let word = word.to_lowercase();
    let mut forms = vec![word.clone()];
    if let Some(stem) = word.strip_suffix("ies") {
        if !stem.is_empty() {
            forms.push(format!("{stem}y"));
        }
    }
    if let Some(stem) = word.strip_suffix("es") {
        if ["s", "x", "z", "ch", "sh", "o"].iter().any(|end| stem.ends_with(end)) {
            forms.push(stem.to_owned());
        }
    }
    if let Some(stem) = word.strip_suffix('s') {
        if !stem.is_empty() && !stem.ends_with('s') {
            forms.push(stem.to_owned());
        }
    }
    forms
}

/// The entry `name` matches: an exact case-insensitive match first, then a
/// plural match; the first in file order within each kind.
pub(crate) fn find(aisle: &AisleConf, name: &str) -> Option<AisleMatch> {
    if name.is_empty() {
        return None;
    }
    let entries = || {
        aisle.categories.iter().flat_map(|category| {
            category.ingredients.iter().flat_map(move |ingredient| {
                let common = ingredient.names.first().copied().unwrap_or_default();
                ingredient.names.iter().map(move |n| (category.name, common, *n))
            })
        })
    };
    let to_match = |(category, common, name): (&str, &str, &str)| AisleMatch {
        name: name.to_owned(),
        common_name: common.to_owned(),
        category: category.to_owned(),
    };
    let lower = name.to_lowercase();
    if let Some(entry) = entries().find(|(_, _, n)| n.to_lowercase() == lower) {
        return Some(to_match(entry));
    }
    let mine = forms(name);
    entries()
        .find(|(_, _, n)| forms(n).iter().any(|form| mine.contains(form)))
        .map(to_match)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AISLE: &str = "[vegetables]\ntomatoes\nberries\nlemons|lemon juice\n\n[bakery]\ncookies\nglass noodles\n\n[dairy]\nmilk|Milch\ntomato paste\n";

    fn conf() -> AisleConf<'static> {
        cooklang::aisle::parse(AISLE).unwrap()
    }

    fn matched(name: &str) -> Option<(String, String, String)> {
        find(&conf(), name).map(|m| (m.name, m.common_name, m.category))
    }

    #[test]
    fn matches_ignoring_case_and_plurals() {
        assert_eq!(matched("Tomato"), Some(("tomatoes".into(), "tomatoes".into(), "vegetables".into())));
        assert_eq!(matched("Berry"), Some(("berries".into(), "berries".into(), "vegetables".into())));
        assert_eq!(matched("LEMON"), Some(("lemons".into(), "lemons".into(), "vegetables".into())));
        assert_eq!(matched("cookie"), Some(("cookies".into(), "cookies".into(), "bakery".into())));
        assert_eq!(matched("MILCH"), Some(("Milch".into(), "milk".into(), "dairy".into())));
    }

    #[test]
    fn an_alias_keeps_its_own_spelling_and_the_common_name() {
        assert_eq!(
            matched("Lemon Juice"),
            Some(("lemon juice".into(), "lemons".into(), "vegetables".into()))
        );
    }

    #[test]
    fn no_match() {
        assert_eq!(matched("bread"), None);
        assert_eq!(matched("glass"), None);
        assert_eq!(matched(""), None);
    }
}
```

`AISLE` uses the aisle.conf syntax `cooklang::aisle::parse` reads (`[category]`, one ingredient per line, `|` between a name and its aliases). If `parse` rejects it, read `src/aisle.rs` tests for the exact syntax and adjust the constant, not the code.

Add to `ingredient-parser/src/lib.rs`:

```rust
mod aisle_match;
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser aisle_match`
Expected: PASS (3 tests).

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/src
git commit -m "feat(ingredient-parser): match names against the aisle configuration"
```

---

### Task 9: Model hook and the public entry points

**Files:**
- Create: `ingredient-parser/src/model.rs`, `ingredient-parser/tests/parse.rs`
- Modify: `ingredient-parser/src/lib.rs`

- [ ] **Step 1: Write the failing integration test**

Create `ingredient-parser/tests/parse.rs`:

```rust
use cooklang_ingredient_parser::{
    parse_ingredient, parse_ingredient_list, Amount, IngredientModel, ModelError, ModelParse,
    ParseSource,
};
use std::cell::RefCell;

/// Answers every line with `answer` and records what it was asked.
struct FakeModel {
    answer: Result<ModelParse, ModelError>,
    asked: RefCell<Vec<String>>,
}

impl FakeModel {
    fn new(answer: Result<ModelParse, ModelError>) -> Self {
        Self { answer, asked: RefCell::new(Vec::new()) }
    }
}

impl IngredientModel for FakeModel {
    fn parse_line(&self, line: &str) -> Result<ModelParse, ModelError> {
        self.asked.borrow_mut().push(line.to_owned());
        self.answer.clone()
    }
}

fn tomatoes(confidence: f32) -> ModelParse {
    ModelParse {
        quantity: Some("2".into()),
        unit: Some("cans".into()),
        size: None,
        name: "tomatoes".into(),
        prep: Some("chopped".into()),
        comment: None,
        confidence,
    }
}

#[test]
fn rules_alone() {
    let parsed = parse_ingredient("Sugar 1.5kg", None, None);
    assert_eq!(parsed.name, "Sugar");
    assert_eq!(parsed.amount, Some(Amount { value: 1.5, unit: Some("kg".into()) }));
    assert_eq!(parsed.source, ParseSource::Rules);
    assert_eq!(parsed.confidence, 1.0);
}

#[test]
fn the_name_is_cleaned() {
    assert_eq!(parse_ingredient("./Milk {fresh} 2 l", None, None).name, "Milk fresh");
}

#[test]
fn the_model_is_not_asked_when_the_rules_found_an_amount_at_the_end_or_no_number_leads() {
    let model = FakeModel::new(Ok(tomatoes(0.99)));
    for line in ["Milk 2 l", "Bread", "7up", "Lemons 3"] {
        let parsed = parse_ingredient(line, None, Some(&model));
        assert_eq!(parsed.source, ParseSource::Rules, "{line}");
    }
    assert!(model.asked.borrow().is_empty());
}

#[test]
fn a_confident_model_answer_replaces_the_rules() {
    let model = FakeModel::new(Ok(tomatoes(0.91)));
    let parsed = parse_ingredient("2 cans chopped tomatoes", None, Some(&model));
    assert_eq!(parsed.name, "tomatoes");
    assert_eq!(parsed.amount, Some(Amount { value: 2.0, unit: Some("cans".into()) }));
    assert_eq!(parsed.prep.as_deref(), Some("chopped"));
    assert_eq!(parsed.source, ParseSource::Model);
    assert_eq!(parsed.confidence, 0.91);
    assert_eq!(model.asked.borrow().as_slice(), ["2 cans chopped tomatoes"]);
}

#[test]
fn a_model_unit_in_the_table_is_stored_short() {
    let answer = ModelParse { unit: Some("teaspoons".into()), quantity: Some("1 1/2".into()), ..tomatoes(0.9) };
    let parsed = parse_ingredient("1 1/2 teaspoons tomatoes", None, Some(&FakeModel::new(Ok(answer))));
    assert_eq!(parsed.amount, Some(Amount { value: 1.5, unit: Some("tsp".into()) }));
}

#[test]
fn a_weak_or_unusable_model_answer_is_ignored() {
    let line = "2 cans chopped tomatoes";
    let rules = parse_ingredient(line, None, None);
    let answers = [
        Ok(tomatoes(0.79)),
        Ok(ModelParse { name: "  {} ".into(), ..tomatoes(0.99) }),
        Ok(ModelParse { quantity: Some("1-2".into()), ..tomatoes(0.99) }),
        Ok(ModelParse { quantity: Some("a few".into()), ..tomatoes(0.99) }),
        Ok(tomatoes(f32::NAN)),
        Err(ModelError("no runtime".into())),
    ];
    for answer in answers {
        let parsed = parse_ingredient(line, None, Some(&FakeModel::new(answer.clone())));
        assert_eq!(parsed, rules, "{answer:?}");
    }
}

#[test]
fn the_aisle_match_runs_on_the_final_name() {
    let aisle = cooklang::aisle::parse("[canned]\ntomatoes\n\n[dairy]\nmilk\n").unwrap();
    let model = FakeModel::new(Ok(ModelParse { name: "Tomato".into(), ..tomatoes(0.95) }));
    let parsed = parse_ingredient("2 cans chopped tomatoes", Some(&aisle), Some(&model));
    assert_eq!(parsed.name, "tomatoes");
    assert_eq!(parsed.aisle_match.unwrap().category, "canned");

    let parsed = parse_ingredient("Milk 2 l", Some(&aisle), None);
    assert_eq!(parsed.name, "milk");
}

#[test]
fn a_list_is_one_result_per_kept_line() {
    let parsed = parse_ingredient_list("## Dairy\n- Milk 2 l\n\n* [x] 3 lemons\n", None, None);
    let names: Vec<_> = parsed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Milk", "lemons"]);
}

#[test]
fn a_blank_line_has_an_empty_name() {
    assert_eq!(parse_ingredient("   ", None, None).name, "");
}
```

- [ ] **Step 2: Run it to see it fail**

Run: `cargo test -p cooklang-ingredient-parser --test parse`
Expected: FAIL to compile: `parse_ingredient`, `IngredientModel`, `ModelParse`, `ModelError` not found.

- [ ] **Step 3: Write `model.rs`**

```rust
//! Plugging in a line-level model, such as `ingredient-tagger` in
//! `cook-md/db`. The rules decide first; see `should_ask`.

use crate::name::cleaned_name;
use crate::number::{number, typed_number, whole};
use crate::rules::{glued_amount, RulesResult};
use crate::units::{match_unit, UnitMatch};
use crate::{Amount, ParseSource, ParsedIngredient};
use serde::{Deserialize, Serialize};

/// A model that splits one ingredient line into its parts.
pub trait IngredientModel {
    fn parse_line(&self, line: &str) -> Result<ModelParse, ModelError>;
}

/// What a model returns; field for field `ingredient_tagger::ParsedIngredient`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelParse {
    /// "2", "1 1/2", "1-2".
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub size: Option<String>,
    pub name: String,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("ingredient model failed: {0}")]
pub struct ModelError(pub String);

/// The lowest model confidence [`crate::parse_ingredient`] accepts.
pub const MODEL_CONFIDENCE_THRESHOLD: f32 = 0.80;

/// Ask only about lines whose first word is a number, and only when the
/// rules did not already take an amount from the end of the line.
pub(crate) fn should_ask(line: &str, rules: &RulesResult) -> bool {
    if rules.amount.is_some() && rules.from_end {
        return false;
    }
    let Some(first) = line.split_whitespace().next() else {
        return false;
    };
    typed_number(first).is_some() || glued_amount(first).is_some()
}

/// The model's answer as a parse, or `None` when it must not be used.
pub(crate) fn accept(answer: Result<ModelParse, ModelError>) -> Option<ParsedIngredient> {
    let answer = answer.ok()?;
    // `>=` is false for NaN, so a NaN confidence is refused too.
    if !(answer.confidence >= MODEL_CONFIDENCE_THRESHOLD) {
        return None;
    }
    let name = cleaned_name(&answer.name);
    if name.is_empty() {
        return None;
    }
    let value = match answer.quantity.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
        None => None,
        Some(quantity) => Some(model_number(quantity)?),
    };
    let amount = value.map(|value| Amount {
        value,
        unit: answer.unit.as_deref().and_then(model_unit),
    });
    Some(ParsedIngredient {
        name,
        amount,
        size: answer.size,
        prep: answer.prep,
        comment: answer.comment,
        confidence: answer.confidence,
        source: ParseSource::Model,
        aisle_match: None,
    })
}

/// One positive number: whole, decimal, any fraction, or mixed ("1 1/2").
fn model_number(quantity: &str) -> Option<f64> {
    let parts: Vec<&str> = quantity.split_whitespace().collect();
    match parts.as_slice() {
        [single] => number(single),
        [whole_part, fraction] if fraction.contains('/') => {
            Some(whole(whole_part)? + number(fraction)?)
        }
        _ => None,
    }
}

/// A table unit in its short form; a piece word is a count (`None`); any
/// other unit as given, without `%`, `{`, `}` and with single spaces.
fn model_unit(unit: &str) -> Option<String> {
    match match_unit(unit) {
        Some(UnitMatch::Unit(stored)) => Some(stored.to_owned()),
        Some(UnitMatch::Count) => None,
        None => {
            let kept: String = unit.chars().filter(|c| !matches!(c, '%' | '{' | '}')).collect();
            let kept = kept.split_whitespace().collect::<Vec<_>>().join(" ");
            (!kept.is_empty()).then_some(kept)
        }
    }
}
```

- [ ] **Step 4: Add the entry points to `lib.rs`**

Append to `ingredient-parser/src/lib.rs`:

```rust
mod model;

pub use model::{IngredientModel, ModelError, ModelParse, MODEL_CONFIDENCE_THRESHOLD};

use cooklang::aisle::AisleConf;

/// Parses one typed ingredient line.
///
/// The rules decide first. `model`, when given, is asked only about a line
/// whose first word is a number and whose amount the rules did not take
/// from the end; its answer is used at [`MODEL_CONFIDENCE_THRESHOLD`] or
/// above. With `aisle`, the final name is matched against the configuration
/// ignoring case and simple English plurals, and takes its spelling.
pub fn parse_ingredient(
    line: &str,
    aisle: Option<&AisleConf>,
    model: Option<&dyn IngredientModel>,
) -> ParsedIngredient {
    let rules = rules::recognize(line);
    let model_answer = model
        .filter(|_| model::should_ask(line, &rules))
        .and_then(|model| model::accept(model.parse_line(line)));
    let mut parsed = model_answer.unwrap_or(ParsedIngredient {
        name: cleaned_name(&rules.name),
        amount: rules.amount,
        size: None,
        prep: None,
        comment: None,
        confidence: 1.0,
        source: ParseSource::Rules,
        aisle_match: None,
    });
    if let Some(found) = aisle.and_then(|aisle| aisle_match::find(aisle, &parsed.name)) {
        parsed.name = found.name.clone();
        parsed.aisle_match = Some(found);
    }
    parsed
}

/// Parses pasted text: one result per non-blank line, without bullets,
/// checkboxes or Markdown headings.
pub fn parse_ingredient_list(
    text: &str,
    aisle: Option<&AisleConf>,
    model: Option<&dyn IngredientModel>,
) -> Vec<ParsedIngredient> {
    list::split_lines(text)
        .iter()
        .map(|line| parse_ingredient(line, aisle, model))
        .collect()
}
```

`rules.amount` is moved out of `rules` after `should_ask` borrowed it; the borrow ends at the `.filter` call, so this compiles. If the borrow checker disagrees, compute `let ask = model.is_some() && model::should_ask(line, &rules);` first.

- [ ] **Step 5: Run the tests**

Run: `cargo test -p cooklang-ingredient-parser`
Expected: PASS, including all 9 tests in `tests/parse.rs`.

- [ ] **Step 6: Commit**

```bash
git add ingredient-parser
git commit -m "feat(ingredient-parser): parse_ingredient with an optional model"
```

---

### Task 10: iOS parity table

**Files:**
- Create: `ingredient-parser/tests/ios_parity.rs`

Every case of `IngredientLineRecognizerTests.swift` at iOS commit `a6f8a1d1`, run through `parse_ingredient` without aisle or model. Names are compared after cleaning; none of these lines contains anything `cleaned_name` changes.

- [ ] **Step 1: Write the test**

```rust
//! The iOS recogniser's test cases (mobile-app-ios a6f8a1d1,
//! Packages/ShoppingList/Tests/ShoppingListTests/IngredientLineRecognizerTests.swift).
//! The two implementations must agree on every line.

use cooklang_ingredient_parser::parse_ingredient;
use test_case::test_case;

fn parsed(line: &str) -> (String, Option<f64>, Option<String>) {
    let p = parse_ingredient(line, None, None);
    let value = p.amount.as_ref().map(|a| a.value);
    (p.name, value, p.amount.and_then(|a| a.unit))
}

fn expect(name: &str, value: Option<f64>, unit: Option<&str>) -> (String, Option<f64>, Option<String>) {
    (name.to_owned(), value, unit.map(str::to_owned))
}

#[test_case("Milk 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk\u{00A0}2\u{202F}l", "Milk", Some(2.0), Some("l"))]
#[test_case("2 l milk", "milk", Some(2.0), Some("l"))]
#[test_case("Milk 2l", "Milk", Some(2.0), Some("l"))]
#[test_case("Sugar 1.5kg", "Sugar", Some(1.5), Some("kg"))]
#[test_case("Sugar 500 grams", "Sugar", Some(500.0), Some("g"))]
#[test_case("Сахар 200 гр", "Сахар", Some(200.0), Some("g"))]
#[test_case("Lemons 3", "Lemons", Some(3.0), None)]
#[test_case("3 lemons", "lemons", Some(3.0), None)]
#[test_case("Milk2l", "Milk2l", None, None)]
#[test_case("7up", "7up", None, None)]
#[test_case("Sugar500 grams", "Sugar500 grams", None, None)]
#[test_case("Diet 7up", "Diet 7up", None, None)]
#[test_case("7up 6", "7up", Some(6.0), None)]
#[test_case("Milk2l 3", "Milk2l", Some(3.0), None)]
#[test_case("2l milk", "milk", Some(2.0), Some("l"))]
#[test_case("3шт яйца", "яйца", Some(3.0), None)]
#[test_case("Lemons 1/2", "Lemons", Some(0.5), None)]
#[test_case("Milk 0", "Milk 0", None, None)]
#[test_case("Eggs 0 pcs", "Eggs 0 pcs", None, None)]
#[test_case("Flour 0/5 kg", "Flour 0/5 kg", None, None)]
#[test_case("Sugar", "Sugar", None, None)]
#[test_case("Milk 2 packs", "Milk 2 packs", None, None)]
#[test_case("Flour 1,5 kg", "Flour", Some(1.5), Some("kg"))]
#[test_case("1/2 kg flour", "flour", Some(0.5), Some("kg"))]
#[test_case("Flour 1/0 kg", "Flour 1/0 kg", None, None)]
#[test_case("Flour 1/2kg", "Flour", Some(0.5), Some("kg"))]
#[test_case("1/2kg flour", "flour", Some(0.5), Some("kg"))]
#[test_case("24/7 pizza", "24/7 pizza", None, None)]
#[test_case("500 g", "500 g", None, None)]
#[test_case("2", "2", None, None)]
#[test_case("", "", None, None)]
#[test_case("2 milk 3", "2 milk", Some(3.0), None)]
#[test_case("  Olive   oil  1 l ", "Olive oil", Some(1.0), Some("l"))]
#[test_case("Salt 1 TSP.", "Salt", Some(1.0), Some("tsp"))]
#[test_case("Соль 2 ч.л.", "Соль", Some(2.0), Some("tsp"))]
#[test_case("Eggs 6 pcs", "Eggs", Some(6.0), None)]
#[test_case("Яйца 10 шт", "Яйца", Some(10.0), None)]
#[test_case("Eier 6 Stück", "Eier", Some(6.0), None)]
#[test_case("potato, 2 kg", "potato", Some(2.0), Some("kg"))]
#[test_case("onion, 1", "onion", Some(1.0), None)]
#[test_case("Milk: 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk - 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk – 2l", "Milk", Some(2.0), Some("l"))]
#[test_case("Milk , 2 l", "Milk", Some(2.0), Some("l"))]
#[test_case("Vitamin C: 500", "Vitamin C", Some(500.0), None)]
#[test_case("olive oil, 2 Tablespoons, 50 ml", "olive oil, 2 Tablespoons", Some(50.0), Some("ml"))]
#[test_case("2 kg - potatoes", "potatoes", Some(2.0), Some("kg"))]
#[test_case("2 kg , potatoes", "potatoes", Some(2.0), Some("kg"))]
#[test_case("bread,", "bread", None, None)]
#[test_case("bread,,", "bread", None, None)]
#[test_case("bread;,", "bread", None, None)]
#[test_case(",bread", ",bread", None, None)]
#[test_case("Milk ,2", "Milk ,2", None, None)]
#[test_case("Milk 2 l,", "Milk", Some(2.0), Some("l"))]
#[test_case("Lemons 3;", "Lemons", Some(3.0), None)]
#[test_case("garlic, 1 head", "garlic, 1 head", None, None)]
#[test_case("Mix 50:50", "Mix 50:50", None, None)]
#[test_case("Note: buy early", "Note: buy early", None, None)]
#[test_case("Coca-Cola 2", "Coca-Cola", Some(2.0), None)]
#[test_case(", 2 kg", ", 2 kg", None, None)]
#[test_case("Milk, 2 l", "Milk", Some(2.0), Some("l"))]
fn recognises(line: &str, name: &str, value: Option<f64>, unit: Option<&str>) {
    assert_eq!(parsed(line), expect(name, value, unit));
}

#[test_case("g", &["g", "gr", "grams", "Gramm", "gramos", "grammes", "gramas", "г", "грамм"])]
#[test_case("kg", &["kg", "kgs", "kilo", "Kilogramm", "kilos", "quilos", "кг", "килограмм"])]
#[test_case("ml", &["ml", "milliliter", "millilitre", "mililitros", "мл"])]
#[test_case("l", &["l", "L", "liter", "litre", "litros", "л", "литр"])]
#[test_case("tsp", &["tsp", "tsps", "teaspoon", "TL", "cdta", "cdtas", "ч.л."])]
#[test_case("tbsp", &["tbsp", "tbsps", "tbs", "tablespoon", "EL", "cda", "cdas", "ст.л."])]
#[test_case("cup", &["cup", "cups", "Tasse", "taza", "tasses", "xícara", "xi\u{301}cara", "стакан"])]
#[test_case("oz", &["oz", "ounces", "Unze", "onzas", "once", "унция"])]
#[test_case("lb", &["lb", "lbs", "pounds", "Pfund", "libras", "livre", "фунт"])]
fn recognises_every_unit(stored: &str, words: &[&str]) {
    for word in words {
        assert_eq!(parsed(&format!("Thing 2 {word}")), expect("Thing", Some(2.0), Some(stored)), "{word}");
    }
}

#[test_case("pc")]
#[test_case("Stk")]
#[test_case("unidad")]
#[test_case("pièce")]
#[test_case("unidade")]
#[test_case("штук")]
fn a_piece_word_makes_a_count(word: &str) {
    assert_eq!(parsed(&format!("Thing 2 {word}")), expect("Thing", Some(2.0), None));
}

#[test]
fn a_number_too_big_for_f64_is_not_an_amount() {
    let line = format!("Flour {} l", "9".repeat(400));
    assert_eq!(parse_ingredient(&line, None, None).amount, None);
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p cooklang-ingredient-parser --test ios_parity`
Expected: PASS. If a case fails, compare the failing branch with the Swift source at `a6f8a1d1` (`git -C ../mobile-app-ios show a6f8a1d1:Packages/ShoppingList/Sources/ShoppingList/Core/IngredientLineRecognizer.swift`) and fix the Rust to match. Do not change an expected value.

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/tests/ios_parity.rs
git commit -m "test(ingredient-parser): run the iOS recogniser's cases"
```

---

### Task 11: Round trip through the Cooklang parsers

**Files:**
- Create: `ingredient-parser/tests/roundtrip.rs`

- [ ] **Step 1: Write the test**

```rust
//! A cleaned name, written as a free-hand `.shopping-list` line, reads back
//! as one item with that name and quantity; `edit_text` reads back through
//! the rules as the same item.

use cooklang::shopping_list::{self, IngredientItem, ShoppingList, ShoppingListItem};
use cooklang_ingredient_parser::{cleaned_name, edit_text, parse_ingredient};
use test_case::test_case;

#[test_case("Tomatoes {ripe}")]
#[test_case("salt -- sea")]
#[test_case("a [- b -] c")]
#[test_case("./Milk")]
#[test_case("50% cream")]
#[test_case("7up")]
#[test_case("Crème fraîche")]
#[test_case("a [-\u{301} b")]
#[test_case("a --\u{301} b")]
#[test_case("./\u{301}Milk")]
#[test_case("- salt")]
#[test_case("-]")]
#[test_case("a\tb")]
#[test_case("? milk")]
fn a_cleaned_name_reads_back_from_the_shopping_list(raw: &str) {
    let name = cleaned_name(raw);
    let items = vec![
        ShoppingListItem::Ingredient(IngredientItem { name: name.clone(), quantity: Some("2%l".into()), optional: false }),
        ShoppingListItem::Ingredient(IngredientItem { name: name.clone(), quantity: None, optional: false }),
    ];
    let list = ShoppingList { items };
    let mut text = Vec::new();
    shopping_list::write(&list, &mut text).unwrap();
    let text = String::from_utf8(text).unwrap();
    assert_eq!(shopping_list::parse(&text).unwrap(), list, "{text}");
}

#[test_case("Milk", Some("2%l"))]
#[test_case("Baking powder", Some("1"))]
#[test_case("Sugar", Some("1.5%kg"))]
#[test_case("Bread", None)]
fn edit_text_reads_back_as_the_same_item(name: &str, quantity: Option<&str>) {
    let parsed = parse_ingredient(&edit_text(name, quantity), None, None);
    assert_eq!(parsed.name, name);
    assert_eq!(parsed.amount.and_then(|a| a.to_quantity()).as_deref(), quantity);
}
```

- [ ] **Step 2: Run it**

Run: `cargo test -p cooklang-ingredient-parser --test roundtrip`
Expected: PASS. A failing `a_cleaned_name_reads_back_from_the_shopping_list` case means `cleaned_name` lets through something the shopping list parser reads differently; print `text` and the parsed list, and fix `cleaned_name` (and add the raw input to `name.rs` tests), not the test.

- [ ] **Step 3: Commit**

```bash
git add ingredient-parser/tests/roundtrip.rs
git commit -m "test(ingredient-parser): cleaned names and edit text round-trip"
```

---

### Task 12: uniffi bindings

**Files:**
- Create: `bindings/src/ingredient_parser.rs`
- Modify: `bindings/Cargo.toml`, `bindings/src/lib.rs`

The bindings already have a record named `Amount` (`bindings/src/model.rs:227`), so these records are prefixed `Ingredient…`.

- [ ] **Step 1: Add the dependency**

In `bindings/Cargo.toml` under `[dependencies]` add:

```toml
cooklang-ingredient-parser = { version = "0.20.0", path = "../ingredient-parser" }
```

- [ ] **Step 2: Write `bindings/src/ingredient_parser.rs`**

```rust
//! Typed ingredient lines for the apps: see the `cooklang-ingredient-parser`
//! crate. The aisle configuration is passed as its text, because this
//! crate's own `AisleConf` record is a flattened cache.

use cooklang::aisle::parse_lenient;
use cooklang_ingredient_parser as parser;

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IngredientAmount {
    pub value: f64,
    pub unit: Option<String>,
    /// Cooklang quantity syntax ("2%l"); `None` if it rounds to zero.
    pub quantity: Option<String>,
    /// "2 l", "x3".
    pub chip_text: String,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct IngredientAisleMatch {
    pub name: String,
    pub common_name: String,
    pub category: String,
}

#[derive(uniffi::Enum, Debug, Clone, Copy, PartialEq)]
pub enum IngredientParseSource {
    Rules,
    Model,
}

#[derive(uniffi::Record, Debug, Clone, PartialEq)]
pub struct ParsedIngredient {
    pub name: String,
    pub amount: Option<IngredientAmount>,
    pub size: Option<String>,
    pub prep: Option<String>,
    pub comment: Option<String>,
    pub confidence: f32,
    pub source: IngredientParseSource,
    pub aisle_match: Option<IngredientAisleMatch>,
}

impl From<parser::ParsedIngredient> for ParsedIngredient {
    fn from(p: parser::ParsedIngredient) -> Self {
        Self {
            name: p.name,
            amount: p.amount.map(|a| IngredientAmount {
                quantity: a.to_quantity(),
                chip_text: a.chip_text(),
                value: a.value,
                unit: a.unit,
            }),
            size: p.size,
            prep: p.prep,
            comment: p.comment,
            confidence: p.confidence,
            source: match p.source {
                parser::ParseSource::Model => IngredientParseSource::Model,
                _ => IngredientParseSource::Rules,
            },
            aisle_match: p.aisle_match.map(|m| IngredientAisleMatch {
                name: m.name,
                common_name: m.common_name,
                category: m.category,
            }),
        }
    }
}

/// Parses one typed ingredient line. `aisle_text` is the aisle.conf content.
#[uniffi::export]
pub fn parse_ingredient(line: String, aisle_text: Option<String>) -> ParsedIngredient {
    let aisle = aisle_text.as_deref().and_then(|t| parse_lenient(t).into_output());
    parser::parse_ingredient(&line, aisle.as_ref(), None).into()
}

/// Parses pasted text, one result per kept line.
#[uniffi::export]
pub fn parse_ingredient_list(text: String, aisle_text: Option<String>) -> Vec<ParsedIngredient> {
    let aisle = aisle_text.as_deref().and_then(|t| parse_lenient(t).into_output());
    parser::parse_ingredient_list(&text, aisle.as_ref(), None)
        .into_iter()
        .map(Into::into)
        .collect()
}

/// The sum of two stored quantities, or `None` when they cannot be added.
/// `Some(None)` cannot be expressed in uniffi, so two absent quantities
/// also give `None`; callers treat that as "keep one line".
#[uniffi::export]
pub fn add_ingredient_quantities(a: Option<String>, b: Option<String>) -> Option<String> {
    parser::add_quantities(a.as_deref(), b.as_deref()).ok().flatten()
}

#[uniffi::export]
pub fn ingredient_edit_text(name: String, quantity: Option<String>) -> String {
    parser::edit_text(&name, quantity.as_deref())
}

#[uniffi::export]
pub fn cleaned_ingredient_name(raw: String) -> String {
    parser::cleaned_name(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_aisle_text() {
        let parsed = parse_ingredient("Tomato 2".into(), Some("[veg]\ntomatoes\n".into()));
        assert_eq!(parsed.name, "tomatoes");
        assert_eq!(parsed.amount.unwrap().chip_text, "x2");
        assert_eq!(parsed.aisle_match.unwrap().category, "veg");
    }

    #[test]
    fn adds_quantities() {
        assert_eq!(add_ingredient_quantities(Some("2%l".into()), Some("1%l".into())), Some("3%l".into()));
        assert_eq!(add_ingredient_quantities(Some("2%l".into()), Some("2".into())), None);
    }
}
```

If `parse_lenient(..).into_output()` does not exist with that name, use what `parse_aisle_config` in `bindings/src/lib.rs` uses (`into_result()` and take `.0`).

- [ ] **Step 3: Register the module**

In `bindings/src/lib.rs`, next to `pub mod shopping_list;` add:

```rust
pub mod ingredient_parser;
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cooklang-bindings`
Expected: PASS (21 existing tests + 2 new).

- [ ] **Step 5: Commit**

```bash
git add bindings Cargo.lock
git commit -m "feat(bindings): expose the ingredient parser"
```

---

### Task 13: wasm bindings

**Files:**
- Modify: `typescript/Cargo.toml`, `typescript/src/lib.rs`, `typescript/index.ts`
- Create: `typescript/test/ingredientParser.test.ts`

- [ ] **Step 1: Add the dependency**

In `typescript/Cargo.toml` under `[dependencies]` add:

```toml
cooklang-ingredient-parser = { path = "../ingredient-parser", features = ["ts"] }
```

- [ ] **Step 2: Append the wasm functions to `typescript/src/lib.rs`**

```rust
/// Parses one typed ingredient line; `aisle_text` is the aisle.conf content.
#[wasm_bindgen]
pub fn parse_ingredient(
    line: &str,
    aisle_text: Option<String>,
) -> cooklang_ingredient_parser::ParsedIngredient {
    let aisle = aisle_text
        .as_deref()
        .and_then(|t| cooklang::aisle::parse_lenient(t).into_output());
    cooklang_ingredient_parser::parse_ingredient(line, aisle.as_ref(), None)
}

/// Parses pasted text, one result per kept line.
#[wasm_bindgen]
pub fn parse_ingredient_list(
    text: &str,
    aisle_text: Option<String>,
) -> Vec<cooklang_ingredient_parser::ParsedIngredient> {
    let aisle = aisle_text
        .as_deref()
        .and_then(|t| cooklang::aisle::parse_lenient(t).into_output());
    cooklang_ingredient_parser::parse_ingredient_list(text, aisle.as_ref(), None)
}

/// The stored form of an amount ("2%l"), for callers that only have the parse.
#[wasm_bindgen]
pub fn ingredient_edit_text(name: &str, quantity: Option<String>) -> String {
    cooklang_ingredient_parser::edit_text(name, quantity.as_deref())
}
```

If `Vec<ParsedIngredient>` is refused as a wasm return type, return `JsValue` built with `serde_wasm_bindgen::to_value(&list).unwrap()` and declare the return type with `#[wasm_bindgen(unchecked_return_type = "ParsedIngredient[]")]`.

- [ ] **Step 3: Re-export in `typescript/index.ts`**

Add `parse_ingredient, parse_ingredient_list, ingredient_edit_text` to the import list from `"./pkg/cooklang_wasm.js"` (line 1-10) and to the `export { ... }` block (line 12-24), and add `ParsedIngredient` to the `export type {...}` line (line 25).

- [ ] **Step 4: Write the vitest test**

Create `typescript/test/ingredientParser.test.ts`:

```ts
import {describe, expect, it} from "vitest";
import {parse_ingredient, parse_ingredient_list, ingredient_edit_text} from "../index.js";

describe("ingredient parser", () => {
    it("reads an amount at the end", () => {
        const parsed = parse_ingredient("Sugar 1.5kg", undefined);
        expect(parsed.name).toEqual("Sugar");
        expect(parsed.amount).toEqual({value: 1.5, unit: "kg"});
    });

    it("matches the aisle configuration", () => {
        const parsed = parse_ingredient("Tomato 2", "[veg]\ntomatoes\n");
        expect(parsed.name).toEqual("tomatoes");
        expect(parsed.aisle_match?.category).toEqual("veg");
    });

    it("splits a pasted list", () => {
        const names = parse_ingredient_list("- Milk 2 l\n\n* 3 lemons", undefined).map(p => p.name);
        expect(names).toEqual(["Milk", "lemons"]);
    });

    it("builds the edit text", () => {
        expect(ingredient_edit_text("Milk", "2%l")).toEqual("Milk 2 l");
    });
});
```

- [ ] **Step 5: Build and test**

Run: `cd typescript && npm run build-wasm && npm run build && npx vitest run && cd ..`
Expected: the build succeeds and vitest reports the 4 new tests passing along with the existing ones.

- [ ] **Step 6: Commit**

```bash
git add typescript Cargo.lock
git commit -m "feat(typescript): expose the ingredient parser to JS"
```

---

### Task 14: Publish step and docs

**Files:**
- Modify: `.github/workflows/release.yml` (job `publish-rust`, around line 776)
- Create: `ingredient-parser/README.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Publish the crate after `cooklang`**

In `.github/workflows/release.yml`, in job `publish-rust`, replace the step

```yaml
      - name: Publish to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
        run: cargo publish --no-verify
```

with

```yaml
      - name: Publish to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ steps.auth.outputs.token }}
        run: |
          cargo publish --no-verify -p cooklang
          cargo publish --no-verify -p cooklang-ingredient-parser
```

`cooklang-ingredient-parser` depends on the `cooklang` version just published; cargo waits for it to appear in the index before the second publish.

- [ ] **Step 2: Bump the crate with the workspace**

Check how a release bumps versions: `git log -p -1 --grep "release" -- Cargo.toml bindings/Cargo.toml`. Wherever the release process (script or manual checklist in `CHANGELOG.md`/README) bumps `bindings/Cargo.toml`, add `ingredient-parser/Cargo.toml` and its `cooklang` dependency version the same way. If the bump is manual, add one line to the release section of `README.md` naming `ingredient-parser/Cargo.toml`.

- [ ] **Step 3: Write `ingredient-parser/README.md`**

```markdown
# cooklang-ingredient-parser

Turns one typed ingredient line into a name and an amount, the same way in
every Cooklang app.

    use cooklang_ingredient_parser::parse_ingredient;

    let parsed = parse_ingredient("Sugar 1.5kg", None, None);
    assert_eq!(parsed.name, "Sugar");
    assert_eq!(parsed.amount.unwrap().to_quantity().as_deref(), Some("1.5%kg"));

- The amount can be at the end or the start ("Milk 2 l", "2l milk"), in six
  languages ("Сахар 200 гр"). A number glued to a word stays in the name ("7up").
- Pass an aisle configuration to take its spelling of a name, ignoring case
  and simple English plurals ("Tomato" → "tomatoes").
- Pass an `IngredientModel` (such as `ingredient-tagger`) to read lines that
  lead with a number the rules cannot place ("2 cans chopped tomatoes").
- `parse_ingredient_list` splits pasted text, dropping bullets, checkboxes
  and Markdown headings.
```

- [ ] **Step 4: Add a changelog entry**

At the top of the unreleased section of `CHANGELOG.md` (follow the file's existing heading style), add:

```markdown
- New crate `cooklang-ingredient-parser`: parses typed ingredient lines ("Milk 2 l") into a name and amount, with aisle matching and an optional model hook; exposed in the Swift/Kotlin bindings and the JS package.
```

- [ ] **Step 5: Full check**

Run: `cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: no formatting changes left, no clippy warnings, all tests pass. Fix any clippy findings in the new crate before committing.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release.yml ingredient-parser/README.md CHANGELOG.md README.md
git commit -m "chore: publish cooklang-ingredient-parser with each release"
```

---

## After this plan

- Open a PR for `feat/ingredient-parser`; after merge, a release publishes the crate.
- CookCLI's plan (`CookCLI/docs/superpowers/plans/2026-11-27-shopping-list-manual-items.md`) uses it through a path dependency until then.
- Follow-ups outside this plan: `ingredient-tagger` implements `IngredientModel` (cook-md/db); iOS replaces its Swift recogniser with the bindings.
