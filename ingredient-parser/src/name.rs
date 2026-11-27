//! A name that reads back from `.shopping-list` and from `@name{}` as the
//! same single ingredient. Ported from `ManualItemQuantity.cleanedName`.

const INVISIBLE: [char; 3] = ['\u{200B}', '\u{2060}', '\u{FEFF}'];

/// Removes braces and zero-width characters, collapses whitespace (line
/// breaks included) to single spaces, drops a `-` after `-` or `[` (no
/// `--` or `[-` comments), and strips a leading `/`, `./` or lone `?`.
///
/// Works on Unicode scalars (`char`), as the Cooklang parser compares bytes.
/// The result can be empty: never write an empty name.
pub fn cleaned_name(raw: &str) -> String {
    let mut chars: Vec<char> = Vec::with_capacity(raw.len());
    for c in raw.chars() {
        if c == '{' || c == '}' || INVISIBLE.contains(&c) {
            continue;
        }
        if c.is_whitespace() {
            if matches!(chars.last(), Some(last) if *last != ' ') {
                chars.push(' ');
            }
            continue;
        }
        if c == '-' && matches!(chars.last(), Some('-') | Some('[')) {
            continue;
        }
        chars.push(c);
    }
    if chars.last() == Some(&' ') {
        chars.pop();
    }
    loop {
        if chars.first() == Some(&'/') {
            chars.remove(0);
        } else if chars.starts_with(&['.', '/']) {
            chars.drain(..2);
        } else if chars.first() == Some(&'?') && (chars.len() == 1 || chars[1] == ' ') {
            chars.remove(0);
        } else {
            break;
        }
        if chars.first() == Some(&' ') {
            chars.remove(0);
        }
    }
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;

    #[test_case("Tomatoes {ripe}" => "Tomatoes ripe")]
    #[test_case("salt -- sea" => "salt - sea")]
    #[test_case("a [- b" => "a [ b")]
    #[test_case("./Milk" => "Milk")]
    #[test_case("/Milk" => "Milk")]
    #[test_case("  Olive   oil \n" => "Olive oil")]
    #[test_case("{}" => "")]
    #[test_case("Olive\u{00A0}oil" => "Olive oil")]
    #[test_case("? milk" => "milk")]
    #[test_case("?" => "")]
    #[test_case("?milk" => "?milk")]
    #[test_case("? ./ ? milk" => "milk")]
    #[test_case("Milk\u{200B}" => "Milk")]
    #[test_case("a\tb\u{2028}c" => "a b c")]
    #[test_case("[--x" => "[x")]
    fn cleans(raw: &str) -> String {
        cleaned_name(raw)
    }
}
