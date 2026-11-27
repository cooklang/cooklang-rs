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
    let mut text = round_decimal_string(&format!("{value}"));
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    text
}

/// Rounds a plain decimal string (as Rust prints the shortest round-trip form
/// of a double) to six places, half to even, like Swift's `%.6f` on the same
/// shortest decimal.
fn round_decimal_string(text: &str) -> String {
    let Some((int, frac)) = text.split_once('.') else {
        return text.to_owned();
    };
    if frac.len() <= 6 {
        return text.to_owned();
    }
    let (keep, rest) = frac.split_at(6);
    let mut digits: Vec<u8> = int.bytes().chain(keep.bytes()).collect();
    let first = rest.as_bytes()[0];
    let tail_zero = rest[1..].bytes().all(|b| b == b'0');
    let last_odd = (digits[digits.len() - 1] - b'0') % 2 == 1;
    let up = first > b'5' || (first == b'5' && (!tail_zero || last_odd));
    if up {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits[i] == b'9' {
                digits[i] = b'0';
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let split = digits.len() - 6;
    let (int, frac) = digits.split_at(split);
    format!(
        "{}.{}",
        String::from_utf8_lossy(int),
        String::from_utf8_lossy(frac)
    )
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
    #[test_case("1e5" => None)]
    #[test_case("inf" => None)]
    #[test_case("+1" => None)]
    #[test_case("NaN" => None)]
    #[test_case("１" => None; "fullwidth digit")]
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
    #[test_case(1.0000005 => "1")]
    #[test_case(1.0000015 => "1.000002")]
    #[test_case(0.0000025 => "0.000002")]
    #[test_case(7.6267995 => "7.6268")]
    #[test_case(2.6004935 => "2.600494")]
    fn formats(value: f64) -> String {
        format_number(value)
    }
}
