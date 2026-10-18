//! The locales `build.rs` generates from `locales/<tag>.toml`: loading one, finding which one a tag reaches, and the
//! review gate that decides whether it may be served to a reader.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, OnceLock};

use super::locale::{Locale, PluralRules, UnitNames};
use super::tag::{LocaleTag, RegionError};
use crate::convert::units::Unit;

/// A locale as `build.rs` writes it from `locales/<tag>.toml`.
pub(crate) struct GeneratedLocale {
    pub tag: &'static str,
    /// Which CLDR release the names were seeded from. Per locale, not one global version: locales are seeded when
    /// someone comes to review them, so they will not share a release.
    pub cldr_version: &'static str,
    /// Whether the file has a `reviewed` key: a speaker has confirmed the names.
    pub reviewed: bool,
    pub plural_rules: PluralRules,
    pub units: &'static [GeneratedUnit],
}

/// A unit in a generated locale: our identifier, the symbol if the locale has a safe one, the aliases that survived
/// adjudication, and the long patterns by plural category that did.
pub(crate) struct GeneratedUnit {
    pub id: &'static str,
    pub symbol: Option<&'static str>,
    pub aliases: &'static [&'static str],
    pub long: &'static [(&'static str, &'static str)],
}

include!(concat!(env!("OUT_DIR"), "/cldr_locales.rs"));

fn generated_index(tag: &str) -> Option<usize> {
    GENERATED_LOCALES
        .iter()
        .position(|locale| locale.tag == tag)
}

fn generated(tag: &str) -> Option<&'static GeneratedLocale> {
    generated_index(tag).map(|i| &GENERATED_LOCALES[i])
}

/// Each vendored locale, built the first time it is asked for and shared from then on, parallel to `GENERATED_LOCALES`.
static BUILT: LazyLock<Vec<OnceLock<Arc<Locale>>>> =
    LazyLock::new(|| GENERATED_LOCALES.iter().map(|_| OnceLock::new()).collect());

pub(crate) fn vendored_tags() -> impl Iterator<Item = &'static str> {
    GENERATED_LOCALES.iter().map(|locale| locale.tag)
}

pub(crate) fn cldr_version(tag: &str) -> Option<&'static str> {
    generated(tag).map(|locale| locale.cldr_version)
}

/// The CLDR-derived locale for `tag`, if it was vendored.
///
/// A unit CLDR does not carry, or that the locale declined to name — a German `Tasse` is 150 ml, so it cannot name the
/// US cup — is left unnamed and is written by its identifier. Nothing is inherited from any other locale: inheriting
/// legacy's English names would reinstate the strings adjudication rejected.
pub(crate) fn vendored_locale(tag: &str) -> Option<Arc<Locale>> {
    let i = generated_index(tag)?;
    Some(
        BUILT[i]
            .get_or_init(|| Arc::new(build(&GENERATED_LOCALES[i])))
            .clone(),
    )
}

fn build(generated: &GeneratedLocale) -> Locale {
    let tag = generated.tag;
    let names: HashMap<Unit, UnitNames> = generated
        .units
        .iter()
        .filter(|unit| unit.symbol.is_some() || !unit.aliases.is_empty())
        .map(|unit| {
            let names = UnitNames {
                symbol: unit.symbol.map(str::to_string),
                aliases: unit.aliases.iter().map(|a| (*a).to_string()).collect(),
                long: unit
                    .long
                    .iter()
                    .map(|(c, p)| ((*c).to_string(), (*p).to_string()))
                    .collect(),
            };
            let unit = Unit::from_id(unit.id).expect("a generated identifier is a unit");
            (unit, names)
        })
        .collect();
    Locale::with_plurals(names, generated.plural_rules)
        .unwrap_or_else(|e| panic!("locale '{tag}' is invalid: {e}"))
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
    generated(tag).is_some_and(|locale| locale.reviewed)
}

/// [`reviewed_locale`], panicking with the whole locale rather than reporting. No caller outside the tests wants that
/// today.
///
/// # Panics
///
/// If nothing vendored covers `tag`, or the locale it reaches is unreviewed.
#[cfg(test)]
pub(crate) fn reviewed_for_tag(tag: &LocaleTag) -> Arc<Locale> {
    match reviewed_locale(tag) {
        Ok(locale) => locale,
        Err(Refusal::Unknown(tag)) => panic!("no vendored locale covers '{tag}'"),
        Err(Refusal::Unreviewed(reached)) => refuse_unless_reviewed(&reached),
        Err(Refusal::NoAudience(error)) => panic!("{error}"),
    }
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

/// The refusal carries the whole locale, not a command to go and run: whoever hits this is the person most likely to
/// speak the language, and they are most likely to answer while they are already looking at it. It is the same text
/// [`crate::locale_markdown`] renders, so `show-locale-as-md` reproduces it later. That makes for a long panic, about
/// 110 lines, deliberately.
#[cfg(test)]
fn refuse_unless_reviewed(reached: &str) -> ! {
    let report = crate::locale_markdown(reached)
        .unwrap_or_else(|e| format!("(the locale could not be rendered: {e})"));
    panic!(
        "locale '{reached}' has not been reviewed, so cooklang will not write unit \
         names for it: nobody who speaks the language has confirmed they are what \
         recipes in it actually write. Naming a unit wrongly is worse than refusing \
         to name it, which is why this is a refusal rather than a guess.\n\n\
         Below is everything '{reached}' would write, as \
         `cargo run --example show-locale-as-md -- {reached}` prints it. If it looks \
         right, or if you can say what is wrong, please open an issue at \
         https://github.com/cooklang/cooklang-rs/issues. locales/README.md says what we \
         are asking and takes about twenty minutes.\n\n\
         {report}"
    );
}
