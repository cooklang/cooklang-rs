//! Reading and writing the numbers of an amount.
//!
//! Ported from `IngredientLineRecognizer.number` / `typedNumber` and
//! `ManualItemQuantity.format` in the iOS app, so both write the same text.

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

/// Only ASCII digits, at least one.
pub(crate) fn whole(text: &str) -> Option<f64> {
    if text.is_empty() || !text.chars().all(is_digit) {
        return None;
    }
    text.parse().ok()
}

/// A whole number, a decimal with "." or ",", or any fraction "a/b", above
/// zero. Reads stored quantities.
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

/// [`number`] for typed text: a fraction counts only below one, so
/// "24/7 pizza" stays a name.
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

/// Six decimals at most, no trailing zeros, "." whatever the locale.
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
