# Shopping List Quantities as Cooklang Quantities — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `cooklang::shopping_list::IngredientItem.quantity` becomes `Option<Quantity>` (parsed with the recipe quantity grammar, written back in brace syntax), `IngredientList::add_shopping_item` merges a free-hand item under its real name, and `cooklang-ingredient-parser`, the uniffi bindings and the wasm package move from quantity strings to quantities.

**Architecture:** A crate-internal `parse_quantity_text` runs the existing `parser::quantity::parse_quantity` on the brace content alone (tokens of that text only, `Extensions::empty()`), so names are never parsed as Cooklang. `Quantity::to_cooklang` writes it back with full float precision. Everything downstream changes type only.

**Tech Stack:** Rust 2021, uniffi 0.32, wasm-bindgen + tsify, vitest.

**Spec:** `docs/superpowers/specs/2026-11-27-shopping-list-quantity-design.md`

**Branch:** `feat/shopping-list-quantity` (spec committed). Run cargo from `/Users/dubadub/Cooklang/cooklang-rs`.

---

## File structure

```
src/quantity.rs                     + Quantity::to_cooklang (and Number/Value helpers)
src/parser/quantity.rs              + pub(crate) parse_quantity_text
src/shopping_list.rs                IngredientItem.quantity: Option<Quantity>; read/write; tests
src/ingredient_list.rs              + IngredientList::add_shopping_item (feature shopping_list)
ingredient-parser/src/quantity.rs   Amount::{to_quantity,from_quantity}, add_quantities, edit_text on Quantity
ingredient-parser/tests/*.rs        adapt
bindings/src/model.rs               + amount_to_quantity / quantity_to_amount
bindings/src/shopping_list.rs       Ingredient.quantity: Option<Amount>
bindings/src/ingredient_parser.rs   Amount-based functions
typescript/src/lib.rs, index.ts, test/ingredientParser.test.ts
CHANGELOG.md
```

---

### Task 1: `Quantity::to_cooklang`

**Files:** Modify `src/quantity.rs` (after `impl Display for Quantity`, ~line 176)

- [ ] **Step 1: Write the failing tests**

Add at the end of `src/quantity.rs` (inside its existing `#[cfg(test)] mod tests` if there is one; otherwise create `#[cfg(test)] mod to_cooklang_tests { use super::*; … }`):

```rust
    fn q(value: Value, unit: Option<&str>) -> Quantity {
        Quantity::new(value, unit.map(str::to_string))
    }

    #[test]
    fn to_cooklang_numbers() {
        assert_eq!(q(Value::Number(Number::Regular(2.0)), Some("l")).to_cooklang(), "2%l");
        assert_eq!(q(Value::Number(Number::Regular(1.5)), Some("kg")).to_cooklang(), "1.5%kg");
        assert_eq!(q(Value::Number(Number::Regular(0.0004)), Some("kg")).to_cooklang(), "0.0004%kg");
        assert_eq!(q(Value::Number(Number::Regular(1.0 / 3.0)), None).to_cooklang(), "0.3333333333333333");
        assert_eq!(q(Value::Number(Number::Regular(3.0)), None).to_cooklang(), "3");
        assert_eq!(q(Value::Number(Number::Regular(1e21)), None).to_cooklang(), "1000000000000000000000");
    }

    #[test]
    fn to_cooklang_fractions() {
        let half = Number::Fraction { whole: 0, num: 1, den: 2, err: 0.0 };
        let one_and_half = Number::Fraction { whole: 1, num: 1, den: 2, err: 0.0 };
        let lossy = Number::Fraction { whole: 0, num: 1, den: 3, err: 0.01 };
        assert_eq!(q(Value::Number(half), Some("cup")).to_cooklang(), "1/2%cup");
        assert_eq!(q(Value::Number(one_and_half), Some("cup")).to_cooklang(), "1 1/2%cup");
        // A fraction carrying an error is written as its exact value.
        assert_eq!(q(Value::Number(lossy), None).to_cooklang(), format!("{}", 1.0 / 3.0 + 0.01));
    }

    #[test]
    fn to_cooklang_ranges_and_text() {
        let range = Value::Range { start: Number::Regular(1.0), end: Number::Regular(2.0) };
        assert_eq!(q(range, Some("cup")).to_cooklang(), "1-2%cup");
        assert_eq!(q(Value::Text("a pinch".into()), None).to_cooklang(), "a pinch");
        assert_eq!(q(Value::Text("1-2".into()), Some("cup")).to_cooklang(), "1-2%cup");
    }

    #[test]
    fn to_cooklang_never_breaks_the_braces_or_the_line() {
        assert_eq!(q(Value::Text("a {big}\npinch".into()), Some("c}u\rp")).to_cooklang(), "a  big  pinch%c u p");
    }
```

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p cooklang --lib to_cooklang`
Expected: FAIL to compile (`no method named to_cooklang`).

- [ ] **Step 3: Implement**

After `impl Display for Quantity`:

```rust
/// `{`, `}` and line breaks would end the braces or the line: spaces instead.
fn brace_safe(text: &str) -> String {
    text.chars()
        .map(|c| if matches!(c, '{' | '}' | '\n' | '\r') { ' ' } else { c })
        .collect()
}

impl Number {
    /// The number in Cooklang syntax, exactly: a regular number in its
    /// shortest round-trip form (never rounded like [`Display`]), a fraction
    /// as `num/den` or `whole num/den`, or its exact value when the fraction
    /// carries an error.
    fn to_cooklang(&self) -> String {
        match *self {
            Number::Regular(n) => format!("{n}"),
            Number::Fraction { whole, num, den, err } if err == 0.0 => match (whole, num) {
                (whole, 0) => format!("{whole}"),
                (0, num) => format!("{num}/{den}"),
                (whole, num) => format!("{whole} {num}/{den}"),
            },
            Number::Fraction { .. } => format!("{}", self.value()),
        }
    }
}

impl Value {
    fn to_cooklang(&self) -> String {
        match self {
            Value::Number(n) => n.to_cooklang(),
            Value::Range { start, end } => format!("{}-{}", start.to_cooklang(), end.to_cooklang()),
            Value::Text(t) => brace_safe(t),
        }
    }
}

impl Quantity {
    /// The quantity in Cooklang brace syntax, without the braces: `2%l`,
    /// `1/2%cup`, `3`, `a pinch`, `1-2%cup`. Reads back as the same value.
    pub fn to_cooklang(&self) -> String {
        let value = self.value.to_cooklang();
        match &self.unit {
            Some(unit) => format!("{value}%{}", brace_safe(unit)),
            None => value,
        }
    }
}
```

`Number::value(self)` takes `self` by value (line ~109); `Number` is `Copy`? If not, use `self.clone().value()`.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cooklang --lib to_cooklang`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src/quantity.rs && git commit -m "feat: write a Quantity in Cooklang brace syntax"
```

---

### Task 2: Parse a quantity from its brace text alone

**Files:** Modify `src/parser/quantity.rs`

- [ ] **Step 1: Write the failing tests**

In the `#[cfg(test)] mod tests` of `src/parser/quantity.rs` add:

```rust
    use crate::quantity::{Number as QNumber, Quantity as Q, Value as QValue};

    fn text_q(input: &str) -> Option<Q> {
        super::parse_quantity_text(input)
    }

    fn num_q(n: f64, unit: Option<&str>) -> Option<Q> {
        Some(Q::new(QValue::Number(QNumber::Regular(n)), unit.map(str::to_string)))
    }

    #[test]
    fn quantity_text_table() {
        assert_eq!(text_q("2%l"), num_q(2.0, Some("l")));
        assert_eq!(text_q(" 2 % l "), num_q(2.0, Some("l")));
        assert_eq!(text_q("3"), num_q(3.0, None));
        assert_eq!(text_q("0.50%kg"), num_q(0.5, Some("kg")));
        assert_eq!(text_q("1/2%cup").unwrap().value(), &QValue::Number(QNumber::Fraction { whole: 0, num: 1, den: 2, err: 0.0 }));
        assert_eq!(text_q("a pinch"), Some(Q::new(QValue::Text("a pinch".into()), None)));
        assert_eq!(text_q("1-2%cup"), Some(Q::new(QValue::Text("1-2".into()), Some("cup".into()))));
        assert_eq!(text_q("1,5%kg").unwrap().value(), &QValue::Text("1,5".into()));
        assert_eq!(text_q("2%"), num_q(2.0, None));
        assert_eq!(text_q(""), None);
        assert_eq!(text_q("   "), None);
    }

    #[test]
    fn quantity_text_with_only_a_unit_is_text_or_empty_but_never_panics() {
        // "%l": no value. Whatever the grammar makes of it, it must not panic
        // and must keep the unit if it keeps anything.
        let q = text_q("%l");
        if let Some(q) = q {
            assert_eq!(q.unit(), Some("l"));
        }
    }
```

If one of these expectations differs from what the grammar actually produces (e.g. `1,5` or `2%`), check `parse_regular_quantity`/`value()` in this file: the spec says "the quantity the grammar produced"; change the test to what the grammar produces only if it's a sensible quantity, and report it. `1-2` must stay text (no `RANGE_VALUES`).

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p cooklang --lib quantity_text`
Expected: FAIL to compile.

- [ ] **Step 3: Implement**

At the end of the non-test part of `src/parser/quantity.rs`:

```rust
/// Parse the text between an ingredient's braces on its own, with no
/// extensions: the quantity of a shopping list line. `None` for empty text.
/// Never fails: whatever the grammar makes of the text (text values for
/// anything that isn't a number) is the quantity; diagnostics are dropped.
pub(crate) fn parse_quantity_text(text: &str) -> Option<crate::quantity::Quantity> {
    if text.trim().is_empty() {
        return None;
    }
    let tokens = super::token_stream::TokenStream::new(text).collect::<Vec<_>>();
    let mut events = std::collections::VecDeque::new();
    let mut bp = BlockParser::new(&tokens, text, &mut events, Extensions::empty());
    let parsed = parse_quantity(&mut bp, &tokens);
    bp.consume_rest();
    bp.finish();
    let quantity = parsed.quantity.into_inner();
    let value = quantity.value.value.into_inner();
    let unit = quantity
        .unit
        .map(|unit| unit.text_trimmed().into_owned())
        .filter(|unit| !unit.is_empty());
    Some(crate::quantity::Quantity::new(value, unit))
}
```

Adjust paths to what compiles (the test macro `t!` at ~line 337 shows the exact setup; `TokenStream` and `BlockParser` imports may already be in scope). If `TokenStream::new(text)` yields whitespace tokens that make `parse_quantity` assert on empty tokens, the `trim().is_empty()` guard covers it.

- [ ] **Step 4: Run the tests**

Run: `cargo test -p cooklang --lib quantity_text`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/parser/quantity.rs && git commit -m "feat: parse a quantity from its brace text alone"
```

---

### Task 3: `IngredientItem.quantity: Option<Quantity>`

**Files:** Modify `src/shopping_list.rs`; `src/parser/mod.rs` if `parse_quantity_text` needs re-exporting to `crate::parser::quantity` visibility (`pub(crate) mod quantity` — check).

- [ ] **Step 1: Change the type and read/write**

In `src/shopping_list.rs`:
- `IngredientItem.quantity: Option<Quantity>` (import `crate::quantity::Quantity`), doc: "Parsed with the recipe quantity grammar and no extensions; written back with [`Quantity::to_cooklang`]".
- `parse_ingredient_line` returns `(String, Option<Quantity>)`: replace `Some(quantity)` with `crate::parser::quantity::parse_quantity_text(&quantity_text)` where `quantity_text = &content[brace_start + 1..content.len() - 1]`.
- In `write_items`, `write!(w, "{{{q}}}")` becomes `write!(w, "{{{}}}", q.to_cooklang())`.
- `Serialize`/`Deserialize` derives keep working: `Quantity` is serde.

- [ ] **Step 2: Fix the existing tests**

Every test that builds or compares `quantity: Some("…".into())` / `.as_deref() == Some("…")`: add a test helper

```rust
    fn qty(text: &str) -> Option<Quantity> {
        crate::parser::quantity::parse_quantity_text(text)
    }
```

and write `quantity: qty("4%l")`, and compare `i.quantity, qty("4%l")`. Keep every expected file text unchanged: those are already in normal form, so they must still round-trip byte for byte.

- [ ] **Step 3: Add the spec's tests**

```rust
    #[test]
    fn quantities_are_parsed_and_names_are_not() {
        let list = parse("#1 flour{2}\n~ramen\n@milk{1%l}\nsalt{a pinch}\nflour{1-2%cup}\n").unwrap();
        let items: Vec<(&str, Option<String>)> = list
            .items
            .iter()
            .map(|item| match item {
                ShoppingListItem::Ingredient(i) => (i.name.as_str(), i.quantity.as_ref().map(|q| q.to_cooklang())),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            items,
            [
                ("#1 flour", Some("2".to_string())),
                ("~ramen", None),
                ("@milk", Some("1%l".to_string())),
                ("salt", Some("a pinch".to_string())),
                ("flour", Some("1-2%cup".to_string())),
            ]
        );
    }

    #[test]
    fn a_rewrite_normalises_spellings_but_keeps_values() {
        let list = parse("butter{0.50%kg}\nmilk{ 2 % l }\neggs{2%}\n").unwrap();
        let mut out = Vec::new();
        write(&list, &mut out).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "butter{0.5%kg}\nmilk{2%l}\neggs{2}\n");
    }

    #[test]
    fn a_selection_line_quantity_is_parsed_too() {
        let list = parse("./Soup{2}\n  ? cream{100%ml}\n").unwrap();
        let ShoppingListItem::Recipe(recipe) = &list.items[0] else { panic!() };
        let ShoppingListItem::Ingredient(cream) = &recipe.children[0] else { panic!() };
        assert!(cream.optional);
        assert_eq!(cream.quantity.as_ref().map(|q| q.to_cooklang()).as_deref(), Some("100%ml"));
    }

    #[test]
    fn parse_write_parse_is_stable() {
        for input in ["a{2%l}\n", "b{1/2%cup}\n", "c{a pinch}\n", "d{1-2%cup}\n", "e{0.50%kg}\n", "f\n", "g{1,5%kg}\n"] {
            let once = parse(input).unwrap();
            let mut out = Vec::new();
            write(&once, &mut out).unwrap();
            let twice = parse(std::str::from_utf8(&out).unwrap()).unwrap();
            assert_eq!(once, twice, "{input}");
        }
    }
```

Check how the existing tests write a recipe line with a multiplier (`./Soup{2}` vs another spelling) and match it.

- [ ] **Step 4: Run**

Run: `cargo test -p cooklang --lib shopping_list`
Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add src && git commit -m "feat!: parse shopping list quantities as Cooklang quantities"
```

---

### Task 4: `IngredientList::add_shopping_item`

**Files:** Modify `src/ingredient_list.rs` (after `add_ingredient`, ~line 337)

- [ ] **Step 1: Failing test** (in `src/ingredient_list.rs` tests, or a new `tests/shopping_item_list.rs` with `#![cfg(feature = "shopping_list")]`):

```rust
#[cfg(feature = "shopping_list")]
#[test]
fn shopping_items_keep_their_names_and_merge() {
    use crate::shopping_list::{parse, ShoppingListItem};
    use crate::Converter;
    let list = parse("#1 flour{2}\n~ramen\n@milk{1%l}\nmilk{500%ml}\n@milk{1%l}\nsalt{a pinch}\n").unwrap();
    let converter = Converter::empty();
    let mut ingredients = IngredientList::default();
    for item in &list.items {
        if let ShoppingListItem::Ingredient(item) = item {
            ingredients.add_shopping_item(item, &converter);
        }
    }
    let names: Vec<&str> = ingredients.iter().map(|(name, _)| name.as_str()).collect();
    assert!(names.contains(&"#1 flour"));
    assert!(names.contains(&"~ramen"));
    assert!(names.contains(&"@milk"));
    assert!(names.contains(&"milk"));
    assert!(names.contains(&"salt"));
    let at_milk = ingredients.iter().find(|(n, _)| n.as_str() == "@milk").unwrap().1;
    assert_eq!(at_milk.to_string(), "2 l");
}
```

Adapt to `IngredientList`'s real API (how it iterates and what its `GroupedQuantity` displays — check `src/ingredient_list.rs` and `GroupedQuantity`'s `Display`); keep the asserts' intent: names unchanged, same name + unit summed.

- [ ] **Step 2: Run to see it fail** — `cargo test -p cooklang shopping_items_keep` → no method.

- [ ] **Step 3: Implement**

```rust
    /// Add a free-hand shopping list item under its own name, its quantity
    /// merged with the same name's as [`add_ingredient`](Self::add_ingredient)
    /// does. An item without a quantity adds an empty one, so it is listed.
    #[cfg(feature = "shopping_list")]
    pub fn add_shopping_item(
        &mut self,
        item: &crate::shopping_list::IngredientItem,
        converter: &Converter,
    ) {
        let mut quantity = GroupedQuantity::empty();
        if let Some(q) = &item.quantity {
            quantity.add(q, converter);
        }
        self.add_ingredient(item.name.clone(), &quantity, converter);
    }
```

- [ ] **Step 4: Run** — PASS. Also `cargo test -p cooklang --no-default-features --lib` builds without the feature.

- [ ] **Step 5: Commit** — `git commit -m "feat: add a shopping list item to an IngredientList under its own name"`

---

### Task 5: `cooklang-ingredient-parser` on `Quantity`

**Files:** `ingredient-parser/src/quantity.rs`, `ingredient-parser/src/lib.rs`, `ingredient-parser/tests/roundtrip.rs`, `ingredient-parser/README.md`

- [ ] **Step 1: Change the API**

In `quantity.rs`:
- `Amount::to_quantity(&self) -> Option<cooklang::quantity::Quantity>`: same checks as today (finite, > 0, six-decimal `format_number` not `"0"`), value `Number::Regular(format_number(v).parse().unwrap())` so the six-decimal rounding still applies, unit as is.
- `Amount::from_quantity(q: &Quantity) -> Option<Amount>`: `Value::Number(n)` with `n.value()` finite and > 0 → `Amount { value, unit: q.unit().map(str::to_owned) }`; anything else `None`. Remove the string parser's `trim_spaces` if nothing else uses it.
- `add_quantities(a: Option<&Quantity>, b: Option<&Quantity>) -> Result<Option<Quantity>, NotAddable>`: same rule via `from_quantity`; sum through `to_quantity`.
- `edit_text(name, quantity: Option<&Quantity>)`: same rule via `from_quantity` and the table-unit check.
- `chip_text`/`display` unchanged.

Keep a string convenience only if the bindings or tests need it — otherwise none.

- [ ] **Step 2: Port the tests**

Rewrite the `quantity.rs` unit tests on `Quantity` values (build them with a small helper `fn q(text) -> Option<Quantity>` that parses `"{text}"` via `cooklang::shopping_list::parse(&format!("x{{{text}}}\n"))` — add `shopping_list` to the crate's dev-dependency features, already there). Keep every expected value; where a test checked a string like `"2%l"`, compare `to_cooklang()` output instead. `from_quantity("1,000%g")` becomes: the shopping list parser reads `1,000` as text, so `from_quantity` gives `None` — same outcome.

`tests/roundtrip.rs`: `IngredientItem { quantity: … }` now takes `Option<Quantity>`; build it with the same helper; `edit_text_reads_back_as_the_same_item` compares `to_cooklang()`.

`tests/ios_parity.rs` and `tests/parse.rs` should need no change (they compare `value`/`unit`); fix compile errors only.

- [ ] **Step 3: Run**

Run: `cargo test -p cooklang-ingredient-parser`
Expected: all suites pass with the same counts as before (plus/minus tests you merged), including the doctest from README (update its example: `parsed.amount.unwrap().to_quantity().map(|q| q.to_cooklang()).as_deref() == Some("1.5%kg")`).

- [ ] **Step 4: Commit** — `git commit -m "feat(ingredient-parser)!: quantities are Cooklang quantities"`

---

### Task 6: uniffi bindings

**Files:** `bindings/src/model.rs`, `bindings/src/shopping_list.rs`, `bindings/src/ingredient_parser.rs`

- [ ] **Step 1: Conversions** in `bindings/src/model.rs` (next to `Amountable`):

```rust
/// A quantity as the bindings show it.
pub(crate) fn quantity_to_amount(q: &OriginalQuantity) -> Amount {
    q.extract_amount()
}

/// An `Amount` back to a quantity: numbers become regular numbers, so a
/// fraction read and written back is written as a decimal. `Empty` is none.
pub(crate) fn amount_to_quantity(a: &Amount) -> Option<OriginalQuantity> {
    use cooklang::quantity::Number;
    let value = match &a.quantity {
        Value::Number { value } => OriginalValue::Number(Number::Regular(*value)),
        Value::Range { start, end } => OriginalValue::Range { start: Number::Regular(*start), end: Number::Regular(*end) },
        Value::Text { value } => OriginalValue::Text(value.clone()),
        Value::Empty => return None,
    };
    Some(OriginalQuantity::new(value, a.units.clone()))
}
```

(Use the existing import names for the cooklang `Quantity`/`Value` in that file.)

- [ ] **Step 2: Shopping list record** — `ShoppingListItem::Ingredient { quantity: Option<Amount>, … }`; reading maps with `quantity_to_amount`, writing with `amount_to_quantity` (`Option` → `and_then`). Update the doc comment.

- [ ] **Step 3: Ingredient-parser functions**
- `IngredientAmount.quantity: Option<Amount>` (was the stored string), from `a.to_quantity().as_ref().map(quantity_to_amount)`.
- `add_ingredient_quantities(a: Option<Amount>, b: Option<Amount>) -> Result<Option<Amount>, IngredientQuantityError>`.
- `ingredient_edit_text(name: String, quantity: Option<Amount>) -> String`.
- Add `#[uniffi::export] pub fn amount_to_cooklang(amount: Amount) -> String` (for apps that show or compare the stored form): `amount_to_quantity(&amount).map(|q| q.to_cooklang()).unwrap_or_default()`.

- [ ] **Step 4: Tests** — update the bindings tests; add: a shopping list `milk{2%l}\nsalt{a pinch}\n` parsed and written back through the bindings' records is unchanged; `cream{1/2%cup}` written back through the records becomes `cream{0.5%cup}`.

- [ ] **Step 5: Run** — `cargo test -p cooklang-bindings`; generate Swift once to check it builds: `cargo build -p cooklang-bindings && cargo run -p cooklang-bindings --features uniffi/cli --bin uniffi-bindgen -- generate --library target/debug/libcooklang_bindings.dylib --language swift --out-dir /private/tmp/claude-501/-Users-dubadub-Cooklang-CookCLI/112e81b8-699b-49cb-a30a-4eed7d7c2d07/scratchpad/swift-0.22`.

- [ ] **Step 6: Commit** — `git commit -m "feat(bindings)!: shopping list quantities are amounts"`

---

### Task 7: wasm

**Files:** `typescript/src/lib.rs`, `typescript/index.ts`, `typescript/test/ingredientParser.test.ts`

- [ ] **Step 1:** In `typescript/src/lib.rs`:
- `add_ingredient_quantities(a: Option<Quantity>, b: Option<Quantity>) -> Result<Option<Quantity>, JsError>` (cooklang's `Quantity` derives tsify `from_wasm_abi`/`into_wasm_abi`).
- `ingredient_edit_text(name: &str, quantity: Option<Quantity>) -> String`.
- `ingredient_amount_quantity(value, unit) -> Option<Quantity>`.
- add `quantity_to_cooklang(q: Quantity) -> String`.
If tsify refuses `Option<Quantity>` parameters, take `JsValue` and `serde_wasm_bindgen::from_value` (and return `JsValue` with `unchecked_return_type`).

- [ ] **Step 2:** Export `quantity_to_cooklang` from `index.ts`; update the vitest tests: add 2%l + 1%l gives a quantity whose `quantity_to_cooklang` is `"3%l"`; 2%l + 2 throws; `ingredient_edit_text("Milk", ingredient_amount_quantity(2, "l"))` is `"Milk 2 l"`.

- [ ] **Step 3:** `cd typescript && npm ci && npm run build-wasm && npm run build && npx vitest run && cd ..` → all pass.

- [ ] **Step 4: Commit** — `git commit -m "feat(typescript)!: ingredient parser quantities are Quantity objects"`

---

### Task 8: Changelog and full check

- [ ] **Step 1:** In `CHANGELOG.md` under `## Unreleased - ReleaseDate` add:

```markdown
- (breaking) Shopping lists: `IngredientItem.quantity` is an `Option<Quantity>`, parsed with the recipe quantity grammar (no extensions) and written back with the new `Quantity::to_cooklang`. Rewriting a list normalises spellings (`0.50%kg` → `0.5%kg`); values are unchanged. Names are no longer at risk of being read as Cooklang anywhere
- `IngredientList::add_shopping_item` adds a free-hand shopping list item under its own name
- (breaking) `cooklang-ingredient-parser`: `Amount::to_quantity`, `add_quantities` and `edit_text` work on `Quantity` instead of strings
- (breaking) bindings: shopping list ingredient quantities and the ingredient parser's quantities are `Amount` records; `amount_to_cooklang` gives the stored form. A fraction an app reads and writes back is written as a decimal. typescript: the same functions take and return `Quantity`
```

- [ ] **Step 2:** `cargo fmt --all && cargo clippy -p cooklang -p cooklang-ingredient-parser -p cooklang-bindings --all-targets && cargo test --workspace --no-fail-fast`. No new warnings in changed files (pre-existing warnings in `src/metadata.rs`, `src/quantity.rs:513-514`, `bindings/src/lib.rs` are known); all pass.

- [ ] **Step 3: Commit** — `git commit -m "docs: changelog for Cooklang quantities in shopping lists"`

## After this plan

PR → review → merge → `release: 0.22.0` (bump `Cargo.toml`, `bindings/Cargo.toml`, `ingredient-parser/Cargo.toml` and its `cooklang` dep, bindings' parser dep) → Release workflow. Then cooklang-reports / cooklang-language-server bumps and the CookCLI PR that uses `add_shopping_item` (see the spec's rollout).
