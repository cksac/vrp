use super::*;

/// A termination criteria which stops when objective goal is satisfied.
pub struct GoalSatisfaction {}

impl Default for GoalSatisfaction {
    fn default() -> Self {
        Self {}
    }
}

impl Termination for GoalSatisfaction {
    fn is_termination(&self, refinement_ctx: &mut RefinementContext, solution: (&Individuum, bool)) -> bool {
        let problem = refinement_ctx.problem.clone();
        let (insertion_ctx, _, _) = &solution.0;

        problem.objective.is_goal_satisfied(refinement_ctx, insertion_ctx).unwrap_or(false)
    }
}
