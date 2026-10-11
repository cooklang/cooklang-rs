use super::*;
use std::collections::HashSet;

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() <= 1e-9 + 1e-9 * expected.abs()
}

fn assert_close(actual: f64, expected: f64) {
    assert!(close(actual, expected), "expected {expected}, got {actual}");
}

fn unit(id: &str) -> Unit {
    Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"))
}

/// Parses `"{value} {id}"`, such as `"1.5 us cup"`.
fn qty(s: &str) -> (f64, Unit) {
    let (value, id) = s
        .split_once(' ')
        .unwrap_or_else(|| panic!("expected '{{value}} {{id}}', got '{s}'"));
    let value = value
        .parse()
        .unwrap_or_else(|_| panic!("invalid value in '{s}'"));
    (value, unit(id))
}

/// Whether `from` converts to exactly the amount `to`.
fn eq(from: &str, to: &str) -> bool {
    let (value, from) = qty(from);
    let (expected, to) = qty(to);
    close(Unit::convert(value, from, to).unwrap(), expected)
}

#[test]
fn known_conversions() {
    assert!(eq("1 kiloliter", "1000000 milliliter"));
    assert!(eq("1 us cup", "236.5882365 milliliter"));
    assert!(eq("1 us gallon", "3.785411784 liter"));
    assert!(eq("1 us gallon", "16 us cup"));
    // US units, not their metric or imperial namesakes
    assert!(!eq("1 us cup", "250 milliliter"));
    assert!(!eq("1 us pint", "568.26125 milliliter"));
    assert!(eq("1 imperial gallon", "4.54609 liter"));
    assert!(eq("1 imperial quart", "1.1365225 liter"));
    assert!(eq("1 imperial pint", "568.26125 milliliter"));
    assert!(eq("1 imperial fluid ounce", "28.4130625 milliliter"));
    assert!(eq("1 imperial pint", "20 imperial fluid ounce"));
    assert!(eq("8 imperial pint", "1 imperial gallon"));
    assert!(eq("4 imperial quart", "1 imperial gallon"));
    // and not the US ones: a US pint is 16 US fl oz, an imperial pint 20
    assert!(!eq("1 imperial pint", "1 us pint"));
    assert!(!eq("1 imperial fluid ounce", "1 us fluid ounce"));
    // metric kitchen measures: round values, not the US ones
    assert!(eq("1 metric teaspoon", "5 milliliter"));
    assert!(eq("1 metric dessert spoon", "10 milliliter"));
    assert!(eq("1 metric tablespoon", "15 milliliter"));
    assert!(eq("1 metric cup", "250 milliliter"));
    assert!(eq("1 metric tablespoon", "3 metric teaspoon"));
    assert!(eq("1 metric dessert spoon", "2 metric teaspoon"));
    assert!(!eq("1 metric teaspoon", "1 us teaspoon"));
    assert!(!eq("1 metric cup", "1 us cup"));
    assert!(eq("1 nordic spice measure", "1 milliliter"));
    assert!(eq("5 nordic spice measure", "1 metric teaspoon"));
    assert!(eq("1 australian tablespoon", "20 milliliter"));
    assert!(eq("1 australian tablespoon", "4 metric teaspoon"));
    assert!(eq("1 japanese cup", "200 milliliter"));
    assert!(eq("1 us legal cup", "240 milliliter"));
    assert!(eq("1 us legal cup", "16 metric tablespoon"));
    assert!(!eq("1 us legal cup", "1 us cup"));
    assert!(!eq("1 japanese cup", "1 metric cup"));
    assert!(eq("1 colombian libra", "500 gram"));
    assert!(eq("2 colombian libra", "1 kilogram"));
    assert!(!eq("1 colombian libra", "1 pound"));

    assert!(eq("1 pound", "453.59237 gram"));
    assert!(eq("1 pound", "16 ounce"));

    assert!(eq("1 inch", "2.54 centimeter"));
    assert!(eq("1 foot", "12 inch"));

    assert!(eq("1 hour", "3600 second"));
    assert!(eq("1 day", "24 hour"));

    assert!(eq("100 celsius", "212 fahrenheit"));
    assert!(eq("0 celsius", "32 fahrenheit"));
    assert!(eq("-40 celsius", "-40 fahrenheit"));
    assert!(eq("212 fahrenheit", "100 celsius"));
}

#[test]
fn every_unit_has_a_distinct_id() {
    let ids: HashSet<_> = Unit::all().map(Unit::id).collect();
    assert_eq!(ids.len(), Unit::all().count());
}

#[test]
fn convert_across_quantities_is_mixed_quantities() {
    let err = Unit::convert(1.0, unit("liter"), unit("gram")).unwrap_err();
    assert_eq!(
        (err.from, err.to),
        (PhysicalQuantity::Volume, PhysicalQuantity::Mass)
    );
}

#[test]
fn from_id_finds_every_unit_by_its_id() {
    for unit in Unit::all() {
        assert_eq!(Unit::from_id(unit.id()), Some(unit));
    }
}

#[test]
fn ids_are_single_spaced_words() {
    // catches a multi-word variant added without a `serialize` override, which
    // would otherwise be given an id with an underscore
    //
    // The space checks are the price of ADR 0007: a hyphen could not hide a
    // double, leading, trailing or non-breaking space, and a space can -- all
    // of them invisible in a diff. So they are asserted rather than trusted.
    for unit in Unit::all() {
        let id = unit.id();
        assert!(
            id.chars().all(|c| c.is_ascii_lowercase() || c == ' '),
            "'{id}' is not lowercase words separated by spaces"
        );
        assert_eq!(id.trim(), id, "'{id}' has a leading or trailing space");
        assert!(!id.contains("  "), "'{id}' has a double space");
        assert!(
            !id.chars().any(|c| c.is_whitespace() && c != ' '),
            "'{id}' has whitespace that is not U+0020"
        );
    }
}

const VALUES: [f64; 7] = [0.0, 1.0, 0.5, 3.0, 17.25, 1000.0, -40.0];

fn same_quantity_pairs() -> impl Iterator<Item = (Unit, Unit)> {
    Unit::all().flat_map(|a| {
        Unit::all()
            .filter(move |b| a.physical_quantity() == b.physical_quantity())
            .map(move |b| (a, b))
    })
}

#[test]
fn converting_to_the_same_unit_is_exact() {
    for unit in Unit::all() {
        for value in VALUES {
            assert_eq!(Unit::convert(value, unit, unit).unwrap(), value, "{unit:?}");
        }
    }
}

#[test]
fn converting_there_and_back_round_trips() {
    for (a, b) in same_quantity_pairs() {
        for value in VALUES {
            let there = Unit::convert(value, a, b).unwrap();
            let back = Unit::convert(there, b, a).unwrap();
            assert_close(back, value);
        }
    }
}
