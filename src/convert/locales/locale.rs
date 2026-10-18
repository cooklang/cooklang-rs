//! How units are written in one language

use std::collections::HashMap;

use super::plural;
use crate::convert::units::Unit;

/// The strings one locale uses for a unit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UnitNames {
    /// Written back after a conversion. Also resolves as input. `None` when the locale has aliases for the unit but no
    /// safe symbol.
    pub symbol: Option<String>,
    pub aliases: Vec<String>,
    /// Long patterns by CLDR plural category, with `{0}` where the number goes: `("few", "{0} łyżeczki")`. Empty when
    /// the locale has no accepted long form.
    pub long: Vec<(String, String)>,
}

/// CLDR plural rules as `(category, condition)` pairs.
pub(crate) type PluralRules = &'static [(&'static str, &'static str)];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum LocaleError {
    #[error("'{alias}' names both '{}' and '{}'", a.id(), b.id())]
    DuplicateAlias { alias: String, a: Unit, b: Unit },
    #[error("'{alias}' names '{}' but is the identifier of '{}'", unit.id(), of.id())]
    AliasIsAnIdentifier { alias: String, unit: Unit, of: Unit },
    #[error("plural rule {0} is not CLDR syntax")]
    InvalidPluralRule(String),
}

/// Names for the units one language has names for.
#[derive(Debug, Clone)]
pub(crate) struct Locale {
    names: HashMap<Unit, UnitNames>,
    index: HashMap<String, Unit>,
    /// Empty when the locale has no rules, in which case every number is `other`.
    plurals: PluralRules,
}

impl Locale {
    /// No string may name two units, and no string may name a unit other than the one whose spaced identifier it
    /// is. Units may be left unnamed.
    pub(crate) fn new(names: HashMap<Unit, UnitNames>) -> Result<Self, LocaleError> {
        Self::with_plurals(names, &[])
    }

    /// As [`Locale::new`], with the CLDR plural rules long forms select by, each of which must be rule syntax.
    pub(crate) fn with_plurals(
        names: HashMap<Unit, UnitNames>,
        plurals: PluralRules,
    ) -> Result<Self, LocaleError> {
        plural::validate(plurals).map_err(LocaleError::InvalidPluralRule)?;
        let mut index = HashMap::new();
        for (&unit, unit_names) in &names {
            for alias in unit_names.symbol.iter().chain(&unit_names.aliases) {
                claim(&mut index, alias, unit)?;
            }
        }
        for unit in Unit::all() {
            claim(&mut index, unit.id(), unit)?;
        }
        Ok(Self {
            names,
            index,
            plurals,
        })
    }

    /// The unit `text` spells, by symbol, alias or identifier.
    pub(crate) fn resolve(&self, text: &str) -> Option<Unit> {
        self.index.get(text).copied()
    }

    pub(crate) fn symbol(&self, unit: Unit) -> Option<&str> {
        self.names.get(&unit)?.symbol.as_deref()
    }

    /// The string a conversion to `unit` writes back: the symbol, or the spaced identifier when the locale has none.
    pub(crate) fn written(&self, unit: Unit) -> &str {
        self.symbol(unit).unwrap_or_else(|| unit.id())
    }

    /// Every string this locale has for `unit`, or `None` if it names it by identifier only.
    pub(crate) fn names(&self, unit: Unit) -> Option<&UnitNames> {
        self.names.get(&unit)
    }

    pub(crate) fn plural_rules(&self) -> PluralRules {
        self.plurals
    }

    /// `value` with `unit` spelled out in the locale's long form, with the plural the number calls for: `1 łyżeczka`,
    /// `5 łyżeczek`, etc.
    pub(crate) fn written_long(&self, value: f64, unit: Unit) -> String {
        let number = format!("{value}");
        let long = self.names(unit).map_or(&[][..], |n| n.long.as_slice());
        let category = plural::category(self.plurals, &number);
        let pattern = long
            .iter()
            .find(|(c, _)| c == category)
            .or_else(|| long.iter().find(|(c, _)| c == "other"));
        match pattern {
            Some((_, pattern)) => pattern.replace("{0}", &number),
            None => format!("{number} {}", self.written(unit)),
        }
    }
}

fn claim(index: &mut HashMap<String, Unit>, text: &str, unit: Unit) -> Result<(), LocaleError> {
    let borrowed_identifier = |unit, of| {
        Err(LocaleError::AliasIsAnIdentifier {
            alias: text.to_string(),
            unit,
            of,
        })
    };
    match index.insert(text.to_string(), unit) {
        Some(other) if other == unit => Ok(()),
        Some(other) if text == unit.id() => borrowed_identifier(other, unit),
        Some(other) if text == other.id() => borrowed_identifier(unit, other),
        Some(other) => Err(LocaleError::DuplicateAlias {
            alias: text.to_string(),
            a: other,
            b: unit,
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::locales::legacy::legacy_locale;

    fn unit(id: &str) -> Unit {
        Unit::from_id(id).unwrap_or_else(|| panic!("no unit with id '{id}'"))
    }

    fn names(symbol: &str, aliases: &[&str]) -> UnitNames {
        UnitNames {
            symbol: Some(symbol.to_string()),
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
            long: Vec::new(),
        }
    }

    /// A locale naming only the given units.
    fn locale_with(entries: &[(&str, UnitNames)]) -> Result<Locale, LocaleError> {
        let map: HashMap<Unit, UnitNames> = entries
            .iter()
            .map(|(id, unit_names)| (unit(id), unit_names.clone()))
            .collect();
        Locale::new(map)
    }

    const POLISH: PluralRules = &[
        ("one", "i = 1 and v = 0"),
        ("few", "v = 0 and i % 10 = 2..4 and i % 100 != 12..14"),
        ("other", ""),
    ];

    #[test]
    fn resolve_matches_symbols_and_aliases_exactly() {
        let locale = locale_with(&[("us cup", names("c", &["cup", "cups"]))]).unwrap();
        let cup = unit("us cup");

        assert_eq!(locale.resolve("c"), Some(cup));
        assert_eq!(locale.resolve("cup"), Some(cup));
        assert_eq!(locale.resolve("cups"), Some(cup));

        // exact and case-sensitive, like the legacy converter
        assert_eq!(locale.resolve("Cup"), None);
        assert_eq!(locale.resolve(" cup"), None);
        assert_eq!(locale.resolve(""), None);
    }

    #[test]
    fn written_is_the_symbol_or_else_the_identifier() {
        let locale = locale_with(&[("us cup", names("c", &["cup"]))]).unwrap();
        assert_eq!(locale.symbol(unit("us cup")), Some("c"));
        assert_eq!(locale.written(unit("us cup")), "c");
        assert_eq!(locale.names(unit("liter")), None);
        assert_eq!(locale.symbol(unit("liter")), None);
        assert_eq!(locale.written(unit("liter")), "liter");
    }

    /// What every locale resolves whatever it names: each unit's identifier, which `with_plurals` claims by construction.
    #[test]
    fn the_identifier_resolves_in_every_locale() {
        let empty = locale_with(&[]).unwrap();
        let legacy = legacy_locale();
        for unit in Unit::all() {
            assert_eq!(empty.resolve(unit.id()), Some(unit), "{unit:?}");
            assert_eq!(legacy.resolve(unit.id()), Some(unit), "{unit:?}");
        }
    }

    /// A condition that is not CLDR rule syntax is refused when the locale is built, not at the first number
    /// evaluated under it.
    #[test]
    fn a_plural_rule_that_is_not_cldr_syntax_is_refused() {
        let err = Locale::with_plurals(HashMap::new(), &[("one", "i = 1 and")]).unwrap_err();
        assert!(
            matches!(err, LocaleError::InvalidPluralRule(ref e) if e.contains("one")),
            "{err}"
        );
    }

    #[test]
    fn a_string_may_name_only_one_unit() {
        let cup = unit("us cup");
        let celsius = unit("celsius");

        let err = locale_with(&[
            ("us cup", names("c", &["cup"])),
            ("celsius", names("°C", &["cup"])),
        ])
        .unwrap_err();
        assert!(matches!(
            err,
            LocaleError::DuplicateAlias { ref alias, a, b }
                if alias == "cup" && [a, b].contains(&cup) && [a, b].contains(&celsius)
        ));

        // the same string twice on one unit is not a clash
        assert!(locale_with(&[("us cup", names("c", &["c", "cup", "cup"]))]).is_ok());
    }

    /// `us cup` must always mean the US cup, so no locale may hand it to another unit; a unit's own identifier is fine as
    /// its alias. Which unit the build reaches first comes out of a `HashMap`, so the clash names the borrower and the
    /// owner the same way whichever order it is found in.
    #[test]
    fn an_alias_may_not_be_another_units_identifier() {
        let borrowed = LocaleError::AliasIsAnIdentifier {
            alias: "us cup".to_string(),
            unit: unit("celsius"),
            of: unit("us cup"),
        };
        let err = locale_with(&[("celsius", names("°C", &["us cup"]))]).unwrap_err();
        assert_eq!(err, borrowed);
        let err = locale_with(&[
            ("us cup", names("c", &["us cup"])),
            ("celsius", names("°C", &["us cup"])),
        ])
        .unwrap_err();
        assert_eq!(err, borrowed);
        assert!(locale_with(&[("us cup", names("c", &["us cup"]))]).is_ok());
    }

    #[test]
    fn without_a_long_form_the_number_precedes_the_written_form() {
        let locale = locale_with(&[("decagram", names("dag", &[]))]).unwrap();
        // a symbol but no long patterns
        assert_eq!(locale.written_long(2.0, unit("decagram")), "2 dag");
        // nothing at all: the identifier
        assert_eq!(locale.written_long(2.0, unit("us cup")), "2 us cup");
        // legacy has no long forms
        assert_eq!(legacy_locale().written_long(2.0, unit("us cup")), "2 c");
    }

    /// The number's CLDR category picks the pattern; a category the unit has no pattern for falls to `other`.
    #[test]
    fn the_long_form_follows_the_numbers_category() {
        let tsp = unit("metric teaspoon");
        let names = UnitNames {
            symbol: Some("ł".to_string()),
            aliases: Vec::new(),
            long: vec![
                ("one".to_string(), "{0} łyżeczka".to_string()),
                ("other".to_string(), "{0} łyżeczek".to_string()),
            ],
        };
        let locale = Locale::with_plurals(HashMap::from([(tsp, names)]), POLISH).unwrap();
        assert_eq!(locale.written_long(1.0, tsp), "1 łyżeczka");
        assert_eq!(locale.written_long(1.5, tsp), "1.5 łyżeczek");
        // 2 is `few`, which this unit has no pattern for
        assert_eq!(locale.written_long(2.0, tsp), "2 łyżeczek");
    }
}
