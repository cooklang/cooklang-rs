//! `Converter::convert` against the old converter, for every spelling, value and target it knew.

use crate::convert::locales::legacy::legacy_locale;
use crate::convert::units::Unit;
use crate::convert::{ConvertError, Converter};
use crate::convert_legacy::{
    ConvertError as BundledError, ConvertTo, ConvertUnit, LegacyConverter,
};
use crate::quantity::{Number, Quantity, Value};

/// The numbers in a converted value, so ranges compare end by end.
fn numbers(value: &Value) -> Vec<f64> {
    match value {
        Value::Number(n) => vec![n.value()],
        Value::Range { start, end } => vec![start.value(), end.value()],
        Value::Text(t) => panic!("text value '{t}'"),
    }
}

fn bundled_convert(
    converter: &LegacyConverter,
    quantity: &Quantity,
    to: &str,
) -> Result<Quantity, BundledError> {
    let mut quantity = quantity.clone();
    quantity.convert(ConvertTo::Unit(ConvertUnit::Key(to)), converter)?;
    Ok(quantity)
}

#[test]
fn matches_bundled_for_every_spelling_value_and_target() {
    let bundled = LegacyConverter::bundled();
    let v2 = Converter::legacy();
    let locale = legacy_locale();

    let range = |start: f64, end: f64| Value::Range {
        start: Number::Regular(start),
        end: Number::Regular(end),
    };
    let values: Vec<Value> = [0.0, 1.0, 0.5, 3.0, 17.25, 1000.0, -40.0]
        .into_iter()
        .map(Value::from)
        .chain([range(1.0, 2.0), range(0.5, 1000.0)])
        .collect();

    // only the units bundled has; legacy names exactly those
    let bundled_units = || Unit::all().filter(|unit| locale.names(*unit).is_some());
    for from in bundled_units() {
        let names = locale.names(from).expect("filtered to named units");
        let spellings = names.symbol.iter().chain(&names.aliases);
        for spelling in spellings {
            for to in
                bundled_units().filter(|to| to.physical_quantity() == from.physical_quantity())
            {
                for value in &values {
                    let quantity =
                        Quantity::new(value.clone(), Some(spelling.clone())).read_with(&v2);
                    let theirs =
                        bundled_convert(&bundled, &quantity, locale.symbol(to).unwrap()).unwrap();
                    let ours = v2.convert(&quantity, to).unwrap();

                    let context = format!("{quantity} -> {}", to.id());
                    assert_eq!(ours.unit(), theirs.unit(), "{context}");
                    for (a, b) in numbers(ours.value())
                        .into_iter()
                        .zip(numbers(theirs.value()))
                    {
                        assert!(
                            (a - b).abs() <= 1e-9 + 1e-6 * b.abs(),
                            "{context}: ours {a}, bundled {b}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn errors_match_bundled() {
    let bundled = LegacyConverter::bundled();
    let v2 = Converter::legacy();
    let liter = Unit::from_id("liter").unwrap();
    let gram = Unit::from_id("gram").unwrap();

    let no_unit = Quantity::new(Value::from(1.0), None).read_with(&v2);
    assert!(matches!(
        bundled_convert(&bundled, &no_unit, "l"),
        Err(BundledError::NoUnit(_))
    ));
    assert!(matches!(
        v2.convert(&no_unit, liter),
        Err(ConvertError::NoUnit)
    ));

    let text = Quantity::new(Value::Text("some".to_string()), Some("l".to_string())).read_with(&v2);
    assert!(matches!(
        bundled_convert(&bundled, &text, "ml"),
        Err(BundledError::TextValue(_))
    ));
    assert!(matches!(
        v2.convert(&text, liter),
        Err(ConvertError::TextValue(_))
    ));

    let unknown = Quantity::new(Value::from(1.0), Some("bunch".to_string())).read_with(&v2);
    assert!(matches!(
        bundled_convert(&bundled, &unknown, "l"),
        Err(BundledError::UnknownUnit(_))
    ));
    assert!(
        matches!(v2.convert(&unknown, liter), Err(ConvertError::UnknownUnit(ref u)) if u == "bunch")
    );

    let mixed = Quantity::new(Value::from(1.0), Some("l".to_string())).read_with(&v2);
    assert!(matches!(
        bundled_convert(&bundled, &mixed, "g"),
        Err(BundledError::MixedQuantities { .. })
    ));
    assert!(matches!(
        v2.convert(&mixed, gram),
        Err(ConvertError::MixedQuantities(_))
    ));
}
