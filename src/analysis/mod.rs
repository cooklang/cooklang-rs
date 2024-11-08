//! Analysis pass of the parser
//!
//! This is just if for some reason you want to split the parsing from the
//! analysis.

use crate::error::{CowStr, PassResult, SourceDiag};
use crate::ScalableRecipe;

mod event_consumer;

pub use event_consumer::parse_events;

pub type AnalysisResult = PassResult<ScalableRecipe>;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum DefineMode {
    All,
    Components,
    Steps,
    Text,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub(crate) enum DuplicateMode {
    New,
    Reference,
}

/// Extra configuration for the analysis of events
#[derive(Default)]
pub struct ParseOptions<'a> {
    /// Check recipe references for existence
    pub recipe_ref_check: Option<RecipeRefCheck<'a>>,
    /// Check metadata entries for validity
    ///
    /// Some checks are performed by default, but you can add your own here.
    /// The function receives the key and the value.
    ///
    /// The boolean returned indicates if the value should be included in the
    /// recipe.
    pub metadata_validator: Option<MetadataValidator<'a>>,
}

/// Return type for check functions in [`ParseOptions`]
///
/// `Error` and `Warning` contain hints to the user with why it
/// failed and/or how to solve it. They should be ordered from most to least
/// important.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckResult {
    Ok,
    Warning(Vec<CowStr>),
    Error(Vec<CowStr>),
}

impl CheckResult {
    pub(crate) fn into_source_diag<F, O>(self, message: F) -> Option<SourceDiag>
    where
        F: FnOnce() -> O,
        O: Into<CowStr>,
    {
        let (severity, hints) = match self {
            CheckResult::Ok => return None,
            CheckResult::Warning(hints) => (crate::error::Severity::Warning, hints),
            CheckResult::Error(hints) => (crate::error::Severity::Error, hints),
        };
        let mut diag = SourceDiag::unlabeled(message(), severity, crate::error::Stage::Analysis);
        for hint in hints {
            diag.add_hint(hint);
        }
        Some(diag)
    }
}

pub type RecipeRefCheck<'a> = Box<dyn FnMut(&str) -> CheckResult + 'a>;
pub type MetadataValidator<'a> =
    Box<dyn FnMut(&serde_yaml::Value, &serde_yaml::Value) -> (CheckResult, bool) + 'a>;
