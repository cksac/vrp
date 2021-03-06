use crate::extensions::SimplePopulation;
use std::ops::Deref;
use std::sync::Arc;
use vrp_core::construction::heuristics::InsertionContext;
use vrp_core::construction::Quota;
use vrp_core::models::{Problem, Solution};
use vrp_core::refinement::acceptance::{Acceptance, Greedy};
use vrp_core::refinement::mutation::{Mutation, RuinAndRecreateMutation};
use vrp_core::refinement::objectives::ObjectiveCost;
use vrp_core::refinement::selection::{SelectRandom, Selection};
use vrp_core::refinement::termination::*;
use vrp_core::refinement::{Individuum, RefinementContext};
use vrp_core::utils::{DefaultRandom, Timer};

/// A skeleton of metaheuristic with default ruin and recreate implementation.
pub struct Solver {
    pub selection: Box<dyn Selection>,
    pub mutation: Box<dyn Mutation>,
    pub acceptance: Box<dyn Acceptance>,
    pub termination: Box<dyn Termination>,
    pub quota: Option<Box<dyn Quota + Sync + Send>>,
    pub initial: Option<InsertionContext>,
    pub logger: Box<dyn Fn(String) -> ()>,
}

impl Default for Solver {
    fn default() -> Self {
        Solver::new(
            Box::new(SelectRandom::default()),
            Box::new(RuinAndRecreateMutation::default()),
            Box::new(Greedy::default()),
            Box::new(CompositeTermination::default()),
            None,
            None,
            Box::new(|msg| println!("{}", msg)),
        )
    }
}

impl Solver {
    /// Creates a new instance of [`Solver`].
    pub fn new(
        selection: Box<dyn Selection>,
        mutation: Box<dyn Mutation>,
        acceptance: Box<dyn Acceptance>,
        termination: Box<dyn Termination>,
        quota: Option<Box<dyn Quota + Sync + Send>>,
        initial: Option<InsertionContext>,
        logger: Box<dyn Fn(String) -> ()>,
    ) -> Self {
        Self { selection, mutation, acceptance, termination, quota, initial, logger }
    }

    /// Solves given problem and returns solution, its cost and generation when it is found.
    /// Return None if no solution found.
    pub fn solve(&mut self, problem: Arc<Problem>) -> Option<(Solution, Box<dyn ObjectiveCost + Send + Sync>, usize)> {
        let mut refinement_ctx =
            RefinementContext::new_with_population(problem.clone(), Box::new(SimplePopulation::new(5)));

        if let Some(quota) = std::mem::replace(&mut self.quota, None) {
            refinement_ctx.set_quota(quota);
        }

        let mut insertion_ctx = match std::mem::replace(&mut self.initial, None) {
            Some(ctx) => {
                let cost = problem.objective.estimate_cost(&mut RefinementContext::new(problem.clone()), &ctx);
                refinement_ctx.population.add((ctx.deep_copy(), cost, 1));
                ctx
            }
            None => InsertionContext::new(problem.clone(), Arc::new(DefaultRandom::default())),
        };

        let refinement_time = Timer::start();
        loop {
            let generation_time = Timer::start();

            insertion_ctx = self.mutation.mutate(&mut refinement_ctx, insertion_ctx);

            let cost = problem.objective.estimate_cost(&mut refinement_ctx, &insertion_ctx);
            let individuum = (insertion_ctx, cost, refinement_ctx.generation);
            let is_accepted = self.acceptance.is_accepted(&mut refinement_ctx, &individuum);
            let is_terminated = self.termination.is_termination(&mut refinement_ctx, (&individuum, is_accepted));
            let is_goal_satisfied =
                problem.objective.is_goal_satisfied(&mut refinement_ctx, &individuum.0).unwrap_or(false);

            if refinement_ctx.generation % 100 == 0 || is_terminated || is_goal_satisfied || is_accepted {
                self.log_generation(&refinement_ctx, &generation_time, &refinement_time, &individuum, is_accepted);
            }

            if refinement_ctx.generation > 0 && refinement_ctx.generation % 1000 == 0 {
                self.log_population(&refinement_ctx, &refinement_time);
            }

            if is_accepted {
                refinement_ctx.population.add(individuum)
            }

            insertion_ctx = self.selection.select(&mut refinement_ctx);

            if is_terminated || is_goal_satisfied {
                self.logger.deref()(format!(
                    "stopped due to termination ({}) or goal satisfaction ({})",
                    is_terminated, is_goal_satisfied
                ));
                break;
            }

            refinement_ctx.generation += 1;
        }

        self.log_speed(&refinement_ctx, &refinement_time);
        self.get_result(refinement_ctx)
    }

    fn log_generation(
        &self,
        refinement_ctx: &RefinementContext,
        generation_time: &Timer,
        refinement_time: &Timer,
        solution: &Individuum,
        is_accepted: bool,
    ) {
        let (insertion_ctx, cost, _) = solution;
        let cost_change = get_cost_change(refinement_ctx, &cost);
        self.logger.deref()(format!(
            "generation {} took {}ms (total {}s), cost: {:.2} ({:.3}%), routes: {}, unassigned: {}, accepted: {}",
            refinement_ctx.generation,
            generation_time.elapsed_millis(),
            refinement_time.elapsed_secs(),
            cost.value(),
            cost_change,
            insertion_ctx.solution.routes.len(),
            insertion_ctx.solution.unassigned.len(),
            is_accepted
        ));
    }

    fn log_population(&self, refinement_ctx: &RefinementContext, refinement_time: &Timer) {
        self.logger.deref()(format!(
            "\tpopulation state after {}s (speed: {:.2} gen/sec):",
            refinement_time.elapsed_secs(),
            refinement_ctx.generation as f64 / refinement_time.elapsed_secs_as_f64(),
        ));
        refinement_ctx.population.all().enumerate().for_each(|(idx, (insertion_ctx, cost, generation))| {
            let cost_change = get_cost_change(refinement_ctx, cost);
            self.logger.deref()(format!(
                "\t\t{} cost: {:.2} ({:.3}%), routes: {}, unassigned: {}, discovered at: {}",
                idx,
                cost.value(),
                cost_change,
                insertion_ctx.solution.routes.len(),
                insertion_ctx.solution.unassigned.len(),
                generation
            ))
        });
    }

    fn log_speed(&self, refinement_ctx: &RefinementContext, refinement_time: &Timer) {
        self.logger.deref()(format!(
            "solving took {}s, total generations: {}, speed: {:.2} gen/sec",
            refinement_time.elapsed_secs(),
            refinement_ctx.generation,
            refinement_ctx.generation as f64 / refinement_time.elapsed_secs_as_f64()
        ));
    }

    fn get_result(
        &self,
        refinement_ctx: RefinementContext,
    ) -> Option<(Solution, Box<dyn ObjectiveCost + Send + Sync>, usize)> {
        if let Some((ctx, cost, generation)) = refinement_ctx.population.best() {
            self.logger.deref()(format!(
                "best solution within cost {} discovered at {} generation",
                cost.value(),
                generation
            ));
            Some((ctx.solution.to_solution(refinement_ctx.problem.extras.clone()), cost.clone_box(), *generation))
        } else {
            None
        }
    }
}

fn get_cost_change(refinement_ctx: &RefinementContext, new_cost: &Box<dyn ObjectiveCost + Send + Sync>) -> f64 {
    refinement_ctx
        .population
        .best()
        .map(|(_, best_cost, _)| (new_cost.value() - best_cost.value()) / best_cost.value() * 100.)
        .unwrap_or(100.)
}
