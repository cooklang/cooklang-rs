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
