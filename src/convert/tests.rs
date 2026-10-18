use super::units::Unit;
use super::Converter;
use crate::quantity::{Quantity, Value};

#[test]
fn convert_preserves_scalable() {
    // converting rebuilds the quantity, and must not lose the flag: the legacy converter went through `Quantity::new`,
    // which resets it
    let quantity = Quantity {
        value: Value::from(1.0),
        unit: Some(crate::quantity::WrittenUnit::unresolved("l")),
        scalable: true,
    };
    let milliliter = Unit::from_id("milliliter").unwrap();

    let quantity = quantity.read_with(&Converter::legacy());
    let converted = Converter::legacy().convert(&quantity, milliliter).unwrap();
    assert!(converted.scalable());
    assert_eq!(converted.unit(), Some("ml"));
}

mod convert_for {
    use super::*;
    use crate::convert::locales::preference::{Audience, System};
    use crate::convert::ConvertError;
    use crate::quantity::Number;
    use icu_locale_core::subtags::region;

    /// Read through the legacy locale, as the parser would read it.
    fn q(value: Value, unit: &str) -> Quantity {
        Quantity::new(value, Some(unit.to_string())).read_with(&Converter::legacy())
    }

    fn range(start: f64, end: f64) -> Value {
        Value::Range {
            start: Number::Regular(start),
            end: Number::Regular(end),
        }
    }

    /// The numbers in a value, so a range can be checked end by end.
    fn numbers(q: &Quantity) -> Vec<f64> {
        match q.value() {
            Value::Number(n) => vec![n.value()],
            Value::Range { start, end } => vec![start.value(), end.value()],
            Value::Text(t) => panic!("text value '{t}'"),
        }
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn the_audience_chooses_the_unit() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        let us = Audience::region(region!("US"));

        let out = v2
            .convert_for(&q(Value::from(1500.0), "ml"), metric)
            .unwrap();
        assert_eq!(out.unit(), Some("l"));
        assert!(close(numbers(&out)[0], 1.5));

        // 500 ml is 2.11 cups, 5.7 % over two and so no fraction of a cup;
        // it falls to fluid ounces, where 16.9 rounds to a whole
        let out = v2.convert_for(&q(Value::from(500.0), "ml"), us).unwrap();
        assert_eq!(out.to_string(), "17 fl oz");
        assert!(close(numbers(&out)[0], 16.907), "{out}");

        // a US reader asking for metric gets millilitres, not a cup
        let out = v2.convert_for(&q(Value::from(1.0), "cup"), metric).unwrap();
        assert_eq!(out.unit(), Some("ml"));
        assert!(close(numbers(&out)[0], 236.588), "{out}");
    }

    #[test]
    fn a_range_is_judged_by_its_start() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        let us = Audience::region(region!("US"));

        let out = v2
            .convert_for(&q(range(900.0, 1100.0), "ml"), metric)
            .unwrap();
        assert_eq!(out.unit(), Some("ml"));
        assert_eq!(numbers(&out), [900.0, 1100.0]);

        let out = v2.convert_for(&q(range(1.0, 2.0), "l"), us).unwrap();
        assert_eq!(out.to_string(), "4 1/4-8 1/2 c");
        let ends = numbers(&out);
        assert!(close(ends[0], 4.2268) && close(ends[1], 8.4535), "{out}");
    }

    #[test]
    fn the_errors_are_the_same_as_convert() {
        let v2 = Converter::legacy();
        let metric = Audience::system(System::Metric);
        assert!(matches!(
            v2.convert_for(
                &Quantity::new(Value::from(1.0), None).read_with(&v2),
                metric
            ),
            Err(ConvertError::NoUnit)
        ));
        assert!(matches!(
            v2.convert_for(&q(Value::Text("some".into()), "l"), metric),
            Err(ConvertError::TextValue(_))
        ));
        assert!(matches!(
            v2.convert_for(&q(Value::from(1.0), "bunch"), metric),
            Err(ConvertError::UnknownUnit(ref u)) if u == "bunch"
        ));
    }
}
