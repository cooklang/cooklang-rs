//! Analysis pass of the parser
//!
//! This is just if for some reason you want to split the parsing from the
//! analysis.

use crate::error::PassResult;
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
