//! This module provides functionality to automatically check that given solution is feasible
//! which means that there is no constraint violations.

use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;
use crate::json::Location;
use crate::parse_time;
use std::collections::HashMap;
use vrp_core::models::common::TimeWindow;

/// Stores problem and solution together and provides some helper methods.
pub struct CheckerContext {
    pub problem: Problem,
    pub matrices: Option<Vec<Matrix>>,
    pub solution: Solution,
    job_map: HashMap<String, Job>,
}

/// Represents all possible activity types.
pub enum ActivityType {
    Terminal,
    Job(Job),
    Break(VehicleBreak),
    Reload(VehicleReload),
}

pub fn create_checker_context(problem: Problem, matrices: Option<Vec<Matrix>>) -> CheckerContext {
    let solution = solve_with_metaheuristic_and_iterations(problem.clone(), matrices.clone(), 10);

    CheckerContext::new(problem, matrices, solution)
}

/// Solves problem and checks results.
pub fn solve_and_check(problem: Problem, matrices: Option<Vec<Matrix>>) -> Result<(), String> {
    let ctx = create_checker_context(problem, matrices);

    check_vehicle_load(&ctx)?;
    check_relations(&ctx)?;
    // TODO break is soft constraint and can be violated, how to improve checker?
    // check_breaks(&ctx)?;
    check_assignment(&ctx)?;

    Ok(())
}

impl CheckerContext {
    pub fn new(problem: Problem, matrices: Option<Vec<Matrix>>, solution: Solution) -> Self {
        let job_map = problem.plan.jobs.iter().map(|job| (job.id.clone(), job.clone())).collect();

        Self { problem, matrices, solution, job_map }
    }

    /// Gets vehicle by its id.
    pub fn get_vehicle(&self, vehicle_id: &str) -> Result<&VehicleType, String> {
        self.problem
            .fleet
            .vehicles
            .iter()
            .find(|v| vehicle_id.starts_with(v.type_id.as_str()))
            .ok_or(format!("Cannot find vehicle with id '{}'", vehicle_id))
    }

    /// Gets activity operation time range in seconds since Unix epoch.
    pub fn get_activity_time(&self, stop: &Stop, activity: &Activity) -> TimeWindow {
        let time = activity
            .time
            .clone()
            .unwrap_or_else(|| Interval { start: stop.time.arrival.clone(), end: stop.time.departure.clone() });

        TimeWindow::new(parse_time(&time.start), parse_time(&time.end))
    }

    /// Gets activity location.
    pub fn get_activity_location(&self, stop: &Stop, activity: &Activity) -> Location {
        activity.location.clone().unwrap_or_else(|| stop.location.clone())
    }

    /// Gets vehicle shift where activity is used.
    pub fn get_vehicle_shift(&self, tour: &Tour) -> Result<VehicleShift, String> {
        let tour_time = TimeWindow::new(
            parse_time(&tour.stops.first().as_ref().ok_or_else(|| format!("Cannot get first activity"))?.time.arrival),
            parse_time(&tour.stops.last().as_ref().ok_or_else(|| format!("Cannot get last activity"))?.time.arrival),
        );

        self.get_vehicle(tour.vehicle_id.as_str())?
            .shifts
            .iter()
            .find(|shift| {
                let shift_time = TimeWindow::new(
                    parse_time(&shift.start.time),
                    shift.end.as_ref().map_or_else(|| std::f64::MAX, |place| parse_time(&place.time)),
                );
                shift_time.intersects(&tour_time)
            })
            .cloned()
            .ok_or_else(|| format!("Cannot find shift for tour with vehicle if: '{}'", tour.vehicle_id))
    }

    /// Returns stop's activity type names.
    pub fn get_stop_activity_types(&self, stop: &Stop) -> Vec<String> {
        stop.activities.iter().map(|a| a.activity_type.clone()).collect()
    }

    /// Gets wrapped activity type.
    pub fn get_activity_type(&self, tour: &Tour, stop: &Stop, activity: &Activity) -> Result<ActivityType, String> {
        let shift = self.get_vehicle_shift(tour)?;
        let time = self.get_activity_time(stop, activity);
        let location = self.get_activity_location(stop, activity);

        match activity.activity_type.as_str() {
            "departure" | "arrival" => Ok(ActivityType::Terminal),
            "pickup" | "delivery" => self.job_map.get(activity.job_id.as_str()).map_or_else(
                || Err(format!("Cannot find job with id '{}'", activity.job_id)),
                |job| Ok(ActivityType::Job(job.clone())),
            ),
            "break" => shift
                .breaks
                .as_ref()
                .and_then(|breaks| {
                    breaks.iter().find(|b| match &b.time {
                        VehicleBreakTime::TimeWindow(tw) => parse_time_window(tw).intersects(&time),
                        VehicleBreakTime::TimeOffset(offset) => {
                            assert_eq!(offset.len(), 2);
                            // NOTE make expected time window wider due to reschedule departure
                            let stops = &tour.stops;
                            let start = parse_time(&stops.first().unwrap().time.arrival) + *offset.first().unwrap();
                            let end = parse_time(&stops.first().unwrap().time.departure) + *offset.last().unwrap();

                            TimeWindow::new(start, end).intersects(&time)
                        }
                    })
                })
                .map(|b| ActivityType::Break(b.clone()))
                .ok_or_else(|| format!("Cannot find break for tour '{}'", tour.vehicle_id)),
            "reload" => shift
                .reloads
                .as_ref()
                // TODO match reload's time windows
                .and_then(|reload| reload.iter().find(|r| r.location == location && r.tag == activity.job_tag))
                .map(|r| ActivityType::Reload(r.clone()))
                .ok_or_else(|| format!("Cannot find reload for tour '{}'", tour.vehicle_id)),

            _ => Err(format!("Unknown activity type: '{}'", activity.activity_type)),
        }
    }

    pub fn visit_job<F1, F2, R>(
        &self,
        activity: &Activity,
        activity_type: &ActivityType,
        job_visitor: F1,
        other_visitor: F2,
    ) -> Result<R, String>
    where
        F1: Fn(&Job, &JobTask) -> R,
        F2: Fn() -> R,
    {
        match activity_type {
            ActivityType::Job(job) => {
                let pickups = job.pickups.as_ref().map_or(0, |p| p.len());
                let deliveries = job.deliveries.as_ref().map_or(0, |p| p.len());

                if pickups < 2 && deliveries < 2 {
                    if activity.activity_type == "pickup" { &job.pickups } else { &job.deliveries }
                        .as_ref()
                        .and_then(|task| task.first())
                } else {
                    activity.job_tag.as_ref().ok_or(format!("Multi job activity must have tag {}", activity.job_id))?;

                    if activity.activity_type == "pickup" { &job.pickups } else { &job.deliveries }
                        .iter()
                        .flat_map(|tasks| tasks.iter())
                        .find(|task| task.tag == activity.job_tag)
                }
                .map(|task| job_visitor(job, task))
            }
            .ok_or("Cannot match activity to job place".to_string()),
            _ => Ok(other_visitor()),
        }
    }
}

fn parse_time_window(tw: &Vec<String>) -> TimeWindow {
    TimeWindow::new(parse_time(tw.first().unwrap()), parse_time(tw.last().unwrap()))
}

fn get_time_window(stop: &Stop, activity: &Activity) -> TimeWindow {
    let (start, end) = activity
        .time
        .as_ref()
        .map_or_else(|| (&stop.time.arrival, &stop.time.departure), |interval| (&interval.start, &interval.end));

    TimeWindow::new(parse_time(start), parse_time(end))
}

fn get_location(stop: &Stop, activity: &Activity) -> Location {
    activity.location.as_ref().unwrap_or_else(|| &stop.location).clone()
}

fn same_locations(left: &Location, right: &Location) -> bool {
    left.lat == right.lat && left.lng == right.lng
}

mod assignment;
pub use self::assignment::*;

mod capacity;
pub use self::capacity::*;

mod breaks;
pub use self::breaks::*;

mod relations;
pub use self::relations::*;
