//! The locales `build.rs` generates from `locales/<tag>.toml`: loading one, finding which one a tag reaches, and the
//! review gate that decides whether it may be served to a reader.

use std::sync::{Arc, LazyLock, OnceLock};

use super::locale::{Locale, PluralRules, UnitNames};
use super::tag::{LocaleTag, RegionError};
use crate::convert::units::Unit;

// `LOCALE_META` is the tag, the CLDR release the names were seeded from and whether a speaker has
// confirmed them, for each locale; `BUILDERS` constructs the locales themselves, in the same order,
// through the two constructors below. The release is per locale because locales are seeded when
// someone comes to review them, so they will not share one.
include!(concat!(env!("OUT_DIR"), "/cldr_locales.rs"));

/// Each locale, built the first time it is asked for and shared from then on.
static BUILT: LazyLock<Vec<OnceLock<Arc<Locale>>>> =
    LazyLock::new(|| BUILDERS.iter().map(|_| OnceLock::new()).collect());

/// One unit's names, as `build.rs` emits them. `build.rs` refuses a unit that names nothing, so a
/// name here is always one the locale resolves.
fn unit(
    id: &str,
    symbol: Option<&str>,
    aliases: &[&str],
    long: &[(&str, &str)],
) -> (Unit, UnitNames) {
    let names = UnitNames {
        symbol: symbol.map(str::to_string),
        aliases: aliases.iter().map(|a| (*a).to_string()).collect(),
        long: long
            .iter()
            .map(|(c, p)| ((*c).to_string(), (*p).to_string()))
            .collect(),
    };
    let unit = Unit::from_id(id).expect("a generated identifier is a unit");
    (unit, names)
}

/// One locale, as `build.rs` emits it.
///
/// A unit CLDR does not carry, or that the locale declined to name — a German `Tasse` is 150 ml, so
/// it cannot name the US cup — is absent and is written by its identifier. Nothing is inherited from
/// any other locale: inheriting legacy's English names would reinstate the strings adjudication
/// rejected.
fn locale(tag: &str, plurals: PluralRules, units: Vec<(Unit, UnitNames)>) -> Arc<Locale> {
    let locale = Locale::with_plurals(units.into_iter().collect(), plurals)
        .unwrap_or_else(|e| panic!("locale '{tag}' is invalid: {e}"));
    Arc::new(locale)
}

fn meta_index(tag: &str) -> Option<usize> {
    LOCALE_META.iter().position(|(t, _, _)| *t == tag)
}

pub(crate) fn vendored_tags() -> impl Iterator<Item = &'static str> {
    LOCALE_META.iter().map(|(tag, _, _)| *tag)
}

pub(crate) fn cldr_version(tag: &str) -> Option<&'static str> {
    meta_index(tag).map(|i| LOCALE_META[i].1)
}

/// The CLDR-derived locale for `tag`, if it was vendored.
pub(crate) fn vendored_locale(tag: &str) -> Option<Arc<Locale>> {
    let i = meta_index(tag)?;
    Some(BUILT[i].get_or_init(BUILDERS[i]).clone())
}

include!(concat!(env!("OUT_DIR"), "/cldr_parents.rs"));

/// The locale whose names apply to `tag`, reviewed or not; [`reviewed_locale`] is what serves a reader.
#[cfg(test)]
pub(crate) fn for_tag(tag: &LocaleTag) -> Option<Arc<Locale>> {
    vendored_locale(tag_for(tag)?)
}

/// The vendored tag whose names apply to `tag`, if any.
///
/// Extensions are ignored: `-u-ms-metric` chooses units, not names. Lookup follows CLDR's parent chain before
/// truncating, which is what sends `en-NZ` to `en-001` rather than to US English.
pub(crate) fn tag_for(tag: &LocaleTag) -> Option<&'static str> {
    let mut current = tag.language_identifier();
    loop {
        if let Some(vendored) = vendored_tags().find(|t| *t == current) {
            return Some(vendored);
        }
        current = match PARENT_LOCALES.iter().find(|(child, _)| *child == current) {
            Some((_, parent)) => (*parent).to_string(),
            None => match current.rsplit_once('-') {
                Some((head, _)) => head.to_string(),
                None => return None,
            },
        };
    }
}

/// Whether a speaker has confirmed this locale's unit names.
///
/// Adjudication is not review: a locale can have a verdict for every ambiguous string and still be unreviewed, until a
/// speaker has checked that what it writes is what recipes in the language write. The `reviewed` key in
/// `locales/<tag>.toml` records that.
pub(crate) fn names_reviewed(tag: &str) -> bool {
    meta_index(tag).is_some_and(|i| LOCALE_META[i].2)
}

/// Why a tag cannot be served to a reader.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    #[error("no locale covers '{0}'")]
    Unknown(String),
    #[error(
        "locale '{0}' has not been reviewed: nobody who speaks the language has confirmed \
         its unit names. `cargo run --example show-locale-as-md -- {0}` shows what it would \
         write, and locales/README.md says how to review it"
    )]
    Unreviewed(String),
    /// The tag names no region and no measurement system, so nothing says which units its readers want (ADR 7).
    #[error(transparent)]
    NoAudience(#[from] RegionError),
}

/// The locale to serve a reader on `tag`, refusing one no speaker has confirmed.
///
/// Refusing rather than degrading is deliberate: falling back to identifiers would ship `2 us cup` to a Swede as though
/// it were a translation, and falling back to the parent would hand a Ukrainian reader Russian. The refusal names the
/// locale reached, since that is the one needing review.
pub(crate) fn reviewed_locale(tag: &LocaleTag) -> Result<Arc<Locale>, Refusal> {
    let reached = tag_for(tag).ok_or_else(|| Refusal::Unknown(tag.language_identifier()))?;
    if !names_reviewed(reached) {
        return Err(Refusal::Unreviewed(reached.to_string()));
    }
    Ok(vendored_locale(reached).expect("tag_for found it"))
}
