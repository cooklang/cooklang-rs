use crate::error::PassResult;
use crate::ScalableRecipe;

mod ast_walker;

pub use ast_walker::parse_events;

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
