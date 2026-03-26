//! Publication guard for diff-only code results and MOL claim/evidence policy.
//!
//! - **Legacy path:** [`ResultPublicationGuard::publication_payload`] does not enforce MOL
//!   evidence (`mol_strict = false`, `claim_evidence = None`).
//! - **MOL strict:** use [`ResultPublicationGuard::publication_payload_with_evidence`]. When
//!   `mol_strict` is true, every [`crate::task::TaskType`] must carry a non-empty [`ClaimEvidenceBinding`]
//!   (at least one non-empty requirement id or evidence ref). Code tasks still require a
//!   validated unified diff; non-code tasks additionally require non-empty `output` text.

use crate::patch_protocol::{DiffOutputValidator, ValidatedAgentOutput};
use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClaimEvidenceBinding {
    pub requirement_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
}

/// Returns true when the binding has at least one non-empty requirement id or evidence ref.
pub fn claim_binding_has_evidence(binding: &ClaimEvidenceBinding) -> bool {
    binding
        .requirement_ids
        .iter()
        .any(|s| !s.trim().is_empty())
        || binding
            .evidence_refs
            .iter()
            .any(|s| !s.trim().is_empty())
}

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
    /// Optional claim/evidence linkage for closure-aware publication.
    #[serde(default)]
    pub claim_evidence: Option<ClaimEvidenceBinding>,
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

    /// Convert an accepted decision into a publication payload (no MOL evidence enforcement).
    pub fn publication_payload(&self, task: &Task, output: &str) -> Result<PublicationPayload> {
        self.publication_payload_with_evidence(task, output, None, false)
    }

    /// Build a publication payload with optional MOL claim/evidence binding.
    ///
    /// When `mol_strict` is true, [`claim_binding_has_evidence`] must hold for the provided
    /// `claim_evidence`, and non-code tasks must not use whitespace-only `output`.
    /// [`TaskType::CodeModification`] still requires a validated unified diff.
    pub fn publication_payload_with_evidence(
        &self,
        task: &Task,
        output: &str,
        claim_evidence: Option<ClaimEvidenceBinding>,
        mol_strict: bool,
    ) -> Result<PublicationPayload> {
        Self::validate_mol_evidence_policy(task, output, claim_evidence.as_ref(), mol_strict)?;

        let decision = self.validate(task, output);
        if let Some(validated) = decision.validated_output {
            Ok(PublicationPayload {
                payload_type: PublicationPayloadType::ValidatedDiff,
                body: validated.raw_output,
                claim_evidence,
            })
        } else if task.is_code_modification() {
            Err(crate::error::AgentError::DiffRequired(decision.message).into())
        } else {
            Ok(PublicationPayload {
                payload_type: PublicationPayloadType::StatusText,
                body: output.to_string(),
                claim_evidence,
            })
        }
    }

    fn validate_mol_evidence_policy(
        task: &Task,
        output: &str,
        claim_evidence: Option<&ClaimEvidenceBinding>,
        mol_strict: bool,
    ) -> Result<()> {
        if !mol_strict {
            return Ok(());
        }
        let binding = claim_evidence.ok_or_else(|| {
            crate::error::AgentError::EvidenceRequired(
                "missing claim_evidence binding for MOL strict publication".to_string(),
            )
        })?;
        if !claim_binding_has_evidence(binding) {
            return Err(crate::error::AgentError::EvidenceRequired(
                "claim_evidence must include at least one requirement id or evidence ref".to_string(),
            )
            .into());
        }
        if !task.is_code_modification() && output.trim().is_empty() {
            return Err(crate::error::AgentError::EvidenceRequired(
                "MOL strict non-code publication requires non-empty output text".to_string(),
            )
            .into());
        }
        Ok(())
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

    #[test]
    fn mol_strict_code_requires_claim_evidence() {
        let task = Task::new("patch auth").with_task_type(TaskType::CodeModification);
        let guard = ResultPublicationGuard::new(256);
        let diff = "--- a/x\n+++ b/x\n@@ -0,0 +1 @@\n+ok\n";
        let r = guard.publication_payload_with_evidence(&task, diff, None, true);
        assert!(r.is_err());
        let ok = guard.publication_payload_with_evidence(
            &task,
            diff,
            Some(ClaimEvidenceBinding {
                requirement_ids: vec!["req-1".to_string()],
                evidence_refs: vec![],
            }),
            true,
        );
        assert!(ok.is_ok());
        assert!(ok.unwrap().claim_evidence.is_some());
    }

    #[test]
    fn mol_strict_non_code_requires_evidence_and_body() {
        let task = Task::new("review").with_task_type(TaskType::Review);
        let guard = ResultPublicationGuard::new(256);
        let r = guard.publication_payload_with_evidence(&task, "LGTM", None, true);
        assert!(r.is_err());
        let r2 = guard.publication_payload_with_evidence(
            &task,
            "LGTM",
            Some(ClaimEvidenceBinding {
                requirement_ids: vec![],
                evidence_refs: vec!["bb:entry/1".to_string()],
            }),
            true,
        );
        assert!(r2.is_ok());
        let r3 = guard.publication_payload_with_evidence(
            &task,
            "   \n",
            Some(ClaimEvidenceBinding {
                requirement_ids: vec!["r".to_string()],
                evidence_refs: vec![],
            }),
            true,
        );
        assert!(r3.is_err());
    }

    #[test]
    fn mol_strict_off_allows_empty_claim_evidence() {
        let task = Task::new("review").with_task_type(TaskType::Review);
        let guard = ResultPublicationGuard::new(256);
        let r = guard.publication_payload_with_evidence(&task, "done", None, false);
        assert!(r.is_ok());
        assert!(r.unwrap().claim_evidence.is_none());
    }

    #[test]
    fn claim_binding_has_evidence_helper() {
        assert!(!claim_binding_has_evidence(&ClaimEvidenceBinding {
            requirement_ids: vec![],
            evidence_refs: vec![],
        }));
        assert!(!claim_binding_has_evidence(&ClaimEvidenceBinding {
            requirement_ids: vec!["  ".to_string()],
            evidence_refs: vec![],
        }));
        assert!(claim_binding_has_evidence(&ClaimEvidenceBinding {
            requirement_ids: vec!["x".to_string()],
            evidence_refs: vec![],
        }));
    }
}
