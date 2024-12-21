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
    /// The function receives the key, value and an [`CheckOptions`] where you
    /// can customize what happens to the key, including not running the default
    /// checks.
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

/// Customize how a metadata entry should be treated
///
/// By default the entry is included and the [`StdKey`](crate::metadata::StdKey)
/// checks run.
pub struct CheckOptions {
    include: bool,
    run_std_checks: bool,
}

impl Default for CheckOptions {
    fn default() -> Self {
        Self {
            include: true,
            run_std_checks: true,
        }
    }
}

impl CheckOptions {
    /// To include or not the metadata entry in the recipe
    ///
    /// If this is `false`, the entry will not be in the recipe. This will avoid
    /// keeping invalid values.
    pub fn include(&mut self, do_include: bool) {
        self.include = do_include;
    }

    /// To run or not the checks for [`StdKey`](crate::metadata::StdKey)
    ///
    /// Disable these checks if you want to change the semantics or structure of a
    /// [`StdKey`](crate::metadata::StdKey) and don't want the parser to issue
    /// warnings about it.
    ///
    /// If the key is **not** an [`StdKey`](crate::metadata::StdKey) this has no effect.
    pub fn run_std_checks(&mut self, do_check: bool) {
        self.run_std_checks = do_check;
    }
}

pub type RecipeRefCheck<'a> = Box<dyn FnMut(&str) -> CheckResult + 'a>;
pub type MetadataValidator<'a> =
    Box<dyn FnMut(&serde_yaml::Value, &serde_yaml::Value, &mut CheckOptions) -> CheckResult + 'a>;
