//! Pasted text to one entry per line.

/// What Foundation's `CharacterSet.newlines` holds.
const NEWLINES: [char; 7] = [
    '\n', '\u{0B}', '\u{0C}', '\r', '\u{85}', '\u{2028}', '\u{2029}',
];

/// One entry per non-blank line, without a leading bullet or checkbox, and
/// without Markdown heading lines ("## Dairy").
pub(crate) fn split_lines(text: &str) -> Vec<String> {
    text.split(NEWLINES)
        .map(without_list_marker)
        .filter(|line| !line.is_empty() && !is_heading(line))
        .collect()
}

/// Foundation's `.whitespaces` also covers the zero-width space.
fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{200B}'
}

fn without_list_marker(line: &str) -> String {
    let mut text = line.trim_matches(is_space);
    if let Some(rest) = ["-", "*", "\u{2013}"]
        .iter()
        .find_map(|m| starts_marker(text, m))
    {
        text = rest.trim_matches(is_space);
    } else if let Some(rest) = text.strip_prefix('\u{2022}') {
        text = rest.trim_matches(is_space);
    }
    if let Some(rest) = ["[ ]", "[x]", "[X]"]
        .iter()
        .find_map(|b| text.strip_prefix(b))
    {
        text = rest.trim_matches(is_space);
    }
    text.to_owned()
}

/// A marker counts when whitespace follows it or it is the whole line.
fn starts_marker<'a>(text: &'a str, marker: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(marker)?;
    (rest.is_empty() || rest.starts_with(char::is_whitespace)).then_some(rest)
}

/// One to six `#`, alone or followed by whitespace.
fn is_heading(text: &str) -> bool {
    let hashes = text.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return false;
    }
    let rest = &text[hashes..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_without_bullets_checkboxes_or_blank_lines() {
        let text = "Can you grab on the way home\n\n- Milk 2 l\n* Lemons 1\n• Avocado\n- [ ] Bread\n[x] Parmigiano\r\n   \n- \n*\n•\n";
        assert_eq!(
            split_lines(text),
            [
                "Can you grab on the way home",
                "Milk 2 l",
                "Lemons 1",
                "Avocado",
                "Bread",
                "Parmigiano"
            ]
        );
    }

    #[test]
    fn drops_markdown_headings_but_not_hashes_in_names() {
        let text = "## Dairy\n\n- [ ] Milk, 2 l\n- [x] Butter\n#\n#1 flour\n#hashtag soup\n###### Deep\n####### seven";
        assert_eq!(
            split_lines(text),
            [
                "Milk, 2 l",
                "Butter",
                "#1 flour",
                "#hashtag soup",
                "####### seven"
            ]
        );
    }

    #[test]
    fn keeps_a_hyphen_inside_a_name() {
        assert_eq!(split_lines("Coca-Cola\n-Zero"), ["Coca-Cola", "-Zero"]);
    }

    #[test]
    fn trims_zero_width_spaces_like_foundation() {
        assert_eq!(split_lines("Milk 2 l\u{200B}"), ["Milk 2 l"]);
        assert_eq!(split_lines("\u{200B}- Milk 2 l"), ["Milk 2 l"]);
        assert_eq!(
            split_lines("\u{200B}## Dairy\n- \u{200B}\n\u{200B}\u{200B}"),
            Vec::<String>::new()
        );
        assert_eq!(split_lines("- \u{200B}[ ] Milk"), ["Milk"]);
        assert_eq!(split_lines("-\u{200B}Milk"), ["-\u{200B}Milk"]);
    }
}
