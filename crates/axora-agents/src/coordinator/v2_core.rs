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

/// Role assigned to a built-in squad member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SquadRole {
    /// Plans architecture and decomposition.
    Architect,
    /// Implements code changes.
    Coder,
    /// Verifies behavior through tests.
    Tester,
    /// Runs bounded local commands and patch actions.
    Executor,
    /// Reviews output contracts and regressions.
    Reviewer,
}

impl SquadRole {
    /// Stable role identifier used across runtime components.
    pub fn as_str(self) -> &'static str {
        match self {
            SquadRole::Architect => "architect",
            SquadRole::Coder => "coder",
            SquadRole::Tester => "tester",
            SquadRole::Executor => "executor",
            SquadRole::Reviewer => "reviewer",
        }
    }

    /// Human-readable display name.
    pub fn display_name(self) -> &'static str {
        match self {
            SquadRole::Architect => "Architect",
            SquadRole::Coder => "Coder",
            SquadRole::Tester => "Tester",
            SquadRole::Executor => "Executor",
            SquadRole::Reviewer => "Reviewer",
        }
    }
}

/// Planning and acting policy attached to a squad member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanningActingPolicy {
    /// Continuous planning with high retrieval context.
    Strategic,
    /// Standard code authoring loop.
    Driver,
    /// Verification-first acting loop.
    Verification,
    /// Tool-heavy acting with bounded reasoning.
    SandboxedExecution,
    /// Arbiter policy focused on review contracts.
    Arbiter,
}

/// Expected output contract for a squad member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputContract {
    /// A short architectural plan or decomposition note.
    Plan,
    /// A code patch or implementation summary.
    Patch,
    /// A test report or execution result.
    TestEvidence,
    /// A bounded command result or patch receipt.
    ExecutionReceipt,
    /// A review verdict with findings.
    ReviewVerdict,
}

/// Static runtime profile for a built-in squad member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerProfile {
    /// Display name shown in runtime state.
    pub name: String,
    /// Semantic role in the Base Squad.
    pub role: SquadRole,
    /// Tools this worker may invoke by default.
    pub tool_permissions: Vec<String>,
    /// ReAct planning/acting policy.
    pub planning_policy: PlanningActingPolicy,
    /// Retry budget for task execution.
    pub retry_budget: u32,
    /// Retrieval budget for context hydration.
    pub retrieval_budget: usize,
    /// Expected output contract.
    pub output_contract: OutputContract,
}

/// Zero-config bootstrapper for the batteries-included Base Squad.
pub struct BaseSquadBootstrapper;

impl BaseSquadBootstrapper {
    /// Build the default runtime squad for the given worker count.
    pub fn build(max_workers: usize, retry_budget: u32, retrieval_budget: usize) -> Vec<WorkerInfo> {
        let mut workers = vec![
            WorkerInfo::with_profile("architect", Self::profile(SquadRole::Architect, retry_budget, retrieval_budget)),
            WorkerInfo::with_profile("coder", Self::profile(SquadRole::Coder, retry_budget, retrieval_budget)),
            WorkerInfo::with_profile("tester", Self::profile(SquadRole::Tester, retry_budget, retrieval_budget)),
            WorkerInfo::with_profile("executor", Self::profile(SquadRole::Executor, retry_budget, retrieval_budget)),
            WorkerInfo::with_profile("reviewer", Self::profile(SquadRole::Reviewer, retry_budget, retrieval_budget)),
        ];

        if max_workers > workers.len() {
            for idx in workers.len()..max_workers {
                workers.push(WorkerInfo::with_profile(
                    format!("executor-{}", idx - 3),
                    Self::profile(SquadRole::Executor, retry_budget, retrieval_budget),
                ));
            }
        } else {
            workers.truncate(max_workers);
        }

        workers
    }

    fn profile(role: SquadRole, retry_budget: u32, retrieval_budget: usize) -> WorkerProfile {
        match role {
            SquadRole::Architect => WorkerProfile {
                name: role.display_name().to_string(),
                role,
                tool_permissions: vec![
                    "read_file".to_string(),
                    "graph_retrieve_skills".to_string(),
                    "graph_retrieve_code".to_string(),
                ],
                planning_policy: PlanningActingPolicy::Strategic,
                retry_budget,
                retrieval_budget,
                output_contract: OutputContract::Plan,
            },
            SquadRole::Coder => WorkerProfile {
                name: role.display_name().to_string(),
                role,
                tool_permissions: vec![
                    "read_file".to_string(),
                    "generate_diff".to_string(),
                    "apply_patch".to_string(),
                    "ast_chunk".to_string(),
                    "graph_retrieve_code".to_string(),
                ],
                planning_policy: PlanningActingPolicy::Driver,
                retry_budget,
                retrieval_budget,
                output_contract: OutputContract::Patch,
            },
            SquadRole::Tester => WorkerProfile {
                name: role.display_name().to_string(),
                role,
                tool_permissions: vec![
                    "read_file".to_string(),
                    "run_command".to_string(),
                    "graph_retrieve_skills".to_string(),
                    "graph_retrieve_code".to_string(),
                ],
                planning_policy: PlanningActingPolicy::Verification,
                retry_budget,
                retrieval_budget: retrieval_budget / 2,
                output_contract: OutputContract::TestEvidence,
            },
            SquadRole::Executor => WorkerProfile {
                name: role.display_name().to_string(),
                role,
                tool_permissions: vec![
                    "read_file".to_string(),
                    "run_command".to_string(),
                    "apply_patch".to_string(),
                    "generate_diff".to_string(),
                ],
                planning_policy: PlanningActingPolicy::SandboxedExecution,
                retry_budget,
                retrieval_budget: retrieval_budget / 2,
                output_contract: OutputContract::ExecutionReceipt,
            },
            SquadRole::Reviewer => WorkerProfile {
                name: role.display_name().to_string(),
                role,
                tool_permissions: vec![
                    "read_file".to_string(),
                    "generate_diff".to_string(),
                    "graph_retrieve_skills".to_string(),
                    "graph_retrieve_code".to_string(),
                ],
                planning_policy: PlanningActingPolicy::Arbiter,
                retry_budget,
                retrieval_budget,
                output_contract: OutputContract::ReviewVerdict,
            },
        }
    }
}

/// Snapshot of runtime worker state tracked by the coordinator.
#[derive(Debug, Clone)]
pub struct WorkerInfo {
    /// Stable worker identifier.
    pub id: WorkerId,
    /// Batteries-included worker profile.
    pub profile: WorkerProfile,
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
        Self::with_profile(
            id,
            BaseSquadBootstrapper::profile(SquadRole::Executor, 1, 1_000),
        )
    }

    /// Creates a new idle worker record with an explicit profile.
    pub fn with_profile(id: impl Into<WorkerId>, profile: WorkerProfile) -> Self {
        Self {
            id: id.into(),
            profile,
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
    use super::{
        BaseSquadBootstrapper, Coordinator, CoordinatorCoreError, SquadRole, WorkerInfo,
        WorkerRegistry,
    };
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
            profile: BaseSquadBootstrapper::profile(SquadRole::Executor, 1, 1000),
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
            profile: BaseSquadBootstrapper::profile(SquadRole::Executor, 1, 1000),
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

    #[test]
    fn base_squad_bootstrapper_returns_canonical_roles() {
        let workers = BaseSquadBootstrapper::build(5, 1, 2_000);
        let roles = workers
            .iter()
            .map(|worker| worker.profile.role.as_str())
            .collect::<Vec<_>>();
        assert_eq!(roles, vec!["architect", "coder", "tester", "executor", "reviewer"]);
    }
}
