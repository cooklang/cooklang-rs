//! Turns one typed ingredient line into a name and an amount.
//!
//! "Milk 2 l", "2l milk", "Сахар 200 гр", "2 cans chopped tomatoes": every
//! Cooklang app that takes typed or pasted ingredients reads them through
//! [`parse_ingredient`], so they all agree on the name and write the same
//! stored quantity. See `docs/superpowers/specs/2026-11-27-ingredient-parser-design.md`.
mod number;
