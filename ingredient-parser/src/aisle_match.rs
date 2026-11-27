//! Matching a typed name against an aisle configuration.

use crate::AisleMatch;
use cooklang::aisle::AisleConf;
use unicode_normalization::UnicodeNormalization;

fn fold(text: &str) -> String {
    text.nfc().collect::<String>().to_lowercase()
}

/// The lowercase word and the singulars it may be the plural of.
fn forms(word: &str) -> Vec<String> {
    let word = fold(word);
    let mut forms = vec![word.clone()];
    if let Some(stem) = word.strip_suffix("ies") {
        if !stem.is_empty() {
            forms.push(format!("{stem}y"));
        }
    }
    if let Some(stem) = word.strip_suffix("es") {
        if ["s", "x", "z", "ch", "sh", "o"]
            .iter()
            .any(|end| stem.ends_with(end))
        {
            forms.push(stem.to_owned());
        }
    }
    if let Some(stem) = word.strip_suffix('s') {
        if !stem.is_empty() && !stem.ends_with('s') {
            forms.push(stem.to_owned());
        }
    }
    forms
}

/// The entry `name` matches: an exact case-insensitive match first, then a
/// plural match; the first in file order within each kind.
pub(crate) fn find(aisle: &AisleConf, name: &str) -> Option<AisleMatch> {
    if name.is_empty() {
        return None;
    }
    let entries = || {
        aisle.categories.iter().flat_map(|category| {
            category.ingredients.iter().flat_map(move |ingredient| {
                let common = ingredient.names.first().copied().unwrap_or_default();
                ingredient
                    .names
                    .iter()
                    .map(move |n| (category.name, common, *n))
            })
        })
    };
    let to_match = |(category, common, name): (&str, &str, &str)| AisleMatch {
        name: name.to_owned(),
        common_name: common.to_owned(),
        category: category.to_owned(),
    };
    let lower = fold(name);
    if let Some(entry) = entries().find(|(_, _, n)| fold(n) == lower) {
        return Some(to_match(entry));
    }
    let mine = forms(name);
    entries()
        .find(|(_, _, n)| forms(n).iter().any(|form| mine.contains(form)))
        .map(to_match)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AISLE: &str = "[vegetables]\ntomatoes\nberries\nlemons|lemon juice\n\n[bakery]\ncookies\nglass noodles\n\n[dairy]\nmilk|Milch\ntomato paste\n";

    fn conf() -> AisleConf<'static> {
        cooklang::aisle::parse(AISLE).unwrap()
    }

    fn matched(name: &str) -> Option<(String, String, String)> {
        find(&conf(), name).map(|m| (m.name, m.common_name, m.category))
    }

    #[test]
    fn matches_ignoring_case_and_plurals() {
        assert_eq!(
            matched("Tomato"),
            Some(("tomatoes".into(), "tomatoes".into(), "vegetables".into()))
        );
        assert_eq!(
            matched("Berry"),
            Some(("berries".into(), "berries".into(), "vegetables".into()))
        );
        assert_eq!(
            matched("LEMON"),
            Some(("lemons".into(), "lemons".into(), "vegetables".into()))
        );
        assert_eq!(
            matched("cookie"),
            Some(("cookies".into(), "cookies".into(), "bakery".into()))
        );
        assert_eq!(
            matched("MILCH"),
            Some(("Milch".into(), "milk".into(), "dairy".into()))
        );
    }

    #[test]
    fn an_alias_keeps_its_own_spelling_and_the_common_name() {
        assert_eq!(
            matched("Lemon Juice"),
            Some(("lemon juice".into(), "lemons".into(), "vegetables".into()))
        );
    }

    #[test]
    fn no_match() {
        assert_eq!(matched("bread"), None);
        assert_eq!(matched("glass"), None);
        assert_eq!(matched(""), None);
    }

    fn category_of(aisle: &str, name: &str) -> Option<String> {
        find(&cooklang::aisle::parse(aisle).unwrap(), name).map(|m| m.category)
    }

    #[test]
    fn exact_beats_plural() {
        let aisle = "[a]\ntomatoes\n\n[b]\ntomato\n";
        assert_eq!(category_of(aisle, "tomato"), Some("b".into()));
        assert_eq!(category_of(aisle, "Tomatoes"), Some("a".into()));
    }

    #[test]
    fn a_prefix_is_not_a_match() {
        assert_eq!(category_of("[a]\nglas\n", "glass"), None);
    }

    #[test]
    fn composed_and_decomposed_accents_match() {
        assert_eq!(
            category_of("[a]\nCr\u{e8}me\n", "Cre\u{300}me"),
            Some("a".into())
        );
    }
}
