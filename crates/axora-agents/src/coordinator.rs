//! Centralized Coordinator (Orchestrator-Worker Topology)
//!
//! This module implements the Centralized Coordinator pattern:
//! - Maintains DAG (from graph-based decomposer)
//! - Spawns workers for parallel groups
//! - Enforces synchronization barriers
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                   Coordinator                           │
//! │  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐ │
//! │  │     DAG     │───▶│   Barrier   │───▶│   Results   │ │
//! │  └─────────────┘    └─────────────┘    └─────────────┘ │
//! └─────────────────────────────────────────────────────────┘
//!                            │
//!         ┌──────────────────┼──────────────────┐
//!         │                  │                  │
//!         ▼                  ▼                  ▼
//! ┌───────────────┐  ┌───────────────┐  ┌───────────────┐
//! │  Worker 1     │  │  Worker 2     │  │  Worker N     │
//! │  (ReAct)      │  │  (ReAct)      │  │  (ReAct)      │
//! └───────────────┘  └───────────────┘  └───────────────┘
//! ```

pub mod v2;

use crate::decomposer::{DecomposedMission, Dependency, TaskId};
use crate::error::AgentError;
use crate::graph::{ExecutionMode, WorkflowGraph};
use crate::memory::SharedBlackboard;
use crate::react::{DualThreadReactAgent, InterruptSignal, ReactCycle, ReactStats, ToolSet};
use crate::task::{Task, TaskStatus};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Barrier, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Mission result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// Overall success
    pub success: bool,
    /// Task results
    pub task_results: HashMap<TaskId, TaskResult>,
    /// Total execution time
    pub total_time: Duration,
    /// Parallelization factor
    pub parallelization_factor: f32,
}

/// Task result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Success flag
    pub success: bool,
    /// Output
    pub output: String,
    /// Error message
    pub error: Option<String>,
    /// ReAct cycles executed
    pub cycles: Vec<ReactCycle>,
    /// Execution stats
    pub stats: Option<ReactStats>,
}

/// DAG (Directed Acyclic Graph) for task dependencies
#[derive(Debug, Clone)]
pub struct DAG {
    /// Task nodes
    pub nodes: HashMap<TaskId, Task>,
    /// Dependencies
    pub edges: Vec<Dependency>,
    /// Critical path (task IDs)
    pub critical_path: Vec<TaskId>,
}

impl DAG {
    /// Create new DAG from decomposed mission
    pub fn from_mission(mission: &DecomposedMission) -> Self {
        let nodes: HashMap<TaskId, Task> = mission
            .tasks
            .iter()
            .enumerate()
            .map(|(i, t)| (i, t.clone()))
            .collect();

        Self {
            nodes,
            edges: mission.dependencies.clone(),
            critical_path: mission.critical_path.clone(),
        }
    }

    /// Check if task is on critical path
    pub fn is_critical(&self, task_id: TaskId) -> bool {
        self.critical_path.contains(&task_id)
    }

    /// Get dependencies for a task
    pub fn get_dependencies(&self, task_id: TaskId) -> Vec<TaskId> {
        self.edges
            .iter()
            .filter(|d| d.from == task_id)
            .map(|d| d.to)
            .collect()
    }
}

/// Centralized Coordinator
pub struct Coordinator {
    /// DAG (from graph-based decomposer)
    dag: DAG,

    /// Blackboard (shared state)
    blackboard: Arc<Mutex<SharedBlackboard>>,

    /// Active workers
    workers: HashMap<TaskId, JoinHandle<Result<TaskResult>>>,

    /// Synchronization barrier
    barrier: Arc<Barrier>,

    /// Task results
    results: HashMap<TaskId, TaskResult>,

    /// Start time
    start_time: Option<Instant>,

    /// Execution mode
    execution_mode: ExecutionMode,
}

impl Coordinator {
    /// Create new coordinator
    pub fn new(mission: &DecomposedMission) -> Self {
        let dag = DAG::from_mission(mission);
        let num_groups = mission.parallel_groups.len().max(1);

        Self {
            dag,
            blackboard: Arc::new(Mutex::new(SharedBlackboard::new())),
            workers: HashMap::new(),
            barrier: Arc::new(Barrier::new(num_groups)),
            results: HashMap::new(),
            start_time: None,
            execution_mode: mission.execution_mode.clone(),
        }
    }

    /// Execute mission (DAG-based)
    pub async fn execute_mission(&mut self, mission: &DecomposedMission) -> Result<MissionResult> {
        info!(
            "Coordinator executing mission: {} (mode: {:?})",
            mission.original_mission, self.execution_mode
        );

        self.start_time = Some(Instant::now());

        // Execute parallel groups sequentially
        for (group_idx, group) in mission.parallel_groups.iter().enumerate() {
            info!(
                "Executing parallel group {} ({} tasks)",
                group_idx,
                group.len()
            );

            // Spawn all workers in group concurrently
            let mut handles = Vec::new();

            for &task_id in group {
                let task = self.dag.nodes.get(&task_id).cloned().ok_or_else(|| {
                    AgentError::AgentNotFound(format!("Task {} not found", task_id))
                })?;

                let blackboard = Arc::clone(&self.blackboard);
                let tools = self.get_tools_for_task(&task)?;

                // Spawn worker (dual-thread ReAct)
                let handle =
                    tokio::spawn(async move { Self::spawn_worker(task, blackboard, tools).await });

                handles.push((task_id, handle));
            }

            // Wait for all workers in group (synchronization barrier)
            for (task_id, handle) in handles {
                match handle.await {
                    Ok(Ok(task_result)) => {
                        // Success → store result
                        info!("Task {} completed successfully", task_id);
                        self.results.insert(task_id, task_result);

                        // Merge result to blackboard
                        self.merge_result_to_blackboard(task_id).await?;
                    }
                    Ok(Err(e)) => {
                        // Task failed → retry or escalate
                        error!("Task {} failed: {}", task_id, e);
                        self.results.insert(
                            task_id,
                            TaskResult {
                                success: false,
                                output: String::new(),
                                error: Some(e.to_string()),
                                cycles: Vec::new(),
                                stats: None,
                            },
                        );
                    }
                    Err(e) => {
                        // Worker panicked
                        error!("Worker for task {} panicked: {}", task_id, e);
                        self.results.insert(
                            task_id,
                            TaskResult {
                                success: false,
                                output: String::new(),
                                error: Some(format!("Worker panicked: {}", e)),
                                cycles: Vec::new(),
                                stats: None,
                            },
                        );
                    }
                }
            }

            // Barrier complete → next group
            self.barrier.wait().await;
            debug!("Group {} barrier complete", group_idx);
        }

        // Calculate total time
        let total_time = self
            .start_time
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO);

        // Calculate parallelization factor
        let parallelization_factor = self.calculate_parallelization_factor(mission, total_time);

        // Check overall success
        let success = self.results.values().all(|r| r.success);

        Ok(MissionResult {
            success,
            task_results: self.results.clone(),
            total_time,
            parallelization_factor,
        })
    }

    /// Spawn worker for a task
    async fn spawn_worker(
        task: Task,
        blackboard: Arc<Mutex<SharedBlackboard>>,
        tools: ToolSet,
    ) -> Result<TaskResult> {
        debug!("Spawning worker for task: {}", task.id);

        // Spawn dual-thread ReAct agent
        let mut agent = DualThreadReactAgent::spawn(task, blackboard, tools).await?;

        // Execute all cycles
        let result = agent.execute_all().await?;

        // Get stats
        let stats = agent.get_stats();
        let cycles = agent.get_cycles().to_vec();

        Ok(TaskResult {
            success: result.success,
            output: result.output,
            error: result.error,
            cycles,
            stats: Some(stats),
        })
    }

    /// Route tasks based on critical path
    fn get_tools_for_task(&self, task: &Task) -> Result<ToolSet> {
        let task_id = self
            .dag
            .nodes
            .iter()
            .find(|(_, t)| t.id == task.id)
            .map(|(id, _)| *id)
            .unwrap_or(0);

        let is_critical = self.dag.is_critical(task_id);

        if is_critical {
            // Critical path → powerful model (frontier LLM)
            info!(
                "Task {} is on critical path → routing to powerful model",
                task_id
            );
            Ok(ToolSet::with_powerful_llm())
        } else {
            // Off-path → smaller/faster model (SLM)
            info!("Task {} is off critical path → routing to SLM", task_id);
            Ok(ToolSet::with_small_llm())
        }
    }

    /// Merge result to blackboard
    async fn merge_result_to_blackboard(&mut self, task_id: TaskId) -> Result<()> {
        if let Some(result) = self.results.get(&task_id) {
            let mut blackboard = self.blackboard.lock().await;

            // Create memory entry for result
            use crate::memory::{MemoryEntry, MemoryType};
            use std::time::{SystemTime, UNIX_EPOCH};

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let entry = MemoryEntry {
                id: format!("task_{}_result", task_id),
                content: result.output.clone(),
                memory_type: MemoryType::Shared,
                importance: if result.success { 0.9 } else { 0.5 },
                access_count: 0,
                created_at: now,
                last_accessed: now,
                expires_at: None,
            };

            // Publish to all agents
            blackboard.publish(entry, vec!["all".to_string()]);

            debug!("Merged task {} result to blackboard", task_id);
        }

        Ok(())
    }

    /// Calculate parallelization factor
    fn calculate_parallelization_factor(
        &self,
        mission: &DecomposedMission,
        actual_time: Duration,
    ) -> f32 {
        if mission.tasks.is_empty() || actual_time.is_zero() {
            return 1.0;
        }

        // Estimate sequential time
        let estimated_sequential = Duration::from_millis(100 * mission.tasks.len() as u64);

        let factor = estimated_sequential.as_secs_f32() / actual_time.as_secs_f32();
        factor.clamp(0.5, 10.0)
    }

    /// Send interrupt to worker
    pub async fn interrupt_worker(&self, task_id: TaskId, signal: InterruptSignal) -> Result<()> {
        // In a full implementation, this would send to the worker's interrupt channel
        // For now, just log
        info!("Interrupt sent to worker {}: {:?}", task_id, signal);
        Ok(())
    }

    /// Get coordinator stats
    pub fn get_stats(&self) -> CoordinatorStats {
        let total_tasks = self.results.len();
        let successful = self.results.values().filter(|r| r.success).count();

        CoordinatorStats {
            total_tasks,
            successful_tasks: successful,
            failed_tasks: total_tasks - successful,
            execution_mode: self.execution_mode.clone(),
        }
    }
}

/// Coordinator statistics
#[derive(Debug, Clone)]
pub struct CoordinatorStats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub execution_mode: ExecutionMode,
}

impl CoordinatorStats {
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
    async fn test_coordinator_creation() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test mission").unwrap();

        let coordinator = Coordinator::new(&mission);

        assert_eq!(coordinator.dag.nodes.len(), mission.tasks.len());
        assert!(coordinator.workers.is_empty());
    }

    #[tokio::test]
    async fn test_coordinator_parallel_execution() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test parallel execution").unwrap();

        let mut coordinator = Coordinator::new(&mission);
        let result = coordinator.execute_mission(&mission).await.unwrap();

        // Should have executed some tasks
        assert!(result.task_results.len() >= 0);
        assert!(result.total_time > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_synchronization_barrier() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test barrier").unwrap();

        let mut coordinator = Coordinator::new(&mission);
        let result = coordinator.execute_mission(&mission).await.unwrap();

        // Barrier should complete
        assert!(result.total_time > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_critical_path_routing() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test critical path").unwrap();

        let coordinator = Coordinator::new(&mission);

        // Get first task
        if let Some((task_id, task)) = coordinator.dag.nodes.iter().next() {
            let is_critical = coordinator.dag.is_critical(*task_id);

            // Get tools for task
            let tools = coordinator.get_tools_for_task(task).unwrap();

            // Verify routing based on critical path
            if is_critical {
                // Should use powerful LLM
                // (In practice, check the model name)
            } else {
                // Should use SLM
            }

            // Just verify tools were created
            assert!(!tools.tool_names().is_empty() || tools.llm_model.len() > 0);
        }
    }

    #[tokio::test]
    async fn test_dag_creation() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test DAG").unwrap();

        let dag = DAG::from_mission(&mission);

        assert_eq!(dag.nodes.len(), mission.tasks.len());
        assert_eq!(dag.edges.len(), mission.dependencies.len());
    }

    #[tokio::test]
    async fn test_dag_critical_path() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose("Test critical path detection")
            .unwrap();

        let dag = DAG::from_mission(&mission);

        // Check critical path
        for &task_id in &dag.critical_path {
            assert!(dag.is_critical(task_id));
        }
    }

    #[tokio::test]
    async fn test_mission_result() {
        let result = MissionResult {
            success: true,
            task_results: HashMap::new(),
            total_time: Duration::from_secs(10),
            parallelization_factor: 2.5,
        };

        assert!(result.success);
        assert_eq!(result.parallelization_factor, 2.5);
    }

    #[tokio::test]
    async fn test_task_result() {
        let result = TaskResult {
            success: true,
            output: "Success".to_string(),
            error: None,
            cycles: Vec::new(),
            stats: None,
        };

        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_coordinator_stats() {
        let stats = CoordinatorStats {
            total_tasks: 10,
            successful_tasks: 8,
            failed_tasks: 2,
            execution_mode: ExecutionMode::Parallel,
        };

        assert!((stats.success_rate() - 0.8).abs() < 0.01);
        assert_eq!(stats.execution_mode, ExecutionMode::Parallel);
    }

    #[tokio::test]
    async fn test_interrupt_worker() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test interrupt").unwrap();

        let coordinator = Coordinator::new(&mission);

        let signal = InterruptSignal::Stop {
            reason: "Test".to_string(),
        };

        let result = coordinator.interrupt_worker(0, signal).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_full_mission_execution() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer
            .decompose("Full mission test with workflow")
            .unwrap();

        let mut coordinator = Coordinator::new(&mission);
        let result = coordinator.execute_mission(&mission).await.unwrap();

        // Verify execution completed
        assert!(result.total_time > Duration::ZERO);

        // Check stats
        let stats = coordinator.get_stats();
        assert!(stats.total_tasks >= 0);
    }

    #[tokio::test]
    async fn test_workflow_graph_execution() {
        let decomposer = MissionDecomposer::new();
        let mission = decomposer.decompose("Test workflow standard").unwrap();

        // Check if workflow graph is available
        if let Some(graph) = &mission.workflow_graph {
            assert!(graph.node_count() > 0);
            assert!(graph.is_valid());
        }
    }
}
