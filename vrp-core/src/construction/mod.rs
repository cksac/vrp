//! Building blocks for arbitrary heuristic to construct a feasible solution.
//!
//!
//! # Insertion heuristic
//!
//! Insertion heuristic is a popular method to find quickly a **feasible** solution, but without a
//! guarantee of good quality. Essentially, it constructs the solution by repeatedly inserting an
//! unrouted customer into a partially constructed route or as a first customer in an additional
//! route.
//!

/// Specifies a computational quota for solving VRP.
/// The main purpose is to allow to stop algorithm in reaction to external events such
/// as user cancellation, timer, etc.
pub trait Quota {
    /// Returns true when computation should be stopped.
    fn is_reached(&self) -> bool;
}

pub const OP_START_MSG: &str = "Optional start is not yet implemented.";

pub mod constraints;
pub mod heuristics;
