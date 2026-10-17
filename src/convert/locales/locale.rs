//! How units are written in one language: the symbol a conversion writes
//! back, and every alias a recipe may use for the unit.
//!
//! A locale names only the units it has a safe name for. A unit it does not
//! name is written as its identifier with spaces (`imperial pint`), and that
//! spaced form resolves in every locale, so what is written can always be
//! read back. See `docs/adr/locale-aware-units.md` (decision 4).

use std::collections::HashMap;

use super::plural;
use crate::convert::units::Unit;

/// The strings one locale uses for a unit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct UnitNames {
    /// Written back after a conversion. Also resolves as input. `None` when
    /// the locale has aliases for the unit but no safe symbol.
    pub symbol: Option<String>,
    pub aliases: Vec<String>,
    /// Long patterns by CLDR plural category, with `{0}` where the number
    /// goes: `("few", "{0} łyżeczki")`. Empty when the locale has no
    /// accepted long form.
    pub long: Vec<(String, String)>,
}

/// CLDR plural rules as `(category, condition)` pairs.
pub(crate) type PluralRules = &'static [(&'static str, &'static str)];

/// Spellings that resolve in every locale, whatever it names.
///
/// The degree symbols in other encodings (`℃`, `℉`) and with the ordinal
/// indicator typed for the degree sign (`ºC`, `ºF`) are the same symbol, not
/// a language's word, so no locale owns them. The bare `C` and `F` are safe
/// only because lookup is case-sensitive: `c` is a cup in `en`, `en-001`,
/// `en-AU` and `en-GB`, so folding case would turn `180 C` into 180 cups.
pub(crate) const UNIVERSAL_ALIASES: &[(&str, &[&str])] = &[
    ("celsius", &["℃", "ºC", "C"]),
    ("fahrenheit", &["℉", "ºF", "F"]),
];

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum LocaleError {
    #[error("'{alias}' names both '{}' and '{}'", a.id(), b.id())]
    DuplicateAlias { alias: String, a: Unit, b: Unit },
    #[error("'{alias}' names '{}' but is the identifier of '{}'", unit.id(), of.id())]
    AliasIsAnIdentifier { alias: String, unit: Unit, of: Unit },
}

/// Names for the units one language has names for. Lookup is exact and
/// case-sensitive: `c` and `C` are different strings.
#[derive(Debug, Clone)]
pub(crate) struct Locale {
    names: HashMap<Unit, UnitNames>,
    index: HashMap<String, Unit>,
    /// Empty when the locale has no rules, in which case every number is
    /// `other`.
    plurals: PluralRules,
}

impl Locale {
    /// No string may name two units, and no string may name a unit other
    /// than the one whose spaced identifier it is. Units may be left unnamed.
    pub(crate) fn new(names: HashMap<Unit, UnitNames>) -> Result<Self, LocaleError> {
        Self::with_plurals(names, &[])
    }

    /// As [`Locale::new`], with the CLDR plural rules long forms select by.
    pub(crate) fn with_plurals(
        names: HashMap<Unit, UnitNames>,
        plurals: PluralRules,
    ) -> Result<Self, LocaleError> {
        let mut index = HashMap::new();
        let duplicate = |alias: &str, a, b| LocaleError::DuplicateAlias {
            alias: alias.to_string(),
            a,
            b,
        };
        for (&unit, unit_names) in &names {
            for alias in unit_names.symbol.iter().chain(&unit_names.aliases) {
                claim(&mut index, alias, unit).map_err(|other| duplicate(alias, other, unit))?;
            }
        }
        for (id, aliases) in UNIVERSAL_ALIASES {
            let unit = Unit::from_id(id).expect("a universal alias names a unit");
            for alias in *aliases {
                claim(&mut index, alias, unit).map_err(|other| duplicate(alias, other, unit))?;
            }
        }
        // the spaced identifier always resolves, so it is indexed like any
        // other spelling; a locale that has given it to another unit is wrong
        for unit in Unit::all() {
            claim(&mut index, unit.id(), unit).map_err(|other| {
                LocaleError::AliasIsAnIdentifier {
                    alias: unit.id().to_string(),
                    unit: other,
                    of: unit,
                }
            })?;
        }
        Ok(Self {
            names,
            index,
            plurals,
        })
    }

    /// The unit written as `text`. Every unit's identifier resolves, named
    /// or not.
    pub(crate) fn resolve(&self, text: &str) -> Option<Unit> {
        self.index.get(text).copied()
    }

    pub(crate) fn symbol(&self, unit: Unit) -> Option<&str> {
        self.names.get(&unit)?.symbol.as_deref()
    }

    /// The string a conversion to `unit` writes back: the symbol, or the
    /// spaced identifier when the locale has none.
    pub(crate) fn written(&self, unit: Unit) -> &str {
        self.symbol(unit).unwrap_or_else(|| unit.id())
    }

    /// Every string this locale has for `unit`, or `None` if it names it
    /// by identifier only.
    pub(crate) fn names(&self, unit: Unit) -> Option<&UnitNames> {
        self.names.get(&unit)
    }

    pub(crate) fn plural_rules(&self) -> PluralRules {
        self.plurals
    }

    /// `value` with `unit` spelled out in the locale's long form, with the
    /// plural the number calls for: `1 łyżeczka`, `5 łyżeczek`, `小さじ 2`.
    /// A locale with no accepted long form for the unit writes the number
    /// followed by [`Locale::written`].
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

/// Indexes `text` as `unit`, or returns the other unit that already has it.
/// The same unit claiming a string twice is not a conflict.
fn claim(index: &mut HashMap<String, Unit>, text: &str, unit: Unit) -> Result<(), Unit> {
    match index.insert(text.to_string(), unit) {
        Some(other) if other != unit => Err(other),
        _ => Ok(()),
    }
}
