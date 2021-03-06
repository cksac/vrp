//! This module provides way to insert activity infos into existing route keeping solution feasible.
//!

use super::decipher::ActivityInfo;
use crate::construction::heuristics::{evaluate_job_insertion_in_route, InsertionPosition};
use crate::construction::heuristics::{InsertionContext, InsertionResult, RouteContext};
use crate::models::problem::{Job, Place, Single};
use hashbrown::{HashMap, HashSet};
use std::sync::Arc;

/// Inserts jobs into tour taking care constraints.
pub struct ActivityInfoInserter<'a> {
    insertion_ctx: &'a mut InsertionContext,
    route_ctx: &'a mut RouteContext,
    unprocessed: &'a mut HashSet<Job>,
    unassigned: &'a mut HashSet<Job>,
    activity_infos: Vec<&'a ActivityInfo>,

    inserted_job_map: HashMap<Job, Vec<usize>>,
}

impl<'a> ActivityInfoInserter<'a> {
    pub fn new(
        insertion_ctx: &'a mut InsertionContext,
        route_ctx: &'a mut RouteContext,
        unprocessed: &'a mut HashSet<Job>,
        unassigned: &'a mut HashSet<Job>,
        activity_infos: Vec<&'a ActivityInfo>,
    ) -> Self {
        let activity_infos = Self::filter_broken(activity_infos);
        Self { insertion_ctx, route_ctx, unprocessed, unassigned, activity_infos, inserted_job_map: Default::default() }
    }

    pub fn insert(&mut self) {
        let mut activity_info_idx = 0_usize;
        while let Some(activity_info) = self.activity_infos.get(activity_info_idx) {
            if let Some((job, single, single_idx)) = create_single_job(activity_info) {
                if self.unprocessed.contains(&job) {
                    if self.try_insert_single(&job, single, single_idx, activity_info_idx) {
                        self.accept_insertion(&job);
                    } else {
                        activity_info_idx = self.discard_insertion(&job, activity_info_idx);
                        continue;
                    }
                }
            }

            activity_info_idx += 1;
        }
    }

    fn try_insert_single(
        &mut self,
        job: &Job,
        single: Arc<Single>,
        single_idx: usize,
        activity_info_idx: usize,
    ) -> bool {
        let single = Job::Single(single);
        let result =
            evaluate_job_insertion_in_route(&single, self.insertion_ctx, self.route_ctx, InsertionPosition::Last, None);

        match result {
            InsertionResult::Success(success) => {
                assert_eq!(success.activities.len(), 1);
                let (mut activity, _) = success.activities.into_iter().next().unwrap();
                activity.job = job
                    .as_multi()
                    .and_then(|multi| multi.jobs.get(single_idx).cloned())
                    .or_else(|| job.as_single().cloned());

                self.route_ctx.route_mut().tour.insert_last(activity);
                self.inserted_job_map.entry(job.clone()).or_insert_with(|| vec![]).push(activity_info_idx);

                true
            }
            InsertionResult::Failure(_) => false,
        }
    }

    fn accept_insertion(&mut self, job: &Job) {
        self.insertion_ctx.problem.constraint.accept_insertion(&mut self.insertion_ctx.solution, self.route_ctx, &job);

        let should_remove =
            job.as_multi().map_or(true, |multi| multi.jobs.len() == self.inserted_job_map.get(job).unwrap().len());

        if should_remove {
            self.insertion_ctx.solution.required.retain(|j| *j != *job);
            self.unprocessed.remove(job);
        }
    }

    /// Removes all job activities from the tour and all its successors. Returns an index of last kept job.
    fn discard_insertion(&mut self, job: &Job, activity_info_idx: usize) -> usize {
        match job {
            // NOTE keep activity info as it might be inserted if some multi job is deleted
            Job::Single(_) => activity_info_idx + 1,
            // NOTE remove everything after first sub job and remove multi job from the list
            Job::Multi(_) => {
                let activity_info_idx = if self.inserted_job_map.get(job).is_some() {
                    let start = self.route_ctx.route.tour.index(job).unwrap();
                    let end = 1 + self.route_ctx.route.tour.activity_count();
                    let jobs = self.route_ctx.route_mut().tour.remove_activities_at(start..end);

                    jobs.iter().for_each(|job| {
                        self.inserted_job_map.remove(job);
                        self.unprocessed.insert(job.clone());
                    });

                    self.insertion_ctx.problem.constraint.accept_route_state(self.route_ctx);

                    self.route_ctx
                        .route
                        .tour
                        .get(self.route_ctx.route.tour.activity_count())
                        .and_then(|a| a.retrieve_job())
                        .and_then(|job| self.inserted_job_map.get(&job))
                        .map(|inserted| inserted.last().cloned().unwrap() + 1)
                        .unwrap_or(0)
                } else {
                    activity_info_idx + 1
                };

                self.unprocessed.remove(job);
                self.unassigned.insert(job.clone());

                activity_info_idx
            }
        }
    }

    /// Get multi jobs within their sub job insertion order.
    fn filter_broken(activity_infos: Vec<&ActivityInfo>) -> Vec<&ActivityInfo> {
        let mut activity_infos = activity_infos;

        let activity_info_map = activity_infos.iter().fold(HashMap::new(), |mut acc, ai| {
            if let ActivityInfo::Job((job, single_idx, _, _)) = ai {
                if job.as_multi().is_some() {
                    acc.entry(job.clone()).or_insert_with(|| vec![]).push(*single_idx)
                }
            }

            acc
        });

        activity_infos.retain(|ai| match ai {
            ActivityInfo::Job((job, _, _, _)) => {
                job.as_multi().map_or(true, |multi| multi.validate(activity_info_map.get(job).unwrap()))
            }
            _ => true,
        });

        activity_infos
    }
}

/// Creates a fake single job with single place and single time window to avoid uncertainty
/// during insertion evaluation process.
pub fn create_single_job(activity_info: &ActivityInfo) -> Option<(Job, Arc<Single>, usize)> {
    match activity_info {
        ActivityInfo::Job(activity_info) => {
            let (job, single_index, place_index, tw_index) = activity_info;
            let single = match job {
                Job::Single(single) => single.clone(),
                Job::Multi(multi) => multi.jobs.get(*single_index).cloned().unwrap(),
            };

            let place = single.places.get(*place_index).unwrap();
            let place = Place {
                location: place.location,
                duration: place.duration,
                times: vec![place.times.get(*tw_index).unwrap().clone()],
            };

            Some((job.clone(), Arc::new(Single { places: vec![place], dimens: single.dimens.clone() }), *single_index))
        }
        ActivityInfo::Terminal(_) => None,
    }
}
