//! LLM-backed raw task decomposition.

use super::DecomposerConfig;
use crate::error::AgentError;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type BackendFuture<'a> = Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>>;

/// Raw task emitted before DAG materialization.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawTask {
    /// Stable raw task identifier.
    pub id: String,
    /// Task description.
    pub description: String,
    /// Raw task IDs this task depends on.
    pub dependencies: Vec<String>,
    /// Estimated duration in minutes.
    pub estimated_duration: u64,
    /// Required capabilities.
    pub capabilities: Vec<String>,
    /// Optional files or symbols that help dependency inference.
    #[serde(default)]
    pub target_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawTaskEnvelope {
    tasks: Vec<RawTask>,
}

/// Interface for backends that return structured decomposition JSON.
pub trait LLMBackend: Send + Sync {
    /// Generates a response for a decomposition prompt.
    fn generate<'a>(&'a self, prompt: &'a str, model: &'a str) -> BackendFuture<'a>;
}

/// Deterministic backend used by default and in tests.
pub struct DeterministicLLMBackend;

impl DeterministicLLMBackend {
    fn synthesize_tasks(mission: &str, max_tasks: usize) -> Vec<RawTask> {
        let normalized = mission
            .replace(" and ", ", ")
            .replace(" then ", ", ")
            .replace(" with ", ", ")
            .replace(" plus ", ", ");
        let mut parts = normalized
            .split(',')
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();

        if parts.is_empty() {
            parts = vec![mission.trim().to_string()];
        }

        if parts.len() > max_tasks {
            parts.truncate(max_tasks);
        }

        parts
            .into_iter()
            .enumerate()
            .map(|(index, part)| {
                let id = format!("task-{index}");
                let description = normalize_description(&part, index);
                let dependencies = dependencies_for_description(index, &description);
                let capabilities = capabilities_for_description(&description);
                let target_files = infer_target_files(&description);

                RawTask {
                    id,
                    description,
                    dependencies,
                    estimated_duration: 10 + ((index as u64 % 4) * 5),
                    capabilities,
                    target_files,
                }
            })
            .collect()
    }
}

impl LLMBackend for DeterministicLLMBackend {
    fn generate<'a>(&'a self, prompt: &'a str, _model: &'a str) -> BackendFuture<'a> {
        Box::pin(async move {
            let mission = prompt
                .lines()
                .find_map(|line| line.strip_prefix("Mission: "))
                .unwrap_or(prompt);
            let max_tasks = prompt
                .lines()
                .find_map(|line| line.strip_prefix("MaxTasks: "))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50);
            let tasks = Self::synthesize_tasks(mission, max_tasks);
            serde_json::to_string(&serde_json::json!({ "tasks": tasks }))
                .map_err(|err| AgentError::Serialization(err.to_string()).into())
        })
    }
}

/// LLM-based mission decomposer.
pub struct LLMDecomposer {
    backend: Arc<dyn LLMBackend>,
    model: String,
    max_tasks: usize,
}

impl LLMDecomposer {
    /// Creates a decomposer with the default deterministic backend.
    pub fn new(model: String, max_tasks: usize) -> Self {
        Self {
            backend: Arc::new(DeterministicLLMBackend),
            model,
            max_tasks,
        }
    }

    /// Creates a decomposer with an injected backend.
    pub fn with_backend(model: String, max_tasks: usize, backend: Arc<dyn LLMBackend>) -> Self {
        Self {
            backend,
            model,
            max_tasks,
        }
    }

    /// Creates from the shared decomposer config.
    pub fn from_config(config: &DecomposerConfig) -> Self {
        Self::new(config.llm_model.clone(), config.max_tasks)
    }

    /// Builds the LLM prompt.
    pub fn build_prompt(&self, mission: &str, feedback: Option<&str>) -> String {
        let mut prompt = format!(
            "You are an expert task planner. Decompose the following mission into discrete, actionable tasks.\n\
Mission: {mission}\n\
MaxTasks: {}\n\
\n\
For each task, provide:\n\
1. Task ID (unique identifier)\n\
2. Task Description (clear, actionable)\n\
3. Dependencies (list of task IDs this task depends on)\n\
4. Estimated Duration (in minutes)\n\
5. Required Capabilities (coding, testing, documentation, refactorer for broad scripted codebase changes, etc.)\n\
\n\
Output format (JSON):\n\
{{\"tasks\":[{{\"id\":\"task-1\",\"description\":\"...\",\"dependencies\":[],\"estimated_duration\":10,\"capabilities\":[\"coding\"],\"target_files\":[]}}]}}\n\
\n\
Rules:\n\
- Each task must be independently executable\n\
- Dependencies must form a DAG (no cycles)\n\
- Max {} tasks\n\
- Identify tasks that can run in parallel",
            self.max_tasks, self.max_tasks
        );

        if let Some(feedback) = feedback {
            prompt.push_str("\n\nPrevious attempt failed validation. Fix these issues:\n");
            prompt.push_str(feedback);
        }

        prompt
    }

    /// Runs decomposition via the backend.
    pub async fn decompose(&self, mission: &str, feedback: Option<&str>) -> Result<Vec<RawTask>> {
        let prompt = self.build_prompt(mission, feedback);
        let response = self.backend.generate(&prompt, &self.model).await?;
        self.parse_response(&response)
    }

    /// Strictly parses backend output.
    pub fn parse_response(&self, response: &str) -> Result<Vec<RawTask>> {
        let envelope: RawTaskEnvelope = serde_json::from_str(response)
            .map_err(|err| AgentError::Serialization(err.to_string()))?;

        if envelope.tasks.is_empty() {
            return Err(
                AgentError::InvalidDecomposition("LLM returned no tasks".to_string()).into(),
            );
        }
        if envelope.tasks.len() > self.max_tasks {
            return Err(AgentError::InvalidDecomposition(format!(
                "LLM returned {} tasks, above configured max {}",
                envelope.tasks.len(),
                self.max_tasks
            ))
            .into());
        }

        let mut ids = HashSet::new();
        for task in &envelope.tasks {
            if task.id.trim().is_empty() {
                return Err(AgentError::InvalidDecomposition(
                    "task id cannot be empty".to_string(),
                )
                .into());
            }
            if !ids.insert(task.id.clone()) {
                return Err(AgentError::InvalidDecomposition(format!(
                    "duplicate task id {}",
                    task.id
                ))
                .into());
            }
            if task.description.trim().is_empty() {
                return Err(AgentError::InvalidDecomposition(format!(
                    "task {} has empty description",
                    task.id
                ))
                .into());
            }
            if task.estimated_duration == 0 {
                return Err(AgentError::InvalidDecomposition(format!(
                    "task {} has non-positive duration",
                    task.id
                ))
                .into());
            }
            if task.estimated_duration > 24 * 60 {
                return Err(AgentError::InvalidDecomposition(format!(
                    "task {} duration {} exceeds 24h bound",
                    task.id, task.estimated_duration
                ))
                .into());
            }
        }

        Ok(envelope.tasks)
    }
}

fn normalize_description(description: &str, index: usize) -> String {
    let lower = description.to_lowercase();
    if lower.contains("test") {
        format!("Write tests for {}", description.trim())
    } else if lower.contains("doc") {
        format!("Document {}", description.trim())
    } else if lower.contains("deploy") || lower.contains("rollout") {
        format!("Validate deployment for {}", description.trim())
    } else if lower.contains("analy") || lower.contains("requirement") {
        format!("Analyze requirements for {}", description.trim())
    } else if index == 0 {
        format!("Analyze {}", description.trim())
    } else {
        format!("Implement {}", description.trim())
    }
}

fn dependencies_for_description(index: usize, description: &str) -> Vec<String> {
    let lower = description.to_lowercase();
    if index == 0 {
        return Vec::new();
    }

    if index > 1 && lower.contains("analyze") {
        Vec::new()
    } else {
        vec![format!("task-{}", index.saturating_sub(1))]
    }
}

fn capabilities_for_description(description: &str) -> Vec<String> {
    let lower = description.to_lowercase();
    let mut capabilities = Vec::new();

    if lower.contains("test") {
        capabilities.push("testing".to_string());
    }
    if [
        "rename",
        "codebase-wide",
        "across files",
        "systematic refactor",
        "mass refactor",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        capabilities.push("refactorer".to_string());
    }
    if lower.contains("document") {
        capabilities.push("documentation".to_string());
    }
    if lower.contains("deploy") || lower.contains("rollout") {
        capabilities.push("operations".to_string());
    }
    if lower.contains("security") {
        capabilities.push("security".to_string());
    }
    if capabilities.is_empty() {
        capabilities.push("coding".to_string());
    }

    capabilities
}

fn infer_target_files(description: &str) -> Vec<String> {
    let lower = description.to_lowercase();
    let mut files = Vec::new();
    for token in description.split_whitespace() {
        let cleaned = token
            .trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':' | '(' | ')' | '"' | '\''))
            .to_string();
        if cleaned.contains('/')
            || [".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".sql"]
                .iter()
                .any(|ext| cleaned.ends_with(ext))
        {
            files.push(cleaned);
        }
    }
    if lower.contains("frontend") {
        files.push("ui/frontend.rs".to_string());
    }
    if lower.contains("backend") || lower.contains("api") {
        files.push("api/server.rs".to_string());
    }
    if lower.contains("database") || lower.contains("schema") {
        files.push("db/schema.sql".to_string());
    }
    files.sort();
    files.dedup();
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    struct StaticBackend {
        payload: String,
    }

    impl LLMBackend for StaticBackend {
        fn generate<'a>(&'a self, _prompt: &'a str, _model: &'a str) -> BackendFuture<'a> {
            Box::pin(async move { Ok(self.payload.clone()) })
        }
    }

    #[tokio::test]
    async fn test_decompose_returns_tasks() {
        let decomposer = LLMDecomposer::new("test-model".to_string(), 10);
        let tasks = decomposer.decompose("build auth API", None).await.unwrap();

        assert!(!tasks.is_empty());
        assert_eq!(tasks[0].id, "task-0");
    }

    #[test]
    fn test_prompt_includes_feedback() {
        let decomposer = LLMDecomposer::new("test-model".to_string(), 10);
        let prompt = decomposer.build_prompt("ship feature", Some("cycle detected"));

        assert!(prompt.contains("cycle detected"));
        assert!(prompt.contains("Mission: ship feature"));
    }

    #[test]
    fn test_parse_response_rejects_duplicate_ids() {
        let decomposer = LLMDecomposer::new("test-model".to_string(), 10);
        let response = r#"{"tasks":[{"id":"task-1","description":"A","dependencies":[],"estimated_duration":10,"capabilities":["coding"]},{"id":"task-1","description":"B","dependencies":[],"estimated_duration":10,"capabilities":["coding"]}]}"#;

        assert!(decomposer.parse_response(response).is_err());
    }

    #[test]
    fn test_parse_response_rejects_zero_duration() {
        let decomposer = LLMDecomposer::new("test-model".to_string(), 10);
        let response = r#"{"tasks":[{"id":"task-1","description":"A","dependencies":[],"estimated_duration":0,"capabilities":["coding"]}]}"#;

        assert!(decomposer.parse_response(response).is_err());
    }

    #[tokio::test]
    async fn test_injected_backend_is_used() {
        let payload = r#"{"tasks":[{"id":"task-1","description":"Implement auth","dependencies":[],"estimated_duration":15,"capabilities":["coding"],"target_files":["api/server.rs"]}]}"#;
        let decomposer = LLMDecomposer::with_backend(
            "test-model".to_string(),
            10,
            Arc::new(StaticBackend {
                payload: payload.to_string(),
            }),
        );

        let tasks = decomposer.decompose("ignored", None).await.unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].target_files, vec!["api/server.rs"]);
    }

    #[test]
    fn test_parse_response_rejects_malformed_json() {
        let decomposer = LLMDecomposer::new("test-model".to_string(), 10);
        assert!(decomposer.parse_response("{").is_err());
    }
}
