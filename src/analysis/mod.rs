use crate::error::PassResult;
use crate::ScalableRecipe;

mod event_consumer;

pub use event_consumer::parse_events;

pub type AnalysisResult = PassResult<ScalableRecipe>;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum DefineMode {
    All,
    Components,
    Steps,
    Text,
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum DuplicateMode {
    New,
    Reference,
}
