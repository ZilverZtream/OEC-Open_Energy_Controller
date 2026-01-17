//! Optimization Strategies
//!
//! This module contains different battery optimization strategies:
//! - Greedy: Simple rule-based optimizer
//! - DP: Dynamic programming optimizer
//! - MILP: Mixed-integer linear programming optimizer (exact solution)

pub mod milp;

pub use milp::*;
