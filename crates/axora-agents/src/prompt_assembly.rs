//! Prompt assembly for model-bound task execution.

use crate::patch_protocol::MetaGlyphCommand;
use crate::provider::{CacheRetention, ChatMessage, ModelBoundaryPayload, ModelRequest, ProviderKind};
use crate::transport::{InternalTaskAssignment, ProtoTransport};
use crate::task::{Task, TaskType};
use axora_proto as proto;
use serde_json::{json, Value};

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
                "description": "Pull dense code chunks that match the active task.",
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
                    "required": ["query"]
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

        let invariant_mission_context = vec![json!({
            "task_id": assignment.task_id.clone(),
            "task_type": format!("{:?}", assignment.task_type),
            "target_files": assignment.target_files.clone(),
            "target_symbols": assignment.target_symbols.clone(),
            "compression_mode": "metaglyph_toon",
            "meta_glyph_count": meta_glyphs.len(),
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
        provider: ProviderKind,
        model: String,
        max_output_tokens: u32,
        temperature: Option<f32>,
        stream: bool,
        cache_retention: CacheRetention,
    ) -> ModelRequest {
        ModelRequest {
            provider,
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
        let task = Task::new("Update src/auth.rs");
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
            target_files: vec!["crates/axora-memory/src/procedural_store.rs".to_string()],
            target_symbols: vec![],
            token_budget: 800,
            context_pack: None,
        };

        let assembly = PromptAssembly::for_task(&task, &assignment);

        assert!(assembly.payload.context_pack.is_none());
        let serialized = serde_json::to_string(&assembly.invariant_mission_context).unwrap();
        assert!(!serialized.contains("SKILL.md"));
    }
}
