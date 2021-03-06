#[cfg(test)]
#[path = "../../../tests/unit/refinement/termination/max_generation_test.rs"]
mod max_generation_test;

use crate::refinement::termination::Termination;
use crate::refinement::{Individuum, RefinementContext};

/// Stops when maximum amount of generations is exceeded.
pub struct MaxGeneration {
    limit: usize,
}

impl MaxGeneration {
    /// Creates a new instance of [`MaxGeneration`].
    pub fn new(limit: usize) -> Self {
        Self { limit }
    }
}

impl Default for MaxGeneration {
    fn default() -> Self {
        Self::new(2000)
    }
}

impl Termination for MaxGeneration {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, _: (&Individuum, bool)) -> bool {
        refinement_ctx.generation >= self.limit
    }
}
