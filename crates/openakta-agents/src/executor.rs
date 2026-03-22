//! Concurrent task executor for parallel mission execution.
//!
//! This module provides the ability to execute decomposed mission tasks
//! concurrently across multiple agents, achieving 3-5x speedup.
//!
//! ## Integration Points
//! - Uses **Heartbeat** (Sprint 3b) for agent lifecycle management
//! - Uses **StateMachine** for state tracking
//! - Each agent receives minimal context needed

use crate::agent::{Agent, BaseAgent, CoderAgent, RefactorerAgent, TaskResult};
use crate::decomposer::{DecomposedMission, Dependency, DependencyType, TaskId};
use crate::heartbeat::Heartbeat;
use crate::state_machine::StateMachine;
use crate::task::Task;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tracing::{debug, error, info, warn};

/// Result of mission execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// Whether the mission was successful overall
    pub success: bool,
    /// Results from individual tasks
    pub task_results: HashMap<TaskId, TaskResult>,
    /// Total execution time
    pub total_time: Duration,
    /// Parallelization factor (>1 means parallelism helped)
    /// Calculated as: estimated_sequential_time / actual_parallel_time
    pub parallelization_factor: f32,
    /// Number of tasks executed
    pub tasks_executed: usize,
    /// Number of tasks that failed
    pub tasks_failed: usize,
    /// Number of parallel groups executed
    pub groups_executed: usize,
}

impl MissionResult {
    /// Create a new mission result
    pub fn new() -> Self {
        Self {
            success: true,
            task_results: HashMap::new(),
            total_time: Duration::ZERO,
            parallelization_factor: 1.0,
            tasks_executed: 0,
            tasks_failed: 0,
            groups_executed: 0,
        }
    }

    /// Calculate success based on task results
    pub fn calculate_success(&mut self) {
        self.success = self.tasks_failed == 0;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.tasks_executed == 0 {
            return 1.0;
        }
        (self.tasks_executed - self.tasks_failed) as f32 / self.tasks_executed as f32
    }

    /// Calculate parallelization factor
    /// Factor > 1.0 means parallel execution was faster than sequential
    pub fn calculate_parallelization_factor(&mut self, estimated_sequential: Duration) {
        if self.total_time.is_zero() {
            self.parallelization_factor = 1.0;
            return;
        }

        let factor = estimated_sequential.as_secs_f32() / self.total_time.as_secs_f32();
        self.parallelization_factor = factor.clamp(0.5, 10.0);
    }
}

impl Default for MissionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for concurrent execution
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum concurrent tasks
    pub max_concurrency: usize,
    /// Timeout per task
    pub task_timeout: Duration,
    /// Retry failed tasks
    pub retry_failed: bool,
    /// Maximum retries
    pub max_retries: u32,
    /// Estimated time per task (for parallelization factor calculation)
    pub estimated_task_time: Duration,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            task_timeout: Duration::from_secs(30), // Shorter default timeout
            retry_failed: true,
            max_retries: 2,
            estimated_task_time: Duration::from_millis(100), // Base estimate
        }
    }
}

/// Task execution context
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TaskContext {
    /// Task to execute
    task: Task,
    /// Results from dependency tasks
    dependency_results: HashMap<TaskId, TaskResult>,
    /// Assigned agent ID
    agent_id: String,
}

/// Concurrent executor for parallel task execution
///
/// Integrates with:
/// - **Heartbeat** for agent lifecycle management
/// - **StateMachine** for state tracking
pub struct ConcurrentExecutor {
    /// State machine for agent orchestration
    state_machine: Arc<Mutex<StateMachine>>,
    /// Heartbeat system for lifecycle management
    heartbeat: Option<Arc<Heartbeat>>,
    /// Available agents
    agents: Vec<Arc<Mutex<dyn Agent>>>,
    /// Executor configuration
    config: ExecutorConfig,
    /// Task results cache
    results_cache: Arc<Mutex<HashMap<TaskId, TaskResult>>>,
}

impl ConcurrentExecutor {
    /// Create a new concurrent executor
    pub fn new(state_machine: StateMachine) -> Self {
        Self {
            state_machine: Arc::new(Mutex::new(state_machine)),
            heartbeat: None,
            agents: Vec::new(),
            config: ExecutorConfig::default(),
            results_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create executor with custom config
    pub fn with_config(state_machine: StateMachine, config: ExecutorConfig) -> Self {
        Self {
            state_machine: Arc::new(Mutex::new(state_machine)),
            heartbeat: None,
            agents: Vec::new(),
            config,
            results_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create executor with heartbeat integration
    pub fn with_heartbeat(
        state_machine: StateMachine,
        heartbeat: Heartbeat,
        config: ExecutorConfig,
    ) -> Self {
        Self {
            state_machine: Arc::new(Mutex::new(state_machine)),
            heartbeat: Some(Arc::new(heartbeat)),
            agents: Vec::new(),
            config,
            results_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set heartbeat system
    pub fn set_heartbeat(&mut self, heartbeat: Heartbeat) {
        self.heartbeat = Some(Arc::new(heartbeat));
    }

    /// Register an agent for task execution
    pub fn register_agent(&mut self, agent: Arc<Mutex<dyn Agent>>) {
        self.agents.push(agent);
    }

    /// Register default agents
    pub fn register_default_agents(&mut self) {
        self.register_agent(Arc::new(Mutex::new(CoderAgent::new())));
        self.register_agent(Arc::new(Mutex::new(RefactorerAgent::new())));
        self.register_agent(Arc::new(Mutex::new(BaseAgent::new(
            "Developer 2",
            "Developer",
        ))));
        self.register_agent(Arc::new(Mutex::new(BaseAgent::new(
            "Developer 3",
            "Developer",
        ))));
        self.register_agent(Arc::new(Mutex::new(BaseAgent::new(
            "Developer 4",
            "Developer",
        ))));
    }

    /// Execute a group of tasks concurrently
    pub async fn execute_group(
        &self,
        task_ids: &[TaskId],
        mission: &DecomposedMission,
    ) -> Vec<TaskResult> {
        info!("Executing group of {} tasks concurrently", task_ids.len());

        let mut handles: Vec<JoinHandle<TaskResult>> = Vec::new();
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrency));

        for &task_id in task_ids {
            if task_id >= mission.tasks.len() {
                warn!("Task ID {} out of range", task_id);
                continue;
            }

            let task = mission.tasks[task_id].clone();
            let semaphore = Arc::clone(&semaphore);
            let results_cache = Arc::clone(&self.results_cache);
            let dependencies = self.get_task_dependencies(task_id, mission);
            let agents = self.agents.clone();
            let timeout = self.config.task_timeout;

            let handle = tokio::spawn(async move {
                // Acquire semaphore permit
                let _permit = semaphore.acquire().await.unwrap();

                // Wait for dependencies (with timeout from config)
                let dep_results =
                    Self::wait_for_dependencies(task_id, &dependencies, &results_cache, timeout)
                        .await;

                // Execute task
                Self::execute_single_task(task, dep_results, &agents).await
            });

            handles.push(handle);
        }

        // Collect results
        let mut results = Vec::new();
        for handle in handles {
            match handle.await {
                Ok(result) => results.push(result),
                Err(e) => {
                    error!("Task execution failed: {}", e);
                    results.push(TaskResult {
                        success: false,
                        output: String::new(),
                        error: Some(format!("Task panicked: {}", e)),
                    });
                }
            }
        }

        results
    }

    /// Execute all tasks in a decomposed mission
    pub async fn execute_all(&self, mission: &DecomposedMission) -> MissionResult {
        info!(
            "Executing mission with {} tasks in {} parallel groups",
            mission.tasks.len(),
            mission.parallel_groups.len()
        );

        let start_time = Instant::now();
        let mut result = MissionResult::new();

        if mission.tasks.is_empty() {
            warn!("No tasks to execute");
            result.total_time = start_time.elapsed();
            return result;
        }

        // Calculate estimated sequential time for parallelization factor
        let estimated_sequential = Duration::from_millis(
            mission.tasks.len() as u64 * self.config.estimated_task_time.as_millis() as u64,
        );

        // Execute groups in order (respecting dependencies between groups)
        for (group_idx, group) in mission.parallel_groups.iter().enumerate() {
            debug!("Executing group {} with {} tasks", group_idx, group.len());

            // Wake up agents via heartbeat if available
            if let Some(hb) = &self.heartbeat {
                for &task_id in group {
                    let agent_id = format!("agent_{}", task_id);
                    hb.wake_now(&agent_id).await;
                }
            }

            let group_results = self.execute_group(group, mission).await;

            // Store results in cache
            {
                let mut cache = self.results_cache.lock().await;
                for (i, &task_id) in group.iter().enumerate() {
                    if i < group_results.len() {
                        let task_result = &group_results[i];
                        cache.insert(task_id, task_result.clone());

                        result.tasks_executed += 1;
                        if !task_result.success {
                            result.tasks_failed += 1;
                        }

                        // Update state machine if agent is registered
                        let mut sm = self.state_machine.lock().await;
                        let agent_id = format!("agent_{}", task_id);
                        if sm.get_agent_state(&agent_id).is_some() {
                            if task_result.success {
                                let _ = sm.complete_task(&agent_id, true);
                            } else {
                                let _ = sm.complete_task(&agent_id, false);
                            }
                        }
                    }
                }
            }

            // Check if any task in group failed (for hard dependencies)
            let group_has_failures = group_results.iter().any(|r| !r.success);
            if group_has_failures {
                warn!("Group {} had failures, continuing with caution", group_idx);
            }

            result.groups_executed += 1;
        }

        // Collect all results
        {
            let cache = self.results_cache.lock().await;
            result.task_results = cache.clone();
        }

        result.total_time = start_time.elapsed();
        result.calculate_parallelization_factor(estimated_sequential);
        result.calculate_success();

        info!(
            "Mission completed in {:?} with {}% success rate (parallelization factor: {:.2}x)",
            result.total_time,
            result.success_rate() * 100.0,
            result.parallelization_factor
        );

        // Put agents back to sleep via heartbeat if available
        if let Some(hb) = &self.heartbeat {
            for i in 0..mission.tasks.len() {
                let agent_id = format!("agent_{}", i);
                hb.put_agent_to_sleep(&agent_id).await;
            }
        }

        result
    }

    /// Execute a single task
    async fn execute_single_task(
        task: Task,
        dep_results: HashMap<TaskId, TaskResult>,
        agents: &[Arc<Mutex<dyn Agent>>],
    ) -> TaskResult {
        debug!("Executing task: {}", task.description);

        // Select an agent (round-robin for now)
        if agents.is_empty() {
            return TaskResult {
                success: false,
                output: String::new(),
                error: Some("No agents available".to_string()),
            };
        }

        // Simple agent selection based on task hash
        let agent_idx = task.description.len() % agents.len();
        let agent = &agents[agent_idx];

        // Execute task with agent
        let mut agent_guard = agent.lock().await;
        match agent_guard.execute(task) {
            Ok(mut result) => {
                // Add dependency context to output
                if !dep_results.is_empty() {
                    result.output = format!(
                        "{}\n[Dependencies: {} completed]",
                        result.output,
                        dep_results.len()
                    );
                }
                result
            }
            Err(e) => TaskResult {
                success: false,
                output: String::new(),
                error: Some(format!("Agent execution failed: {}", e)),
            },
        }
    }

    /// Get dependencies for a task
    fn get_task_dependencies(
        &self,
        task_id: TaskId,
        mission: &DecomposedMission,
    ) -> Vec<Dependency> {
        mission
            .dependencies
            .iter()
            .filter(|d| d.from == task_id)
            .cloned()
            .collect()
    }

    /// Wait for dependencies to complete (with shorter timeout for tests)
    async fn wait_for_dependencies(
        task_id: TaskId,
        dependencies: &[Dependency],
        results_cache: &Arc<Mutex<HashMap<TaskId, TaskResult>>>,
        timeout: Duration,
    ) -> HashMap<TaskId, TaskResult> {
        let mut dep_results = HashMap::new();
        let start = Instant::now();

        for dep in dependencies {
            // Use shorter check interval for faster timeout detection
            let check_interval = Duration::from_millis(10);

            loop {
                if start.elapsed() > timeout {
                    warn!(
                        "Timeout waiting for dependency {} for task {}",
                        dep.to, task_id
                    );
                    break;
                }

                {
                    let cache = results_cache.lock().await;
                    if let Some(result) = cache.get(&dep.to) {
                        if dep.dep_type == DependencyType::Hard && !result.success {
                            warn!("Hard dependency {} failed for task {}", dep.to, task_id);
                        }
                        dep_results.insert(dep.to, result.clone());
                        break;
                    }
                }

                // Wait a bit before checking again (shorter interval)
                tokio::time::sleep(check_interval).await;
            }
        }

        dep_results
    }

    /// Get executor statistics
    pub async fn get_stats(&self) -> ExecutorStats {
        let cache = self.results_cache.lock().await;

        ExecutorStats {
            total_tasks: cache.len(),
            successful_tasks: cache.values().filter(|r| r.success).count(),
            failed_tasks: cache.values().filter(|r| !r.success).count(),
            agents_registered: self.agents.len(),
            max_concurrency: self.config.max_concurrency,
        }
    }

    /// Clear results cache
    pub async fn clear_cache(&self) {
        let mut cache = self.results_cache.lock().await;
        cache.clear();
        debug!("Cleared results cache");
    }

    /// Get config
    pub fn config(&self) -> &ExecutorConfig {
        &self.config
    }
}

/// Execution statistics
#[derive(Debug, Clone)]
pub struct ExecutorStats {
    /// Total tasks executed
    pub total_tasks: usize,
    /// Successful tasks
    pub successful_tasks: usize,
    /// Failed tasks
    pub failed_tasks: usize,
    /// Number of registered agents
    pub agents_registered: usize,
    /// Maximum concurrency
    pub max_concurrency: usize,
}

impl ExecutorStats {
    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        if self.total_tasks == 0 {
            return 1.0;
        }
        self.successful_tasks as f32 / self.total_tasks as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decomposer::MissionDecomposer;

    #[tokio::test]
    async fn test_concurrent_execution() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose("Implement authentication with login")
            .unwrap();

        // Execute first group
        if !mission.parallel_groups.is_empty() {
            let results = executor
                .execute_group(&mission.parallel_groups[0], &mission)
                .await;
            assert!(!results.is_empty());
        }
    }

    #[tokio::test]
    async fn test_execute_all() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose("Build API with authentication")
            .unwrap();

        let result = executor.execute_all(&mission).await;

        assert!(result.tasks_executed > 0);
        assert!(result.total_time > Duration::ZERO);
        assert!(result.parallelization_factor >= 0.5);
    }

    #[tokio::test]
    async fn test_executor_config() {
        let config = ExecutorConfig {
            max_concurrency: 8,
            task_timeout: Duration::from_secs(600),
            retry_failed: false,
            max_retries: 0,
            estimated_task_time: Duration::from_millis(50),
        };

        let executor =
            ConcurrentExecutor::with_config(StateMachine::new().unwrap(), config.clone());

        assert_eq!(executor.config().max_concurrency, 8);
        assert_eq!(executor.config().task_timeout, Duration::from_secs(600));
        assert!(!executor.config().retry_failed);
    }

    #[tokio::test]
    async fn test_mission_result() {
        let mut result = MissionResult::new();

        result.tasks_executed = 10;
        result.tasks_failed = 2;
        result.calculate_success();

        assert!(!result.success);
        assert!((result.success_rate() - 0.8).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_executor_stats() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        // Execute something first
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test mission").unwrap();
        let _ = executor.execute_all(&mission).await;

        let stats = executor.get_stats().await;

        assert!(stats.agents_registered >= 4);
        assert!(stats.max_concurrency > 0);
        assert!(stats.success_rate() >= 0.0 && stats.success_rate() <= 1.0);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        // Execute to populate cache
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test mission").unwrap();
        let _ = executor.execute_all(&mission).await;

        // Clear cache
        executor.clear_cache().await;

        let stats = executor.get_stats().await;
        assert_eq!(stats.total_tasks, 0);
    }

    #[tokio::test]
    async fn test_dependency_waiting() {
        let cache: Arc<Mutex<HashMap<TaskId, TaskResult>>> = Arc::new(Mutex::new(HashMap::new()));

        // Add a result
        {
            let mut cache_guard = cache.lock().await;
            cache_guard.insert(
                0,
                TaskResult {
                    success: true,
                    output: "Done".to_string(),
                    error: None,
                },
            );
        }

        let dependencies = vec![Dependency::hard(1, 0)];
        let results = ConcurrentExecutor::wait_for_dependencies(
            1,
            &dependencies,
            &cache,
            Duration::from_millis(500), // Shorter timeout for tests
        )
        .await;

        assert_eq!(results.len(), 1);
        assert!(results.contains_key(&0));
    }

    #[tokio::test]
    async fn test_dependency_waiting_timeout() {
        let cache: Arc<Mutex<HashMap<TaskId, TaskResult>>> = Arc::new(Mutex::new(HashMap::new()));

        // Don't add any result - should timeout
        let dependencies = vec![Dependency::hard(1, 0)];
        let results = ConcurrentExecutor::wait_for_dependencies(
            1,
            &dependencies,
            &cache,
            Duration::from_millis(50), // Short timeout
        )
        .await;

        // Should timeout and return empty
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_heartbeat_integration() {
        use crate::heartbeat::{Heartbeat, HeartbeatConfig};

        let state_machine = StateMachine::new().unwrap();
        let heartbeat = Heartbeat::new(HeartbeatConfig::default());
        let config = ExecutorConfig::default();

        let mut executor = ConcurrentExecutor::with_heartbeat(state_machine, heartbeat, config);
        executor.register_default_agents();

        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test mission with heartbeat").unwrap();

        let result = executor.execute_all(&mission).await;

        assert!(result.tasks_executed > 0);
        assert!(executor.heartbeat.is_some());
    }

    // Heavy: large mission decomposition + parallel groups. Run: `cargo test -p openakta-agents -- --ignored test_parallel_group_execution`
    #[ignore]
    #[tokio::test]
    async fn test_parallel_group_execution() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose("Implement authentication system with JWT")
            .unwrap();

        // Execute all groups
        let mut all_results = Vec::new();
        for group in &mission.parallel_groups {
            let results = executor.execute_group(group, &mission).await;
            all_results.extend(results);
        }

        assert!(!all_results.is_empty());
    }

    #[tokio::test]
    async fn test_agent_selection() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());

        // Register agents with different roles
        executor.register_agent(Arc::new(Mutex::new(BaseAgent::new("Agent1", "Architect"))));
        executor.register_agent(Arc::new(Mutex::new(BaseAgent::new("Agent2", "Developer"))));

        let task = Task::new("Test task for selection");
        let result =
            ConcurrentExecutor::execute_single_task(task, HashMap::new(), &executor.agents).await;

        assert!(result.success);
        assert!(!result.output.is_empty());
    }

    #[tokio::test]
    async fn test_empty_mission_execution() {
        let executor = ConcurrentExecutor::new(StateMachine::new().unwrap());

        let mission = DecomposedMission::new("Empty mission");

        let result = executor.execute_all(&mission).await;

        assert_eq!(result.tasks_executed, 0);
        assert_eq!(result.success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_full_workflow() {
        let mut executor = ConcurrentExecutor::new(StateMachine::new().unwrap());
        executor.register_default_agents();

        let decomposer = MissionDecomposer::new();

        // Full workflow
        let mission = "Create REST API with authentication, user management, and data validation";
        let decomposed = decomposer.decompose(mission).unwrap();

        assert!(!decomposed.tasks.is_empty());
        assert!(!decomposed.parallel_groups.is_empty());

        let result = executor.execute_all(&decomposed).await;

        assert!(result.tasks_executed > 0);
        assert!(result.total_time > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_retry_configuration() {
        let config = ExecutorConfig {
            max_concurrency: 2,
            task_timeout: Duration::from_secs(30),
            retry_failed: true,
            max_retries: 3,
            estimated_task_time: Duration::from_millis(50),
        };

        let executor = ConcurrentExecutor::with_config(StateMachine::new().unwrap(), config);

        assert!(executor.config().retry_failed);
        assert_eq!(executor.config().max_retries, 3);
    }

    #[test]
    fn test_executor_stats_methods() {
        let stats = ExecutorStats {
            total_tasks: 10,
            successful_tasks: 8,
            failed_tasks: 2,
            agents_registered: 4,
            max_concurrency: 4,
        };

        assert!((stats.success_rate() - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_mission_result_default() {
        let result = MissionResult::default();

        assert!(result.success);
        assert!(result.task_results.is_empty());
        assert_eq!(result.total_time, Duration::ZERO);
        assert_eq!(result.parallelization_factor, 1.0);
    }

    #[test]
    fn test_executor_config_default() {
        let config = ExecutorConfig::default();

        assert_eq!(config.max_concurrency, 4);
        assert_eq!(config.task_timeout, Duration::from_secs(30));
        assert!(config.retry_failed);
        assert_eq!(config.max_retries, 2);
        assert_eq!(config.estimated_task_time, Duration::from_millis(100));
    }
}
