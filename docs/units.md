# Units

Generated from the unit definitions and the preference rows. **Do not edit**: run
`UPDATE_UNITS_DOC=1 cargo test` to refresh it.

Unit names are seeded from CLDR once per locale and hand-edited since; each
`locales/<tag>.toml` records which release it was seeded from.

## Units

Sizes are given against each quantity's reference unit. Temperature is affine,
so it has no ratio.

| Identifier | Quantity | Size |
|---|---|---|
| `kiloliter` | Volume | 1000 liter |
| `hectoliter` | Volume | 100 liter |
| `decaliter` | Volume | 10 liter |
| `liter` | Volume | 1 liter |
| `deciliter` | Volume | 0.1 liter |
| `centiliter` | Volume | 0.01 liter |
| `milliliter` | Volume | 0.001 liter |
| `us teaspoon` | Volume | 0.00492892159375 liter |
| `us tablespoon` | Volume | 0.01478676478125 liter |
| `us fluid ounce` | Volume | 0.0295735295625 liter |
| `us cup` | Volume | 0.2365882365 liter |
| `us pint` | Volume | 0.473176473 liter |
| `us quart` | Volume | 0.946352946 liter |
| `us gallon` | Volume | 3.785411784 liter |
| `imperial fluid ounce` | Volume | 0.0284130625 liter |
| `imperial pint` | Volume | 0.56826125 liter |
| `imperial quart` | Volume | 1.1365225 liter |
| `imperial gallon` | Volume | 4.54609 liter |
| `metric teaspoon` | Volume | 0.005 liter |
| `metric dessert spoon` | Volume | 0.01 liter |
| `metric tablespoon` | Volume | 0.015 liter |
| `metric cup` | Volume | 0.25 liter |
| `nordic spice measure` | Volume | 0.001 liter |
| `danish knife point` | Volume | 0.00025 liter |
| `australian tablespoon` | Volume | 0.02 liter |
| `japanese cup` | Volume | 0.2 liter |
| `us legal cup` | Volume | 0.24 liter |
| `kilogram` | Mass | 1000 gram |
| `hectogram` | Mass | 100 gram |
| `decagram` | Mass | 10 gram |
| `gram` | Mass | 1 gram |
| `decigram` | Mass | 0.1 gram |
| `centigram` | Mass | 0.01 gram |
| `milligram` | Mass | 0.001 gram |
| `ounce` | Mass | 28.349523125 gram |
| `pound` | Mass | 453.59237 gram |
| `colombian libra` | Mass | 500 gram |
| `kilometer` | Length | 1000 meter |
| `hectometer` | Length | 100 meter |
| `decameter` | Length | 10 meter |
| `meter` | Length | 1 meter |
| `decimeter` | Length | 0.1 meter |
| `centimeter` | Length | 0.01 meter |
| `millimeter` | Length | 0.001 meter |
| `foot` | Length | 0.3048 meter |
| `inch` | Length | 0.0254 meter |
| `second` | Time | 1 second |
| `minute` | Time | 60 second |
| `hour` | Time | 3600 second |
| `day` | Time | 86400 second |
| `celsius` | Temperature | — |
| `fahrenheit` | Temperature | — |

## Preferred units by system

Largest first; a value is shown in the first unit it reaches a whole one of,
falling back to the last. A region gets its official system: only the
United States is US customary, every other region is metric, and `-u-ms-`
selects a system outright. Time has one list, under Metric.

| System | Quantity | Order |
|---|---|---|
| Metric | Volume | `liter` → `milliliter` → `metric tablespoon` (/2) → `metric teaspoon` (/4) → `milliliter` |
| Metric | Mass | `kilogram` → `gram` → `milligram` |
| Metric | Length | `meter` → `centimeter` → `millimeter` |
| Metric | Temperature | `celsius` |
| Metric | Time | `day` → `hour` → `minute` → `second` |
| UsCustomary | Volume | `us cup` (/4) → `us fluid ounce` (/2) → `us tablespoon` (/3) → `us teaspoon` (/8) |
| UsCustomary | Mass | `pound` (/4) → `ounce` (/4) |
| UsCustomary | Length | `foot` → `inch` (/4) |
| UsCustomary | Temperature | `fahrenheit` |
| Imperial | Volume | `imperial pint` (/4) → `imperial fluid ounce` (/2) → `metric tablespoon` (/2) → `metric teaspoon` (/4) |
| Imperial | Mass | `pound` (/4) → `ounce` (/4) |
| Imperial | Length | `foot` → `inch` (/4) |
| Imperial | Temperature | `celsius` |

## Preferred units by region

A region's own rows, overriding its official system's for that quantity
only. Thresholds are the amount of the unit a value must reach; none is
stated by any source (`research/best-units-by-region.md` §7.1).

| Region | Quantity | Order |
|---|---|---|
| SE | Volume | `liter` ≥ 1 → `deciliter` ≥ 0.5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.5 → `nordic spice measure` ≥ 1 |
| NO | Volume | `liter` ≥ 1 → `deciliter` ≥ 0.5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 → `nordic spice measure` ≥ 1 |
| FI | Volume | `liter` ≥ 1 → `deciliter` ≥ 0.5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.5 → `nordic spice measure` ≥ 1 |
| DK | Volume | `liter` ≥ 0.25 → `deciliter` ≥ 0.5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.5 |
| CH | Volume | `liter` ≥ 1 → `deciliter` ≥ 1 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| HU | Volume | `liter` ≥ 1 → `deciliter` ≥ 0.5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| HU | Mass | `kilogram` ≥ 0.5 → `decagram` ≥ 0.5 → `gram` ≥ 1 |
| FR | Volume | `liter` ≥ 1 → `centiliter` ≥ 5 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.5 |
| BE | Volume | `liter` ≥ 1 → `deciliter` ≥ 1 → `milliliter` ≥ 50 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| NZ | Volume | `metric cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| CA | Volume | `metric cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| AU | Volume | `metric cup` ≥ 0.25 → `australian tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| JP | Volume | `japanese cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| KR | Volume | `japanese cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.25 |
| BR | Volume | `liter` ≥ 1 → `us legal cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric dessert spoon` ≥ 1 → `metric teaspoon` ≥ 0.5 |
| MX | Volume | `liter` ≥ 1 → `us legal cup` ≥ 0.25 → `metric tablespoon` ≥ 1 → `metric teaspoon` ≥ 0.5 |
| CO | Mass | `kilogram` ≥ 1 → `colombian libra` ≥ 0.25 → `gram` ≥ 1 |

## Preferred units of the legacy locale

What an untagged recipe is fitted with: `units.toml`'s `best` lists and
fraction config, frozen, for the system the quantity was written in
(ADR 7). Bundled's "imperial" is US customary.

| System | Quantity | Order |
|---|---|---|
| Metric | Volume | `liter` ≥ 1 → `milliliter` ≥ 1 |
| Metric | Mass | `kilogram` ≥ 1 → `gram` ≥ 1 → `milligram` ≥ 1 |
| Metric | Length | `meter` ≥ 1 → `centimeter` ≥ 1 → `millimeter` ≥ 1 |
| Metric | Temperature | `celsius` ≥ 1 |
| Metric | Time | `day` ≥ 1 → `hour` ≥ 1 → `minute` ≥ 1 → `second` ≥ 1 |
| UsCustomary | Volume | `us cup` ≥ 1 (/4) or decimal → `us cup` ≥ 0.3 (/4) → `us tablespoon` ≥ 2 (/3) or decimal → `us tablespoon` ≥ 0.5 (/3) → `us teaspoon` ≥ 0 (/8) |
| UsCustomary | Mass | `pound` ≥ 1 (/8) → `ounce` ≥ 0 (/4) |
| UsCustomary | Length | `foot` ≥ 1 (/4) → `inch` ≥ 0 (/4) |
| UsCustomary | Temperature | `fahrenheit` ≥ 1 |
