//! Priority-aware task queue with dependency tracking and load balancing.
//!
//! ```rust
//! use axora_agents::{Task, TaskQueue, TaskQueueConfig};
//!
//! let queue = TaskQueue::new(TaskQueueConfig::default());
//! let root = queue.add_task(Task::new("root"), 90, vec![]).unwrap();
//! let child = queue.add_task(Task::new("child"), 80, vec![root.clone()]).unwrap();
//! assert_eq!(queue.get_next_ready_task().unwrap().task_id, root);
//! queue.mark_completed(&root).unwrap();
//! assert_eq!(queue.get_next_ready_task().unwrap().task_id, child);
//! ```

mod dependency_tracker;
mod load_balancer;
mod priority_scheduler;

use crate::task::Task;
use crate::worker_pool::WorkerId;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use thiserror::Error;

pub use dependency_tracker::{DependencyTracker, DependencyTrackerError};
pub use load_balancer::{LoadBalancer, LoadBalancingTask, WorkerAssignment};
pub use priority_scheduler::{PriorityScheduler, PrioritySchedulerError};

/// Task identifier used by the runtime queue.
pub type QueueTaskId = String;

/// Result type for task-queue operations.
pub type Result<T> = std::result::Result<T, TaskQueueError>;

const RESERVED_WORKER_ID: &str = "__reserved__";

/// Queue-level task state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskQueueStatus {
    /// Waiting for dependencies to complete.
    Pending,
    /// Ready to be dispatched.
    Ready,
    /// Reserved or running on a worker.
    InProgress {
        /// Worker responsible for the task.
        worker_id: WorkerId,
    },
    /// Task completed successfully.
    Completed,
    /// Task execution failed.
    Failed {
        /// Failure reason.
        error: String,
    },
    /// Task cannot currently run.
    Blocked {
        /// Blocking reason.
        reason: String,
    },
}

/// Task snapshot stored in the queue.
#[derive(Debug, Clone)]
pub struct QueuedTask {
    /// Task identifier.
    pub task_id: QueueTaskId,
    /// Full task payload.
    pub task: Task,
    /// Queue-specific numeric priority.
    pub priority: u8,
    /// Dependency identifiers.
    pub dependencies: Vec<QueueTaskId>,
    /// Current queue state.
    pub status: TaskQueueStatus,
    /// Time when the task entered the queue.
    pub added_at: Instant,
    /// Length of the task's current critical path.
    pub critical_path_length: usize,
}

/// Task-queue configuration.
#[derive(Debug, Clone)]
pub struct TaskQueueConfig {
    /// Maximum number of tasks accepted by the queue.
    pub max_queue_size: usize,
    /// Default priority used by helper flows.
    pub default_priority: u8,
    /// Whether to compute load-balancing hints.
    pub enable_load_balancing: bool,
}

impl Default for TaskQueueConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            default_priority: 50,
            enable_load_balancing: true,
        }
    }
}

/// Queue statistics snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueStats {
    /// Total tasks known to the queue.
    pub total_tasks: usize,
    /// Tasks waiting on dependencies.
    pub pending_tasks: usize,
    /// Tasks ready for dispatch.
    pub ready_tasks: usize,
    /// Tasks reserved or running.
    pub in_progress_tasks: usize,
    /// Tasks completed successfully.
    pub completed_tasks: usize,
    /// Tasks that failed.
    pub failed_tasks: usize,
    /// Average waiting time across unfinished tasks.
    pub avg_wait_time: Duration,
    /// Estimated completion time based on the critical path.
    pub estimated_completion: Duration,
}

/// Errors returned by the queue runtime.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TaskQueueError {
    /// The queue reached its configured capacity.
    #[error("task queue is at capacity")]
    QueueFull,
    /// The task already exists in the queue.
    #[error("duplicate task id: {0}")]
    DuplicateTaskId(QueueTaskId),
    /// Priority values must stay in range.
    #[error("invalid priority: {0}")]
    InvalidPriority(u8),
    /// A dependency was not found in the queue.
    #[error("dependency not found: {0}")]
    DependencyMissing(QueueTaskId),
    /// The task could not be located.
    #[error("task not found: {0}")]
    TaskNotFound(QueueTaskId),
    /// The dependency graph would become cyclic.
    #[error(transparent)]
    CircularDependency(#[from] DependencyTrackerError),
}

/// Unified queue composed from scheduler, dependency tracker, and load balancer.
pub struct TaskQueue {
    scheduler: RwLock<PriorityScheduler>,
    dependency_tracker: RwLock<DependencyTracker>,
    load_balancer: LoadBalancer,
    tasks: DashMap<QueueTaskId, QueuedTask>,
    config: TaskQueueConfig,
}

impl std::fmt::Debug for TaskQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskQueue")
            .field("task_count", &self.tasks.len())
            .field("config", &self.config)
            .finish()
    }
}

impl TaskQueue {
    /// Create a new task queue.
    pub fn new(config: TaskQueueConfig) -> Self {
        Self {
            scheduler: RwLock::new(PriorityScheduler::new()),
            dependency_tracker: RwLock::new(DependencyTracker::new()),
            load_balancer: LoadBalancer::new(),
            tasks: DashMap::new(),
            config,
        }
    }

    /// Add a task to the queue with a numeric priority and dependencies.
    pub fn add_task(
        &self,
        task: Task,
        priority: u8,
        dependencies: Vec<QueueTaskId>,
    ) -> Result<QueueTaskId> {
        if self.tasks.len() >= self.config.max_queue_size {
            return Err(TaskQueueError::QueueFull);
        }
        if priority > 100 {
            return Err(TaskQueueError::InvalidPriority(priority));
        }

        let task_id = task.id.clone();
        if self.tasks.contains_key(&task_id) {
            return Err(TaskQueueError::DuplicateTaskId(task_id));
        }

        for dependency in &dependencies {
            if !self.tasks.contains_key(dependency) {
                return Err(TaskQueueError::DependencyMissing(dependency.clone()));
            }
        }

        {
            let mut tracker = self.dependency_tracker.write();
            tracker.register_task(task_id.clone());
            for dependency in &dependencies {
                tracker.add_dependency(task_id.clone(), dependency.clone())?;
            }
        }

        let status = if dependencies.is_empty() {
            TaskQueueStatus::Ready
        } else {
            TaskQueueStatus::Pending
        };

        self.tasks.insert(
            task_id.clone(),
            QueuedTask {
                task_id: task_id.clone(),
                task,
                priority,
                dependencies,
                status,
                added_at: Instant::now(),
                critical_path_length: 1,
            },
        );

        self.refresh_critical_path_lengths();
        self.refresh_ready_tasks()?;
        Ok(task_id)
    }

    /// Return and reserve the next ready task according to priority and dependency state.
    pub fn get_next_ready_task(&self) -> Option<QueuedTask> {
        self.refresh_ready_tasks().ok()?;
        let task_id = self.scheduler.write().get_next_task()?;
        let mut task = self.tasks.get_mut(&task_id)?;
        task.status = TaskQueueStatus::InProgress {
            worker_id: RESERVED_WORKER_ID.to_string(),
        };
        Some(task.clone())
    }

    /// Mark a task complete and unblock any newly ready dependents.
    pub fn mark_completed(&self, task_id: impl AsRef<str>) -> Result<()> {
        let task_id = task_id.as_ref().to_string();
        let mut task = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskQueueError::TaskNotFound(task_id.clone()))?;
        task.status = TaskQueueStatus::Completed;
        drop(task);

        self.scheduler.write().remove_task(&task_id);
        self.dependency_tracker.write().mark_completed(&task_id);
        self.refresh_ready_tasks()?;
        Ok(())
    }

    /// Mark a task failed.
    pub fn mark_failed(&self, task_id: impl AsRef<str>, error: String) -> Result<()> {
        let task_id = task_id.as_ref().to_string();
        let mut task = self
            .tasks
            .get_mut(&task_id)
            .ok_or_else(|| TaskQueueError::TaskNotFound(task_id.clone()))?;
        task.status = TaskQueueStatus::Failed { error };
        self.scheduler.write().remove_task(&task_id);
        Ok(())
    }

    /// Return the current queue statistics.
    pub fn get_queue_stats(&self) -> QueueStats {
        let mut stats = QueueStats {
            total_tasks: 0,
            pending_tasks: 0,
            ready_tasks: 0,
            in_progress_tasks: 0,
            completed_tasks: 0,
            failed_tasks: 0,
            avg_wait_time: Duration::default(),
            estimated_completion: self
                .load_balancer
                .estimate_completion_time(&self.load_balancing_tasks()),
        };
        let mut total_wait = Duration::default();
        let mut waiting = 0usize;

        for entry in self.tasks.iter() {
            stats.total_tasks += 1;
            match &entry.status {
                TaskQueueStatus::Pending | TaskQueueStatus::Blocked { .. } => {
                    stats.pending_tasks += 1;
                    total_wait += entry.added_at.elapsed();
                    waiting += 1;
                }
                TaskQueueStatus::Ready => {
                    stats.ready_tasks += 1;
                    total_wait += entry.added_at.elapsed();
                    waiting += 1;
                }
                TaskQueueStatus::InProgress { .. } => {
                    stats.in_progress_tasks += 1;
                    total_wait += entry.added_at.elapsed();
                    waiting += 1;
                }
                TaskQueueStatus::Completed => stats.completed_tasks += 1,
                TaskQueueStatus::Failed { .. } => stats.failed_tasks += 1,
            }
        }

        if waiting > 0 {
            stats.avg_wait_time = total_wait / waiting as u32;
        }
        stats
    }

    /// Return the current critical path.
    pub fn get_critical_path(&self) -> Vec<QueueTaskId> {
        self.load_balancer
            .calculate_critical_path(&self.load_balancing_tasks())
    }

    /// Produce current worker assignments for the provided workers.
    pub fn balance_load(&self, workers: &[WorkerId]) -> Vec<WorkerAssignment> {
        if !self.config.enable_load_balancing {
            return workers
                .iter()
                .cloned()
                .map(|worker_id| WorkerAssignment {
                    worker_id,
                    task_ids: Vec::new(),
                })
                .collect();
        }

        let ready = self
            .tasks
            .iter()
            .filter(|entry| matches!(entry.status, TaskQueueStatus::Ready))
            .map(|entry| LoadBalancingTask {
                task_id: entry.task_id.clone(),
                dependencies: entry.dependencies.clone(),
                priority: entry.priority,
                critical_path_length: entry.critical_path_length,
            })
            .collect::<Vec<_>>();

        self.load_balancer.balance_load(workers, &ready)
    }

    fn refresh_ready_tasks(&self) -> Result<()> {
        let ready = self.dependency_tracker.read().get_ready_tasks();
        let priorities = self
            .tasks
            .iter()
            .map(|entry| (entry.task_id.clone(), entry.priority))
            .collect::<HashMap<_, _>>();

        {
            let mut scheduler = self.scheduler.write();
            for task_id in ready {
                if let Some(mut task) = self.tasks.get_mut(&task_id) {
                    match &task.status {
                        TaskQueueStatus::Pending | TaskQueueStatus::Blocked { .. } => {
                            task.status = TaskQueueStatus::Ready;
                            scheduler.add_task(task_id.clone(), task.priority).map_err(|error| {
                                match error {
                                    PrioritySchedulerError::InvalidPriority(priority) => {
                                        TaskQueueError::InvalidPriority(priority)
                                    }
                                }
                            })?;
                        }
                        TaskQueueStatus::Ready => {
                            if !scheduler.contains(&task_id) {
                                scheduler.add_task(task_id.clone(), task.priority).map_err(|error| {
                                    match error {
                                        PrioritySchedulerError::InvalidPriority(priority) => {
                                            TaskQueueError::InvalidPriority(priority)
                                        }
                                    }
                                })?;
                            }
                        }
                        TaskQueueStatus::InProgress { .. }
                        | TaskQueueStatus::Completed
                        | TaskQueueStatus::Failed { .. } => {}
                    }
                }
            }
            scheduler.reorder_queue(&priorities).map_err(|error| match error {
                PrioritySchedulerError::InvalidPriority(priority) => {
                    TaskQueueError::InvalidPriority(priority)
                }
            })?;
        }

        for mut entry in self.tasks.iter_mut() {
            if matches!(entry.status, TaskQueueStatus::Pending)
                && !self.dependency_tracker.read().is_ready(&entry.task_id)
            {
                entry.status = TaskQueueStatus::Blocked {
                    reason: "waiting for dependencies".to_string(),
                };
            }
        }

        Ok(())
    }

    fn load_balancing_tasks(&self) -> Vec<LoadBalancingTask> {
        self.tasks
            .iter()
            .map(|entry| LoadBalancingTask {
                task_id: entry.task_id.clone(),
                dependencies: entry.dependencies.clone(),
                priority: entry.priority,
                critical_path_length: entry.critical_path_length,
            })
            .collect()
    }

    fn refresh_critical_path_lengths(&self) {
        let path = self.get_critical_path();
        let path_positions = path
            .iter()
            .enumerate()
            .map(|(index, task_id)| (task_id.clone(), path.len() - index))
            .collect::<HashMap<_, _>>();

        for mut entry in self.tasks.iter_mut() {
            entry.critical_path_length = path_positions.get(&entry.task_id).copied().unwrap_or(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> TaskQueue {
        TaskQueue::new(TaskQueueConfig::default())
    }

    fn task(description: &str) -> Task {
        Task::new(description)
    }

    #[test]
    fn add_task_rejects_missing_dependency() {
        let queue = queue();
        let error = queue
            .add_task(task("child"), 50, vec!["missing".to_string()])
            .unwrap_err();

        assert_eq!(error, TaskQueueError::DependencyMissing("missing".to_string()));
    }

    #[test]
    fn get_next_ready_task_prefers_high_priority_ready_task() {
        let queue = queue();
        let low = queue.add_task(task("low"), 10, vec![]).unwrap();
        let high = queue.add_task(task("high"), 90, vec![]).unwrap();

        assert_eq!(queue.get_next_ready_task().unwrap().task_id, high);
        assert_eq!(queue.get_next_ready_task().unwrap().task_id, low);
    }

    #[test]
    fn dependency_resolution_unblocks_child_after_completion() {
        let queue = queue();
        let parent = queue.add_task(task("parent"), 60, vec![]).unwrap();
        let child = queue.add_task(task("child"), 90, vec![parent.clone()]).unwrap();

        assert_eq!(queue.get_next_ready_task().unwrap().task_id, parent);
        queue.mark_completed(&parent).unwrap();
        assert_eq!(queue.get_next_ready_task().unwrap().task_id, child);
    }

    #[test]
    fn dependent_tasks_start_blocked_until_unblocked() {
        let queue = queue();
        let root = queue.add_task(task("root"), 80, vec![]).unwrap();
        let child = queue.add_task(task("child"), 70, vec![root.clone()]).unwrap();

        let child = queue.tasks.get(&child).unwrap();
        assert!(matches!(child.status, TaskQueueStatus::Blocked { .. }));
    }

    #[test]
    fn mark_failed_removes_task_from_scheduler() {
        let queue = queue();
        let task_id = queue.add_task(task("will fail"), 50, vec![]).unwrap();

        queue.mark_failed(&task_id, "boom".to_string()).unwrap();

        assert!(queue.get_next_ready_task().is_none());
    }

    #[test]
    fn critical_path_tracks_longest_dependency_chain() {
        let queue = queue();
        let a = queue.add_task(task("a"), 10, vec![]).unwrap();
        let b = queue.add_task(task("b"), 20, vec![a.clone()]).unwrap();
        let c = queue.add_task(task("c"), 30, vec![b.clone()]).unwrap();
        let d = queue.add_task(task("d"), 40, vec![a.clone()]).unwrap();

        assert_eq!(queue.get_critical_path(), vec![a, b, c]);
        assert!(!queue.get_critical_path().contains(&d));
    }

    #[test]
    fn load_balancing_distributes_ready_tasks_evenly() {
        let queue = queue();
        queue.add_task(task("a"), 80, vec![]).unwrap();
        queue.add_task(task("b"), 70, vec![]).unwrap();
        queue.add_task(task("c"), 60, vec![]).unwrap();
        queue.add_task(task("d"), 50, vec![]).unwrap();

        let assignments = queue.balance_load(&["w1".to_string(), "w2".to_string()]);

        assert_eq!(assignments.len(), 2);
        assert_eq!(
            assignments.iter().map(|assignment| assignment.task_ids.len()).sum::<usize>(),
            4
        );
        let difference = assignments[0]
            .task_ids
            .len()
            .abs_diff(assignments[1].task_ids.len());
        assert!(difference <= 1);
    }

    #[test]
    fn queue_stats_count_task_states() {
        let queue = queue();
        let ready = queue.add_task(task("ready"), 80, vec![]).unwrap();
        let blocked_parent = queue.add_task(task("parent"), 70, vec![]).unwrap();
        let child = queue
            .add_task(task("blocked"), 60, vec![blocked_parent.clone()])
            .unwrap();

        let reserved = queue.get_next_ready_task().unwrap();
        queue.mark_completed(&blocked_parent).unwrap();
        queue.mark_failed(&child, "failed".to_string()).unwrap();

        let stats = queue.get_queue_stats();

        assert_eq!(reserved.task_id, ready);
        assert_eq!(stats.total_tasks, 3);
        assert_eq!(stats.in_progress_tasks, 1);
        assert_eq!(stats.completed_tasks, 1);
        assert_eq!(stats.failed_tasks, 1);
    }

    #[test]
    fn queue_enforces_max_capacity() {
        let queue = TaskQueue::new(TaskQueueConfig {
            max_queue_size: 1,
            ..TaskQueueConfig::default()
        });
        queue.add_task(task("one"), 50, vec![]).unwrap();

        let error = queue.add_task(task("two"), 50, vec![]).unwrap_err();
        assert_eq!(error, TaskQueueError::QueueFull);
    }

    #[test]
    fn queue_rejects_invalid_priority() {
        let queue = queue();
        let error = queue.add_task(task("bad"), 101, vec![]).unwrap_err();
        assert_eq!(error, TaskQueueError::InvalidPriority(101));
    }
}
