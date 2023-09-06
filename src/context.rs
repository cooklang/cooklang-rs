#[derive(Debug)]
pub struct Context<E, W> {
    pub errors: Vec<E>,
    pub warnings: Vec<W>,
}

impl<E, W> Default for Context<E, W> {
    fn default() -> Self {
        Self {
            errors: vec![],
            warnings: vec![],
        }
    }
}

impl<E, W> Context<E, W> {
    pub fn error(&mut self, e: impl Into<E>) {
        self.errors.push(e.into());
    }

    pub fn warn(&mut self, w: impl Into<W>) {
        self.warnings.push(w.into());
    }

    #[allow(unused)] // currently only used in tests
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    pub fn finish<T>(self, output: Option<T>) -> PassResult<T, E, W> {
        PassResult::new(output, self.warnings, self.errors)
    }
}

use crate::error::PassResult;

pub trait Recover {
    fn recover() -> Self;
}

impl<T: Default> Recover for T {
    fn recover() -> Self {
        Self::default()
    }
}
