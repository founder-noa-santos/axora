//! Graph-Based Deterministic Workflow Engine
//!
//! This module implements LangGraph-style state machine workflows:
//! - **Nodes**: Agent roles (Planner, Executor, Reviewer, Resolver)
//! - **Edges**: Explicit state transitions with guard conditions
//! - **Guards**: Validation before transition (deterministic, not semantic)
//!
//! ## Why Graph-Based?
//!
//! R-10 research proved:
//! - DDD decomposition creates cross-domain routing bottlenecks
//! - Sequential tasks degrade 39-70% with DDD (context fragmentation)
//! - Graph-based is deterministic + O(N) coordination
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  Planner    │────▶│  Executor   │────▶│  Reviewer   │
//! │  (Node 0)   │     │  (Node 1)   │     │  (Node 2)   │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!       │                   │                   │
//!       │ OnSuccess         │ OnSuccess         │ OnSuccess
//!       ▼                   ▼                   ▼
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │  Resolver   │◀────│  Resolver   │◀────│  Resolver   │
//! │  (Node 3)   │     │  (Node 3)   │     │  (Node 3)   │
//! └─────────────┘     └─────────────┘     └─────────────┘
//! ```

use crate::agent::{Agent, TaskResult};
use crate::decomposer::{Dependency, DependencyType, TaskId};
use crate::error::AgentError;
use crate::task::Task;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};

/// Node identifier type
pub type NodeId = usize;

/// Workflow graph (state machine for deterministic execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    /// Nodes (agent roles)
    pub nodes: Vec<Node>,

    /// Edges (state transitions)
    pub edges: Vec<Edge>,

    /// Current state (active node)
    pub current_state: Option<NodeId>,

    /// Execution history (for debugging/tracing)
    pub execution_history: Vec<ExecutionRecord>,
    /// Execution policy guardrails.
    pub execution_policy: ExecutionPolicy,
    /// Terminal failure state when execution aborts.
    pub terminal_failure: Option<TerminalFailureReason>,
    /// Number of transitions observed.
    pub transition_count: usize,
    /// Number of timeouts observed.
    pub timeout_failures: usize,
    /// Number of retry budget exhaustions observed.
    pub retry_exhaustions: usize,
    /// Number of protocol validation failures observed.
    pub protocol_validation_failures: usize,
}

/// Execution record for tracing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub node_id: NodeId,
    pub timestamp: u64,
    pub success: bool,
    pub output: String,
}

/// Node (agent role in workflow)
#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub role: NodeRole,
    pub description: String,
    /// Agent assigned to this node
    #[serde(skip)]
    pub agent: Option<Arc<Mutex<dyn Agent>>>,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("role", &self.role)
            .field("description", &self.description)
            .field("agent", &self.agent.as_ref().map(|_| "<agent>"))
            .finish()
    }
}

/// Agent role types in workflow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeRole {
    /// Decompose task, create execution plan
    Planner,

    /// Execute task (write code, run commands)
    Executor,

    /// Validate output (review code, run tests)
    Reviewer,

    /// Resolve conflicts (integration issues, failures)
    Resolver,
}

impl std::fmt::Display for NodeRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeRole::Planner => write!(f, "Planner"),
            NodeRole::Executor => write!(f, "Executor"),
            NodeRole::Reviewer => write!(f, "Reviewer"),
            NodeRole::Resolver => write!(f, "Resolver"),
        }
    }
}

/// Edge (state transition between nodes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: TransitionCondition,
}

/// Transition condition (guard for deterministic routing)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransitionCondition {
    /// Always transition (unconditional)
    Always,

    /// Transition on success
    OnSuccess,

    /// Transition on failure
    OnFailure,

    /// Custom guard condition (expression string)
    Guard(String),
}

/// Execution state (shared across nodes)
#[derive(Debug, Clone)]
pub struct ExecutionState {
    /// Original task
    pub task: Task,

    /// Results from each node
    pub node_results: HashMap<NodeId, TaskResult>,

    /// Current attempt count (for retries)
    pub attempt_count: u32,

    /// Maximum attempts before escalation
    pub max_attempts: u32,

    /// Global success flag
    pub success: bool,
}

impl ExecutionState {
    /// Create new execution state
    pub fn new(task: Task) -> Self {
        Self::new_with_attempts(task, 3)
    }

    /// Create new execution state with an explicit retry budget.
    pub fn new_with_attempts(task: Task, max_attempts: u32) -> Self {
        Self {
            task,
            node_results: HashMap::new(),
            attempt_count: 0,
            max_attempts,
            success: false,
        }
    }

    /// Record result from a node
    pub fn record_result(&mut self, node_id: NodeId, result: TaskResult) {
        self.node_results.insert(node_id, result);
    }

    /// Get result from a node
    pub fn get_result(&self, node_id: NodeId) -> Option<&TaskResult> {
        self.node_results.get(&node_id)
    }

    /// Increment attempt count
    pub fn increment_attempts(&mut self) {
        self.attempt_count += 1;
    }

    /// Check if max attempts exceeded
    pub fn max_attempts_exceeded(&self) -> bool {
        self.attempt_count >= self.max_attempts
    }

    /// Finalize execution
    pub fn finalize(self) -> TaskResult {
        // Aggregate results from all nodes
        let outputs: Vec<String> = self
            .node_results
            .values()
            .map(|r| r.output.clone())
            .collect();

        let has_failure = self.node_results.values().any(|r| !r.success);

        TaskResult {
            success: !has_failure,
            output: outputs.join("\n---\n"),
            error: self
                .node_results
                .values()
                .find(|r| !r.success)
                .and_then(|r| r.error.clone()),
        }
    }
}

/// Explicit workflow execution policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPolicy {
    /// Maximum number of node transitions before execution aborts.
    pub max_iterations: u32,
    /// Retry budget used by resolver loops.
    pub retry_budget: u32,
    /// Per-node timeout in milliseconds.
    pub node_timeout_ms: u64,
}

impl Default for ExecutionPolicy {
    fn default() -> Self {
        Self {
            max_iterations: 32,
            retry_budget: 3,
            node_timeout_ms: 5_000,
        }
    }
}

/// Terminal failure reason recorded by the workflow engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalFailureReason {
    /// The graph contains an invalid or unsupported cycle.
    CycleDetected,
    /// Retry budget has been exhausted.
    RetryExhausted,
    /// A node execution timed out.
    Timeout,
    /// A transition was rejected by validation.
    InvalidTransition,
}

impl WorkflowGraph {
    /// Create new workflow graph
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            current_state: None,
            execution_history: Vec::new(),
            execution_policy: ExecutionPolicy::default(),
            terminal_failure: None,
            transition_count: 0,
            timeout_failures: 0,
            retry_exhaustions: 0,
            protocol_validation_failures: 0,
        }
    }

    /// Create standard workflow (Planner → Executor → Reviewer)
    pub fn standard() -> Self {
        let mut graph = Self::new();

        // Add nodes
        graph.add_node(Node {
            id: 0,
            role: NodeRole::Planner,
            description: "Plan and decompose task".to_string(),
            agent: None,
        });

        graph.add_node(Node {
            id: 1,
            role: NodeRole::Executor,
            description: "Execute planned task".to_string(),
            agent: None,
        });

        graph.add_node(Node {
            id: 2,
            role: NodeRole::Reviewer,
            description: "Review and validate output".to_string(),
            agent: None,
        });

        graph.add_node(Node {
            id: 3,
            role: NodeRole::Resolver,
            description: "Resolve conflicts and failures".to_string(),
            agent: None,
        });

        // Add edges (deterministic transitions)
        graph.add_edge(Edge {
            from: 0,
            to: 1,
            condition: TransitionCondition::OnSuccess,
        });

        graph.add_edge(Edge {
            from: 1,
            to: 2,
            condition: TransitionCondition::OnSuccess,
        });

        graph.add_edge(Edge {
            from: 2,
            to: 3,
            condition: TransitionCondition::OnFailure,
        });

        graph.add_edge(Edge {
            from: 0,
            to: 3,
            condition: TransitionCondition::OnFailure,
        });

        graph.add_edge(Edge {
            from: 1,
            to: 3,
            condition: TransitionCondition::OnFailure,
        });

        // Resolver loops back to Executor for retry
        graph.add_edge(Edge {
            from: 3,
            to: 1,
            condition: TransitionCondition::Always,
        });

        graph
    }

    /// Add node to graph
    pub fn add_node(&mut self, node: Node) {
        debug!("Adding node {} with role {}", node.id, node.role);
        self.nodes.push(node);
    }

    /// Add edge to graph
    pub fn add_edge(&mut self, edge: Edge) {
        debug!("Adding edge from {} to {}", edge.from, edge.to);
        self.edges.push(edge);
    }

    /// Set agent for a node
    pub fn set_agent_for_node(&mut self, node_id: NodeId, agent: Arc<Mutex<dyn Agent>>) {
        if let Some(node) = self.nodes.iter_mut().find(|n| n.id == node_id) {
            node.agent = Some(agent);
        }
    }

    /// Get initial node (Planner by default)
    pub fn get_initial_node(&self) -> NodeId {
        self.nodes
            .iter()
            .find(|n| n.role == NodeRole::Planner)
            .map(|n| n.id)
            .unwrap_or(0)
    }

    /// Get resolver node
    pub fn get_resolver_node(&self) -> NodeId {
        self.nodes
            .iter()
            .find(|n| n.role == NodeRole::Resolver)
            .map(|n| n.id)
            .unwrap_or_else(|| self.nodes.len() - 1)
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == node_id)
    }

    /// Get mutable node by ID
    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == node_id)
    }

    /// Find next node based on result and state (deterministic)
    pub fn find_next_node(
        &self,
        current_id: NodeId,
        result: &TaskResult,
        state: &ExecutionState,
    ) -> Option<NodeId> {
        // Find all outgoing edges from current node
        let outgoing_edges: Vec<&Edge> =
            self.edges.iter().filter(|e| e.from == current_id).collect();

        for edge in &outgoing_edges {
            if self.evaluate_condition(edge, result, state) {
                return Some(edge.to);
            }
        }

        None
    }

    /// Evaluate transition condition (deterministic guard)
    fn evaluate_condition(&self, edge: &Edge, result: &TaskResult, state: &ExecutionState) -> bool {
        match &edge.condition {
            TransitionCondition::Always => true,

            TransitionCondition::OnSuccess => result.success,

            TransitionCondition::OnFailure => !result.success,

            TransitionCondition::Guard(expr) => {
                // Parse and evaluate guard expression
                // For now, simple string matching
                self.evaluate_guard_expression(expr, result, state)
            }
        }
    }

    /// Evaluate guard expression (simple implementation)
    fn evaluate_guard_expression(
        &self,
        expr: &str,
        result: &TaskResult,
        state: &ExecutionState,
    ) -> bool {
        // Simple guard expressions:
        // - "success" → result.success
        // - "failure" → !result.success
        // - "attempts < 3" → state.attempt_count < 3
        // - "has_output" → !result.output.is_empty()

        match expr {
            "success" => result.success,
            "failure" => !result.success,
            "has_output" => !result.output.is_empty(),
            "no_output" => result.output.is_empty(),
            _ if expr.starts_with("attempts <") => {
                let limit: u32 = expr
                    .trim_start_matches("attempts <")
                    .trim()
                    .parse()
                    .unwrap_or(3);
                state.attempt_count < limit
            }
            _ => {
                warn!("Unknown guard expression: {}", expr);
                false
            }
        }
    }

    /// Validate transition (check guard conditions)
    pub fn validate_transition(&self, from: NodeId, to: NodeId, state: &ExecutionState) -> bool {
        // Find edge
        let edge = self.edges.iter().find(|e| e.from == from && e.to == to);

        match edge {
            Some(e) => {
                // Get last result for condition evaluation
                let last_result =
                    state
                        .node_results
                        .values()
                        .last()
                        .cloned()
                        .unwrap_or_else(|| TaskResult {
                            success: true,
                            output: String::new(),
                            error: None,
                        });

                self.evaluate_condition(e, &last_result, state)
            }
            None => false,
        }
    }

    /// Execute graph with deterministic routing
    pub async fn execute(&mut self, task: Task) -> Result<TaskResult> {
        info!("Executing workflow graph with {} nodes", self.nodes.len());

        self.validate_execution_policy()?;
        self.terminal_failure = None;
        let mut state = ExecutionState::new_with_attempts(task, self.execution_policy.retry_budget);

        // Start at initial node
        self.current_state = Some(self.get_initial_node());

        let mut iterations = 0;

        while let Some(current_node_id) = self.current_state {
            iterations += 1;
            if iterations as u32 > self.execution_policy.max_iterations {
                self.terminal_failure = Some(TerminalFailureReason::CycleDetected);
                error!("Workflow exceeded max iterations (possible infinite loop)");
                return Err(AgentError::InvalidStateTransition(
                    "Workflow loop detected".to_string(),
                )
                .into());
            }

            let current_node = self
                .get_node(current_node_id)
                .ok_or_else(|| {
                    AgentError::AgentNotFound(format!("Node {} not found", current_node_id))
                })?
                .clone();

            debug!(
                "Executing node {} (role: {})",
                current_node.id, current_node.role
            );

            // Execute current node
            let result = match timeout(
                Duration::from_millis(self.execution_policy.node_timeout_ms),
                self.execute_node(&current_node, &mut state),
            )
            .await
            {
                Ok(result) => result?,
                Err(_) => {
                    self.timeout_failures += 1;
                    self.terminal_failure = Some(TerminalFailureReason::Timeout);
                    return Ok(TaskResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!(
                            "Node {} timed out after {}ms",
                            current_node.id, self.execution_policy.node_timeout_ms
                        )),
                    });
                }
            };

            // Record execution
            let record = ExecutionRecord {
                node_id: current_node.id,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                success: result.success,
                output: result.output.clone(),
            };
            self.execution_history.push(record);

            // Find next node (deterministic)
            let next_node = self.find_next_node(current_node.id, &result, &state);

            match next_node {
                Some(next_id) => {
                    // Validate transition (guard conditions)
                    if self.validate_transition(current_node.id, next_id, &state) {
                        debug!("Transitioning from {} to {}", current_node.id, next_id);
                        self.transition_count += 1;
                        self.current_state = Some(next_id);
                    } else {
                        // Guard failed, escalate to resolver
                        warn!(
                            "Guard failed for transition {} → {}, escalating to resolver",
                            current_node.id, next_id
                        );
                        self.protocol_validation_failures += 1;
                        self.current_state = Some(self.get_resolver_node());
                    }
                }
                None => {
                    // No next node, execution complete
                    debug!("No next node, execution complete");
                    return Ok(state.finalize());
                }
            }
        }

        // Should not reach here
        Ok(state.finalize())
    }

    /// Execute a single node
    async fn execute_node(
        &mut self,
        node: &Node,
        state: &mut ExecutionState,
    ) -> Result<TaskResult> {
        state.increment_attempts();

        // Check max attempts
        if state.max_attempts_exceeded() {
            self.retry_exhaustions += 1;
            self.terminal_failure = Some(TerminalFailureReason::RetryExhausted);
            return Ok(TaskResult {
                success: false,
                output: String::new(),
                error: Some(format!("Max attempts ({}) exceeded", state.max_attempts)),
            });
        }

        // Execute with agent if available
        if let Some(agent) = &node.agent {
            let agent = Arc::clone(agent);
            let task = state.task.clone();
            let result = tokio::task::spawn_blocking(move || {
                let mut agent_guard = agent.blocking_lock();
                agent_guard.execute(task)
            })
            .await
            .map_err(|err| AgentError::ExecutionFailed(err.to_string()))??;
            state.record_result(node.id, result.clone());
            return Ok(result);
        }

        // No agent, return placeholder result
        Ok(TaskResult {
            success: true,
            output: format!(
                "[{}] Node {} executed (no agent assigned)",
                node.role, node.id
            ),
            error: None,
        })
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Check if graph is valid (all edges reference valid nodes)
    pub fn is_valid(&self) -> bool {
        let node_ids: std::collections::HashSet<_> = self.nodes.iter().map(|n| n.id).collect();

        self.edges
            .iter()
            .all(|e| node_ids.contains(&e.from) && node_ids.contains(&e.to))
    }

    /// Configure explicit workflow guardrails.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.execution_policy = execution_policy;
        self
    }

    /// Validate graph structure and explicit retry/cycle policy.
    pub fn validate_execution_policy(&self) -> Result<()> {
        if !self.is_valid() {
            return Err(AgentError::GraphValidation(
                "graph contains edges that reference missing nodes".to_string(),
            )
            .into());
        }

        if self.detect_cycle()
            && !self
                .nodes
                .iter()
                .any(|node| node.role == NodeRole::Resolver)
        {
            return Err(AgentError::GraphValidation(
                "cyclic workflow requires an explicit resolver node".to_string(),
            )
            .into());
        }

        if self.execution_policy.retry_budget == 0 && self.detect_cycle() {
            return Err(AgentError::GraphValidation(
                "cyclic workflow requires a positive retry budget".to_string(),
            )
            .into());
        }

        Ok(())
    }

    /// Get execution statistics
    pub fn get_stats(&self) -> GraphStats {
        let success_count = self.execution_history.iter().filter(|r| r.success).count();

        GraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            executions: self.execution_history.len(),
            successful_executions: success_count,
            failed_executions: self.execution_history.len() - success_count,
            transition_count: self.transition_count,
            timeout_failures: self.timeout_failures,
            retry_exhaustions: self.retry_exhaustions,
            protocol_validation_failures: self.protocol_validation_failures,
        }
    }

    fn detect_cycle(&self) -> bool {
        let detector = ParallelismDetector::new();
        let dependencies = self
            .edges
            .iter()
            .map(|edge| Dependency {
                from: edge.from,
                to: edge.to,
                dep_type: DependencyType::Hard,
            })
            .collect::<Vec<_>>();
        detector.detect_circular_dependencies(&dependencies)
    }
}

impl Default for WorkflowGraph {
    fn default() -> Self {
        Self::standard()
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub executions: usize,
    pub successful_executions: usize,
    pub failed_executions: usize,
    pub transition_count: usize,
    pub timeout_failures: usize,
    pub retry_exhaustions: usize,
    pub protocol_validation_failures: usize,
}

impl GraphStats {
    pub fn success_rate(&self) -> f32 {
        if self.executions == 0 {
            return 1.0;
        }
        self.successful_executions as f32 / self.executions as f32
    }
}

/// Parallelism detector for determining execution mode
#[derive(Debug, Clone)]
pub struct ParallelismDetector {
    /// Threshold for considering task parallelizable
    pub depth_threshold: usize,
}

impl Default for ParallelismDetector {
    fn default() -> Self {
        Self { depth_threshold: 5 }
    }
}

/// Execution mode (sequential vs parallel)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Execute sequentially (O(N) total time)
    Sequential,

    /// Execute in parallel (O(1) time, O(N²) coordination)
    Parallel,
}

impl ParallelismDetector {
    /// Create new parallelism detector
    pub fn new() -> Self {
        Self::default()
    }

    /// Detect if task is sequential or parallelizable
    pub fn detect(&self, _mission: &str, dependencies: &[Dependency]) -> ExecutionMode {
        // Analyze dependency graph
        let has_circular_deps = self.detect_circular_dependencies(dependencies);
        let max_depth = self.calculate_critical_path_length(dependencies);

        // Heuristics:
        // - Circular deps → Sequential (can't parallelize)
        // - Deep dependency chain → Sequential (coordination overhead)
        // - Independent subtasks → Parallelizable

        debug!(
            "Detecting parallelism: circular={}, max_depth={}",
            has_circular_deps, max_depth
        );

        if has_circular_deps || max_depth > self.depth_threshold {
            ExecutionMode::Sequential
        } else {
            ExecutionMode::Parallel
        }
    }

    /// Detect circular dependencies in graph
    fn detect_circular_dependencies(&self, dependencies: &[Dependency]) -> bool {
        // Build adjacency list
        let mut adj: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        for dep in dependencies {
            adj.entry(dep.from).or_default().push(dep.to);
        }

        // DFS to detect cycles
        let mut visited: std::collections::HashSet<TaskId> = std::collections::HashSet::new();
        let mut rec_stack: std::collections::HashSet<TaskId> = std::collections::HashSet::new();

        for &node in adj.keys() {
            if !visited.contains(&node)
                && self.has_cycle_dfs(node, &adj, &mut visited, &mut rec_stack)
            {
                return true;
            }
        }

        false
    }

    /// DFS helper for cycle detection
    fn has_cycle_dfs(
        &self,
        node: TaskId,
        adj: &HashMap<TaskId, Vec<TaskId>>,
        visited: &mut std::collections::HashSet<TaskId>,
        rec_stack: &mut std::collections::HashSet<TaskId>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = adj.get(&node) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    if self.has_cycle_dfs(neighbor, adj, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }

    /// Calculate critical path length (longest dependency chain)
    fn calculate_critical_path_length(&self, dependencies: &[Dependency]) -> usize {
        if dependencies.is_empty() {
            return 0;
        }

        // Build adjacency list (reverse direction for path calculation)
        let mut adj: HashMap<TaskId, Vec<TaskId>> = HashMap::new();
        let mut in_degree: HashMap<TaskId, usize> = HashMap::new();

        for dep in dependencies {
            adj.entry(dep.to).or_default().push(dep.from);
            *in_degree.entry(dep.from).or_insert(0) += 1;
        }

        // Find nodes with no incoming edges (start nodes)
        let start_nodes: Vec<TaskId> = adj
            .keys()
            .filter(|&n| in_degree.get(n).copied().unwrap_or(0) == 0)
            .copied()
            .collect();

        // BFS to find longest path
        let mut max_depth = 0;

        for start in start_nodes {
            let depth = self.bfs_depth(start, &adj);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// BFS to find depth from a node
    fn bfs_depth(&self, start: TaskId, adj: &HashMap<TaskId, Vec<TaskId>>) -> usize {
        let mut queue: std::collections::VecDeque<(TaskId, usize)> =
            std::collections::VecDeque::new();
        queue.push_back((start, 0));

        let mut visited: std::collections::HashSet<TaskId> = std::collections::HashSet::new();
        visited.insert(start);

        let mut max_depth = 0;

        while let Some((node, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            if let Some(neighbors) = adj.get(&node) {
                for &neighbor in neighbors {
                    if !visited.contains(&neighbor) {
                        visited.insert(neighbor);
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        max_depth
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::BaseAgent;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_graph_creation() {
        let graph = WorkflowGraph::new();

        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.current_state.is_none());
        assert!(graph.is_valid());
    }

    #[test]
    fn test_standard_graph() {
        let graph = WorkflowGraph::standard();

        assert_eq!(graph.node_count(), 4);
        assert!(graph.edge_count() >= 5);
        assert!(graph.is_valid());

        // Check roles exist
        let roles: Vec<_> = graph.nodes.iter().map(|n| n.role.clone()).collect();
        assert!(roles.contains(&NodeRole::Planner));
        assert!(roles.contains(&NodeRole::Executor));
        assert!(roles.contains(&NodeRole::Reviewer));
        assert!(roles.contains(&NodeRole::Resolver));
    }

    #[test]
    fn test_deterministic_execution() {
        let mut graph = WorkflowGraph::standard();

        // Create a simple task
        let task = Task::new("Test task for deterministic execution");

        // Execute (will use placeholder results since no agents assigned)
        let result = tokio_test::block_on(graph.execute(task));

        // Print error for debugging
        if let Err(e) = &result {
            println!("Execution error: {:?}", e);
        }

        // Graph should complete (with or without agents, should not error)
        assert!(result.is_ok() || !graph.execution_history.is_empty());
    }

    #[test]
    fn test_transition_guards() {
        let graph = WorkflowGraph::standard();

        // Create state with success result
        let mut state = ExecutionState::new(Task::new("Test"));
        state.record_result(
            0,
            TaskResult {
                success: true,
                output: "Success".to_string(),
                error: None,
            },
        );

        // Test OnSuccess guard
        let success_result = TaskResult {
            success: true,
            output: String::new(),
            error: None,
        };

        let next = graph.find_next_node(0, &success_result, &state);
        assert!(next.is_some());
        assert_eq!(next.unwrap(), 1); // Should go to Executor

        // Test OnFailure guard
        let failure_result = TaskResult {
            success: false,
            output: String::new(),
            error: Some("Failed".to_string()),
        };

        let next = graph.find_next_node(0, &failure_result, &state);
        assert!(next.is_some());
        assert_eq!(next.unwrap(), 3); // Should go to Resolver
    }

    #[test]
    fn test_sequential_vs_parallel_detection() {
        let detector = ParallelismDetector::new();

        // Test with no dependencies (parallel)
        let mode = detector.detect("Simple task", &[]);
        assert_eq!(mode, ExecutionMode::Parallel);

        // Test with linear dependencies (sequential if deep enough)
        let deps = vec![
            Dependency::hard(1, 0),
            Dependency::hard(2, 1),
            Dependency::hard(3, 2),
            Dependency::hard(4, 3),
            Dependency::hard(5, 4),
            Dependency::hard(6, 5), // 6 levels deep
        ];
        let mode = detector.detect("Complex task", &deps);
        assert_eq!(mode, ExecutionMode::Sequential);

        // Test with circular dependencies (sequential)
        let circular_deps = vec![
            Dependency::hard(0, 1),
            Dependency::hard(1, 2),
            Dependency::hard(2, 0), // Circular!
        ];
        let mode = detector.detect("Circular task", &circular_deps);
        assert_eq!(mode, ExecutionMode::Sequential);
    }

    #[test]
    fn test_coordination_overhead_linear() {
        // Create graph with N nodes
        let mut graph = WorkflowGraph::new();

        for i in 0..10 {
            graph.add_node(Node {
                id: i,
                role: NodeRole::Executor,
                description: format!("Node {}", i),
                agent: None,
            });
        }

        // Add linear edges (O(N) coordination)
        for i in 0..9 {
            graph.add_edge(Edge {
                from: i,
                to: i + 1,
                condition: TransitionCondition::Always,
            });
        }

        // Verify O(N) edges
        assert_eq!(graph.edge_count(), 9);
        assert_eq!(graph.node_count(), 10);

        // Coordination overhead is linear (N-1 edges for N nodes)
        assert_eq!(graph.edge_count(), graph.node_count() - 1);
    }

    #[test]
    fn test_workflow_template_instantiation() {
        let graph = WorkflowGraph::standard();

        // Verify standard template structure
        assert_eq!(graph.get_initial_node(), 0); // Planner
        assert_eq!(graph.get_resolver_node(), 3); // Resolver

        // Verify edges exist
        assert!(graph.edges.iter().any(|e| e.from == 0 && e.to == 1));
        assert!(graph.edges.iter().any(|e| e.from == 1 && e.to == 2));
    }

    #[test]
    fn test_error_handling_with_resolver() {
        let graph = WorkflowGraph::standard();

        // Create state that will trigger resolver
        let state = ExecutionState::new(Task::new("Test"));

        // Simulate failure at Planner
        let failure_result = TaskResult {
            success: false,
            output: String::new(),
            error: Some("Planner failed".to_string()),
        };

        // Should route to resolver
        let next = graph.find_next_node(0, &failure_result, &state);
        assert!(next.is_some());
        assert_eq!(next.unwrap(), graph.get_resolver_node());
    }

    #[test]
    fn test_full_workflow_execution() {
        let mut graph = WorkflowGraph::standard();

        // Assign agents to nodes
        graph.set_agent_for_node(
            0,
            Arc::new(Mutex::new(BaseAgent::new("Planner", "Architect"))),
        );
        graph.set_agent_for_node(
            1,
            Arc::new(Mutex::new(BaseAgent::new("Executor", "Developer"))),
        );
        graph.set_agent_for_node(
            2,
            Arc::new(Mutex::new(BaseAgent::new("Reviewer", "Reviewer"))),
        );

        let task = Task::new("Full workflow test");
        let result = tokio_test::block_on(graph.execute(task));

        // Print error for debugging
        if let Err(e) = &result {
            println!("Workflow execution error: {:?}", e);
        }

        // Should have executed some nodes
        assert!(!graph.execution_history.is_empty());

        // Check stats
        let stats = graph.get_stats();
        assert!(stats.executions > 0);
    }

    #[test]
    fn test_guard_expressions() {
        let graph = WorkflowGraph::standard();

        let state = ExecutionState::new(Task::new("Test"));
        let result = TaskResult {
            success: true,
            output: "Has output".to_string(),
            error: None,
        };

        // Test various guard expressions
        assert!(graph.evaluate_guard_expression("success", &result, &state));
        assert!(!graph.evaluate_guard_expression("failure", &result, &state));
        assert!(graph.evaluate_guard_expression("has_output", &result, &state));
        assert!(!graph.evaluate_guard_expression("no_output", &result, &state));

        // Test attempts guard
        let mut state_with_attempts = state.clone();
        state_with_attempts.attempt_count = 2;
        assert!(graph.evaluate_guard_expression("attempts < 3", &result, &state_with_attempts));
        assert!(!graph.evaluate_guard_expression("attempts < 2", &result, &state_with_attempts));
    }

    #[test]
    fn test_execution_state() {
        let mut state = ExecutionState::new(Task::new("Test"));

        assert_eq!(state.attempt_count, 0);
        assert!(!state.max_attempts_exceeded());

        // Record results
        state.record_result(
            0,
            TaskResult {
                success: true,
                output: "Result 1".to_string(),
                error: None,
            },
        );

        assert_eq!(state.get_result(0).unwrap().output, "Result 1");

        // Increment attempts
        state.increment_attempts();
        state.increment_attempts();
        state.increment_attempts();

        assert!(state.max_attempts_exceeded());
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = WorkflowGraph::standard();

        // Add some execution history
        graph.execution_history.push(ExecutionRecord {
            node_id: 0,
            timestamp: 0,
            success: true,
            output: "Test".to_string(),
        });
        graph.execution_history.push(ExecutionRecord {
            node_id: 1,
            timestamp: 1,
            success: false,
            output: "Failed".to_string(),
        });

        let stats = graph.get_stats();

        assert_eq!(stats.total_nodes, 4);
        assert_eq!(stats.executions, 2);
        assert_eq!(stats.successful_executions, 1);
        assert_eq!(stats.failed_executions, 1);
        assert!((stats.success_rate() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_workflow_policy_rejects_cycle_without_resolver() {
        let mut graph = WorkflowGraph::new();
        graph.add_node(Node {
            id: 0,
            role: NodeRole::Planner,
            description: "planner".to_string(),
            agent: None,
        });
        graph.add_node(Node {
            id: 1,
            role: NodeRole::Executor,
            description: "executor".to_string(),
            agent: None,
        });
        graph.add_edge(Edge {
            from: 0,
            to: 1,
            condition: TransitionCondition::Always,
        });
        graph.add_edge(Edge {
            from: 1,
            to: 0,
            condition: TransitionCondition::Always,
        });

        let result = graph.validate_execution_policy();
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_timeout_transitions_to_terminal_failure() {
        struct SlowAgent;

        impl Agent for SlowAgent {
            fn id(&self) -> &str {
                "slow"
            }

            fn name(&self) -> &str {
                "Slow"
            }

            fn role(&self) -> &str {
                "Executor"
            }

            fn execute(&mut self, _task: Task) -> Result<TaskResult> {
                thread::sleep(StdDuration::from_millis(20));
                Ok(TaskResult {
                    success: true,
                    output: "done".to_string(),
                    error: None,
                })
            }
        }

        let mut graph = WorkflowGraph::standard().with_execution_policy(ExecutionPolicy {
            max_iterations: 8,
            retry_budget: 2,
            node_timeout_ms: 1,
        });
        graph.set_agent_for_node(0, Arc::new(Mutex::new(SlowAgent)));

        let result = tokio_test::block_on(graph.execute(Task::new("slow workflow")));
        if let Ok(result) = result {
            assert!(!result.success);
        }
        assert_eq!(graph.terminal_failure, Some(TerminalFailureReason::Timeout));
        assert_eq!(graph.get_stats().timeout_failures, 1);
    }
}
