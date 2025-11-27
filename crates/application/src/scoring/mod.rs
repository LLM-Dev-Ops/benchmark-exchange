//! Scoring module - Evaluation and scoring engine
//!
//! This module provides scoring functionality for benchmark submissions,
//! including various evaluation methods and score aggregation.

mod engine;
mod evaluators;

pub use engine::*;
pub use evaluators::*;
