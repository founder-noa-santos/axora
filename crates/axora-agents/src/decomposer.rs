//! Mission decomposition system for breaking down complex missions into concurrent tasks.
//!
//! This module provides automatic mission decomposition into independent tasks that can
//! be executed concurrently by multiple agents, achieving 3-5x speedup.
//!
//! ## Graph-Based Decomposition (Sprint 8 Pivot)
//!
//! R-10 research proved DDD decomposition creates bottlenecks. This module now uses:
//! - **Graph-based deterministic workflows** (LangGraph-style)
//! - **O(N) coordination** (not O(N²))
//! - **Sequential vs Parallel detection** via dependency analysis

use crate::error::AgentError;
use crate::graph::{ExecutionMode, ParallelismDetector, WorkflowGraph};
use crate::task::{Priority, Task, TaskStatus};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tracing::{debug, info, warn};

/// Task identifier type
pub type TaskId = usize;

/// Dependency type between tasks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DependencyType {
    /// Hard dependency - must wait (blocking)
    Hard,
    /// Soft dependency - should wait (can proceed with risk)
    Soft,
    /// Data dependency - needs output data from other task
    Data,
}

/// Dependency between two tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Task that depends on another
    pub from: TaskId,
    /// Task that must complete first
    pub to: TaskId,
    /// Type of dependency
    pub dep_type: DependencyType,
}

impl Dependency {
    /// Create a new dependency
    pub fn new(from: TaskId, to: TaskId, dep_type: DependencyType) -> Self {
        Self { from, to, dep_type }
    }

    /// Create a hard dependency
    pub fn hard(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Hard)
    }

    /// Create a soft dependency
    pub fn soft(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Soft)
    }

    /// Create a data dependency
    pub fn data(from: TaskId, to: TaskId) -> Self {
        Self::new(from, to, DependencyType::Data)
    }
}

/// Template for decomposing missions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionTemplate {
    /// Task templates to create
    pub task_templates: Vec<TaskTemplate>,
    /// Default dependencies between tasks (from_index, to_index)
    pub default_dependencies: Vec<(usize, usize)>,
}

/// Template for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    /// Pattern to match in mission description
    pub pattern: String,
    /// Description template for the task
    pub description: String,
    /// Suggested agent role for this task
    pub suggested_role: String,
    /// Priority for this task
    pub priority: Priority,
    /// Estimated complexity (1-10)
    pub complexity: u8,
}

impl TaskTemplate {
    /// Create a new task template
    pub fn new(pattern: &str, description: &str, suggested_role: &str) -> Self {
        Self {
            pattern: pattern.to_string(),
            description: description.to_string(),
            suggested_role: suggested_role.to_string(),
            priority: Priority::Normal,
            complexity: 5,
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set complexity
    pub fn with_complexity(mut self, complexity: u8) -> Self {
        self.complexity = complexity.min(10);
        self
    }
}

/// Rule for decomposing missions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionRule {
    /// Pattern to match in mission description
    pub pattern: String,
    /// Template to use when pattern matches
    pub template: MissionTemplate,
    /// Keywords that trigger this rule
    pub keywords: Vec<String>,
}

impl DecompositionRule {
    /// Create a new decomposition rule
    pub fn new(pattern: &str, template: MissionTemplate) -> Self {
        Self {
            pattern: pattern.to_string(),
            template,
            keywords: Vec::new(),
        }
    }

    /// Add keywords to the rule
    pub fn with_keywords(mut self, keywords: Vec<&str>) -> Self {
        self.keywords = keywords.iter().map(|s| s.to_string()).collect();
        self
    }
}

/// Result of mission decomposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecomposedMission {
    /// All tasks in the mission
    pub tasks: Vec<Task>,
    /// Dependencies between tasks
    pub dependencies: Vec<Dependency>,
    /// Critical path (indices of tasks on the critical path)
    pub critical_path: Vec<TaskId>,
    /// Groups of tasks that can run concurrently
    pub parallel_groups: Vec<Vec<TaskId>>,
    /// Original mission description
    pub original_mission: String,
    /// Execution mode (sequential or parallel)
    pub execution_mode: ExecutionMode,
    /// Workflow graph for deterministic execution
    #[serde(skip)]
    pub workflow_graph: Option<WorkflowGraph>,
}

impl DecomposedMission {
    /// Create a new decomposed mission
    pub fn new(original_mission: &str) -> Self {
        Self {
            tasks: Vec::new(),
            dependencies: Vec::new(),
            critical_path: Vec::new(),
            parallel_groups: Vec::new(),
            original_mission: original_mission.to_string(),
            execution_mode: ExecutionMode::Parallel,
            workflow_graph: None,
        }
    }

    /// Create from workflow graph
    pub fn from_workflow(original_mission: &str, graph: WorkflowGraph) -> Self {
        let tasks: Vec<Task> = graph
            .nodes
            .iter()
            .map(|n| {
                let mut task = Task::new(&format!("{}: {}", n.role, n.description));
                task.assigned_to = Some(format!("node_{}", n.id));
                task
            })
            .collect();

        let dependencies: Vec<Dependency> = graph
            .edges
            .iter()
            .map(|e| Dependency::hard(e.from, e.to))
            .collect();

        Self {
            tasks,
            dependencies,
            critical_path: Vec::new(),
            parallel_groups: Vec::new(),
            original_mission: original_mission.to_string(),
            execution_mode: ExecutionMode::Sequential, // Graph-based is sequential by default
            workflow_graph: Some(graph),
        }
    }

    /// Get task count
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get total parallelism potential
    pub fn parallelism_factor(&self) -> f32 {
        if self.tasks.is_empty() {
            return 1.0;
        }
        let max_group_size = self
            .parallel_groups
            .iter()
            .map(|g| g.len())
            .max()
            .unwrap_or(1);
        max_group_size as f32
    }

    /// Check if mission should execute sequentially
    pub fn is_sequential(&self) -> bool {
        matches!(self.execution_mode, ExecutionMode::Sequential)
    }

    /// Check if mission can execute in parallel
    pub fn is_parallel(&self) -> bool {
        matches!(self.execution_mode, ExecutionMode::Parallel)
    }
}

/// Mission decomposer - breaks down missions into concurrent tasks
///
/// ## Graph-Based Decomposition (Sprint 8)
///
/// Uses workflow templates and parallelism detection instead of DDD-based decomposition.
pub struct MissionDecomposer {
    /// Decomposition rules
    rules: Vec<DecompositionRule>,
    /// Default templates for common mission types
    default_templates: HashMap<String, MissionTemplate>,
    /// Graph-based workflow templates
    workflow_templates: HashMap<String, WorkflowGraph>,
    /// Parallelism detector for sequential vs parallel
    parallelism_detector: ParallelismDetector,
}

impl MissionDecomposer {
    /// Create a new mission decomposer with default rules
    pub fn new() -> Self {
        let mut decomposer = Self {
            rules: Vec::new(),
            default_templates: HashMap::new(),
            workflow_templates: HashMap::new(),
            parallelism_detector: ParallelismDetector::default(),
        };

        // Add default decomposition rules
        decomposer.add_default_rules();
        
        // Add workflow templates
        decomposer.add_workflow_templates();

        decomposer
    }

    /// Add workflow templates for graph-based decomposition
    fn add_workflow_templates(&mut self) {
        // Standard workflow template (Planner → Executor → Reviewer)
        self.workflow_templates.insert(
            "standard".to_string(),
            WorkflowGraph::standard(),
        );

        // Simple workflow (just Executor)
        let mut simple_graph = WorkflowGraph::new();
        simple_graph.add_node(crate::graph::Node {
            id: 0,
            role: crate::graph::NodeRole::Executor,
            description: "Execute simple task".to_string(),
            agent: None,
        });
        self.workflow_templates.insert("simple".to_string(), simple_graph);
    }

    /// Add default decomposition rules for common mission types
    fn add_default_rules(&mut self) {
        // Authentication system rule
        let auth_template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new(
                    "design|schema|model",
                    "Design database schema for authentication",
                    "Architect",
                )
                .with_priority(Priority::High)
                .with_complexity(7),
                TaskTemplate::new(
                    "research|best.practices|security",
                    "Research authentication best practices",
                    "Architect",
                )
                .with_complexity(5),
                TaskTemplate::new(
                    "setup|structure|boilerplate",
                    "Set up project structure",
                    "Developer",
                )
                .with_complexity(4),
                TaskTemplate::new(
                    "implement|user.model|entity",
                    "Implement user model/entity",
                    "Developer",
                )
                .with_priority(Priority::High)
                .with_complexity(6),
                TaskTemplate::new(
                    "jwt|token|utils",
                    "Implement JWT utilities",
                    "Developer",
                )
                .with_complexity(6),
                TaskTemplate::new(
                    "test|tests|testing",
                    "Write tests for authentication",
                    "QA Engineer",
                )
                .with_complexity(5),
                TaskTemplate::new(
                    "login|endpoint|api",
                    "Implement login endpoint",
                    "Developer",
                )
                .with_priority(Priority::High)
                .with_complexity(7),
                TaskTemplate::new(
                    "signup|register|endpoint",
                    "Implement signup endpoint",
                    "Developer",
                )
                .with_complexity(7),
            ],
            default_dependencies: vec![
                (0, 3), // schema -> user model
                (0, 6), // schema -> login endpoint
                (0, 7), // schema -> signup endpoint
                (3, 6), // user model -> login endpoint
                (3, 7), // user model -> signup endpoint
                (4, 6), // jwt utils -> login endpoint
                (4, 7), // jwt utils -> signup endpoint
                (5, 6), // tests should wait for login
                (5, 7), // tests should wait for signup
            ],
        };

        self.default_templates.insert("authentication".to_string(), auth_template);

        // API development rule
        let api_template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new(
                    "design|api|spec",
                    "Design API specification",
                    "Architect",
                )
                .with_priority(Priority::High)
                .with_complexity(6),
                TaskTemplate::new(
                    "setup|routes|router",
                    "Set up routing structure",
                    "Developer",
                )
                .with_complexity(4),
                TaskTemplate::new(
                    "implement|endpoint|handler",
                    "Implement endpoint handlers",
                    "Developer",
                )
                .with_priority(Priority::High)
                .with_complexity(7),
                TaskTemplate::new(
                    "validation|middleware",
                    "Implement validation middleware",
                    "Developer",
                )
                .with_complexity(5),
                TaskTemplate::new(
                    "test|api.test|integration",
                    "Write API integration tests",
                    "QA Engineer",
                )
                .with_complexity(6),
                TaskTemplate::new(
                    "document|docs|openapi",
                    "Generate API documentation",
                    "Developer",
                )
                .with_complexity(3),
            ],
            default_dependencies: vec![
                (0, 1), // spec -> routes
                (0, 2), // spec -> handlers
                (1, 2), // routes -> handlers
                (2, 4), // handlers -> tests
                (2, 5), // handlers -> docs
            ],
        };

        self.default_templates.insert("api".to_string(), api_template);

        // Testing rule
        let test_template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new(
                    "analyze|coverage|gaps",
                    "Analyze test coverage gaps",
                    "QA Engineer",
                )
                .with_priority(Priority::High)
                .with_complexity(5),
                TaskTemplate::new(
                    "unit.test|unit",
                    "Write unit tests",
                    "QA Engineer",
                )
                .with_complexity(6),
                TaskTemplate::new(
                    "integration.test|integration",
                    "Write integration tests",
                    "QA Engineer",
                )
                .with_complexity(7),
                TaskTemplate::new(
                    "e2e|end.to.end|e2e.test",
                    "Write end-to-end tests",
                    "QA Engineer",
                )
                .with_complexity(8),
                TaskTemplate::new(
                    "mock|stub|fixture",
                    "Create test mocks and fixtures",
                    "Developer",
                )
                .with_complexity(4),
            ],
            default_dependencies: vec![
                (0, 1), // analysis -> unit tests
                (0, 2), // analysis -> integration tests
                (4, 1), // mocks -> unit tests
                (4, 2), // mocks -> integration tests
            ],
        };

        self.default_templates.insert("testing".to_string(), test_template);
    }

    /// Add a custom decomposition rule
    pub fn add_rule(&mut self, rule: DecompositionRule) {
        debug!("Adding decomposition rule with pattern: {}", rule.pattern);
        self.rules.push(rule);
    }

    /// Decompose a mission into tasks using graph-based approach
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        info!("Decomposing mission: {}", mission);

        let mission_lower = mission.to_lowercase();

        // Step 1: Detect task type (sequential vs parallelizable)
        let mut decomposed = DecomposedMission::new(mission);

        // Step 2: Try workflow templates first (graph-based)
        for (key, workflow) in &self.workflow_templates {
            if mission_lower.contains(key) {
                debug!("Matched workflow template: {}", key);
                decomposed = DecomposedMission::from_workflow(mission, workflow.clone());
                break;
            }
        }

        // Step 3: If no workflow matched, try domain templates
        if decomposed.workflow_graph.is_none() {
            for (key, template) in &self.default_templates {
                if mission_lower.contains(key) {
                    debug!("Matched domain template: {}", key);
                    decomposed = self.apply_template(template, mission)?;
                    break;
                }
            }
        }

        // Step 4: Try custom rules
        if decomposed.tasks.is_empty() {
            for rule in &self.rules {
                if self.matches_rule(mission, rule) {
                    debug!("Matched custom rule: {}", rule.pattern);
                    decomposed = self.apply_template(&rule.template, mission)?;
                    break;
                }
            }
        }

        // Step 5: Fallback to simple decomposition
        if decomposed.tasks.is_empty() {
            decomposed = self.simple_decompose(mission)?;
        }

        // Step 6: Detect execution mode (sequential vs parallel)
        decomposed.execution_mode = self.parallelism_detector.detect(mission, &decomposed.dependencies);

        // Step 7: Calculate critical path and parallel groups (for non-graph missions)
        if decomposed.workflow_graph.is_none() {
            decomposed.critical_path = self.calculate_critical_path(&decomposed.tasks, &decomposed.dependencies);
            decomposed.parallel_groups = self.identify_parallel_groups(&decomposed.tasks, &decomposed.dependencies);
        }

        info!(
            "Decomposed mission into {} tasks (mode: {:?})",
            decomposed.tasks.len(),
            decomposed.execution_mode
        );

        Ok(decomposed)
    }

    /// Check if mission matches a rule
    fn matches_rule(&self, mission: &str, rule: &DecompositionRule) -> bool {
        let mission_lower = mission.to_lowercase();

        // Check keywords
        for keyword in &rule.keywords {
            if mission_lower.contains(&keyword.to_lowercase()) {
                return true;
            }
        }

        // Check pattern (simple substring match for now, could be regex in future)
        if mission_lower.contains(&rule.pattern.to_lowercase()) {
            return true;
        }

        false
    }

    /// Apply a template to create tasks
    fn apply_template(&self, template: &MissionTemplate, mission: &str) -> Result<DecomposedMission> {
        let mut decomposed = DecomposedMission::new(mission);

        // Create tasks from templates
        for (idx, task_template) in template.task_templates.iter().enumerate() {
            let mut task = Task::new(&task_template.description);
            task.priority = task_template.priority.clone();

            // Add mission context to task
            task.description = format!("[{}] {}", mission, task_template.description);

            decomposed.tasks.push(task);

            debug!("Created task {}: {}", idx, task_template.description);
        }

        // Add dependencies
        for &(from, to) in &template.default_dependencies {
            if from < decomposed.tasks.len() && to < decomposed.tasks.len() {
                decomposed.dependencies.push(Dependency::hard(from, to));
            }
        }

        Ok(decomposed)
    }

    /// Simple decomposition for missions without matching templates
    fn simple_decompose(&self, mission: &str) -> Result<DecomposedMission> {
        let mut decomposed = DecomposedMission::new(mission);

        // Create generic tasks based on mission complexity
        let task_count = self.estimate_task_count(mission);

        for i in 0..task_count {
            let task = Task::new(&format!("Task {} of: {}", i + 1, mission));
            decomposed.tasks.push(task);
        }

        // Add sequential dependencies
        for i in 1..decomposed.tasks.len() {
            decomposed.dependencies.push(Dependency::hard(i, i - 1));
        }

        Ok(decomposed)
    }

    /// Estimate number of tasks based on mission complexity
    fn estimate_task_count(&self, mission: &str) -> usize {
        let word_count = mission.split_whitespace().count();

        // Simple heuristic: 1 task per 5 words, min 1, max 10
        (word_count / 5).clamp(1, 10)
    }

    /// Calculate critical path using topological sort
    fn calculate_critical_path(&self, tasks: &[Task], dependencies: &[Dependency]) -> Vec<TaskId> {
        if tasks.is_empty() {
            return Vec::new();
        }

        // Build adjacency list and in-degree count
        let mut adj: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();

        for i in 0..tasks.len() {
            adj.insert(i, Vec::new());
            in_degree.insert(i, 0);
        }

        for dep in dependencies {
            if let Some(neighbors) = adj.get_mut(&dep.to) {
                neighbors.push(dep.from);
            }
            if let Some(count) = in_degree.get_mut(&dep.from) {
                *count += 1;
            }
        }

        // Find tasks with no dependencies (starting points)
        let mut queue: Vec<TaskId> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut critical_path = Vec::new();
        let mut max_dist: HashMap<TaskId, usize> = HashMap::new();

        // Initialize distances
        for i in 0..tasks.len() {
            max_dist.insert(i, 0);
        }

        // Process in topological order
        while let Some(node) = queue.pop() {
            critical_path.push(node);

            if let Some(neighbors) = adj.get(&node) {
                for &neighbor in neighbors {
                    let new_dist = max_dist[&node] + 1;
                    if new_dist > max_dist[&neighbor] {
                        max_dist.insert(neighbor, new_dist);
                    }

                    if let Some(count) = in_degree.get_mut(&neighbor) {
                        *count -= 1;
                        if *count == 0 {
                            queue.push(neighbor);
                        }
                    }
                }
            }
        }

        // Find the longest path (critical path)
        let max_node = max_dist
            .iter()
            .max_by_key(|(_, &dist)| dist)
            .map(|(&id, _)| id)
            .unwrap_or(0);

        // Backtrack to find critical path
        self.backtrack_critical_path(max_node, &max_dist, dependencies)
    }

    /// Backtrack to find the actual critical path
    fn backtrack_critical_path(
        &self,
        end: TaskId,
        distances: &HashMap<TaskId, usize>,
        dependencies: &[Dependency],
    ) -> Vec<TaskId> {
        let mut path = vec![end];
        let mut current = end;

        loop {
            let current_dist = distances[&current];

            if current_dist == 0 {
                break;
            }

            // Find predecessor with distance = current_dist - 1
            let predecessor = dependencies
                .iter()
                .filter(|d| d.from == current && d.dep_type == DependencyType::Hard)
                .find_map(|d| {
                    if distances.get(&d.to).copied() == Some(current_dist - 1) {
                        Some(d.to)
                    } else {
                        None
                    }
                });

            match predecessor {
                Some(pred) => {
                    path.push(pred);
                    current = pred;
                }
                None => break,
            }
        }

        path.reverse();
        path
    }

    /// Identify groups of tasks that can run in parallel
    fn identify_parallel_groups(&self, tasks: &[Task], dependencies: &[Dependency]) -> Vec<Vec<TaskId>> {
        if tasks.is_empty() {
            return Vec::new();
        }

        // Build dependency graph
        let mut depends_on: HashMap<TaskId, HashSet<TaskId>> = HashMap::new();

        for i in 0..tasks.len() {
            depends_on.insert(i, HashSet::new());
        }

        for dep in dependencies {
            if dep.dep_type == DependencyType::Hard {
                if let Some(deps) = depends_on.get_mut(&dep.from) {
                    deps.insert(dep.to);
                }
            }
        }

        // Group tasks by their "level" (distance from root)
        let mut levels: HashMap<usize, Vec<TaskId>> = HashMap::new();
        let mut task_levels: HashMap<TaskId, usize> = HashMap::new();

        // Calculate level for each task
        for i in 0..tasks.len() {
            let level = self.calculate_task_level(i, &depends_on, &task_levels);
            task_levels.insert(i, level);
            levels.entry(level).or_default().push(i);
        }

        // Convert to sorted groups
        let max_level = levels.keys().max().copied().unwrap_or(0);
        let mut groups = Vec::new();

        for level in 0..=max_level {
            if let Some(group) = levels.get(&level) {
                if !group.is_empty() {
                    groups.push(group.clone());
                }
            }
        }

        groups
    }

    /// Calculate the level of a task (distance from root in dependency graph)
    fn calculate_task_level(
        &self,
        task_id: TaskId,
        depends_on: &HashMap<TaskId, HashSet<TaskId>>,
        task_levels: &HashMap<TaskId, usize>,
    ) -> usize {
        if let Some(&level) = task_levels.get(&task_id) {
            return level;
        }

        let deps = depends_on.get(&task_id).cloned().unwrap_or_default();

        if deps.is_empty() {
            return 0;
        }

        let max_dep_level = deps
            .iter()
            .filter_map(|&dep_id| task_levels.get(&dep_id).copied())
            .max()
            .unwrap_or(0);

        max_dep_level + 1
    }

    /// Get the number of registered rules
    pub fn rule_count(&self) -> usize {
        self.rules.len() + self.default_templates.len()
    }
}

impl Default for MissionDecomposer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_simple_mission() {
        let decomposer = MissionDecomposer::new();

        // Simple mission that matches authentication template
        let mission = "Implement authentication system with login and signup";
        let result = decomposer.decompose(mission).unwrap();

        assert!(!result.tasks.is_empty());
        assert!(result.tasks.len() >= 3);
        assert_eq!(result.original_mission, mission);
    }

    #[test]
    fn test_decompose_complex_mission() {
        let decomposer = MissionDecomposer::new();

        // Complex mission
        let mission = "Build a complete REST API with authentication, user management, and data validation";
        let result = decomposer.decompose(mission).unwrap();

        assert!(!result.tasks.is_empty());
        assert!(!result.dependencies.is_empty());
        assert!(!result.parallel_groups.is_empty());
    }

    #[test]
    fn test_identify_dependencies() {
        let decomposer = MissionDecomposer::new();

        let mission = "Create authentication with JWT tokens and user model";
        let result = decomposer.decompose(mission).unwrap();

        // Should have identified dependencies
        assert!(!result.dependencies.is_empty());

        // All dependencies should have valid task indices
        for dep in &result.dependencies {
            assert!(dep.from < result.tasks.len());
            assert!(dep.to < result.tasks.len());
        }
    }

    #[test]
    fn test_parallel_groups() {
        let decomposer = MissionDecomposer::new();

        let mission = "Implement authentication system with login, signup, and JWT";
        let result = decomposer.decompose(mission).unwrap();

        // Should have identified parallel groups
        assert!(!result.parallel_groups.is_empty());

        // All tasks should be in some group
        let all_grouped: HashSet<TaskId> = result.parallel_groups.iter().flatten().copied().collect();
        assert_eq!(all_grouped.len(), result.tasks.len());
    }

    #[test]
    fn test_critical_path() {
        let decomposer = MissionDecomposer::new();

        let mission = "Build API with authentication";
        let result = decomposer.decompose(mission).unwrap();

        // Should have calculated critical path
        assert!(!result.critical_path.is_empty());

        // Critical path should be valid task indices
        for &task_id in &result.critical_path {
            assert!(task_id < result.tasks.len());
        }
    }

    #[test]
    fn test_dependency_hard_vs_soft() {
        // Create dependencies of different types
        let hard = Dependency::hard(0, 1);
        let soft = Dependency::soft(0, 1);
        let data = Dependency::data(0, 1);

        assert_eq!(hard.dep_type, DependencyType::Hard);
        assert_eq!(soft.dep_type, DependencyType::Soft);
        assert_eq!(data.dep_type, DependencyType::Data);
    }

    #[test]
    fn test_task_assignment_to_agents() {
        let decomposer = MissionDecomposer::new();

        let mission = "Implement authentication with login endpoint";
        let result = decomposer.decompose(mission).unwrap();

        // Tasks should be assignable (have valid structure)
        for task in &result.tasks {
            assert!(!task.id.is_empty());
            assert!(!task.description.is_empty());
            assert_eq!(task.status, TaskStatus::Pending);
            assert!(task.assigned_to.is_none());
        }
    }

    #[test]
    fn test_mission_with_cross_domain_tasks() {
        let decomposer = MissionDecomposer::new();

        // Mission spanning multiple domains
        let mission = "Build full-stack feature with database schema, API endpoints, and frontend components";
        let result = decomposer.decompose(mission).unwrap();

        // Should decompose into multiple tasks
        assert!(result.tasks.len() >= 1);

        // Should have some structure
        assert!(!result.parallel_groups.is_empty() || !result.dependencies.is_empty());
    }

    #[test]
    fn test_full_workflow() {
        let decomposer = MissionDecomposer::new();

        // Full workflow: decompose mission
        let mission = "Create authentication system with JWT, login, and signup endpoints";
        let decomposed = decomposer.decompose(mission).unwrap();

        // Verify decomposition
        assert!(!decomposed.tasks.is_empty());
        assert!(!decomposed.dependencies.is_empty());
        assert!(!decomposed.critical_path.is_empty());
        assert!(!decomposed.parallel_groups.is_empty());

        // Verify parallelism factor
        let parallelism = decomposed.parallelism_factor();
        assert!(parallelism >= 1.0);

        // Verify task count matches grouped tasks
        let total_grouped: usize = decomposed.parallel_groups.iter().map(|g| g.len()).sum();
        assert_eq!(total_grouped, decomposed.tasks.len());
    }

    #[test]
    fn test_custom_decomposition_rule() {
        let mut decomposer = MissionDecomposer::new();

        // Add custom rule
        let custom_template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new("custom1", "Custom task 1", "Developer"),
                TaskTemplate::new("custom2", "Custom task 2", "Developer"),
                TaskTemplate::new("custom3", "Custom task 3", "QA Engineer"),
            ],
            default_dependencies: vec![(0, 1), (1, 2)],
        };

        let rule = DecompositionRule::new("custom-mission", custom_template)
            .with_keywords(vec!["custom", "special"]);

        decomposer.add_rule(rule);

        // Test custom rule matching
        let result = decomposer.decompose("This is a custom-mission task").unwrap();

        // Should use custom template
        assert_eq!(result.tasks.len(), 3);
        assert_eq!(result.dependencies.len(), 2);
    }

    #[test]
    fn test_dependency_creation_helpers() {
        let hard = Dependency::hard(1, 2);
        let soft = Dependency::soft(3, 4);
        let data = Dependency::data(5, 6);

        assert_eq!(hard.from, 1);
        assert_eq!(hard.to, 2);
        assert_eq!(hard.dep_type, DependencyType::Hard);

        assert_eq!(soft.from, 3);
        assert_eq!(soft.to, 4);
        assert_eq!(soft.dep_type, DependencyType::Soft);

        assert_eq!(data.from, 5);
        assert_eq!(data.to, 6);
        assert_eq!(data.dep_type, DependencyType::Data);
    }

    #[test]
    fn test_mission_template_creation() {
        let template = MissionTemplate {
            task_templates: vec![
                TaskTemplate::new("pattern1", "desc1", "role1"),
                TaskTemplate::new("pattern2", "desc2", "role2"),
            ],
            default_dependencies: vec![(0, 1)],
        };

        assert_eq!(template.task_templates.len(), 2);
        assert_eq!(template.default_dependencies.len(), 1);
    }

    #[test]
    fn test_task_template_builder() {
        let template = TaskTemplate::new("pattern", "description", "role")
            .with_priority(Priority::High)
            .with_complexity(8);

        assert_eq!(template.priority, Priority::High);
        assert_eq!(template.complexity, 8);
    }

    #[test]
    fn test_decomposed_mission_methods() {
        let mut mission = DecomposedMission::new("Test mission");

        assert_eq!(mission.task_count(), 0);
        assert_eq!(mission.parallelism_factor(), 1.0);

        // Add some tasks and groups
        mission.tasks.push(Task::new("Task 1"));
        mission.tasks.push(Task::new("Task 2"));
        mission.parallel_groups.push(vec![0, 1]);

        assert_eq!(mission.task_count(), 2);
        assert_eq!(mission.parallelism_factor(), 2.0);
    }

    #[test]
    fn test_rule_count() {
        let decomposer = MissionDecomposer::new();

        // Should have default rules
        assert!(decomposer.rule_count() > 0);
    }

    #[test]
    fn test_simple_decomposition_fallback() {
        let decomposer = MissionDecomposer::new();

        // Mission that doesn't match any template
        let mission = "Do something completely random and unusual";
        let result = decomposer.decompose(mission).unwrap();

        // Should still create some tasks
        assert!(!result.tasks.is_empty());
        assert_eq!(result.original_mission, mission);
    }
}
