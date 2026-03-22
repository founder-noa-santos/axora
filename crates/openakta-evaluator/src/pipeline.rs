//! Verified construction of [`crate::types::EvaluatorOutput`] from the decision matrix.
//!
//! Downstream code must use [`evaluate_verdict`] (or other `pub(crate)` builders in this crate);
//! [`crate::types::EvaluatorOutput`] cannot be instantiated outside this crate.

use crate::engine::evaluate_full;
use crate::types::{ArtifactEvaluationMatrix, Contradiction, EvaluatorOutput};

/// Build a sealed [`EvaluatorOutput`] from a fully specified matrix and contradiction list.
/// [`evaluate_full`] drives [`EvaluatorOutput::decision`]; numeric fields are taken from `matrix`
/// so they cannot be fabricated independently of the matrix.
pub fn evaluate_verdict(
    matrix: &ArtifactEvaluationMatrix,
    contradictions: Vec<Contradiction>,
) -> EvaluatorOutput {
    let decision = evaluate_full(matrix);
    EvaluatorOutput::from_verified_parts(
        decision,
        matrix.confidence,
        matrix.risk_level,
        matrix.issues,
        contradictions,
    )
}
