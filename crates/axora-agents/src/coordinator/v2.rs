//! Coordinator v2 core orchestration.
//!
//! This is the Phase 3 coordinator foundation: it tracks workers, loads a
//! decomposed mission into a queue, dispatches ready tasks, monitors worker
//! health, and merges basic task output into a shared blackboard.
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use axora_agents::coordinator::v2::{BlackboardV2, Coordinator, CoordinatorConfig};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let blackboard = Arc::new(BlackboardV2::default());
//! let mut coordinator = Coordinator::new(CoordinatorConfig::default(), blackboard)?;
//! let result = coordinator.execute_mission("simple task").await?;
//!
//! assert!(result.success);
//! # Ok(())
//! # }
//! ```

#[path = "v2_core.rs"]
pub mod v2_core;
#[path = "v2_dispatcher.rs"]
pub mod v2_dispatcher;
#[path = "v2_queue_integration.rs"]
pub mod v2_queue_integration;

use self::v2_core::{CoordinatorCoreError, Result as CoreResult};
use self::v2_dispatcher::{
    CompletionReport, DispatchCompletion, DispatchWorker, DispatchWorkerStatus, Dispatcher,
    DispatcherError,
};
use self::v2_queue_integration::{QueueIntegrationError, TaskQueueIntegration};
use crate::decomposer::MissionDecomposer;
use crate::memory::{MemoryEntry, MemoryType, SharedBlackboard};
use crate::task::{Task, TaskStatus};
use crate::worker_pool::{WorkerId, WorkerStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::sleep;
use uuid::Uuid;

pub use self::v2_core::{
    Coordinator as CoordinatorCore, WorkerInfo, WorkerInfo as RegisteredWorkerInfo, WorkerRegistry,
};
pub use self::v2_dispatcher::{
    CompletionReport as DispatchCompletionReport, DispatchLoopReport,
    DispatchWorker as CoordinatorDispatchWorker,
    DispatchWorkerStatus as CoordinatorDispatchWorkerStatus, Dispatcher as CoordinatorDispatcher,
    MonitorReport,
};
pub use self::v2_queue_integration::TaskQueueIntegration as CoordinatorTaskQueue;

/// Shared blackboard used by Coordinator v2.
pub type BlackboardV2 = Mutex<SharedBlackboard>;

/// Result type for Coordinator v2 operations.
pub type Result<T> = std::result::Result<T, CoordinatorV2Error>;

/// Errors produced by Coordinator v2.
#[derive(Debug, Error)]
pub enum CoordinatorV2Error {
    /// Invalid configuration was supplied.
    #[error("invalid coordinator config: {0}")]
    InvalidConfig(String),

    /// Worker registry operation failed.
    #[error("worker registry error: {0}")]
    Core(#[from] CoordinatorCoreError),

    /// Queue integration operation failed.
    #[error("task queue error: {0}")]
    Queue(#[from] QueueIntegrationError),

    /// Dispatcher operation failed.
    #[error("dispatcher error: {0}")]
    Dispatcher(#[from] DispatcherError),

    /// Mission decomposition failed.
    #[error("mission decomposition failed: {0}")]
    Decomposition(String),

    /// No worker was available when work was ready.
    #[error("no available worker")]
    NoAvailableWorker,

    /// The mission could not make progress.
    #[error("mission stalled: {0}")]
    StalledMission(String),
}

/// Coordinator runtime configuration.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Maximum workers managed by the coordinator.
    pub max_workers: usize,
    /// Delay between idle dispatch passes.
    pub dispatch_interval: Duration,
    /// Whether worker monitoring is enabled.
    pub enable_monitoring: bool,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_workers: 10,
            dispatch_interval: Duration::from_secs(1),
            enable_monitoring: true,
        }
    }
}

/// Final result of a mission execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionResult {
    /// Generated mission identifier.
    pub mission_id: String,
    /// Whether all tasks succeeded.
    pub success: bool,
    /// Merged output from completed tasks.
    pub output: String,
    /// Number of completed tasks.
    pub tasks_completed: usize,
    /// Number of failed tasks.
    pub tasks_failed: usize,
    /// Total mission duration.
    pub duration: Duration,
}

/// Current coordinator mission status.
#[derive(Debug, Clone, PartialEq)]
pub struct MissionStatus {
    /// Active mission identifier.
    pub mission_id: String,
    /// Progress in the range `0.0..=100.0`.
    pub progress: f32,
    /// Estimated time remaining when progress is non-zero.
    pub eta: Option<Duration>,
    /// Number of workers currently executing tasks.
    pub active_workers: usize,
    /// Number of completed tasks.
    pub completed_tasks: usize,
    /// Total tasks in the mission.
    pub total_tasks: usize,
}

/// Phase 3 coordinator.
pub struct Coordinator {
    /// Registry of workers under coordinator control.
    pub worker_registry: WorkerRegistry,
    /// Task queue integration for the active mission.
    pub task_queue: TaskQueueIntegration,
    /// Dispatcher for task execution flow.
    pub dispatcher: Dispatcher,
    /// Shared blackboard.
    pub blackboard: Arc<BlackboardV2>,
    /// Runtime configuration.
    pub config: CoordinatorConfig,
    mission_id: Option<String>,
    mission_started_at: Option<Instant>,
    merged_outputs: Vec<String>,
    tasks_failed: usize,
}

impl Coordinator {
    /// Creates a new Coordinator v2 and pre-registers worker slots.
    pub fn new(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Result<Self> {
        if config.max_workers == 0 {
            return Err(CoordinatorV2Error::InvalidConfig(
                "max_workers must be at least 1".to_string(),
            ));
        }

        let registry = WorkerRegistry::new();
        for idx in 0..config.max_workers {
            registry.register_worker(WorkerInfo::new(format!("worker-{}", idx + 1)));
        }

        Ok(Self {
            worker_registry: registry,
            task_queue: TaskQueueIntegration::new(),
            dispatcher: Dispatcher::new(),
            blackboard,
            config,
            mission_id: None,
            mission_started_at: None,
            merged_outputs: Vec::new(),
            tasks_failed: 0,
        })
    }

    /// Executes a mission using decompose → dispatch → monitor → merge.
    pub async fn execute_mission(&mut self, mission: &str) -> Result<MissionResult> {
        let mission_id = Uuid::new_v4().to_string();
        self.mission_id = Some(mission_id.clone());
        self.mission_started_at = Some(Instant::now());
        self.merged_outputs.clear();
        self.tasks_failed = 0;

        let decomposed = MissionDecomposer::new()
            .decompose_async(mission)
            .await
            .map_err(|error| CoordinatorV2Error::Decomposition(error.to_string()))?;
        self.task_queue.load_tasks(&decomposed)?;

        while !self.task_queue.is_complete() {
            if self.config.enable_monitoring {
                let mut dispatch_workers = self.dispatch_workers_snapshot();
                let report = self
                    .dispatcher
                    .monitor_workers(
                        &mut dispatch_workers,
                        self.config.dispatch_interval.saturating_mul(3),
                    )
                    .await;
                self.apply_monitor_report(&report, &dispatch_workers);
            }

            let Some(_) = self.get_available_worker() else {
                sleep(self.config.dispatch_interval).await;
                continue;
            };

            let Some(task) = self.task_queue.get_next_dispatchable_task() else {
                if self.dispatcher.active_assignment_count() == 0 {
                    return Err(CoordinatorV2Error::StalledMission(
                        "no dispatchable tasks and no active assignments".to_string(),
                    ));
                }
                sleep(self.config.dispatch_interval).await;
                continue;
            };

            let assigned_worker = self.dispatch_task(task.clone()).await?;
            let completion = DispatchCompletion::success(
                task.id.clone(),
                assigned_worker.clone(),
                format!("Completed: {}", task.description),
            );
            self.dispatcher.submit_completion(completion).await;

            let completion_report = self.handle_dispatcher_completions().await?;
            if completion_report.completed == 0 && completion_report.failed == 0 {
                return Err(CoordinatorV2Error::StalledMission(
                    "dispatcher did not yield a completion".to_string(),
                ));
            }
        }

        let duration = self
            .mission_started_at
            .map(|started| started.elapsed())
            .unwrap_or(Duration::ZERO);
        let tasks_completed = self.task_queue.completed_tasks();

        Ok(MissionResult {
            mission_id,
            success: self.tasks_failed == 0,
            output: self.merged_outputs.join("\n"),
            tasks_completed,
            tasks_failed: self.tasks_failed,
            duration,
        })
    }

    /// Returns the next idle worker, if one exists.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.worker_registry.get_available_worker()
    }

    /// Assigns a task to a worker in the registry.
    pub fn assign_task(&mut self, worker_id: WorkerId, task: Task) -> Result<()> {
        let inner: CoreResult<()> = self.worker_registry.assign_task(&worker_id, &task);
        inner?;
        Ok(())
    }

    /// Returns the current mission status.
    pub fn get_mission_status(&self) -> MissionStatus {
        let completed = self.task_queue.completed_tasks();
        let total = self.task_queue.total_tasks();
        let progress = if total == 0 {
            0.0
        } else {
            (completed as f32 / total as f32) * 100.0
        };

        let eta = self
            .mission_started_at
            .and_then(|started| estimate_eta(started.elapsed(), progress));

        let active_workers = self
            .worker_registry
            .workers
            .iter()
            .fold(0usize, |count, entry| {
                if matches!(entry.value().status, WorkerStatus::Busy(_)) {
                    count + 1
                } else {
                    count
                }
            });

        MissionStatus {
            mission_id: self
                .mission_id
                .clone()
                .unwrap_or_else(|| "uninitialized".to_string()),
            progress,
            eta,
            active_workers,
            completed_tasks: completed,
            total_tasks: total,
        }
    }

    async fn dispatch_task(&mut self, task: Task) -> Result<WorkerId> {
        let mut dispatch_task = task.clone();
        dispatch_task.status = TaskStatus::Pending;
        dispatch_task.assigned_to = None;

        let mut dispatch_workers = self.dispatch_workers_snapshot();
        let report = self
            .dispatcher
            .dispatch_loop(
                std::slice::from_mut(&mut dispatch_task),
                &mut dispatch_workers,
            )
            .await?;

        if report.dispatched == 0 {
            return Err(CoordinatorV2Error::NoAvailableWorker);
        }

        let worker_id = self
            .dispatcher
            .assigned_worker(&dispatch_task.id)
            .ok_or(CoordinatorV2Error::NoAvailableWorker)?;

        self.assign_task(worker_id.clone(), task)?;
        self.sync_workers_from_dispatcher(&dispatch_workers);
        Ok(worker_id)
    }

    async fn handle_dispatcher_completions(&mut self) -> Result<CompletionReport> {
        let mut workers = self.dispatch_workers_snapshot();
        let mut tasks = self.current_assigned_tasks();
        let report = self
            .dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await?;

        for task in tasks {
            match task.status {
                TaskStatus::Completed => {
                    self.task_queue.mark_task_complete(&task.id)?;
                    self.publish_completion(&task.id, &task.description, true, None)
                        .await;
                    self.merged_outputs
                        .push(format!("{}: completed", task.description));
                }
                TaskStatus::Failed => {
                    self.task_queue.mark_task_complete(&task.id)?;
                    self.tasks_failed += 1;
                    self.publish_completion(&task.id, &task.description, false, Some("failed"))
                        .await;
                }
                _ => {}
            }
        }

        self.sync_workers_from_dispatcher(&workers);
        Ok(report)
    }

    fn current_assigned_tasks(&self) -> Vec<Task> {
        let mut tasks = Vec::new();

        for entry in &self.worker_registry.workers {
            let worker = entry.value();
            let Some(task_id) = worker.current_task.as_ref() else {
                continue;
            };

            if let Some(task) = self.task_queue.get_task(task_id).cloned() {
                tasks.push(task);
            }
        }

        tasks
    }

    fn dispatch_workers_snapshot(&self) -> Vec<DispatchWorker> {
        let mut workers = Vec::new();

        for entry in &self.worker_registry.workers {
            let worker = entry.value();
            workers.push(DispatchWorker {
                id: worker.id.clone(),
                status: to_dispatch_status(&worker.status),
                current_task: worker.current_task.clone(),
                last_heartbeat: worker.last_heartbeat,
            });
        }

        workers
    }

    fn sync_workers_from_dispatcher(&self, workers: &[DispatchWorker]) {
        for dispatch_worker in workers {
            if let Some(mut worker) = self.worker_registry.workers.get_mut(&dispatch_worker.id) {
                worker.current_task = dispatch_worker.current_task.clone();
                worker.last_heartbeat = dispatch_worker.last_heartbeat;
                worker.status = match dispatch_worker.status {
                    DispatchWorkerStatus::Idle => WorkerStatus::Idle,
                    DispatchWorkerStatus::Busy => WorkerStatus::Busy(
                        dispatch_worker
                            .current_task
                            .clone()
                            .unwrap_or_else(|| "unknown-task".to_string()),
                    ),
                    DispatchWorkerStatus::Unhealthy => WorkerStatus::Unhealthy {
                        reason: "dispatcher marked worker unhealthy".to_string(),
                    },
                    DispatchWorkerStatus::Failed => WorkerStatus::Failed {
                        error: "dispatcher recorded worker failure".to_string(),
                    },
                };
            }
        }
    }

    fn apply_monitor_report(
        &self,
        _report: &self::v2_dispatcher::MonitorReport,
        workers: &[DispatchWorker],
    ) {
        self.sync_workers_from_dispatcher(workers);
    }

    async fn publish_completion(
        &self,
        task_id: &str,
        description: &str,
        success: bool,
        error: Option<&str>,
    ) {
        let mut blackboard = self.blackboard.lock().await;
        let now = now_secs();
        let content = if success {
            format!("Task {} completed: {}", task_id, description)
        } else {
            format!(
                "Task {} failed: {} ({})",
                task_id,
                description,
                error.unwrap_or("unknown error")
            )
        };

        blackboard.publish(
            MemoryEntry {
                id: format!("mission-result-{}", task_id),
                content,
                memory_type: MemoryType::Shared,
                importance: if success { 0.8 } else { 1.0 },
                access_count: 0,
                created_at: now,
                last_accessed: now,
                expires_at: None,
            },
            vec!["coordinator".to_string()],
        );
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn estimate_eta(elapsed: Duration, progress: f32) -> Option<Duration> {
    if progress <= 0.0 || progress >= 100.0 {
        return None;
    }

    let elapsed_secs = elapsed.as_secs_f32();
    let remaining_ratio = (100.0 - progress) / progress;
    Some(Duration::from_secs_f32(elapsed_secs * remaining_ratio))
}

fn to_dispatch_status(status: &WorkerStatus) -> DispatchWorkerStatus {
    match status {
        WorkerStatus::Idle => DispatchWorkerStatus::Idle,
        WorkerStatus::Busy(_) => DispatchWorkerStatus::Busy,
        WorkerStatus::Unhealthy { .. } => DispatchWorkerStatus::Unhealthy,
        WorkerStatus::Failed { .. } | WorkerStatus::Terminated => DispatchWorkerStatus::Failed,
    }
}

#[cfg(test)]
mod tests {
    use super::{BlackboardV2, Coordinator, CoordinatorConfig};
    use std::sync::Arc;

    #[tokio::test]
    async fn new_registers_configured_workers() {
        let coordinator = Coordinator::new(
            CoordinatorConfig {
                max_workers: 3,
                ..CoordinatorConfig::default()
            },
            Arc::new(BlackboardV2::default()),
        )
        .unwrap();

        assert_eq!(coordinator.worker_registry.len(), 3);
        assert!(coordinator.get_available_worker().is_some());
    }

    #[tokio::test]
    async fn execute_mission_runs_simple_workflow() {
        let mut coordinator = Coordinator::new(
            CoordinatorConfig::default(),
            Arc::new(BlackboardV2::default()),
        )
        .unwrap();

        let result = coordinator.execute_mission("simple task").await.unwrap();

        assert!(result.success);
        assert!(result.tasks_completed >= 1);
        assert_eq!(result.tasks_failed, 0);
        assert!(result.output.contains("completed"));
    }

    #[tokio::test]
    async fn status_reaches_full_progress_after_execution() {
        let mut coordinator = Coordinator::new(
            CoordinatorConfig::default(),
            Arc::new(BlackboardV2::default()),
        )
        .unwrap();

        let result = coordinator.execute_mission("simple task").await.unwrap();
        let status = coordinator.get_mission_status();

        assert_eq!(status.mission_id, result.mission_id);
        assert_eq!(status.progress, 100.0);
        assert_eq!(status.completed_tasks, result.tasks_completed);
    }
}
