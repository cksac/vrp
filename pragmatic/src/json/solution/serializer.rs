use crate::json::Location;
use serde::Serialize;
use serde_json::Error;
use std::io::{BufWriter, Write};

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Timing {
    pub driving: i32,
    pub serving: i32,
    pub waiting: i32,
    #[serde(rename(serialize = "break"))]
    pub break_time: i32,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Statistic {
    pub cost: f64,
    pub distance: i32,
    pub duration: i32,
    pub times: Timing,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Schedule {
    pub arrival: String,
    pub departure: String,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Interval {
    pub start: String,
    pub end: String,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub job_id: String,
    #[serde(rename(serialize = "type"))]
    pub activity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<Interval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "tag"))]
    pub job_tag: Option<String>,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Stop {
    pub location: Location,
    pub time: Schedule,
    pub load: Vec<i32>,
    pub activities: Vec<Activity>,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tour {
    pub vehicle_id: String,
    pub type_id: String,
    pub stops: Vec<Stop>,
    pub statistic: Statistic,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct UnassignedJobReason {
    pub code: i32,
    pub description: String,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UnassignedJob {
    pub job_id: String,
    pub reasons: Vec<UnassignedJobReason>,
}

/// Defines iteration model.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Iteration {
    /// Iteration number.
    pub number: i32,
    /// Best known cost
    pub cost: f64,
    /// Elapsed time in seconds.
    pub timestamp: f64,
    /// Amount of tours
    pub tours: usize,
    /// Amount of unassigned jobs.
    pub unassinged: usize,
}

/// Contains extra information.
#[derive(Clone, Serialize, PartialEq, Debug)]
pub struct Extras {
    /// Stores information about iteration performance.
    pub performance: Vec<Iteration>,
}

#[derive(Clone, Serialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub problem_id: String,
    pub statistic: Statistic,
    pub tours: Vec<Tour>,
    pub unassigned: Vec<UnassignedJob>,
    pub extras: Extras,
}

pub fn serialize_solution<W: Write>(writer: BufWriter<W>, solution: &Solution) -> Result<(), Error> {
    serde_json::to_writer_pretty(writer, solution)
}