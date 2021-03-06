use crate::helpers::models::domain::{create_empty_insertion_context, create_empty_problem};
use crate::refinement::acceptance::greedy::Greedy;
use crate::refinement::acceptance::Acceptance;
use crate::refinement::objectives::{MeasurableObjectiveCost, ObjectiveCost};
use crate::refinement::RefinementContext;

parameterized_test! {can_identify_cheapest_solution, (new_cost, old_cost, expected), {
    can_identify_cheapest_solution_impl(Box::new(MeasurableObjectiveCost::new(new_cost)),
                                        Box::new(MeasurableObjectiveCost::new(old_cost)), expected);
}}

can_identify_cheapest_solution! {
    case_01: (10., 20., true),
    case_02: (20., 10., false),
    case_03: (10., 10., false),
}

fn can_identify_cheapest_solution_impl(
    new_cost: Box<dyn ObjectiveCost + Send + Sync>,
    old_cost: Box<dyn ObjectiveCost + Send + Sync>,
    expected: bool,
) {
    let mut refinement_ctx = RefinementContext::new(create_empty_problem());
    refinement_ctx.population.add((create_empty_insertion_context(), old_cost, 0));
    let individuum = (create_empty_insertion_context(), new_cost, refinement_ctx.generation);

    let result = Greedy::default().is_accepted(&mut refinement_ctx, &individuum);

    assert_eq!(result, expected);
}
