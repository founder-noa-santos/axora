//! Mission decomposition system for converting missions into validated task DAGs.
//!
//! Phase 3 adds a hybrid pipeline:
//! - an LLM-shaped decomposer proposes raw tasks
//! - a graph builder validates and materializes a DAG
//! - a parallel group identifier derives execution waves and a critical path

mod graph_builder;
mod llm_decomposer;
mod parallel_groups;

use crate::error::AgentError;
use crate::graph::ExecutionMode;
use crate::task::{Priority, Task, TaskStatus, TaskType};
use crate::Result;
pub use graph_builder::GraphBuilder;
pub use llm_decomposer::{DeterministicLLMBackend, LLMBackend, LLMDecomposer, RawTask};
use openakta_indexing::InfluenceGraph;
pub use parallel_groups::ParallelGroupIdentifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Task identifier used by the runtime DAG.
pub type TaskId = usize;

/// Dependency type between tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DependencyType {
    /// Must complete before dependent task starts.
    Hard,
    /// Prefer to complete first, but can be relaxed in the future.
    Soft,
    /// Provides output data to another task.
    Data,
}

/// Dependency between two tasks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// Task that depends on another task.
    pub from: TaskId,
    /// Task that must complete first.
    pub to: TaskId,
    /// Dependency strength.
    pub dep_type: DependencyType,
}

impl Dependency {
    /// Creates a new dependency.
    pub fn new(from: TaskId, to: TaskId, dep_type: DependencyType) -> Self {
        Self { from, to, dep_type }
    }

    /// Creates a hard dependency.
    pub fn hard(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Hard)
    }

    /// Creates a soft dependency.
    pub fn soft(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Soft)
    }

    /// Creates a data dependency.
    pub fn data(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Data)
    }
}

/// Template for a single task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    /// Pattern to match in mission description.
    pub pattern: String,
    /// Description template for the task.
    pub description: String,
    /// Suggested role for this task.
    pub suggested_role: String,
    /// Task priority.
    pub priority: Priority,
    /// Relative complexity from 1 to 10.
    pub complexity: u8,
}

impl TaskTemplate {
    /// Creates a new task template.
    pub fn new(pattern: &str, description: &str, suggested_role: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            description: description.to_string(),
            suggested_role: suggested_role.to_string(),
            priority: Priority::Normal,
            complexity: 5,
        }
    }

    /// Sets task priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Sets task complexity.
    pub fn with_complexity(mut self, complexity: u8) -> Self {
        self.complexity = complexity.clamp(1, 10);
        self
    }
}

/// Template for decomposing a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionTemplate {
    /// Task templates to instantiate.
    pub task_templates: Vec<TaskTemplate>,
    /// Default dependencies between template indices.
    pub default_dependencies: Vec<(usize, usize)>,
}

/// Rule for selecting a template-based decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionRule {
    /// Pattern to match in mission text.
    pub pattern: String,
    /// Template used when the pattern matches.
    pub template: MissionTemplate,
    /// Additional keywords that trigger the rule.
    pub keywords: Vec<String>,
}

impl DecompositionRule {
    /// Creates a new decomposition rule.
    pub fn new(pattern: &str, template: MissionTemplate) -> Self {
        Self {
            pattern: pattern.to_string(),
            template,
            keywords: Vec::new(),
        }
    }

    /// Adds keywords to the rule.
    pub fn with_keywords(mut self, keywords: Vec<&str>) -> Self {
        self.keywords = keywords.into_iter().map(str::to_string).collect();
        self
    }
}

/// Directed acyclic task graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskDAG {
    /// Runtime task IDs.
    pub nodes: Vec<TaskId>,
    /// Edges in dependency order `(from, to)` meaning `from` depends on `to`.
    pub edges: Vec<(TaskId, TaskId)>,
}

/// Parallelizable execution group.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParallelGroup {
    /// Group identifier.
    pub group_id: usize,
    /// Tasks that can run together.
    pub task_ids: Vec<TaskId>,
    /// Whether tasks in the group are safe to run concurrently.
    pub can_run_in_parallel: bool,
    /// Whether predecessor groups satisfy all dependencies.
    pub dependencies_satisfied: bool,
}

/// Decomposer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposerConfig {
    /// Maximum number of tasks returned by decomposition.
    pub max_tasks: usize,
    /// Maximum number of tasks in one parallel group.
    pub max_parallelism: usize,
    /// Requested LLM model name.
    pub llm_model: String,
    /// Retries decomposition when graph validation fails.
    pub retry_on_invalid: bool,
    /// Maximum retry count.
    pub max_retries: usize,
}

impl Default for DecomposerConfig {
    fn default() -> Self {
        Self {
            max_tasks: 50,
            max_parallelism: 10,
            llm_model: "gpt-4".to_string(),
            retry_on_invalid: true,
            max_retries: 3,
        }
    }
}

/// Result of mission decomposition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedMission {
    /// Stable mission ID derived from mission text.
    pub mission_id: String,
    /// All executable tasks.
    pub tasks: Vec<Task>,
    /// Full task DAG.
    pub dependency_graph: TaskDAG,
    /// Compatibility edge list for existing executors/coordinators.
    pub dependencies: Vec<Dependency>,
    /// Detailed parallel groups.
    pub parallel_group_details: Vec<ParallelGroup>,
    /// Compatibility representation of parallel groups.
    pub parallel_groups: Vec<Vec<TaskId>>,
    /// Critical path through the DAG.
    pub critical_path: Vec<TaskId>,
    /// Estimated total duration.
    pub estimated_duration: Duration,
    /// Original mission text.
    pub original_mission: String,
    /// Sequential/parallel mode hint.
    pub execution_mode: ExecutionMode,
}

impl DecomposedMission {
    /// Creates an empty mission decomposition.
    pub fn new(original_mission: &str) -> Self {
        Self {
            mission_id: mission_identifier(original_mission),
            tasks: Vec::new(),
            dependency_graph: TaskDAG {
                nodes: Vec::new(),
                edges: Vec::new(),
            },
            dependencies: Vec::new(),
            parallel_group_details: Vec::new(),
            parallel_groups: Vec::new(),
            critical_path: Vec::new(),
            estimated_duration: Duration::ZERO,
            original_mission: original_mission.to_string(),
            execution_mode: ExecutionMode::Parallel,
        }
    }

    /// Returns the number of tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Returns a simple parallelism factor estimate.
    pub fn parallelism_factor(&self) -> f32 {
        self.parallel_groups.iter().map(Vec::len).max().unwrap_or(1) as f32
    }

    /// Returns true if the mission should run sequentially.
    pub fn is_sequential(&self) -> bool {
        matches!(self.execution_mode, ExecutionMode::Sequential)
    }

    /// Returns true if the mission can run in parallel.
    pub fn is_parallel(&self) -> bool {
        matches!(self.execution_mode, ExecutionMode::Parallel)
    }
}

/// Main hybrid mission decomposer.
pub struct MissionDecomposer {
    /// LLM-backed raw task generator.
    pub llm_decomposer: LLMDecomposer,
    /// Graph materializer and validator.
    pub graph_builder: GraphBuilder,
    /// Parallel group and critical path calculator.
    pub parallel_identifier: ParallelGroupIdentifier,
    /// Influence graph used for dependency inference.
    pub influence_graph: Arc<InfluenceGraph>,
    /// Runtime configuration.
    pub config: DecomposerConfig,
    rules: Vec<DecompositionRule>,
    cache: Mutex<HashMap<String, DecomposedMission>>,
}

impl MissionDecomposer {
    /// Creates a decomposer with default config and an empty influence graph.
    pub fn new() -> Self {
        Self::new_with_config(Arc::new(InfluenceGraph::new()), DecomposerConfig::default())
    }

    /// Creates a decomposer with explicit influence graph and config.
    pub fn new_with_config(influence_graph: Arc<InfluenceGraph>, config: DecomposerConfig) -> Self {
        let llm_decomposer = LLMDecomposer::new(config.llm_model.clone(), config.max_tasks);
        let graph_builder = GraphBuilder::new(Some(Arc::clone(&influence_graph)));
        let parallel_identifier = ParallelGroupIdentifier::new(config.max_parallelism);

        Self {
            llm_decomposer,
            graph_builder,
            parallel_identifier,
            influence_graph,
            config,
            rules: Vec::new(),
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Adds a template rule that can override the LLM path.
    pub fn add_rule(&mut self, rule: DecompositionRule) {
        self.rules.push(rule);
    }

    /// Synchronously decomposes a mission.
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            if handle.runtime_flavor() == tokio::runtime::RuntimeFlavor::MultiThread {
                return tokio::task::block_in_place(|| {
                    handle.block_on(self.decompose_async(mission))
                });
            }

            return std::thread::scope(|scope| {
                let join_handle = scope.spawn(|| {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(|err| AgentError::ExecutionFailed(err.to_string()))?;
                    runtime.block_on(self.decompose_async(mission))
                });
                join_handle.join().map_err(|_| {
                    AgentError::ExecutionFailed("decomposition thread panicked".to_string())
                })?
            });
        }

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| AgentError::ExecutionFailed(err.to_string()))?;
        runtime.block_on(self.decompose_async(mission))
    }

    /// Asynchronously decomposes a mission.
    pub async fn decompose_async(&self, mission: &str) -> Result<DecomposedMission> {
        if let Some(cached) = self
            .cache
            .lock()
            .map_err(|_| AgentError::ExecutionFailed("cache lock poisoned".to_string()))?
            .get(mission)
            .cloned()
        {
            return Ok(cached);
        }

        let mut attempt = 0usize;
        let mut feedback = None;

        loop {
            let raw_tasks = if let Some(template) = self.matched_template(mission) {
                self.template_to_raw_tasks(template)?
            } else {
                self.llm_decomposer
                    .decompose(mission, feedback.as_deref())
                    .await?
            };

            match self.materialize_mission(mission, raw_tasks) {
                Ok(decomposed) => {
                    self.cache
                        .lock()
                        .map_err(|_| {
                            AgentError::ExecutionFailed("cache lock poisoned".to_string())
                        })?
                        .insert(mission.to_string(), decomposed.clone());
                    return Ok(decomposed);
                }
                Err(err) if self.config.retry_on_invalid && attempt < self.config.max_retries => {
                    attempt += 1;
                    feedback = Some(err.to_string());
                }
                Err(err) => return Err(err),
            }
        }
    }

    /// Validates a decomposed mission.
    pub fn validate_decomposition(&self, mission: &DecomposedMission) -> Result<()> {
        self.graph_builder
            .validate_runtime_dag(&mission.dependency_graph, mission.tasks.len())?;

        let all_grouped: usize = mission.parallel_groups.iter().map(Vec::len).sum();
        if all_grouped != mission.tasks.len() {
            return Err(AgentError::GraphValidation(format!(
                "parallel groups cover {} tasks but mission has {}",
                all_grouped,
                mission.tasks.len()
            ))
            .into());
        }

        for group in &mission.parallel_group_details {
            if !group.dependencies_satisfied {
                return Err(AgentError::GraphValidation(format!(
                    "parallel group {} has unsatisfied dependencies",
                    group.group_id
                ))
                .into());
            }
        }

        Ok(())
    }

    fn matched_template(&self, mission: &str) -> Option<&MissionTemplate> {
        let mission_lower = mission.to_lowercase();

        self.rules.iter().find_map(|rule| {
            let pattern_match = mission_lower.contains(&rule.pattern.to_lowercase());
            let keyword_match = rule
                .keywords
                .iter()
                .any(|keyword| mission_lower.contains(&keyword.to_lowercase()));

            if pattern_match || keyword_match {
                Some(&rule.template)
            } else {
                None
            }
        })
    }

    fn template_to_raw_tasks(&self, template: &MissionTemplate) -> Result<Vec<RawTask>> {
        if template.task_templates.len() > self.config.max_tasks {
            return Err(AgentError::InvalidDecomposition(format!(
                "template produces {} tasks, above configured max {}",
                template.task_templates.len(),
                self.config.max_tasks
            ))
            .into());
        }

        let mut raw_tasks = Vec::with_capacity(template.task_templates.len());

        for (index, task_template) in template.task_templates.iter().enumerate() {
            let dependencies = template
                .default_dependencies
                .iter()
                .filter(|(from, _)| *from == index)
                .map(|(_, to)| format!("task-{}", to))
                .collect();

            raw_tasks.push(RawTask {
                id: format!("task-{}", index),
                description: task_template.description.clone(),
                dependencies,
                estimated_duration: 15 + (task_template.complexity as u64 * 5),
                capabilities: vec![task_template.suggested_role.clone()],
                target_files: Vec::new(),
            });
        }

        Ok(raw_tasks)
    }

    fn materialize_mission(
        &self,
        mission: &str,
        raw_tasks: Vec<RawTask>,
    ) -> Result<DecomposedMission> {
        let build = self.graph_builder.build_dag(&raw_tasks)?;
        let durations = build
            .id_map
            .iter()
            .map(|(raw_id, runtime_id)| {
                let raw = raw_tasks
                    .iter()
                    .find(|task| task.id == *raw_id)
                    .expect("raw task must exist");
                (
                    *runtime_id,
                    Duration::from_secs(raw.estimated_duration * 60),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut tasks = vec![Task::new(""); raw_tasks.len()];
        for raw_task in &raw_tasks {
            let runtime_id = *build.id_map.get(&raw_task.id).ok_or_else(|| {
                AgentError::GraphValidation(format!("missing runtime mapping for {}", raw_task.id))
            })?;

            let mut task = Task::new(&raw_task.description);
            task.id = raw_task.id.clone();
            task.description = raw_task.description.clone();
            task.priority = priority_from_capabilities(&raw_task.capabilities);
            task.status = TaskStatus::Pending;
            task.assigned_to = None;
            task.parent_task = None;
            task.task_type = infer_task_type(raw_task);
            tasks[runtime_id] = task;
        }

        let mut groups = self.parallel_identifier.identify_groups(&build.dag)?;
        groups = self
            .parallel_identifier
            .optimize_for_parallelism(groups, &build.dag);
        let critical_path = self
            .parallel_identifier
            .calculate_critical_path(&build.dag, &durations)?;

        let estimated_duration = critical_path
            .iter()
            .filter_map(|task_id| durations.get(task_id))
            .copied()
            .fold(Duration::ZERO, |acc, next| acc + next);

        let parallel_groups = groups
            .iter()
            .map(|group| group.task_ids.clone())
            .collect::<Vec<_>>();

        let mut mission_out = DecomposedMission {
            mission_id: mission_identifier(mission),
            tasks,
            dependency_graph: build.dag.clone(),
            dependencies: build.dependencies,
            parallel_group_details: groups,
            parallel_groups,
            critical_path,
            estimated_duration,
            original_mission: mission.to_string(),
            execution_mode: ExecutionMode::Parallel,
        };

        self.validate_decomposition(&mission_out)?;
        normalize_parallel_groups(&mut mission_out);
        Ok(mission_out)
    }
}

impl Default for MissionDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_parallel_groups(mission: &mut DecomposedMission) {
    if mission.parallel_groups.is_empty() && !mission.tasks.is_empty() {
        mission.parallel_groups = vec![(0..mission.tasks.len()).collect()];
        mission.parallel_group_details = vec![ParallelGroup {
            group_id: 0,
            task_ids: mission.parallel_groups[0].clone(),
            can_run_in_parallel: mission.tasks.len() > 1,
            dependencies_satisfied: true,
        }];
    }
}

fn mission_identifier(mission: &str) -> String {
    let mut hash = 1469598103934665603u64;
    for byte in mission.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("mission-{hash:016x}")
}

fn priority_from_capabilities(capabilities: &[String]) -> Priority {
    if capabilities
        .iter()
        .any(|cap| cap.eq_ignore_ascii_case("critical") || cap.eq_ignore_ascii_case("security"))
    {
        Priority::Critical
    } else if capabilities
        .iter()
        .any(|cap| cap.eq_ignore_ascii_case("testing") || cap.eq_ignore_ascii_case("review"))
    {
        Priority::High
    } else {
        Priority::Normal
    }
}

fn infer_task_type(raw_task: &RawTask) -> TaskType {
    let description = raw_task.description.to_lowercase();
    let capabilities = raw_task
        .capabilities
        .iter()
        .map(|capability| capability.to_lowercase())
        .collect::<Vec<_>>();
    let has_explicit_target = !raw_task.target_files.is_empty();
    let has_edit_intent = ["update", "edit", "fix", "refactor", "patch"]
        .iter()
        .any(|keyword| description.contains(keyword));
    let has_implementation_intent = description.contains("implement");

    if capabilities.iter().any(|value| value.contains("review")) || description.contains("review") {
        TaskType::Review
    } else if ["retrieve", "search", "index", "context"]
        .iter()
        .any(|keyword| description.contains(keyword))
    {
        TaskType::Retrieval
    } else if has_explicit_target
        && (has_edit_intent
            || has_implementation_intent
            || capabilities
                .iter()
                .any(|value| value.contains("developer") || value.contains("executor")))
    {
        TaskType::CodeModification
    } else {
        TaskType::General
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complex_mission() -> &'static str {
        "Implement authentication, database migrations, API endpoints, integration tests, documentation, observability, deployment validation, security review, frontend form, and rollout checklist"
    }

    #[tokio::test]
    async fn test_decompose_async_returns_cached_result() {
        let decomposer = MissionDecomposer::new();
        let first = decomposer.decompose_async("build auth API").await.unwrap();
        let second = decomposer.decompose_async("build auth API").await.unwrap();

        assert_eq!(first.mission_id, second.mission_id);
        assert_eq!(first.tasks.len(), second.tasks.len());
    }

    #[test]
    fn test_decompose_sync_outside_runtime() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("ship endpoint with tests").unwrap();

        assert!(!mission.tasks.is_empty());
        assert_eq!(
            mission.parallel_groups.iter().map(Vec::len).sum::<usize>(),
            mission.tasks.len()
        );
    }

    #[tokio::test]
    async fn test_rule_based_template_overrides_llm_path() {
        let mut decomposer = MissionDecomposer::new();
        let template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new("a", "Design schema", "design"),
                TaskTemplate::new("b", "Implement endpoint", "coding"),
                TaskTemplate::new("c", "Write tests", "testing"),
            ],
            default_dependencies: vec![(1, 0), (2, 1)],
        };
        decomposer.add_rule(DecompositionRule::new("schema-first", template));

        let mission = decomposer
            .decompose_async("schema-first migration")
            .await
            .unwrap();

        assert_eq!(mission.tasks.len(), 3);
        assert_eq!(mission.dependencies.len(), 2);
        assert_eq!(mission.critical_path, vec![0, 1, 2]);
    }

    #[tokio::test]
    async fn test_complex_mission_generates_many_tasks() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose_async(complex_mission()).await.unwrap();

        assert!(mission.tasks.len() >= 10);
        assert!(!mission.parallel_group_details.is_empty());
    }

    #[tokio::test]
    async fn test_parallel_groups_respect_dependencies() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose_async("implement backend, frontend, tests, docs")
            .await
            .unwrap();

        for dep in &mission.dependencies {
            let dep_group = mission
                .parallel_group_details
                .iter()
                .find(|group| group.task_ids.contains(&dep.to))
                .unwrap()
                .group_id;
            let task_group = mission
                .parallel_group_details
                .iter()
                .find(|group| group.task_ids.contains(&dep.from))
                .unwrap()
                .group_id;

            assert!(dep_group <= task_group);
        }
    }

    #[tokio::test]
    async fn test_validate_decomposition_accepts_valid_mission() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose_async("design implement test deploy")
            .await
            .unwrap();

        decomposer.validate_decomposition(&mission).unwrap();
    }

    #[test]
    fn test_decomposed_mission_helpers() {
        let mission = DecomposedMission::new("test mission");

        assert_eq!(mission.task_count(), 0);
        assert_eq!(mission.parallelism_factor(), 1.0);
        assert!(mission.is_parallel());
    }

    #[tokio::test]
    async fn test_critical_path_duration_is_non_zero() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose_async("design schema implement model write tests deploy")
            .await
            .unwrap();

        assert!(!mission.critical_path.is_empty());
        assert!(mission.estimated_duration > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_mission_id_is_stable() {
        let decomposer = MissionDecomposer::new();
        let a = decomposer.decompose_async("same mission").await.unwrap();
        let b = decomposer.decompose_async("same mission").await.unwrap();

        assert_eq!(a.mission_id, b.mission_id);
    }

    #[tokio::test]
    async fn test_human_like_decomposition_shape() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose_async("Create login API with persistence, tests, docs, and rollout")
            .await
            .unwrap();

        let descriptions = mission
            .tasks
            .iter()
            .map(|task| task.description.to_lowercase())
            .collect::<Vec<_>>();

        assert!(descriptions.iter().any(|desc| desc.contains("implement")));
        assert!(descriptions.iter().any(|desc| desc.contains("test")));
        assert!(descriptions.iter().any(|desc| desc.contains("document")));
    }
}
