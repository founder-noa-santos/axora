//! Canonical model-visible tool registry.

use crate::provider::ModelToolSchema;
use crate::task::TaskType;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// High-level tool kind for provider/runtime compatibility and UI rendering.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Retrieval,
    Filesystem,
    Command,
    Approval,
    Refactor,
}

/// Backend executor for a tool.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutorKind {
    Mcp,
    Runtime,
}

/// Relative tool cost class.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CostClass {
    Low,
    Medium,
    High,
}

/// Preferred UI renderer for the result.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UiRenderer {
    ToolCall,
    FileRead,
    SearchResults,
    ShellCommand,
    ApprovalRequest,
}

/// Tool-result normalization profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResultNormalizerKind {
    PlainText,
    Json,
    FileRead,
    SearchResults,
}

/// Canonical tool specification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters_json_schema: serde_json::Value,
    pub strict: bool,
    pub tool_kind: ToolKind,
    pub read_only: bool,
    pub mutating: bool,
    pub executor_kind: ExecutorKind,
    pub allowed_roles: Vec<String>,
    pub allowed_task_types: Vec<TaskType>,
    pub supported_models: Vec<String>,
    pub cost_class: CostClass,
    pub ui_renderer: UiRenderer,
    pub result_normalizer: ResultNormalizerKind,
    pub provider_safe: bool,
    pub requires_approval: bool,
}

impl ToolSpec {
    pub fn to_model_schema(&self) -> ModelToolSchema {
        ModelToolSchema {
            name: self.name.clone(),
            description: self.description.clone(),
            parameters: self.parameters_json_schema.clone(),
            strict: self.strict,
            tool_kind: Some(tool_kind_slug(self.tool_kind).to_string()),
            read_only: self.read_only,
            mutating: self.mutating,
            requires_approval: self.requires_approval,
            ui_renderer: Some(ui_renderer_slug(self.ui_renderer).to_string()),
        }
    }

    pub fn supports_role(&self, role: &str) -> bool {
        self.allowed_roles.iter().any(|allowed| allowed == role)
    }

    pub fn supports_task_type(&self, task_type: &TaskType) -> bool {
        self.allowed_task_types
            .iter()
            .any(|allowed| allowed == task_type)
    }

    pub fn supports_model(&self, model: &str) -> bool {
        let normalized = model.rsplit('/').next().unwrap_or(model);
        self.supported_models.is_empty()
            || self.supported_models.iter().any(|pattern| {
                model == pattern
                    || model.starts_with(pattern)
                    || normalized == pattern
                    || normalized.starts_with(pattern)
            })
    }
}

/// Built-in tool registry used by prompt assembly and the coordinator tool loop.
#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    specs: Vec<ToolSpec>,
}

impl ToolRegistry {
    pub fn builtin() -> Self {
        Self {
            specs: vec![
                ToolSpec {
                    name: "read_file".to_string(),
                    description: "Read a UTF-8 file inside the workspace.".to_string(),
                    parameters_json_schema: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"}
                        },
                        "required": ["path"]
                    }),
                    strict: false,
                    tool_kind: ToolKind::Filesystem,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["architect", "coder", "executor", "reviewer", "tester"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    allowed_task_types: vec![
                        TaskType::General,
                        TaskType::CodeModification,
                        TaskType::Review,
                        TaskType::Retrieval,
                    ],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string(), "deepseek".to_string()],
                    cost_class: CostClass::Low,
                    ui_renderer: UiRenderer::FileRead,
                    result_normalizer: ResultNormalizerKind::FileRead,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "list_dir".to_string(),
                    description: "List workspace entries inside a directory.".to_string(),
                    parameters_json_schema: json!({
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "max_entries": {"type": "integer"}
                        }
                    }),
                    strict: false,
                    tool_kind: ToolKind::Filesystem,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["architect", "coder", "executor", "reviewer", "tester", "refactorer"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    allowed_task_types: vec![
                        TaskType::General,
                        TaskType::CodeModification,
                        TaskType::Review,
                        TaskType::Retrieval,
                    ],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string(), "deepseek".to_string()],
                    cost_class: CostClass::Low,
                    ui_renderer: UiRenderer::SearchResults,
                    result_normalizer: ResultNormalizerKind::SearchResults,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "glob_paths".to_string(),
                    description: "Find workspace paths with a glob pattern.".to_string(),
                    parameters_json_schema: json!({
                        "type": "object",
                        "properties": {
                            "pattern": {"type": "string"},
                            "path": {"type": "string"},
                            "max_results": {"type": "integer"}
                        },
                        "required": ["pattern"]
                    }),
                    strict: false,
                    tool_kind: ToolKind::Filesystem,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["architect", "coder", "executor", "reviewer", "tester", "refactorer"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    allowed_task_types: vec![
                        TaskType::General,
                        TaskType::CodeModification,
                        TaskType::Review,
                        TaskType::Retrieval,
                    ],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string(), "deepseek".to_string()],
                    cost_class: CostClass::Low,
                    ui_renderer: UiRenderer::SearchResults,
                    result_normalizer: ResultNormalizerKind::SearchResults,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "graph_retrieve_skills".to_string(),
                    description: "Pull statistically relevant SKILL.md guidance on demand.".to_string(),
                    parameters_json_schema: json!({
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
                    }),
                    strict: false,
                    tool_kind: ToolKind::Retrieval,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["architect", "coder", "executor", "refactorer", "tester", "reviewer"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    allowed_task_types: vec![
                        TaskType::General,
                        TaskType::CodeModification,
                        TaskType::Review,
                        TaskType::Retrieval,
                    ],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string(), "deepseek".to_string()],
                    cost_class: CostClass::Medium,
                    ui_renderer: UiRenderer::SearchResults,
                    result_normalizer: ResultNormalizerKind::SearchResults,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "graph_retrieve_code".to_string(),
                    description: "Pull structurally reachable code anchored to at least one focal file or focal symbol.".to_string(),
                    parameters_json_schema: json!({
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
                    }),
                    strict: false,
                    tool_kind: ToolKind::Retrieval,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["architect", "coder", "executor", "refactorer", "tester", "reviewer"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    allowed_task_types: vec![
                        TaskType::General,
                        TaskType::CodeModification,
                        TaskType::Review,
                        TaskType::Retrieval,
                    ],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string(), "deepseek".to_string()],
                    cost_class: CostClass::Medium,
                    ui_renderer: UiRenderer::SearchResults,
                    result_normalizer: ResultNormalizerKind::SearchResults,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "run_command".to_string(),
                    description: "Run a bounded command in the workspace.".to_string(),
                    parameters_json_schema: json!({
                        "type": "object",
                        "properties": {
                            "program": {"type": "string"},
                            "args": {"type": "array", "items": {"type": "string"}}
                        },
                        "required": ["program"]
                    }),
                    strict: false,
                    tool_kind: ToolKind::Command,
                    read_only: false,
                    mutating: true,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["executor", "tester"].into_iter().map(str::to_string).collect(),
                    allowed_task_types: vec![TaskType::General, TaskType::Review],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string()],
                    cost_class: CostClass::High,
                    ui_renderer: UiRenderer::ShellCommand,
                    result_normalizer: ResultNormalizerKind::Json,
                    provider_safe: true,
                    requires_approval: true,
                },
                ToolSpec {
                    name: "request_user_input".to_string(),
                    description: "Ask the human to choose between explicit options before continuing.".to_string(),
                    parameters_json_schema: json!({
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
                    }),
                    strict: false,
                    tool_kind: ToolKind::Approval,
                    read_only: true,
                    mutating: false,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["refactorer"].into_iter().map(str::to_string).collect(),
                    allowed_task_types: vec![TaskType::CodeModification, TaskType::General],
                    supported_models: vec!["gpt-".to_string(), "qwen".to_string()],
                    cost_class: CostClass::Low,
                    ui_renderer: UiRenderer::ApprovalRequest,
                    result_normalizer: ResultNormalizerKind::Json,
                    provider_safe: true,
                    requires_approval: false,
                },
                ToolSpec {
                    name: "mass_refactor".to_string(),
                    description: "Run a sandboxed Python refactor against staged target paths after explicit human approval.".to_string(),
                    parameters_json_schema: json!({
                        "type": "object",
                        "properties": {
                            "script": {"type": "string"},
                            "target_paths": {"type": "array", "items": {"type": "string"}},
                            "consent_mode": {"type": "string"},
                            "timeout_secs": {"type": "integer"}
                        },
                        "required": ["script", "target_paths", "consent_mode"]
                    }),
                    strict: false,
                    tool_kind: ToolKind::Refactor,
                    read_only: false,
                    mutating: true,
                    executor_kind: ExecutorKind::Mcp,
                    allowed_roles: vec!["refactorer"].into_iter().map(str::to_string).collect(),
                    allowed_task_types: vec![TaskType::CodeModification],
                    supported_models: vec!["gpt-".to_string()],
                    cost_class: CostClass::High,
                    ui_renderer: UiRenderer::ToolCall,
                    result_normalizer: ResultNormalizerKind::Json,
                    provider_safe: true,
                    requires_approval: true,
                },
            ],
        }
    }

    pub fn specs(&self) -> &[ToolSpec] {
        &self.specs
    }

    pub fn get(&self, name: &str) -> Option<&ToolSpec> {
        self.specs.iter().find(|spec| spec.name == name)
    }

    pub fn slice(&self, role: &str, task_type: &TaskType, model: &str) -> Vec<ToolSpec> {
        self.specs
            .iter()
            .filter(|spec| spec.provider_safe)
            .filter(|spec| spec.supports_role(role))
            .filter(|spec| spec.supports_task_type(task_type))
            .filter(|spec| spec.supports_model(model))
            .cloned()
            .collect()
    }

    pub fn allowed_tool_names(&self, role: &str, task_type: &TaskType) -> Vec<String> {
        self.specs
            .iter()
            .filter(|spec| spec.provider_safe)
            .filter(|spec| spec.supports_role(role))
            .filter(|spec| spec.supports_task_type(task_type))
            .map(|spec| spec.name.clone())
            .collect()
    }

    pub fn model_schemas_for(
        &self,
        role: &str,
        task_type: &TaskType,
        model: &str,
    ) -> Vec<ModelToolSchema> {
        self.slice(role, task_type, model)
            .into_iter()
            .map(|spec| spec.to_model_schema())
            .collect()
    }

    pub fn model_schemas_for_allowed_tools(
        &self,
        role: &str,
        task_type: &TaskType,
        model: &str,
        allowed_tools: &[String],
    ) -> Vec<ModelToolSchema> {
        self.slice(role, task_type, model)
            .into_iter()
            .filter(|spec| allowed_tools.iter().any(|allowed| allowed == &spec.name))
            .map(|spec| spec.to_model_schema())
            .collect()
    }
}

fn tool_kind_slug(kind: ToolKind) -> &'static str {
    match kind {
        ToolKind::Retrieval => "retrieval",
        ToolKind::Filesystem => "filesystem",
        ToolKind::Command => "command",
        ToolKind::Approval => "approval",
        ToolKind::Refactor => "refactor",
    }
}

fn ui_renderer_slug(renderer: UiRenderer) -> &'static str {
    match renderer {
        UiRenderer::ToolCall => "tool_call",
        UiRenderer::FileRead => "file_read",
        UiRenderer::SearchResults => "search_results",
        UiRenderer::ShellCommand => "shell_command",
        UiRenderer::ApprovalRequest => "approval_request",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_provider_prefixed_model_labels() {
        let registry = ToolRegistry::builtin();
        let tools = registry.slice("architect", &TaskType::Retrieval, "openai/gpt-5.4");
        let tool_names = tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>();

        assert!(tool_names.contains(&"read_file"));
        assert!(tool_names.contains(&"list_dir"));
        assert!(tool_names.contains(&"glob_paths"));
    }

    #[test]
    fn retrieval_slice_stays_read_only_for_architect_role() {
        let registry = ToolRegistry::builtin();
        let tools = registry.slice("architect", &TaskType::Retrieval, "gpt-5.4");

        assert!(tools.iter().all(|tool| tool.read_only));
        assert!(tools.iter().all(|tool| !tool.mutating));
        assert!(!tools.iter().any(|tool| tool.name == "apply_patch"));
        assert!(!tools.iter().any(|tool| tool.name == "run_command"));
    }
}
