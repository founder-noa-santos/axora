//! Core worker-management primitives for the v2 coordinator.
//!
//! This module intentionally focuses on worker orchestration state so the
//! top-level `v2` coordinator can compose queue integration and dispatch logic
//! around it without duplicating registry behavior.

use crate::task::Task;
use crate::worker_pool::{WorkerId, WorkerStatus};
use dashmap::DashMap;
use std::time::Instant;
use thiserror::Error;

/// Result type for coordinator core operations.
pub type Result<T> = std::result::Result<T, CoordinatorCoreError>;

/// Errors produced by the coordinator core.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoordinatorCoreError {
    /// The requested worker does not exist.
    #[error("worker {0} not found")]
    WorkerNotFound(String),

    /// The requested worker is not idle and cannot accept a task.
    #[error("worker {0} is not available")]
    WorkerUnavailable(String),
}

/// Snapshot of runtime worker state tracked by the coordinator.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Stable worker identifier.
    pub id: WorkerId,
    /// Current worker status.
    pub status: WorkerStatus,
    /// Current task, if any.
    pub current_task: Option<String>,
    /// Last heartbeat received from the worker.
    pub last_heartbeat: Instant,
}

impl WorkerInfo {
    /// Creates a new idle worker record.
    pub fn new(id: impl Into<WorkerId>) -> Self {
        Self {
            id: id.into(),
            status: WorkerStatus::Idle,
            current_task: None,
            last_heartbeat: Instant::now(),
        }
    }

    /// Returns true when the worker is ready to accept work.
    pub fn is_available(&self) -> bool {
        matches!(self.status, WorkerStatus::Idle)
    }
}

/// Concurrent worker registry used by the coordinator.
#[derive(Debug, Default)]
pub struct WorkerRegistry {
    /// Known workers indexed by id.
    pub workers: DashMap<WorkerId, WorkerInfo>,
}

impl WorkerRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a worker as available for future assignment.
    pub fn register_worker(&self, worker: WorkerInfo) {
        self.workers.insert(worker.id.clone(), worker);
    }

    /// Removes a worker from the registry.
    pub fn remove_worker(&self, worker_id: &str) -> Option<WorkerInfo> {
        self.workers.remove(worker_id).map(|(_, worker)| worker)
    }

    /// Returns the next idle worker, if any.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.workers.iter().find_map(|entry| {
            let worker = entry.value();
            if worker.is_available() {
                Some(worker.id.clone())
            } else {
                None
            }
        })
    }

    /// Assigns a task to an idle worker.
    pub fn assign_task(&self, worker_id: &str, task: &Task) -> Result<()> {
        let mut worker = self
            .workers
            .get_mut(worker_id)
            .ok_or_else(|| CoordinatorCoreError::WorkerNotFound(worker_id.to_string()))?;

        if !worker.is_available() {
            return Err(CoordinatorCoreError::WorkerUnavailable(
                worker_id.to_string(),
            ));
        }

        worker.current_task = Some(task.id.clone());
        worker.status = WorkerStatus::Busy(task.id.clone());
        worker.last_heartbeat = Instant::now();
        Ok(())
    }

    /// Marks a worker idle again after task completion.
    pub fn mark_worker_idle(&self, worker_id: &str) -> Result<()> {
        let mut worker = self
            .workers
            .get_mut(worker_id)
            .ok_or_else(|| CoordinatorCoreError::WorkerNotFound(worker_id.to_string()))?;

        worker.current_task = None;
        worker.status = WorkerStatus::Idle;
        worker.last_heartbeat = Instant::now();
        Ok(())
    }

    /// Updates the heartbeat for a worker.
    pub fn touch_heartbeat(&self, worker_id: &str) -> Result<()> {
        let mut worker = self
            .workers
            .get_mut(worker_id)
            .ok_or_else(|| CoordinatorCoreError::WorkerNotFound(worker_id.to_string()))?;
        worker.last_heartbeat = Instant::now();
        Ok(())
    }

    /// Returns the number of workers currently tracked.
    pub fn len(&self) -> usize {
        self.workers.len()
    }

    /// Returns true when the registry has no workers.
    pub fn is_empty(&self) -> bool {
        self.workers.is_empty()
    }
}

/// Core coordinator wrapper around the worker registry.
#[derive(Debug, Default)]
pub struct Coordinator {
    /// Registry of workers available to the coordinator.
    pub worker_registry: WorkerRegistry,
}

impl Coordinator {
    /// Creates a coordinator with an empty worker registry.
    pub fn new() -> Self {
        Self {
            worker_registry: WorkerRegistry::new(),
        }
    }

    /// Registers a worker with the coordinator.
    pub fn register_worker(&self, worker: WorkerInfo) {
        self.worker_registry.register_worker(worker);
    }

    /// Returns the next idle worker, if one exists.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.worker_registry.get_available_worker()
    }

    /// Assigns a task to the specified worker.
    pub fn assign_task(&self, worker_id: &str, task: &Task) -> Result<()> {
        self.worker_registry.assign_task(worker_id, task)
    }
}

#[cfg(test)]
mod tests {
    use super::{Coordinator, CoordinatorCoreError, WorkerInfo, WorkerRegistry};
    use crate::task::Task;
    use crate::worker_pool::WorkerStatus;

    #[test]
    fn registry_tracks_registered_workers() {
        let registry = WorkerRegistry::new();
        registry.register_worker(WorkerInfo::new("worker-1"));
        registry.register_worker(WorkerInfo::new("worker-2"));

        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
    }

    #[test]
    fn get_available_worker_returns_idle_worker() {
        let registry = WorkerRegistry::new();
        registry.register_worker(WorkerInfo::new("worker-1"));

        assert_eq!(
            registry.get_available_worker(),
            Some("worker-1".to_string())
        );
    }

    #[test]
    fn get_available_worker_skips_busy_workers() {
        let registry = WorkerRegistry::new();
        registry.register_worker(WorkerInfo {
            id: "worker-1".to_string(),
            status: WorkerStatus::Busy("task-1".to_string()),
            current_task: Some("task-1".to_string()),
            last_heartbeat: std::time::Instant::now(),
        });
        registry.register_worker(WorkerInfo::new("worker-2"));

        assert_eq!(
            registry.get_available_worker(),
            Some("worker-2".to_string())
        );
    }

    #[test]
    fn assign_task_marks_worker_busy() {
        let registry = WorkerRegistry::new();
        registry.register_worker(WorkerInfo::new("worker-1"));
        let task = Task::new("implement coordinator");

        registry.assign_task("worker-1", &task).unwrap();

        let worker = registry.workers.get("worker-1").unwrap();
        assert_eq!(worker.current_task.as_deref(), Some(task.id.as_str()));
        assert_eq!(worker.status, WorkerStatus::Busy(task.id.clone()));
    }

    #[test]
    fn assign_task_rejects_unknown_worker() {
        let registry = WorkerRegistry::new();
        let task = Task::new("implement coordinator");

        let error = registry.assign_task("missing-worker", &task).unwrap_err();
        assert_eq!(
            error,
            CoordinatorCoreError::WorkerNotFound("missing-worker".to_string())
        );
    }

    #[test]
    fn assign_task_rejects_unavailable_worker() {
        let registry = WorkerRegistry::new();
        registry.register_worker(WorkerInfo {
            id: "worker-1".to_string(),
            status: WorkerStatus::Failed {
                error: "panic".to_string(),
            },
            current_task: Some("task-1".to_string()),
            last_heartbeat: std::time::Instant::now(),
        });
        let task = Task::new("retry failed worker");

        let error = registry.assign_task("worker-1", &task).unwrap_err();
        assert_eq!(
            error,
            CoordinatorCoreError::WorkerUnavailable("worker-1".to_string())
        );
    }

    #[test]
    fn mark_worker_idle_clears_task_assignment() {
        let registry = WorkerRegistry::new();
        let task = Task::new("finish coordinator");
        registry.register_worker(WorkerInfo::new("worker-1"));
        registry.assign_task("worker-1", &task).unwrap();

        registry.mark_worker_idle("worker-1").unwrap();

        let worker = registry.workers.get("worker-1").unwrap();
        assert_eq!(worker.status, WorkerStatus::Idle);
        assert!(worker.current_task.is_none());
    }

    #[test]
    fn coordinator_delegates_registry_operations() {
        let coordinator = Coordinator::new();
        coordinator.register_worker(WorkerInfo::new("worker-1"));
        let task = Task::new("dispatch task");

        let available = coordinator.get_available_worker();
        coordinator
            .assign_task(available.as_deref().unwrap(), &task)
            .unwrap();

        let worker = coordinator.worker_registry.workers.get("worker-1").unwrap();
        assert_eq!(worker.status, WorkerStatus::Busy(task.id.clone()));
    }
}
