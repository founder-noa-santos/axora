//! EVAL-001 / EVAL-003 / EVAL-004 validation for [`EvaluatorInput`](crate::types::EvaluatorInput).

use std::sync::LazyLock;

use regex::Regex;

use crate::types::{Artifact, EvaluationContext, EvaluationType, EvaluatorInput};

static EVALUATION_ID_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^eval_[a-z0-9]{16}$").expect("valid regex"));

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EvaluatorInputError {
    #[error("EVAL-001: evaluation_id must match ^eval_[a-z0-9]{{16}}$")]
    InvalidEvaluationId,
    #[error("EVAL-003: artifact.content must not be empty")]
    EmptyArtifactContent,
    #[error("EVAL-004: context.user_intent.description must not be empty")]
    EmptyUserIntentDescription,
}

/// Validates structural rules after deserialization or construction.
pub fn validate_evaluator_input(input: &EvaluatorInput) -> Result<(), EvaluatorInputError> {
    if !EVALUATION_ID_PATTERN.is_match(&input.evaluation_id) {
        return Err(EvaluatorInputError::InvalidEvaluationId);
    }
    if input.artifact.content.trim().is_empty() {
        return Err(EvaluatorInputError::EmptyArtifactContent);
    }
    if input.context.user_intent.description.trim().is_empty() {
        return Err(EvaluatorInputError::EmptyUserIntentDescription);
    }
    Ok(())
}

impl EvaluatorInput {
    /// Fails on EVAL-001 / EVAL-003 / EVAL-004.
    pub fn validate(&self) -> Result<(), EvaluatorInputError> {
        validate_evaluator_input(self)
    }

    /// Validated constructor: only returns `Ok` when EVAL-001/003/004 pass.
    pub fn try_new(
        evaluation_id: String,
        evaluation_type: EvaluationType,
        artifact: Artifact,
        context: EvaluationContext,
    ) -> Result<Self, EvaluatorInputError> {
        let input = EvaluatorInput {
            evaluation_id,
            evaluation_type,
            artifact,
            context,
        };
        validate_evaluator_input(&input)?;
        Ok(input)
    }
}
