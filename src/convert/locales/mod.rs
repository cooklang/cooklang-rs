//! How units are written in a language ([`locale`], [`plural`]), which units a
//! reader is shown ([`preference`]), and the tag that selects both ([`tag`]).
//! The locales themselves are [`legacy`] and the generated [`vendored`] ones.

pub(crate) mod legacy;
pub(crate) mod locale;
pub(crate) mod plural;
pub(crate) mod preference;
pub(crate) mod reference;
pub(crate) mod tag;
pub(crate) mod vendored;

#[cfg(test)]
mod snapshot;
