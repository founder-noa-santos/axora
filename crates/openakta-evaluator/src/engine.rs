//! Weighted quality score and deterministic decision matrix (strict evaluation order).

use crate::types::{
    ArtifactEvaluationMatrix, ContextSufficiency, Decision, DimensionScores, IssueCounts, RiskLevel,
};

/// Minimum confidence required for [`Decision::Accept`] when all other accept preconditions hold.
pub const ACCEPT_CONFIDENCE_THRESHOLD: f64 = 0.7;

/// Exclusive upper bound for the low-confidence [`Decision::Reject`] band (`confidence < 0.5`).
pub const CONFIDENCE_REJECT_BELOW: f64 = 0.5;

/// Below this aggregate [`calculate_quality_score`] value, the artifact is rejected outright
/// (no “accept” path), regardless of mid-band confidence.
pub const QUALITY_HARD_REJECT_BELOW: f64 = 0.35;

/// Minimum quality required alongside [`ACCEPT_CONFIDENCE_THRESHOLD`] for [`Decision::Accept`].
pub const QUALITY_ACCEPT_MIN: f64 = 0.65;

// Weights sum to 1.0 — single source of truth for Plan 10.
const W_CORRECTNESS: f64 = 0.25;
const W_SECURITY: f64 = 0.20;
const W_BEST_PRACTICES: f64 = 0.15;
const W_PERFORMANCE: f64 = 0.15;
const W_MAINTAINABILITY: f64 = 0.15;
const W_COMPLETENESS: f64 = 0.10;

/// Sum of dimension weights (compile-time single definition; verified in tests for FP drift).
pub const WEIGHT_SUM: f64 = W_CORRECTNESS
    + W_SECURITY
    + W_BEST_PRACTICES
    + W_PERFORMANCE
    + W_MAINTAINABILITY
    + W_COMPLETENESS;

#[inline]
fn sanitize_dimension(x: f64) -> f64 {
    if !x.is_finite() {
        return 0.0;
    }
    x.clamp(0.0, 1.0)
}

/// Aggregates dimension scores into a single **[0.0, 1.0]** quality score.
///
/// Non-finite inputs (`NaN`, `±Inf`) are treated as **0.0** per dimension (no panic, no NaN propagation).
pub fn calculate_quality_score(dimensions: &DimensionScores) -> f64 {
    let c = sanitize_dimension(dimensions.correctness);
    let s = sanitize_dimension(dimensions.security);
    let bp = sanitize_dimension(dimensions.best_practices);
    let p = sanitize_dimension(dimensions.performance);
    let m = sanitize_dimension(dimensions.maintainability);
    let x = sanitize_dimension(dimensions.completeness);
    let raw = c * W_CORRECTNESS
        + s * W_SECURITY
        + bp * W_BEST_PRACTICES
        + p * W_PERFORMANCE
        + m * W_MAINTAINABILITY
        + x * W_COMPLETENESS;
    sanitize_dimension(raw)
}

#[inline]
fn confidence_is_gate_valid(c: f64) -> bool {
    c.is_finite() && (0.0..=1.0).contains(&c)
}

/// Deterministic decision engine: **exactly one** of the four states.
///
/// [`calculate_quality_score`] drives reject and accept gates together with confidence on
/// **[0, 1]** with no dead zones between bands:
/// - **[0, 0.5)** confidence → reject (when other reject preconditions are clear),
/// - **[0.5, 0.7)** confidence → never accept; at most [`Decision::Flag`],
/// - **[0.7, 1]** confidence → accept only if quality and issue caps allow.
pub fn evaluate_full(m: &ArtifactEvaluationMatrix) -> Decision {
    let quality = calculate_quality_score(&m.dimensions);

    if matches!(
        m.context_sufficiency,
        ContextSufficiency::Insufficient | ContextSufficiency::Ambiguous
    ) {
        return Decision::RequestInfo;
    }

    if !confidence_is_gate_valid(m.confidence)
        || m.issues.critical > 0
        || m.confidence < CONFIDENCE_REJECT_BELOW
        || quality < QUALITY_HARD_REJECT_BELOW
        || m.risk_level == RiskLevel::Critical
    {
        return Decision::Reject;
    }

    if m.issues.high > 2
        || m.issues.medium > 5
        || m.contradiction_count > 0
        || m.risk_level == RiskLevel::High
    {
        return Decision::Flag;
    }

    if m.confidence >= ACCEPT_CONFIDENCE_THRESHOLD
        && quality >= QUALITY_ACCEPT_MIN
        && m.issues.critical == 0
        && m.issues.high <= 2
        && m.risk_level <= RiskLevel::Medium
    {
        return Decision::Accept;
    }

    Decision::Flag
}

/// Minimal entry point with explicit [`DimensionScores`].
pub fn evaluate_artifact(
    dimensions: &DimensionScores,
    issues: &IssueCounts,
    risk: RiskLevel,
    confidence: f64,
) -> Decision {
    evaluate_full(&ArtifactEvaluationMatrix::new(
        *dimensions,
        confidence,
        risk,
        *issues,
        0,
        ContextSufficiency::Sufficient,
    ))
}
