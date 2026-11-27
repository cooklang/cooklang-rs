//! The unit words a typed line may use, in six languages, each mapped to the
//! one short form that is stored. Copied from the iOS `ManualItemUnits.swift`.

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnitMatch {
    /// A unit, in its stored short form.
    Unit(&'static str),
    /// A piece word ("pcs", "шт"): the number is a plain count.
    Count,
}

/// `""` as the stored form marks piece words.
const ALIASES: &[(&str, &[&str])] = &[
    ("g", &[
        "g", "gr", "gram", "grams", "gramm", "gramme", "grammes", "gramo", "gramos",
        "grama", "gramas", "г", "гр", "грамм", "грамма", "граммов",
    ]),
    ("kg", &[
        "kg", "kgs", "kilo", "kilos", "kilogram", "kilograms", "kilogramm", "kilogramme",
        "kilogrammes", "kilogramo", "kilogramos", "quilo", "quilos", "quilograma",
        "quilogramas", "кг", "кило", "килограмм", "килограмма", "килограммов",
    ]),
    ("ml", &[
        "ml", "milliliter", "milliliters", "millilitre", "millilitres", "mililitro",
        "mililitros", "мл", "миллилитр", "миллилитра", "миллилитров",
    ]),
    ("l", &[
        "l", "liter", "liters", "litre", "litres", "litro", "litros",
        "л", "литр", "литра", "литров",
    ]),
    ("tsp", &[
        "tsp", "tsps", "teaspoon", "teaspoons", "tl", "teelöffel", "cdta", "cdtas", "cucharadita",
        "cucharaditas", "ч.л", "чл",
    ]),
    ("tbsp", &[
        "tbsp", "tbsps", "tbs", "tablespoon", "tablespoons", "el", "esslöffel", "cda", "cdas", "cucharada",
        "cucharadas", "ст.л", "стл",
    ]),
    ("cup", &[
        "cup", "cups", "tasse", "tassen", "tasses", "taza", "tazas", "xícara", "xícaras",
        "xicara", "xicaras", "стакан", "стакана", "стаканов",
    ]),
    ("oz", &[
        "oz", "ounce", "ounces", "unze", "unzen", "onza", "onzas", "once", "onces", "onça",
        "onças", "унция", "унции", "унций",
    ]),
    ("lb", &[
        "lb", "lbs", "pound", "pounds", "pfund", "libra", "libras", "livre", "livres",
        "фунт", "фунта", "фунтов",
    ]),
    ("", &[
        "pc", "pcs", "piece", "pieces", "stück", "stk", "unidad", "unidades", "ud", "uds",
        "pièce", "pièces", "unidade", "шт", "штук", "штука", "штуки",
    ]),
];

/// The unit a word names, ignoring case, Unicode composition and trailing
/// full stops ("TSP.", "ч.л.", "xi\u{301}cara").
pub(crate) fn match_unit(word: &str) -> Option<UnitMatch> {
    let mut key: String = word.nfc().collect::<String>().to_lowercase();
    while key.ends_with('.') {
        key.pop();
    }
    ALIASES
        .iter()
        .find(|(_, words)| words.contains(&key.as_str()))
        .map(|(stored, _)| {
            if stored.is_empty() {
                UnitMatch::Count
            } else {
                UnitMatch::Unit(stored)
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn no_word_is_listed_twice() {
        let mut seen = HashSet::new();
        for (_, words) in ALIASES {
            for word in *words {
                assert!(seen.insert(*word), "{word} is listed under two units");
            }
        }
    }

    #[test]
    fn every_word_is_lowercase_and_composed() {
        for (_, words) in ALIASES {
            for word in *words {
                assert_eq!(word.to_lowercase(), *word);
                assert_eq!(word.nfc().collect::<String>(), *word);
            }
        }
    }

    #[test]
    fn case_composition_and_full_stops_do_not_matter() {
        assert_eq!(match_unit("TSP."), Some(UnitMatch::Unit("tsp")));
        assert_eq!(match_unit("ч.л."), Some(UnitMatch::Unit("tsp")));
        assert_eq!(match_unit("xi\u{301}cara"), Some(UnitMatch::Unit("cup")));
        assert_eq!(match_unit("Stück"), Some(UnitMatch::Count));
        assert_eq!(match_unit("packs"), None);
        assert_eq!(match_unit(""), None);
    }
}
