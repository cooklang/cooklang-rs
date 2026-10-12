# A quantity carries its resolved unit

A parsed `Quantity` records both the unit as the recipe wrote it and the [`Unit`](0002-unit-identifiers.md)
that spelling names in the recipe's locale, resolved once, by the parser. Operations that only need to
know what a unit *is* — adding, subtracting, grouping, comparing by meaning — take no converter. A
converter is needed only where there is a reader to write for: fitting after scaling, and writing a
list in its target locale. Decided 2026-10-12. This reverses one line of ADR 0009, "there is no `Unit`
id on `Quantity`"; the rest of that decision stands.

## Why

ADR 0009 kept the unit as text so that serialised recipes keep their shape and text units stay
representable. The price was that every operation on a quantity had to be handed the converter that
could read its text, and the caller had to know which one: a Swedish recipe's `5 dl` and `2 msk`
did not add through the parser's converter, and its `prep time: 30 minuter` read as nothing. The
operations on a recipe were made to derive the converter from the recipe's own `locale` key, which
fixed the recipe-level calls, but a `Quantity` compared or added on its own still needs a converter
it cannot name. The converter parameter was the price of the "text only" line, and it was being paid
in every signature.

Units are now a closed set with stable identifiers, and locales are part of the crate rather than a
units file a caller may or may not have loaded. So there is exactly one correct resolution of a unit
string, fixed at parse time by the recipe's naming. Storing it is recording a fact the parser already
established, not adding configuration.

## The type

```rust
pub struct Quantity {
    value: Value,
    unit: Option<WrittenUnit>,
    scalable: bool,
}

/// A unit as the recipe wrote it, and what it names when the recipe's
/// locale knows that spelling.
pub struct WrittenUnit {
    text: String,
    unit: Option<Unit>,
}
```

Text and resolution live in one value so there is no invariant between two sibling fields. `Unit` and
`PhysicalQuantity` become public, as ADR 0009 planned.

- `Quantity::unit()` is still the text. `Quantity::unit_id()` is what it names, if known.
- `Quantity::new(value, unit)` still builds an unresolved quantity. An unresolved unit behaves exactly
  as an unknown unit does today: it adds only to the same text, compares as text, and is never
  fitted. `Quantity::read_with(&Converter)` resolves it through that converter's locale, for the few
  callers that build quantities by hand.
- `PartialEq` stays "as written" and compares the text only. Two parses that resolved differently are
  the same text, which is what as-written means. The resolved unit is what `SemanticEq` reads.
- `Recipe::read_units()` resolves every quantity through the recipe's own naming, for a recipe
  deserialised from a document written before this change.

## The serialised shape

One additive field, `unit_id`, holding the spaced identifier of ADR 0002 and omitted when the unit is
unknown:

```json
{ "value": { "type": "number", "value": { "type": "regular", "value": 2 } },
  "unit": "msk", "unit_id": "metric tablespoon", "scalable": true }
```

Deserialising applies two rules: a `unit_id` with no `unit` text is an error, and a `unit_id` that
names no unit is an error, since the identifier set is closed and versioned with the crate. A
document without `unit_id` deserialises unresolved. The TypeScript `Quantity` type gains
`unit_id?: string`.

## What changes around it

- The parser resolves at the one place that already knows the locale, where it builds a quantity.
  The collector's own unit lookups go; the timer check and reference compatibility read the
  resolved unit.
- `Quantity::try_add`, `try_sub` and `compatible_unit`, and `GroupedQuantity::add`, `merge`,
  `try_saturating_sub` and `try_saturating_sub_group`, take no converter. Converting between two
  known units is `Unit::convert`; the result keeps the left operand's text, as before.
- `SemanticEq::equals(&self, other)` takes no converter on any type. A `Quantity` converts both
  known units to the metric base and compares there, else compares as text.
- `Converter` keeps `fit` and `convert_for`, which need an audience, and gains `rewrite`, which
  writes a resolved quantity in the converter's locale; `rewrite_from` goes, since the quantity now
  says what unit it is. A quantity with no resolved unit is an `UnknownUnit` error to all three.
- `Metadata::yield_quantity` resolves through the metadata's own naming, so scaling to a yield
  needs no lookup of its own.

## Costs

- A custom serde representation on `Quantity` where a derive did the job, and a change to the
  TypeScript type.
- `Unit`, `PhysicalQuantity` and `WrittenUnit` enter the public API, so adding a unit becomes a
  visible change. ADR 0001 already accepts that.
- Every `Quantity` built outside the parser must be resolved explicitly or it stays unknown. That is
  today's behaviour for an unknown unit, but it is a step a caller can forget, so `read_with`
  appears in every example that builds one.

## Considered and rejected

- Folding `SemanticEq` into `PartialEq`. `PartialEq` has to stay as-written: serde round trips and
  diffing two parses of one file depend on it, and Rust's `PartialEq` requires transitivity, which a
  tolerance-based comparison does not have.
- Two sibling fields, `unit: Option<String>` and `unit_id: Option<Unit>`. They carry an invariant
  nothing enforces; one value that holds both cannot disagree with itself.
- Resolving lazily, on first use, through the recipe's naming. A `Quantity` on its own has no
  recipe to ask, which is the problem being solved.
- Pantry subtraction matching by resolved unit instead of by text, so `1%l` in the pantry covers
  `500%ml` in the list. Follows from this decision but is a behaviour change of its own; not done
  here.
