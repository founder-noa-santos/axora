//! Serde data contracts for the Evaluator Agent.

use serde::{Deserialize, Serialize};

/// `eval_[a-z0-9]{16}` — validated by [`crate::validation::validate_evaluator_input`].
pub type EvaluationId = String;

/// Review modality for the evaluator (Plan 10 — six concrete modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationType {
    CodeReview,
    DocReview,
    PlanReview,
    ResearchReview,
    ArchReview,
    ConfigReview,
}

/// Terminal state of the quality gate (four mutually exclusive outcomes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Decision {
    Accept,
    Reject,
    Flag,
    RequestInfo,
}

/// Ordinal risk used by the decision matrix and output contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Classification for a detected contradiction (aligns with evaluator-output taxonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContradictionType {
    Internal,
    IntentViolation,
    BestPractice,
    Historical,
    ExternalSource,
}

/// Artifact payload under evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub artifact_type: ArtifactType,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Code,
    Documentation,
    Plan,
    ResearchFindings,
    ArchitectureDesign,
    Configuration,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserIntent {
    pub description: String,
    #[serde(default)]
    pub constraints: Vec<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
}

/// Whether upstream context is adequate for a verdict ([`Decision::RequestInfo`] path).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextSufficiency {
    #[default]
    Sufficient,
    Insufficient,
    Ambiguous,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluationContext {
    pub user_intent: UserIntent,
    /// When not [`ContextSufficiency::Sufficient`], the matrix yields [`Decision::RequestInfo`].
    #[serde(default)]
    pub context_sufficiency: ContextSufficiency,
}

/// Inbound envelope for the Evaluator Agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorInput {
    pub evaluation_id: EvaluationId,
    pub evaluation_type: EvaluationType,
    pub artifact: Artifact,
    pub context: EvaluationContext,
}

/// Per-severity issue counts driving [`crate::engine::evaluate_full`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct IssueCounts {
    pub critical: u32,
    pub high: u32,
    pub medium: u32,
    pub low: u32,
}

/// Single contradiction record in the output bundle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contradiction {
    pub contradiction_type: ContradictionType,
    pub description: String,
}

/// Outbound verdict bundle from the Evaluator Agent.
///
/// Construct only via [`crate::pipeline::evaluate_verdict`] in this crate; fields are not
/// `Deserialize` so untrusted JSON cannot synthesize a verdict.
#[derive(Debug, Clone, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorOutput {
    decision: Decision,
    /// Model or aggregate confidence in the assessment, ∈ [0.0, 1.0].
    confidence: f64,
    risk_level: RiskLevel,
    issues: IssueCounts,
    contradictions: Vec<Contradiction>,
}

impl EvaluatorOutput {
    pub(crate) fn from_verified_parts(
        decision: Decision,
        confidence: f64,
        risk_level: RiskLevel,
        issues: IssueCounts,
        contradictions: Vec<Contradiction>,
    ) -> Self {
        Self {
            decision,
            confidence,
            risk_level,
            issues,
            contradictions,
        }
    }

    pub fn decision(&self) -> Decision {
        self.decision
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn risk_level(&self) -> RiskLevel {
        self.risk_level
    }

    pub fn issues(&self) -> IssueCounts {
        self.issues
    }

    pub fn contradictions(&self) -> &[Contradiction] {
        &self.contradictions
    }
}

/// Six independent dimension scores in **[0.0, 1.0]** before weighting.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionScores {
    pub correctness: f64,
    pub security: f64,
    pub best_practices: f64,
    pub performance: f64,
    pub maintainability: f64,
    pub completeness: f64,
}

/// Full signal set for the decision matrix (includes context and contradiction count).
///
/// Use [`ArtifactEvaluationMatrix::new`] with explicit [`DimensionScores`] so
/// [`crate::engine::calculate_quality_score`] is the single definition of aggregate quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArtifactEvaluationMatrix {
    pub dimensions: DimensionScores,
    pub confidence: f64,
    pub risk_level: RiskLevel,
    pub issues: IssueCounts,
    pub contradiction_count: usize,
    pub context_sufficiency: ContextSufficiency,
}

impl ArtifactEvaluationMatrix {
    pub fn new(
        dimensions: DimensionScores,
        confidence: f64,
        risk_level: RiskLevel,
        issues: IssueCounts,
        contradiction_count: usize,
        context_sufficiency: ContextSufficiency,
    ) -> Self {
        Self {
            dimensions,
            confidence,
            risk_level,
            issues,
            contradiction_count,
            context_sufficiency,
        }
    }
}
