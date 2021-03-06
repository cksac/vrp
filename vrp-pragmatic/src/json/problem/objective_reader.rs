use crate::constraints::WorkBalance;
use crate::extensions::MultiDimensionalCapacity;
use crate::json::problem::reader::{ApiProblem, ProblemProperties};
use crate::json::problem::Objective::*;
use crate::json::problem::*;
use std::sync::Arc;
use vrp_core::construction::constraints::{ConstraintModule, ConstraintPipeline, FleetUsageConstraintModule};
use vrp_core::refinement::objectives::Objective as CoreObjective;
use vrp_core::refinement::objectives::*;

pub fn create_objective(
    api_problem: &ApiProblem,
    constraint: &mut ConstraintPipeline,
    props: &ProblemProperties,
) -> MultiObjective {
    if let Some(objectives) = &api_problem.objectives {
        let mut map_objectives = |objectives: &Vec<_>| {
            let mut core_objectives: Vec<Box<dyn CoreObjective + Send + Sync>> = vec![];
            let mut cost_idx = None;
            objectives.iter().enumerate().for_each(|(idx, objective)| match objective {
                MinimizeCost { goal, tolerance } => {
                    cost_idx = Some(idx);
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalTransportCost::new(
                        value_goal,
                        variation_goal,
                        tolerance.clone(),
                    )))
                }
                MinimizeTours { goal } => {
                    constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalRoutes::new(value_goal, variation_goal, true)))
                }
                MinimizeUnassignedJobs { goal } => {
                    let (value_goal, variation_goal) = split_goal(goal);
                    core_objectives.push(Box::new(TotalUnassignedJobs::new(value_goal, variation_goal)))
                }
                BalanceMaxLoad { threshold, tolerance } => {
                    let (module, objective) = get_load_balance(props, threshold.clone(), tolerance.clone());
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceActivities { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_activity_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDistance { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_distance_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
                BalanceDuration { threshold, tolerance } => {
                    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance.clone());
                    let (module, objective) =
                        WorkBalance::new_duration_balanced(threshold.clone(), solution_tolerance, route_tolerance);
                    constraint.add_module(module);
                    core_objectives.push(objective);
                }
            });
            (core_objectives, cost_idx)
        };

        let (primary, primary_cost_idx) = map_objectives(&objectives.primary);
        let (secondary, secondary_cost_idx) = map_objectives(&objectives.secondary.clone().unwrap_or_else(|| vec![]));

        MultiObjective::new(
            primary,
            secondary,
            Arc::new(move |primary, secondary| {
                primary_cost_idx
                    .map(|idx| primary.get(idx).unwrap())
                    .or(secondary_cost_idx.map(|idx| secondary.get(idx).unwrap()))
                    .expect("Cannot get cost value objective")
                    .value()
            }),
        )
    } else {
        constraint.add_module(Box::new(FleetUsageConstraintModule::new_minimized()));
        MultiObjective::default()
    }
}

fn get_load_balance(
    props: &ProblemProperties,
    threshold: Option<f64>,
    tolerance: Option<BalanceTolerance>,
) -> (Box<dyn ConstraintModule + Send + Sync>, Box<dyn CoreObjective + Send + Sync>) {
    let (solution_tolerance, route_tolerance) = get_balance_tolerance_params(tolerance);
    if props.has_multi_dimen_capacity {
        WorkBalance::new_load_balanced::<MultiDimensionalCapacity>(
            threshold,
            solution_tolerance,
            route_tolerance,
            Arc::new(|loaded, total| {
                let mut max_ratio = 0_f64;

                for (idx, value) in total.capacity.iter().enumerate() {
                    let ratio = loaded.capacity[idx] as f64 / *value as f64;
                    max_ratio = max_ratio.max(ratio);
                }

                max_ratio
            }),
        )
    } else {
        WorkBalance::new_load_balanced::<i32>(
            threshold,
            solution_tolerance,
            route_tolerance,
            Arc::new(|loaded, capacity| *loaded as f64 / *capacity as f64),
        )
    }
}

fn get_balance_tolerance_params(tolerance: Option<BalanceTolerance>) -> (Option<f64>, Option<f64>) {
    if let Some(tolerance) = tolerance {
        (tolerance.solution, tolerance.route)
    } else {
        (None, None)
    }
}

fn split_goal<T: Clone>(goal: &Option<GoalSatisfactionCriteria<T>>) -> (Option<T>, Option<(usize, f64)>) {
    goal.as_ref()
        .map_or((None, None), |goal| (goal.value.clone(), goal.variation.as_ref().map(|vc| (vc.sample, vc.variation))))
}
