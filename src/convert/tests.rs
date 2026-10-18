use super::locales::preference::{Audience, System};
use super::units::Unit;
use super::{ConvertError, Converter};
use crate::quantity::tests::qty as unread;
use crate::quantity::Quantity;
use icu_locale_core::subtags::region;

const METRIC: Audience = Audience::system(System::Metric);
const US: Audience = Audience::region(region!("US"));

/// Read through the legacy locale, as the parser would read it.
fn qty(q: &str) -> Quantity {
    unread(q).read_with(&Converter::legacy())
}

#[test]
fn convert_for_writes_the_amount_as_the_audience_would() {
    let v2 = Converter::legacy();
    let fit = |q: &str, audience| v2.convert_for(&qty(q), audience).unwrap().to_string();

    assert_eq!(fit("1500%ml", METRIC), "1.5 l");
    // 500 ml is 2.11 cups, 5.7 % over two and so no fraction of a cup; it falls to fluid ounces, where 16.9 rounds to
    // a whole
    assert_eq!(fit("500%ml", US), "17 fl oz");
    // a US reader asking for metric gets millilitres, not a cup
    assert_eq!(fit("1%cup", METRIC), "236.588 ml");
    // a range is judged by its start, and its end written in the unit that chose
    assert_eq!(fit("900-1100%ml", METRIC), "900-1100 ml");
    assert_eq!(fit("1-2%l", US), "4 1/4-8 1/2 c");
}

#[test]
fn the_errors_are_the_same_as_convert() {
    let v2 = Converter::legacy();
    let err = |q: &str| v2.convert_for(&qty(q), METRIC).unwrap_err();

    assert!(matches!(err("1"), ConvertError::NoUnit));
    assert!(matches!(err("some%l"), ConvertError::TextValue(_)));
    assert!(matches!(err("1%bunch"), ConvertError::UnknownUnit(ref u) if u == "bunch"));
}

#[test]
fn convert_preserves_scalable() {
    // converting rebuilds the quantity, and must not lose the flag: the legacy converter went through `Quantity::new`,
    // which resets it
    let mut liter = qty("1%l");
    liter.scalable = true;
    let milliliter = Unit::from_id("milliliter").unwrap();

    let converted = Converter::legacy().convert(&liter, milliliter).unwrap();
    assert!(converted.scalable());
    assert_eq!(converted.to_string(), "1000 ml");
}
