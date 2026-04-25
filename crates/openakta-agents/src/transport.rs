//! Typed protobuf transport adapters for coordinator and worker messages.

use crate::assignment_contract::{
    default_expected_artifacts, default_lane_for_task_type, default_termination_condition,
    default_worker_role, PlanningOriginRef, WorkerAssignmentContract, WorkerExecutionBudget,
};
use crate::patch_protocol::{
    ContextPack, MetaGlyphCommand, MetaGlyphOpcode, PatchEnvelope, PatchFormat, PatchReceipt,
    ValidationFact,
};
use crate::task::{Task, TaskType};
use crate::tool_registry::ToolRegistry;
use openakta_proto as proto;
use serde::{Deserialize, Serialize};

/// Internal context reference.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalContextReference {
    /// File path.
    pub file_path: String,
    /// Symbol path.
    pub symbol_path: Option<String>,
    /// Start line.
    pub start_line: u32,
    /// End line.
    pub end_line: u32,
    /// Stable block identifier.
    pub block_id: Option<String>,
}

/// Internal token usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalTokenUsage {
    /// Provider name.
    pub provider: String,
    /// Input tokens.
    pub input_tokens: u32,
    /// Output tokens.
    pub output_tokens: u32,
    /// Cache write tokens.
    pub cache_write_tokens: u32,
    /// Cache read tokens.
    pub cache_read_tokens: u32,
    /// Uncached input tokens.
    pub uncached_input_tokens: u32,
    /// Effective tokens saved.
    pub effective_tokens_saved: u32,
}

/// Internal task assignment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalTaskAssignment {
    /// Task identifier.
    pub task_id: String,
    /// Title.
    pub title: String,
    /// Description.
    pub description: String,
    /// Task type.
    pub task_type: TaskType,
    /// Target files.
    pub target_files: Vec<String>,
    /// Target symbols.
    pub target_symbols: Vec<String>,
    /// Token budget.
    pub token_budget: u32,
    /// Typed context pack carried on the orchestration side.
    pub context_pack: Option<ContextPack>,
    /// Canonical runtime assignment contract.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_contract: Option<WorkerAssignmentContract>,
}

impl InternalTaskAssignment {
    /// Build the compatibility payload from the canonical worker assignment contract.
    pub fn from_contract(
        contract: WorkerAssignmentContract,
        context_pack: Option<ContextPack>,
    ) -> Self {
        let token_budget = contract.budget.token_budget;
        Self {
            task_id: contract.task_id.clone(),
            title: contract.legacy_title().to_string(),
            description: contract.legacy_description().to_string(),
            task_type: contract.task_type.clone(),
            target_files: contract.target_files.clone(),
            target_symbols: contract.target_symbols.clone(),
            token_budget,
            context_pack,
            canonical_contract: Some(contract),
        }
    }

    /// Return the canonical contract, deriving compatibility defaults for legacy callers.
    pub fn canonical_contract(&self) -> WorkerAssignmentContract {
        if let Some(contract) = &self.canonical_contract {
            return contract.clone();
        }

        let lane = default_lane_for_task_type(&self.task_type);
        let allowed_tools = ToolRegistry::builtin()
            .allowed_tool_names(default_worker_role(&self.task_type), &self.task_type);
        let workspace_revision_token = self.context_pack.as_ref().and_then(|pack| {
            (!pack.base_revision.trim().is_empty()).then(|| pack.base_revision.clone())
        });
        let context_artifact_refs = self
            .context_pack
            .as_ref()
            .map(|pack| vec![format!("context_pack:{}", pack.id)])
            .unwrap_or_default();

        WorkerAssignmentContract {
            session_id: self.task_id.clone(),
            story_id: None,
            task_id: self.task_id.clone(),
            task_type: self.task_type.clone(),
            lane,
            goal: self.description.clone(),
            requirement_refs: Vec::new(),
            context_artifact_refs,
            target_files: self.target_files.clone(),
            target_symbols: self.target_symbols.clone(),
            expected_artifacts: default_expected_artifacts(lane, &self.task_type),
            allowed_tools,
            budget: WorkerExecutionBudget::compat_defaults(self.token_budget),
            termination_condition: default_termination_condition(lane, &self.task_type),
            verification_required: self.task_type == TaskType::CodeModification,
            workspace_revision_token,
            planning_origin_ref: PlanningOriginRef::direct(),
        }
    }
}

/// Internal progress update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InternalProgressUpdate {
    /// Task identifier.
    pub task_id: String,
    /// Stage label.
    pub stage: String,
    /// Status message.
    pub message: String,
    /// Completion ratio.
    pub completion_ratio: f32,
}

/// Internal result submission.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalResultSubmission {
    /// Task identifier.
    pub task_id: String,
    /// Success flag.
    pub success: bool,
    /// Validated patch envelope for code-edit tasks.
    pub patch: Option<PatchEnvelope>,
    /// Deterministic patch application receipt when applicable.
    pub patch_receipt: Option<PatchReceipt>,
    /// Usage.
    pub token_usage: InternalTokenUsage,
    /// Context references.
    pub context_references: Vec<InternalContextReference>,
    /// Summary.
    pub summary: String,
    /// Error message when unsuccessful.
    pub error_message: String,
    /// Structured diagnostic payload encoded as TOON.
    pub diagnostic_toon: Option<String>,
}

/// Internal blocker alert.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalBlockerAlert {
    /// Task identifier.
    pub task_id: String,
    /// Severity label.
    pub severity: String,
    /// Message.
    pub message: String,
    /// Whether the blocker is retryable.
    pub retryable: bool,
}

/// Internal workflow transition event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InternalWorkflowTransitionEvent {
    /// Task identifier.
    pub task_id: String,
    /// From state.
    pub from_state: String,
    /// To state.
    pub to_state: String,
    /// Reason.
    pub reason: String,
    /// Retry count.
    pub retry_count: u32,
    /// Whether this state is terminal.
    pub terminal: bool,
}

/// Proto transport adapter.
pub struct ProtoTransport;

impl ProtoTransport {
    /// Create a typed task assignment from an internal task.
    pub fn task_assignment(task: &Task, title: &str, token_budget: u32) -> proto::TaskAssignment {
        proto::TaskAssignment {
            task_id: task.id.clone(),
            title: title.to_string(),
            description: task.description.clone(),
            task_type: to_proto_task_type(task.task_type.clone()) as i32,
            target_files: Vec::new(),
            target_symbols: Vec::new(),
            token_budget,
            context_pack: None,
        }
    }

    /// Convert an internal task assignment into a protobuf task assignment payload.
    pub fn typed_task_assignment(assignment: &InternalTaskAssignment) -> proto::TaskAssignment {
        proto::TaskAssignment {
            task_id: assignment.task_id.clone(),
            title: assignment.title.clone(),
            description: assignment.description.clone(),
            task_type: to_proto_task_type(assignment.task_type.clone()) as i32,
            target_files: assignment.target_files.clone(),
            target_symbols: assignment.target_symbols.clone(),
            token_budget: assignment.token_budget,
            context_pack: assignment
                .context_pack
                .as_ref()
                .map(|pack| proto::ContextPack {
                    id: pack.id.clone(),
                    task_id: pack.task_id.clone(),
                    target_files: pack.target_files.clone(),
                    symbols: pack.symbols.clone(),
                    spans: pack
                        .spans
                        .iter()
                        .map(|span| proto::ContextSpan {
                            file_path: span.file_path.clone(),
                            start_line: span.start_line as u32,
                            end_line: span.end_line as u32,
                            symbol_path: span.symbol_path.clone(),
                        })
                        .collect(),
                    toon_payload: pack.to_toon().unwrap_or_default(),
                    base_revision: pack.base_revision.clone(),
                    meta_glyph_commands: default_meta_glyphs(assignment)
                        .into_iter()
                        .map(|command| proto::MetaGlyphCommand {
                            opcode: match command.opcode {
                                MetaGlyphOpcode::Read => proto::MetaGlyphOpcode::Read as i32,
                                MetaGlyphOpcode::Patch => proto::MetaGlyphOpcode::Patch as i32,
                                MetaGlyphOpcode::Test => proto::MetaGlyphOpcode::Test as i32,
                                MetaGlyphOpcode::Debug => proto::MetaGlyphOpcode::Debug as i32,
                            },
                            operand: command.operand,
                        })
                        .collect(),
                    compression_mode: proto::CompressionMode::MetaglyphToon as i32,
                    latent_context: Vec::new(),
                    latent_context_handle: String::new(),
                    cryptographic_signature: Vec::new(),
                    audit_correlation_id: format!("ctx:{}", pack.task_id),
                }),
        }
    }

    /// Convert an internal result submission into a protobuf message payload.
    pub fn result_submission(result: &InternalResultSubmission) -> proto::ResultSubmission {
        proto::ResultSubmission {
            task_id: result.task_id.clone(),
            success: result.success,
            patch: result.patch.as_ref().map(to_proto_patch),
            token_usage: Some(proto::TokenUsage {
                provider: result.token_usage.provider.clone(),
                input_tokens: result.token_usage.input_tokens,
                output_tokens: result.token_usage.output_tokens,
                total_tokens: result.token_usage.input_tokens + result.token_usage.output_tokens,
                cache_write_tokens: result.token_usage.cache_write_tokens,
                cache_read_tokens: result.token_usage.cache_read_tokens,
                uncached_input_tokens: result.token_usage.uncached_input_tokens,
                effective_tokens_saved: result.token_usage.effective_tokens_saved,
            }),
            context_references: result
                .context_references
                .iter()
                .map(|reference| proto::ContextReference {
                    file_path: reference.file_path.clone(),
                    symbol_path: reference.symbol_path.clone().unwrap_or_default(),
                    start_line: reference.start_line,
                    end_line: reference.end_line,
                    block_id: reference.block_id.clone().unwrap_or_default(),
                })
                .collect(),
            summary: result.summary.clone(),
            error_message: result.error_message.clone(),
            patch_receipt: result.patch_receipt.as_ref().map(to_proto_patch_receipt),
            diagnostic_toon: result.diagnostic_toon.clone().unwrap_or_default(),
        }
    }
}

fn to_proto_task_type(task_type: TaskType) -> proto::TaskPayloadType {
    match task_type {
        TaskType::General => proto::TaskPayloadType::General,
        TaskType::CodeModification => proto::TaskPayloadType::CodeModification,
        TaskType::Review => proto::TaskPayloadType::Review,
        TaskType::Retrieval => proto::TaskPayloadType::Retrieval,
    }
}

fn default_meta_glyphs(assignment: &InternalTaskAssignment) -> Vec<MetaGlyphCommand> {
    let mut commands = assignment
        .target_files
        .iter()
        .map(|file| MetaGlyphCommand {
            opcode: MetaGlyphOpcode::Read,
            operand: file.clone(),
        })
        .collect::<Vec<_>>();

    if assignment.task_type == TaskType::CodeModification {
        if let Some(file) = assignment.target_files.first() {
            commands.push(MetaGlyphCommand {
                opcode: MetaGlyphOpcode::Patch,
                operand: file.clone(),
            });
        }
        commands.push(MetaGlyphCommand {
            opcode: MetaGlyphOpcode::Test,
            operand: assignment.task_id.clone(),
        });
    }

    commands
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::assignment_contract::{
        PlanningOriginRef, WorkerAssignmentContract, WorkerAssignmentLane, WorkerExecutionBudget,
        WorkerTerminationCondition,
    };

    #[test]
    fn from_contract_keeps_canonical_assignment() {
        let contract = WorkerAssignmentContract {
            session_id: "session-1".to_string(),
            story_id: Some("story-1".to_string()),
            task_id: "task-1".to_string(),
            task_type: TaskType::CodeModification,
            lane: WorkerAssignmentLane::Execution,
            goal: "Patch src/lib.rs".to_string(),
            requirement_refs: vec!["req-1".to_string()],
            context_artifact_refs: vec!["context_pack:ctx-1".to_string()],
            target_files: vec!["src/lib.rs".to_string()],
            target_symbols: vec!["lib::run".to_string()],
            expected_artifacts: vec!["validated_patch".to_string()],
            allowed_tools: vec!["read_file".to_string()],
            budget: WorkerExecutionBudget::compat_defaults(900),
            termination_condition: WorkerTerminationCondition::ValidatedPatchRequired,
            verification_required: true,
            workspace_revision_token: Some("rev-1".to_string()),
            planning_origin_ref: PlanningOriginRef::planned(
                Some("prepared-1".to_string()),
                Some("work-1".to_string()),
                Some("plan-1".to_string()),
            ),
        };
        let context_pack = Some(ContextPack {
            id: "ctx-1".to_string(),
            task_id: "task-1".to_string(),
            target_files: vec!["src/lib.rs".to_string()],
            symbols: vec!["lib::run".to_string()],
            spans: Vec::new(),
            retrieval_hits: Vec::new(),
            ast_summaries: Vec::new(),
            symbol_maps: Vec::new(),
            validation_facts: Vec::new(),
            base_revision: "rev-1".to_string(),
        });

        let assignment = InternalTaskAssignment::from_contract(contract.clone(), context_pack);
        assert_eq!(assignment.task_id, "task-1");
        assert_eq!(assignment.token_budget, 900);
        assert_eq!(assignment.canonical_contract(), contract);
    }

    #[test]
    fn canonical_contract_derives_workspace_revision_from_context_pack() {
        let assignment = InternalTaskAssignment {
            task_id: "task-2".to_string(),
            title: "Inspect".to_string(),
            description: "Inspect".to_string(),
            task_type: TaskType::Retrieval,
            target_files: vec!["Cargo.toml".to_string()],
            target_symbols: Vec::new(),
            token_budget: 400,
            context_pack: Some(ContextPack {
                id: "ctx-2".to_string(),
                task_id: "task-2".to_string(),
                target_files: vec!["Cargo.toml".to_string()],
                symbols: Vec::new(),
                spans: Vec::new(),
                retrieval_hits: Vec::new(),
                ast_summaries: Vec::new(),
                symbol_maps: Vec::new(),
                validation_facts: Vec::new(),
                base_revision: "rev-2".to_string(),
            }),
            canonical_contract: None,
        };

        let contract = assignment.canonical_contract();
        assert_eq!(contract.workspace_revision_token.as_deref(), Some("rev-2"));
        assert_eq!(contract.context_artifact_refs, vec!["context_pack:ctx-2"]);
    }
}

fn to_proto_patch(patch: &PatchEnvelope) -> proto::PatchEnvelope {
    proto::PatchEnvelope {
        task_id: patch.task_id.clone(),
        target_files: patch.target_files.clone(),
        format: match patch.format {
            PatchFormat::UnifiedDiffZero => proto::PatchFormat::UnifiedDiffZero as i32,
            PatchFormat::AstSearchReplace => proto::PatchFormat::AstSearchReplace as i32,
        },
        patch_text: patch.patch_text.clone().unwrap_or_default(),
        search_replace_blocks: patch
            .search_replace_blocks
            .iter()
            .map(|block| proto::SearchReplaceBlock {
                file_path: block.file_path.clone(),
                symbol_path: block.symbol_path.clone().unwrap_or_default(),
                start_line: block.start_line.unwrap_or_default() as u32,
                end_line: block.end_line.unwrap_or_default() as u32,
                search: block.search.clone(),
                replace: block.replace.clone(),
            })
            .collect(),
        base_revision: patch.base_revision.clone(),
        validation: patch
            .validation
            .iter()
            .map(|fact: &ValidationFact| proto::ValidationFact {
                key: fact.key.clone(),
                value: fact.value.clone(),
            })
            .collect(),
    }
}

fn to_proto_patch_receipt(receipt: &PatchReceipt) -> proto::PatchReceipt {
    proto::PatchReceipt {
        task_id: receipt.task_id.clone(),
        status: match receipt.status {
            crate::patch_protocol::PatchApplyStatus::Applied => {
                proto::PatchApplyStatus::Applied as i32
            }
            crate::patch_protocol::PatchApplyStatus::Conflict => {
                proto::PatchApplyStatus::Conflict as i32
            }
            crate::patch_protocol::PatchApplyStatus::Invalid => {
                proto::PatchApplyStatus::Invalid as i32
            }
            crate::patch_protocol::PatchApplyStatus::StaleBase => {
                proto::PatchApplyStatus::StaleBase as i32
            }
        },
        applied_revision: receipt.applied_revision.clone(),
        message: receipt.message.clone(),
        affected_files: receipt.affected_files.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proto_result_submission_contains_patch_and_usage() {
        let result = InternalResultSubmission {
            task_id: "task-1".to_string(),
            success: true,
            patch: Some(PatchEnvelope {
                task_id: "task-1".to_string(),
                target_files: vec!["src/auth.rs".to_string()],
                format: PatchFormat::UnifiedDiffZero,
                patch_text: Some(
                    "--- src/auth.rs\n+++ src/auth.rs\n@@ -1,0 +1,1 @@\n+fn login() {}\n"
                        .to_string(),
                ),
                search_replace_blocks: Vec::new(),
                base_revision: "rev-1".to_string(),
                validation: vec![ValidationFact {
                    key: "diff_only".to_string(),
                    value: "true".to_string(),
                }],
            }),
            patch_receipt: Some(PatchReceipt {
                task_id: "task-1".to_string(),
                status: crate::patch_protocol::PatchApplyStatus::Applied,
                applied_revision: "rev-2".to_string(),
                message: "patch applied".to_string(),
                affected_files: vec!["src/auth.rs".to_string()],
            }),
            token_usage: InternalTokenUsage {
                provider: "openai".to_string(),
                input_tokens: 100,
                output_tokens: 20,
                cache_write_tokens: 60,
                cache_read_tokens: 0,
                uncached_input_tokens: 40,
                effective_tokens_saved: 0,
            },
            context_references: vec![InternalContextReference {
                file_path: "src/auth.rs".to_string(),
                symbol_path: Some("auth::login".to_string()),
                start_line: 10,
                end_line: 20,
                block_id: Some("block-1".to_string()),
            }],
            summary: "updated login".to_string(),
            error_message: String::new(),
            diagnostic_toon: None,
        };

        let proto = ProtoTransport::result_submission(&result);
        assert!(proto.patch.is_some());
        assert!(proto.patch_receipt.is_some());
        assert!(proto.token_usage.is_some());
        assert_eq!(proto.context_references.len(), 1);
    }
}
