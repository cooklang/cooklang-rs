# Shopping list quantities as Cooklang quantities

**Date:** 2026-11-27
**Status:** Approved in conversation (replace the field; no extensions; cooklang 0.22.0)
**Follows:** `2026-11-27-ingredient-parser-design.md`, cooklang/cookcli#860

## Problem

`cooklang::shopping_list` keeps a free-hand line's quantity as the raw text between the braces: `IngredientItem { name, quantity: Option<String>, optional }`, with `"4%l"`. Nothing public turns that text into a `Quantity` (the quantity parser is `pub(crate)`), so every caller fills the gap its own way:

- CookCLI merges free-hand lines into an `IngredientList` by writing `@{name}{{quantity}}` and parsing it as a one-line recipe. The name goes through the recipe parser too: `#1 flour` becomes cookware, `~ramen` a timer, `@milk` loses its `@`, and those items never reach the aisle list.
- The iOS app reads the strings with its own Swift amount reader.
- `cooklang-ingredient-parser` adds and edits quantities as strings.

## Change

### 1. The type

`IngredientItem.quantity` becomes `Option<Quantity>`. The same struct is used for top-level free-hand lines and for selection lines (`? name{…}`) nested under a recipe, so both change.

`ShoppingListItem`, `RecipeItem` and `ShoppingList` keep their shape. `collect_ingredient_names` is unchanged.

### 2. Reading

The text between the braces is parsed with the recipe quantity grammar and `Extensions::empty()`, the core spec without extensions (CookCLI's rule, cookcli#219, #502):

| Braces | `Quantity` |
| --- | --- |
| `{2%l}` | `Number(2)`, unit `l` |
| `{1/2%cup}` | `Number(Fraction 1/2)`, unit `cup` |
| `{3}` | `Number(3)`, no unit |
| `{a pinch}` | `Text("a pinch")` |
| `{1-2%cup}` | `Text("1-2")`, unit `cup` (ranges need `RANGE_VALUES`) |
| `{}` | `None`, as an empty quantity in a recipe |

A line never fails to parse because of its quantity. Whatever the grammar reports as an error or a warning for the brace content, the item gets the quantity the grammar produced, or, when it produced none, `Text` of the trimmed content with no unit, so that text another tool wrote survives. Errors about the name (an empty name, a top-level selection line) stay errors, as today.

The parsing goes through the crate's own quantity grammar, not through a recipe round trip, so the name is never parsed as Cooklang.

### 3. Writing

New public method, used by `shopping_list::write`:

```rust
impl Quantity {
    /// The quantity in Cooklang brace syntax, without the braces:
    /// "2%l", "1/2%cup", "3", "a pinch", "1-2%cup".
    pub fn to_cooklang(&self) -> String;
}
```

- `Number::Regular` is written in Rust's shortest round-trip form (`format!("{n}")` on the `f64`), never with the 3-decimal rounding of `Display`. Every value the parser reads, and every value `cooklang-ingredient-parser` writes (six decimals at most), comes back unchanged.
- `Number::Fraction` is written `num/den` or `whole num/den`.
- `Value::Range` is written `start-end` (only reachable when a caller builds one).
- `Value::Text` is written as is.
- The unit follows a `%`.
- A text value or unit containing `{`, `}` or a line break cannot be written back faithfully. These are replaced by spaces when writing, and a test pins that.

Consequence, accepted in the conversation: a file rewritten by the crate is normalised. `0.50%kg` becomes `0.5%kg`, `2 %l` becomes `2%l`, and `2%` becomes `2`. Values stay the same; only spellings change.

### 4. Adding a shopping list item to an `IngredientList`

```rust
impl IngredientList {
    /// Add a free-hand shopping list item under its own name, merging its
    /// quantity with the same name's like `add_ingredient` does. An item
    /// without a quantity adds an empty quantity.
    pub fn add_shopping_item(&mut self, item: &IngredientItem, converter: &Converter);
}
```

It is behind the `shopping_list` feature, and is what CookCLI calls instead of the recipe round trip.

### 5. `cooklang-ingredient-parser`

The crate stops passing quantities around as strings:

- `Amount::to_quantity(&self) -> Option<cooklang::quantity::Quantity>`: a `Number::Regular` rounded to six decimals, as today, so the iOS formatting rule still holds; `None` when it rounds to zero.
- `Amount::from_quantity(&Quantity) -> Option<Amount>`: a positive number with an optional unit; `None` for text, ranges and fractions of zero.
- `add_quantities(a: Option<&Quantity>, b: Option<&Quantity>) -> Result<Option<Quantity>, NotAddable>`: the same rule (both numbers, units equal exactly, or both absent).
- `edit_text(name, quantity: Option<&Quantity>)`: the same rule as today.
- `ParsedIngredient` is unchanged apart from `amount`'s methods.

The crate's dependency on `cooklang` gains the `shopping_list` feature only if a function needs `IngredientItem`; it should not.

### 6. Bindings

- **uniffi.** The bindings' shopping list `IngredientItem` record's `quantity: Option<String>` becomes `Option<Amount>`, the record recipes already use (`Amount { quantity: Value, units }`). Reading maps `Quantity` to `Amount` as recipe ingredients do. Writing maps it back: `Value::Number { value }` becomes `Number::Regular(value)`, so a fraction an app reads and writes back is written as a decimal (`1/2` becomes `0.5`). That is accepted; it changes no value.
- **Ingredient-parser functions** in the bindings (`add_ingredient_quantities`, `ingredient_edit_text`, `IngredientAmount.quantity`) take and give `Amount` instead of strings.
- **wasm.** The same shape through tsify: `quantity` becomes the `Quantity` type the recipe API already exports.

### 7. Version and rollout

This is breaking for Rust, Swift, Kotlin and JS callers, so it ships as **0.22.0**. Then:

1. `cooklang-reports` and `cooklang-language-server` move to `cooklang` ^0.22, if they compile against the changed types; they likely do not touch `IngredientItem`.
2. CookCLI moves to all of them in one PR and:
   - replaces `parse_extra_item` / `add_extra_item` with `add_shopping_item`, which fixes the `#` / `~` / `@` names;
   - keeps its JSON API's quantities as strings via `to_cooklang()`, so the web page does not change.
3. The iOS and Android apps adapt their shopping list code when they take 0.22. Until then they stay on 0.21.

## Testing

- **Parse table**: every row of the table in section 2, plus `{ 2 % l }`, `{0.50%kg}`, `{2%}`, `{%l}`, `{1,5%kg}` (not a number in Cooklang, so text), and a nested selection line. The names in every case are unchanged by the quantity.
- **Write table**: `to_cooklang` for regular numbers (`2`, `1.5`, `0.0004`, `1/3` as a float giving its shortest form), fractions, ranges, text, a unit, and the `{`/`}`/newline replacement.
- **Round trip**: for every parse-table input, `parse(write(parse(x))) == parse(x)`. For inputs already in normal form, `write(parse(x)) == x` byte for byte, including the existing `shopping_list` tests' files.
- **`add_shopping_item`**:
  - `#1 flour{2}`, `~ramen`, `@milk{1%l}` keep their names;
  - same name and unit merge;
  - different units are grouped as `add_ingredient` groups them;
  - a text quantity is kept.
- **Ingredient parser**: the existing tables, ported to `Quantity`. The iOS parity table is unchanged: it compares values, not strings. The six-decimal rounding still holds through `to_quantity` and then `to_cooklang`.
- **Bindings**: a shopping list written by the bindings reads back with the same values, and an `Amount` round-trips except for fraction → decimal.

## Out of scope

- An `Extensions` argument for the shopping list parser (add it when something needs ranges as ranges).
- Changing how CookCLI or the apps store or show lists, beyond adapting to the type.
