//! Prompt assembly for model-bound task execution.

use crate::patch_protocol::MetaGlyphCommand;
use crate::provider::{CacheRetention, ChatMessage, ModelBoundaryPayload, ModelRequest};
use crate::task::{Task, TaskType};
use crate::transport::{InternalTaskAssignment, ProtoTransport};
use openakta_proto as proto;
use serde_json::{json, Value};

pub const REFACTORER_SYSTEM_PROMPT: &str = "You are the OPENAKTA RefactorerAgent. Your job is to design deterministic Python scripts for staged, sandboxed codebase transformations. Do not hand-write large Rust or TypeScript patches when a systematic script is more appropriate. Before using the sandboxed script path, request explicit human consent for Mass Script Mode. When approved, operate only on the declared target paths, preserve existing staged logic, and return concise execution outcomes.";
pub const MASS_REFACTOR_CONSENT_TEXT: &str = "Choose how to apply this codebase-wide refactor.\n\nOption A (Normal/Safe Mode): The LLM reads the files and generates deterministic unified diffs. Slower, consumes more tokens, but semantically safer.\n\nOption B (Mass Script Mode): The LLM generates a Python script and runs it in a sandboxed container against a staged workspace. Faster and more token-efficient, but a flawed script can overwrite intended staged logic across many files.\n\nApprove Mass Script Mode only if you want the refactor to run through the sandboxed Python workflow.";
pub const MASS_REFACTOR_SAFE_MODE_ID: &str = "safe_mode";
pub const MASS_REFACTOR_FAST_MODE_ID: &str = "mass_script_mode";
pub const MASS_REFACTOR_CONSENT_APPROVED: &str = "mass_script_approved";

/// Assembled prompt inputs before provider-specific shaping.
#[derive(Debug, Clone)]
pub struct PromptAssembly {
    /// Symbolic control commands rendered into the prompt prefix.
    pub meta_glyphs: Vec<MetaGlyphCommand>,
    /// Immutable system instructions.
    pub system_instructions: Vec<String>,
    /// Tool schemas exposed to the model.
    pub tool_schemas: Vec<Value>,
    /// Invariant mission context.
    pub invariant_mission_context: Vec<Value>,
    /// Dynamic task messages.
    pub recent_messages: Vec<ChatMessage>,
    /// Payload delivered through the model boundary.
    pub payload: ModelBoundaryPayload,
    /// Compression mode advertised to downstream consumers.
    pub compression_mode: proto::CompressionMode,
}

impl PromptAssembly {
    /// Build a prompt assembly for a task assignment.
    pub fn for_task(task: &Task, assignment: &InternalTaskAssignment) -> Self {
        Self::for_worker_task(task, assignment, None)
    }

    /// Build a prompt assembly for a specific worker.
    pub fn for_worker_task(
        task: &Task,
        assignment: &InternalTaskAssignment,
        worker_id: Option<&str>,
    ) -> Self {
        let proto_assignment = ProtoTransport::typed_task_assignment(assignment);
        let meta_glyphs = default_meta_glyphs(task, assignment);
        let mut system_instructions = vec![
            "You are executing a coordinator-issued task.".to_string(),
            "Use only the provided typed context.".to_string(),
        ];

        if !meta_glyphs.is_empty() {
            let rendered = meta_glyphs
                .iter()
                .map(MetaGlyphCommand::render)
                .collect::<Vec<_>>()
                .join("\n");
            system_instructions.push(format!("Control plane:\n{rendered}"));
        }

        if task.task_type == TaskType::CodeModification {
            system_instructions
                .push("Return git diff --unified=0 or AST SEARCH/REPLACE blocks only.".to_string());
        } else {
            system_instructions.push("Return a concise execution summary.".to_string());
        }
        if worker_id == Some("refactorer") {
            system_instructions.push(REFACTORER_SYSTEM_PROMPT.to_string());
            system_instructions.push(format!(
                "Before using mass_refactor, call request_user_input with this exact prompt text:\n{}",
                MASS_REFACTOR_CONSENT_TEXT
            ));
            system_instructions.push(format!(
                "Only invoke mass_refactor after the user explicitly selects '{}' and pass consent_mode='{}'.",
                MASS_REFACTOR_FAST_MODE_ID, MASS_REFACTOR_CONSENT_APPROVED
            ));
        }

        let mut tool_schemas = vec![
            json!({
                "name": "graph_retrieve_skills",
                "description": "Pull statistically relevant SKILL.md guidance on demand.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "task_id": {"type": "string"},
                        "focal_files": {"type": "array", "items": {"type": "string"}},
                        "focal_symbols": {"type": "array", "items": {"type": "string"}},
                        "skill_token_budget": {"type": "integer"},
                        "dense_limit": {"type": "integer"},
                        "bm25_limit": {"type": "integer"},
                        "include_diagnostics": {"type": "boolean"}
                    },
                    "required": ["query"]
                }
            }),
            json!({
                "name": "graph_retrieve_code",
                "description": "Pull structurally reachable code anchored to at least one focal file or focal symbol.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "task_id": {"type": "string"},
                        "focal_files": {"type": "array", "items": {"type": "string"}},
                        "focal_symbols": {"type": "array", "items": {"type": "string"}},
                        "token_budget": {"type": "integer"},
                        "dense_limit": {"type": "integer"},
                        "include_diagnostics": {"type": "boolean"}
                    },
                    "required": ["query"],
                    "anyOf": [
                        {"required": ["focal_files"]},
                        {"required": ["focal_symbols"]}
                    ]
                }
            }),
        ];

        if task.task_type == TaskType::CodeModification {
            tool_schemas.push(json!({
                "name": "patch_contract",
                "description": "emit patch-only output",
                "input_schema": {"type": "object"}
            }));
        }
        if worker_id == Some("refactorer") {
            tool_schemas.push(json!({
                "name": "request_user_input",
                "description": "Ask the human to choose Safe Mode or Mass Script Mode before running a staged scripted refactor.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "mission_id": {"type": "string"},
                        "turn_index": {"type": "integer"},
                        "kind": {"type": "string"},
                        "text": {"type": "string"},
                        "options_json": {"type": "string"},
                        "constraints_json": {"type": "string"},
                        "sensitive": {"type": "string"}
                    },
                    "required": ["mission_id", "turn_index", "kind", "text", "options_json"]
                }
            }));
            tool_schemas.push(json!({
                "name": "mass_refactor",
                "description": "Run a container-only Python refactor against staged target paths after explicit human approval.",
                "input_schema": {
                    "type": "object",
                    "properties": {
                        "script": {"type": "string"},
                        "target_paths": {"type": "array", "items": {"type": "string"}},
                        "consent_mode": {"type": "string"},
                        "timeout_secs": {"type": "integer"}
                    },
                    "required": ["script", "target_paths", "consent_mode"]
                }
            }));
        }

        let invariant_mission_context = vec![json!({
            "task_id": assignment.task_id.clone(),
            "task_type": format!("{:?}", assignment.task_type),
            "target_files": assignment.target_files.clone(),
            "target_symbols": assignment.target_symbols.clone(),
            "compression_mode": "metaglyph_toon",
            "meta_glyph_count": meta_glyphs.len(),
            "worker_id": worker_id.unwrap_or_default(),
        })];

        Self {
            meta_glyphs,
            system_instructions,
            tool_schemas,
            invariant_mission_context,
            recent_messages: vec![ChatMessage {
                role: "user".to_string(),
                content: task.description.clone(),
            }],
            payload: ModelBoundaryPayload::from_task_assignment(
                &proto_assignment,
                assignment.context_pack.clone(),
            ),
            compression_mode: proto::CompressionMode::MetaglyphToon,
        }
    }

    /// Convert the prompt assembly into a provider request.
    pub fn into_model_request(
        self,
        wire_profile: crate::wire_profile::WireProfile,
        model: String,
        max_output_tokens: u32,
        temperature: Option<f32>,
        stream: bool,
        cache_retention: CacheRetention,
    ) -> ModelRequest {
        ModelRequest {
            provider: wire_profile,
            model,
            system_instructions: self.system_instructions,
            tool_schemas: self.tool_schemas,
            invariant_mission_context: self.invariant_mission_context,
            payload: self.payload,
            recent_messages: self.recent_messages,
            max_output_tokens,
            temperature,
            stream,
            cache_retention,
        }
    }
}

fn default_meta_glyphs(task: &Task, assignment: &InternalTaskAssignment) -> Vec<MetaGlyphCommand> {
    let mut commands = assignment
        .target_files
        .iter()
        .map(|file| MetaGlyphCommand {
            opcode: crate::patch_protocol::MetaGlyphOpcode::Read,
            operand: file.clone(),
        })
        .collect::<Vec<_>>();

    if task.task_type == TaskType::CodeModification {
        if let Some(file) = assignment.target_files.first() {
            commands.push(MetaGlyphCommand {
                opcode: crate::patch_protocol::MetaGlyphOpcode::Patch,
                operand: file.clone(),
            });
        }
        commands.push(MetaGlyphCommand {
            opcode: crate::patch_protocol::MetaGlyphOpcode::Test,
            operand: assignment.task_id.clone(),
        });
    }

    commands
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_assembly_includes_rendered_meta_glyphs() {
        let task = Task::new("Update src/auth.rs").with_task_type(TaskType::CodeModification);
        let assignment = InternalTaskAssignment {
            task_id: "task-1".to_string(),
            title: "Update src/auth.rs".to_string(),
            description: "Update src/auth.rs".to_string(),
            task_type: TaskType::CodeModification,
            target_files: vec!["src/auth.rs".to_string()],
            target_symbols: vec!["auth::login".to_string()],
            token_budget: 1200,
            context_pack: None,
        };

        let assembly = PromptAssembly::for_task(&task, &assignment);
        let combined = assembly.system_instructions.join("\n");

        assert!(combined.contains("⟦READ⟧ src/auth.rs"));
        assert!(combined.contains("⟦PATCH⟧ src/auth.rs"));
        assert!(assembly
            .tool_schemas
            .iter()
            .any(|tool| tool.get("name") == Some(&json!("graph_retrieve_skills"))));
    }

    #[test]
    fn prompt_assembly_keeps_initial_payload_skill_free() {
        let task = Task::new("Inspect retrieval");
        let assignment = InternalTaskAssignment {
            task_id: "task-2".to_string(),
            title: "Inspect retrieval".to_string(),
            description: "Inspect retrieval".to_string(),
            task_type: TaskType::General,
            target_files: vec!["crates/openakta-memory/src/procedural_store.rs".to_string()],
            target_symbols: vec![],
            token_budget: 800,
            context_pack: None,
        };

        let assembly = PromptAssembly::for_task(&task, &assignment);

        assert!(assembly.payload.context_pack.is_none());
        let serialized = serde_json::to_string(&assembly.invariant_mission_context).unwrap();
        assert!(!serialized.contains("SKILL.md"));
    }

    #[test]
    fn refactorer_prompt_exposes_mass_refactor_contract() {
        let task = Task::new("Rename provider labels across files")
            .with_task_type(TaskType::CodeModification);
        let assignment = InternalTaskAssignment {
            task_id: "task-3".to_string(),
            title: "Rename provider labels".to_string(),
            description: "Rename provider labels across files".to_string(),
            task_type: TaskType::CodeModification,
            target_files: vec!["src/lib.rs".to_string(), "src/config.rs".to_string()],
            target_symbols: vec![],
            token_budget: 1600,
            context_pack: None,
        };

        let assembly = PromptAssembly::for_worker_task(&task, &assignment, Some("refactorer"));
        let combined = assembly.system_instructions.join("\n");

        assert!(combined.contains("RefactorerAgent"));
        assert!(combined.contains(MASS_REFACTOR_CONSENT_TEXT));
        assert!(assembly
            .tool_schemas
            .iter()
            .any(|tool| tool.get("name") == Some(&json!("mass_refactor"))));
        assert!(assembly
            .tool_schemas
            .iter()
            .any(|tool| tool.get("name") == Some(&json!("request_user_input"))));
    }
}
