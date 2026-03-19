//! Publication guard for diff-only code results.

use crate::patch_protocol::{DiffOutputValidator, ValidatedAgentOutput};
use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};

/// Publication payload category.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PublicationPayloadType {
    /// Result is a validated code diff.
    ValidatedDiff,
    /// Result is a status payload that may be published as-is.
    StatusText,
}

/// Publication payload returned by the guard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicationPayload {
    /// Payload category.
    pub payload_type: PublicationPayloadType,
    /// Payload body.
    pub body: String,
}

/// Decision emitted by the diff validation layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiffValidationDecision {
    /// Whether publication is allowed.
    pub accepted: bool,
    /// Human-readable message.
    pub message: String,
    /// Validated patch output when accepted as a diff.
    pub validated_output: Option<ValidatedAgentOutput>,
}

/// Publication guard that rejects invalid code-edit results before publication.
#[derive(Debug, Clone)]
pub struct ResultPublicationGuard {
    validator: DiffOutputValidator,
}

impl ResultPublicationGuard {
    /// Create a new guard with a plaintext threshold.
    pub fn new(max_plaintext_bytes: usize) -> Self {
        Self {
            validator: DiffOutputValidator::new(max_plaintext_bytes),
        }
    }

    /// Validate a task result before merge or blackboard publication.
    pub fn validate(&self, task: &Task, output: &str) -> DiffValidationDecision {
        if !task.is_code_modification() {
            return DiffValidationDecision {
                accepted: true,
                message: "non-code task bypassed diff-only enforcement".to_string(),
                validated_output: None,
            };
        }

        match self.validator.validate(output) {
            Ok(validated) => DiffValidationDecision {
                accepted: true,
                message: "validated unified diff output".to_string(),
                validated_output: Some(validated),
            },
            Err(err) => DiffValidationDecision {
                accepted: false,
                message: err.to_string(),
                validated_output: None,
            },
        }
    }

    /// Convert an accepted decision into a publication payload.
    pub fn publication_payload(&self, task: &Task, output: &str) -> Result<PublicationPayload> {
        let decision = self.validate(task, output);
        if let Some(validated) = decision.validated_output {
            Ok(PublicationPayload {
                payload_type: PublicationPayloadType::ValidatedDiff,
                body: validated.raw_output,
            })
        } else if task.is_code_modification() {
            Err(crate::error::AgentError::DiffRequired(decision.message).into())
        } else {
            Ok(PublicationPayload {
                payload_type: PublicationPayloadType::StatusText,
                body: output.to_string(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{Task, TaskType};

    #[test]
    fn test_diff_guard_accepts_valid_code_patch() {
        let task = Task::new("patch auth").with_task_type(TaskType::CodeModification);
        let guard = ResultPublicationGuard::new(256);
        let decision = guard.validate(
            &task,
            "--- src/auth.rs\n+++ src/auth.rs\n@@ -1,0 +1,1 @@\n+fn login() {}\n",
        );
        assert!(decision.accepted);
    }

    #[test]
    fn test_diff_guard_rejects_full_file_output() {
        let task = Task::new("patch auth").with_task_type(TaskType::CodeModification);
        let guard = ResultPublicationGuard::new(16);
        let decision = guard.validate(&task, "fn login() {\n    println!(\"hi\");\n}\n");
        assert!(!decision.accepted);
    }

    #[test]
    fn test_publication_payload_rejects_invalid_code_output() {
        let task = Task::new("patch auth").with_task_type(TaskType::CodeModification);
        let guard = ResultPublicationGuard::new(16);
        let result = guard.publication_payload(&task, "fn login() {}\n");
        assert!(result.is_err());
    }
}
