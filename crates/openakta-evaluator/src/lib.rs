//! OPENAKTA Evaluator Agent — structural I/O contracts and a deterministic decision matrix.
//!
//! Persistence for downstream consumers is **local-only**: coordinator/daemon code binds verdicts
//! to SQLite and in-process structures. There is **no** Redis, PostgreSQL, or hosted CI gate in
//! this crate—those were spec drift from enterprise templates.
//!
//! Inputs validate under fixed rules; [`evaluate_full`](engine::evaluate_full) maps numeric
//! signals to exactly one of [`Decision`](types::Decision).

pub mod engine;
pub mod pipeline;
pub mod types;
pub mod validation;

pub use engine::{
    calculate_quality_score, evaluate_artifact, evaluate_full, ACCEPT_CONFIDENCE_THRESHOLD,
    CONFIDENCE_REJECT_BELOW, QUALITY_ACCEPT_MIN, QUALITY_HARD_REJECT_BELOW, WEIGHT_SUM,
};
pub use pipeline::evaluate_verdict;
pub use types::{
    Artifact, ArtifactEvaluationMatrix, ArtifactType, ContextSufficiency, Contradiction,
    ContradictionType, Decision, DimensionScores, EvaluationContext, EvaluationType,
    EvaluatorInput, EvaluatorOutput, IssueCounts, RiskLevel, UserIntent,
};
pub use validation::{validate_evaluator_input, EvaluatorInputError};

#[cfg(test)]
mod tests;
