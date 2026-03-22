//! Basic dispatch loop for Coordinator v2.
//!
//! This module stays self-contained so a higher-level `v2.rs` can compose it
//! with queue and coordinator-core components without circular dependencies.

use crate::task::{Task, TaskStatus};
use crate::transport::InternalResultSubmission;
use dashmap::DashMap;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

/// Worker identifier used by the dispatcher.
pub type WorkerId = String;

/// Task identifier used by the dispatcher.
pub type TaskId = String;

/// Result type for dispatcher operations.
pub type Result<T> = std::result::Result<T, DispatcherError>;

/// Errors produced by the v2 dispatcher.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DispatcherError {
    /// A completion referenced a task the dispatcher is not tracking.
    #[error("task {0} was not found")]
    TaskNotFound(String),

    /// A completion referenced a worker the dispatcher is not tracking.
    #[error("worker {0} was not found")]
    WorkerNotFound(String),
}

/// Worker status snapshot used by the dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchWorkerStatus {
    /// Worker is ready for work.
    Idle,
    /// Worker currently has a task.
    Busy,
    /// Worker missed its heartbeat window.
    Unhealthy,
    /// Worker failed while handling a task.
    Failed,
}

/// Mutable worker state consumed by the dispatcher loop.
#[derive(Debug, Clone)]
pub struct DispatchWorker {
    /// Stable worker ID.
    pub id: WorkerId,
    /// Current health and availability status.
    pub status: DispatchWorkerStatus,
    /// Task currently assigned to the worker.
    pub current_task: Option<TaskId>,
    /// Last heartbeat time observed for the worker.
    pub last_heartbeat: Instant,
}

impl DispatchWorker {
    /// Create a new idle worker.
    pub fn idle(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: DispatchWorkerStatus::Idle,
            current_task: None,
            last_heartbeat: Instant::now(),
        }
    }
}

/// Completion payload submitted back to the dispatcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchCompletion {
    /// Task that finished.
    pub task_id: TaskId,
    /// Worker that finished the task.
    pub worker_id: WorkerId,
    /// Whether the task succeeded.
    pub success: bool,
    /// Summary returned by the worker on success.
    pub summary: String,
    /// Typed result submission when one exists.
    pub result_submission: Option<InternalResultSubmission>,
    /// Error returned by the worker on failure.
    pub error: Option<String>,
}

impl DispatchCompletion {
    /// Build a successful completion.
    pub fn success(
        task_id: impl Into<String>,
        worker_id: impl Into<String>,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            success: true,
            summary: summary.into(),
            result_submission: None,
            error: None,
        }
    }

    /// Build a successful completion with a typed result submission.
    pub fn success_with_result(
        task_id: impl Into<String>,
        worker_id: impl Into<String>,
        summary: impl Into<String>,
        result_submission: InternalResultSubmission,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            success: true,
            summary: summary.into(),
            result_submission: Some(result_submission),
            error: None,
        }
    }

    /// Build a failed completion.
    pub fn failure(
        task_id: impl Into<String>,
        worker_id: impl Into<String>,
        error: impl Into<String>,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            success: false,
            summary: String::new(),
            result_submission: None,
            error: Some(error.into()),
        }
    }

    /// Build a failed completion with a typed result submission.
    pub fn failure_with_result(
        task_id: impl Into<String>,
        worker_id: impl Into<String>,
        error: impl Into<String>,
        result_submission: InternalResultSubmission,
    ) -> Self {
        Self {
            task_id: task_id.into(),
            worker_id: worker_id.into(),
            success: false,
            summary: String::new(),
            result_submission: Some(result_submission),
            error: Some(error.into()),
        }
    }
}

/// Summary of a single dispatch pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DispatchLoopReport {
    /// Number of tasks dispatched to workers.
    pub dispatched: usize,
    /// Number of tasks left pending because no worker was available.
    pub skipped: usize,
}

/// Summary of worker monitoring.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MonitorReport {
    /// Workers marked unhealthy during the pass.
    pub unhealthy_workers: Vec<WorkerId>,
}

/// Summary of processed completions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompletionReport {
    /// Number of successful completions applied.
    pub completed: usize,
    /// Number of failed completions applied.
    pub failed: usize,
    /// Processed completions in application order.
    pub processed_completions: Vec<DispatchCompletion>,
}

/// Basic dispatcher for Coordinator v2.
#[derive(Debug, Clone, Default)]
pub struct Dispatcher {
    active_assignments: Arc<DashMap<TaskId, WorkerId>>,
    completions: Arc<Mutex<VecDeque<DispatchCompletion>>>,
}

impl Dispatcher {
    /// Create a new dispatcher.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a completion to be handled during the next completion pass.
    pub async fn submit_completion(&self, completion: DispatchCompletion) {
        self.completions.lock().await.push_back(completion);
    }

    #[cfg(test)]
    pub(crate) async fn pending_completions_len(&self) -> usize {
        self.completions.lock().await.len()
    }

    /// Dispatch pending tasks to idle workers.
    pub async fn dispatch_loop(
        &self,
        tasks: &mut [Task],
        workers: &mut [DispatchWorker],
    ) -> Result<DispatchLoopReport> {
        let mut report = DispatchLoopReport::default();

        for task in tasks
            .iter_mut()
            .filter(|task| task.status == TaskStatus::Pending)
        {
            let preferred_worker_id = task.assigned_to.clone();
            let preferred_index = preferred_worker_id
                .as_ref()
                .and_then(|preferred_worker_id| {
                    workers.iter().position(|worker| {
                        worker.status == DispatchWorkerStatus::Idle
                            && &worker.id == preferred_worker_id
                    })
                });
            let fallback_index = workers
                .iter()
                .position(|worker| worker.status == DispatchWorkerStatus::Idle);
            let Some(worker_index) = preferred_index.or(fallback_index) else {
                report.skipped += 1;
                continue;
            };
            let worker = &mut workers[worker_index];

            task.assign(&worker.id);
            task.start();

            worker.status = DispatchWorkerStatus::Busy;
            worker.current_task = Some(task.id.clone());
            worker.last_heartbeat = Instant::now();

            self.active_assignments
                .insert(task.id.clone(), worker.id.clone());

            report.dispatched += 1;
        }

        Ok(report)
    }

    /// Monitor worker heartbeats and flag stale busy workers as unhealthy.
    pub async fn monitor_workers(
        &self,
        workers: &mut [DispatchWorker],
        heartbeat_timeout: Duration,
    ) -> MonitorReport {
        let now = Instant::now();
        let mut report = MonitorReport::default();

        for worker in workers.iter_mut() {
            if worker.status == DispatchWorkerStatus::Busy
                && now.duration_since(worker.last_heartbeat) > heartbeat_timeout
            {
                worker.status = DispatchWorkerStatus::Unhealthy;
                report.unhealthy_workers.push(worker.id.clone());
            }
        }

        report
    }

    /// Apply queued completions, updating tasks and worker availability.
    pub async fn handle_completions(
        &self,
        tasks: &mut [Task],
        workers: &mut [DispatchWorker],
    ) -> Result<CompletionReport> {
        let mut queue = self.completions.lock().await;
        let mut report = CompletionReport::default();

        while !queue.is_empty() {
            let (task_id, worker_id) = {
                let front = queue.front().expect("queue non-empty");
                (front.task_id.clone(), front.worker_id.clone())
            };

            if !tasks.iter().any(|task| task.id == task_id) {
                return Err(DispatcherError::TaskNotFound(task_id));
            }
            if !workers.iter().any(|w| w.id == worker_id) {
                return Err(DispatcherError::WorkerNotFound(worker_id));
            }

            let completion = queue.pop_front().expect("queue non-empty");

            let task = tasks
                .iter_mut()
                .find(|task| task.id == completion.task_id)
                .expect("task validated above");
            let worker = workers
                .iter_mut()
                .find(|worker| worker.id == completion.worker_id)
                .expect("worker validated above");

            if completion.success {
                task.complete();
                worker.status = DispatchWorkerStatus::Idle;
                report.completed += 1;
            } else {
                task.fail();
                worker.status = DispatchWorkerStatus::Failed;
                report.failed += 1;
            }

            worker.current_task = None;
            worker.last_heartbeat = Instant::now();
            self.active_assignments.remove(&completion.task_id);
            report.processed_completions.push(completion);
        }

        Ok(report)
    }

    /// Return the currently assigned worker for a task, if any.
    pub fn assigned_worker(&self, task_id: &str) -> Option<WorkerId> {
        self.active_assignments
            .get(task_id)
            .map(|entry| entry.value().clone())
    }

    /// Return the number of active assignments tracked by the dispatcher.
    pub fn active_assignment_count(&self) -> usize {
        self.active_assignments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompletionReport, DispatchCompletion, DispatchWorker, DispatchWorkerStatus, Dispatcher,
        DispatcherError,
    };
    use crate::task::{Task, TaskStatus};
    use std::time::{Duration, Instant};

    fn task(description: &str) -> Task {
        Task::new(description)
    }

    fn worker(id: &str) -> DispatchWorker {
        DispatchWorker::idle(id)
    }

    #[tokio::test]
    async fn dispatch_loop_assigns_pending_tasks_to_idle_workers() {
        let dispatcher = Dispatcher::new();
        let mut tasks = vec![task("task-1"), task("task-2")];
        let mut workers = vec![worker("worker-1"), worker("worker-2")];

        let report = dispatcher
            .dispatch_loop(&mut tasks, &mut workers)
            .await
            .unwrap();

        assert_eq!(report.dispatched, 2);
        assert_eq!(report.skipped, 0);
        assert_eq!(tasks[0].status, TaskStatus::InProgress);
        assert_eq!(workers[0].status, DispatchWorkerStatus::Busy);
        assert_eq!(dispatcher.active_assignment_count(), 2);
    }

    #[tokio::test]
    async fn dispatch_loop_leaves_work_pending_when_no_worker_is_idle() {
        let dispatcher = Dispatcher::new();
        let mut tasks = vec![task("task-1")];
        let mut workers = vec![DispatchWorker {
            id: "worker-1".to_string(),
            status: DispatchWorkerStatus::Busy,
            current_task: Some("existing-task".to_string()),
            last_heartbeat: Instant::now(),
        }];

        let report = dispatcher
            .dispatch_loop(&mut tasks, &mut workers)
            .await
            .unwrap();

        assert_eq!(report.dispatched, 0);
        assert_eq!(report.skipped, 1);
        assert_eq!(tasks[0].status, TaskStatus::Pending);
        assert_eq!(dispatcher.active_assignment_count(), 0);
    }

    #[tokio::test]
    async fn dispatch_loop_prefers_assigned_worker_when_present() {
        let dispatcher = Dispatcher::new();
        let mut assigned_task = task("rename provider labels");
        assigned_task.assigned_to = Some("refactorer".to_string());
        let mut tasks = vec![assigned_task];
        let mut workers = vec![worker("coder"), worker("refactorer")];

        let report = dispatcher
            .dispatch_loop(&mut tasks, &mut workers)
            .await
            .unwrap();

        assert_eq!(report.dispatched, 1);
        assert_eq!(tasks[0].assigned_to.as_deref(), Some("refactorer"));
        assert_eq!(workers[1].status, DispatchWorkerStatus::Busy);
        assert_eq!(
            workers[1].current_task.as_deref(),
            Some(tasks[0].id.as_str())
        );
    }

    #[tokio::test]
    async fn monitor_workers_marks_stale_busy_workers_unhealthy() {
        let dispatcher = Dispatcher::new();
        let mut workers = vec![DispatchWorker {
            id: "worker-1".to_string(),
            status: DispatchWorkerStatus::Busy,
            current_task: Some("task-1".to_string()),
            last_heartbeat: Instant::now() - Duration::from_secs(30),
        }];

        let report = dispatcher
            .monitor_workers(&mut workers, Duration::from_secs(5))
            .await;

        assert_eq!(report.unhealthy_workers, vec!["worker-1".to_string()]);
        assert_eq!(workers[0].status, DispatchWorkerStatus::Unhealthy);
    }

    #[tokio::test]
    async fn handle_completions_marks_task_complete_and_frees_worker() {
        let dispatcher = Dispatcher::new();
        let mut tasks = vec![task("task-1")];
        let mut workers = vec![worker("worker-1")];
        dispatcher
            .dispatch_loop(&mut tasks, &mut workers)
            .await
            .unwrap();

        dispatcher
            .submit_completion(DispatchCompletion::success(
                tasks[0].id.clone(),
                "worker-1",
                "done",
            ))
            .await;

        let report = dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await
            .unwrap();

        assert_eq!(
            report,
            CompletionReport {
                completed: 1,
                failed: 0,
                processed_completions: vec![DispatchCompletion::success(
                    tasks[0].id.clone(),
                    "worker-1",
                    "done",
                )],
            }
        );
        assert_eq!(tasks[0].status, TaskStatus::Completed);
        assert_eq!(workers[0].status, DispatchWorkerStatus::Idle);
        assert!(workers[0].current_task.is_none());
        assert_eq!(dispatcher.active_assignment_count(), 0);
    }

    #[tokio::test]
    async fn handle_completions_marks_failures_and_worker_failure_state() {
        let dispatcher = Dispatcher::new();
        let mut tasks = vec![task("task-1")];
        let mut workers = vec![worker("worker-1")];
        dispatcher
            .dispatch_loop(&mut tasks, &mut workers)
            .await
            .unwrap();

        dispatcher
            .submit_completion(DispatchCompletion::failure(
                tasks[0].id.clone(),
                "worker-1",
                "boom",
            ))
            .await;

        let report = dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await
            .unwrap();

        assert_eq!(report.failed, 1);
        assert_eq!(report.processed_completions.len(), 1);
        assert_eq!(tasks[0].status, TaskStatus::Failed);
        assert_eq!(workers[0].status, DispatchWorkerStatus::Failed);
        assert_eq!(dispatcher.active_assignment_count(), 0);
    }

    #[tokio::test]
    async fn handle_completions_rejects_unknown_task_without_dropping_completion() {
        let dispatcher = Dispatcher::new();
        let mut tasks = Vec::new();
        let mut workers = vec![worker("worker-1")];

        dispatcher
            .submit_completion(DispatchCompletion::success(
                "missing-task",
                "worker-1",
                "done",
            ))
            .await;

        let error = dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await
            .unwrap_err();

        assert_eq!(
            error,
            DispatcherError::TaskNotFound("missing-task".to_string())
        );
        assert_eq!(dispatcher.pending_completions_len().await, 1);

        let mut fixed = vec![task("task-1")];
        fixed[0].id = "missing-task".to_string();
        fixed[0].status = TaskStatus::InProgress;

        let report = dispatcher
            .handle_completions(&mut fixed, &mut workers)
            .await
            .unwrap();
        assert_eq!(report.completed, 1);
        assert_eq!(dispatcher.pending_completions_len().await, 0);
    }

    #[tokio::test]
    async fn handle_completions_rejects_unknown_worker_without_dropping_completion() {
        let dispatcher = Dispatcher::new();
        let mut tasks = vec![task("task-1")];
        tasks[0].id = "t1".to_string();
        tasks[0].status = TaskStatus::InProgress;
        let mut workers = vec![worker("worker-other")];

        dispatcher
            .submit_completion(DispatchCompletion::success("t1", "worker-expected", "done"))
            .await;

        let err = dispatcher
            .handle_completions(&mut tasks, &mut workers)
            .await
            .unwrap_err();
        assert_eq!(
            err,
            DispatcherError::WorkerNotFound("worker-expected".to_string())
        );
        assert_eq!(dispatcher.pending_completions_len().await, 1);
    }
}
