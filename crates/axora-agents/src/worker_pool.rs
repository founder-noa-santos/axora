//! Dynamic worker pool management.
//!
//! ```rust
//! use axora_agents::{Task, WorkerPool, WorkerPoolConfig};
//!
//! let pool = WorkerPool::new(WorkerPoolConfig::default()).unwrap();
//! let worker_id = pool.get_available_worker().unwrap();
//! pool.dispatch_task(&worker_id, Task::new("index repository")).unwrap();
//! assert_eq!(pool.get_pool_stats().busy_workers, 0);
//! ```

#[path = "worker_pool/dispatcher.rs"]
mod dispatcher;
#[path = "worker_pool/health_monitor.rs"]
mod health_monitor;
#[path = "worker_pool/lifecycle.rs"]
mod lifecycle;
#[path = "worker_pool/spawner.rs"]
mod spawner;

use crate::agent::{Agent, BaseAgent};
use crate::task::{Task, TaskStatus};
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;

pub use dispatcher::{DispatchRecord, TaskDispatcher};
pub use health_monitor::{HealthMonitor, WorkerHealth};
pub use lifecycle::{WorkerLifecycle, WorkerLifecycleManager};
pub use spawner::{ScaleAction, WorkerSpawner};

/// Compatibility alias for worker health state snapshots.
pub type HealthState = WorkerHealth;

pub(crate) type SharedWorker = Arc<RwLock<Worker>>;
pub(crate) type WorkerRegistry = DashMap<WorkerId, SharedWorker>;
pub(crate) type SharedWorkers = Arc<WorkerRegistry>;
pub(crate) type AgentHandle = Arc<Mutex<Box<dyn Agent>>>;

/// Factory used to create worker agents.
pub type WorkerFactory = Arc<dyn Fn() -> Box<dyn Agent> + Send + Sync>;

/// Result type for worker-pool operations.
pub type Result<T> = std::result::Result<T, WorkerPoolError>;

/// Task identifier tracked by the worker pool.
pub type PoolTaskId = String;

/// Worker identifier.
pub type WorkerId = String;

/// Errors returned by worker-pool operations.
#[derive(Debug, Error)]
pub enum WorkerPoolError {
    /// The pool configuration is invalid.
    #[error("invalid worker pool config: {0}")]
    InvalidConfig(String),
    /// The requested worker does not exist.
    #[error("worker not found: {0}")]
    WorkerNotFound(WorkerId),
    /// The requested task does not exist.
    #[error("task not found: {0}")]
    TaskNotFound(String),
    /// The worker cannot accept a new task.
    #[error("worker unavailable: {0}")]
    WorkerUnavailable(String),
    /// The operation requires an active task for the worker.
    #[error("worker {worker_id} has no active task")]
    WorkerHasNoTask {
        /// Worker that lacks an active task.
        worker_id: WorkerId,
    },
    /// The pool has reached maximum capacity.
    #[error("worker pool is at capacity")]
    CapacityReached,
    /// A lock was poisoned while mutating pool state.
    #[error("lock poisoned: {0}")]
    LockPoisoned(&'static str),
    /// The worker execution failed.
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
}

/// Worker execution state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerStatus {
    /// Ready to accept work.
    Idle,
    /// Actively running a task.
    Busy(String),
    /// Heartbeats have degraded beyond the health threshold.
    Unhealthy {
        /// Failure reason.
        reason: String,
    },
    /// Task execution failed.
    Failed {
        /// Failure message.
        error: String,
    },
    /// Worker was removed from the pool.
    Terminated,
}

/// Worker metadata tracked by the pool.
pub struct Worker {
    /// Unique worker ID.
    pub id: WorkerId,
    /// Backing agent used to execute tasks.
    pub agent: AgentHandle,
    /// Current lifecycle stage.
    pub lifecycle: WorkerLifecycle,
    /// Current execution state.
    pub status: WorkerStatus,
    /// Active task, if any.
    pub current_task: Option<String>,
    /// Most recent heartbeat timestamp.
    pub last_heartbeat: Instant,
    /// Current health score from `0.0` to `1.0`.
    pub health_score: f32,
    /// Consecutive missed heartbeat windows.
    pub missed_heartbeats: usize,
}

impl Worker {
    pub(crate) fn new(id: WorkerId, agent: Box<dyn Agent>) -> Self {
        Self {
            id,
            agent: Arc::new(Mutex::new(agent)),
            lifecycle: WorkerLifecycle::Spawning,
            status: WorkerStatus::Idle,
            current_task: None,
            last_heartbeat: Instant::now(),
            health_score: 1.0,
            missed_heartbeats: 0,
        }
    }

    pub(crate) fn is_idle(&self) -> bool {
        self.lifecycle == WorkerLifecycle::Ready && matches!(self.status, WorkerStatus::Idle)
    }
}

/// Worker-pool configuration.
#[derive(Clone)]
pub struct WorkerPoolConfig {
    /// Minimum number of workers kept ready.
    pub min_workers: usize,
    /// Maximum number of workers allowed.
    pub max_workers: usize,
    /// Time between expected heartbeats.
    pub health_check_interval: Duration,
    /// Missed heartbeat windows before a worker is unhealthy.
    pub unhealthy_threshold: usize,
    /// Whether unhealthy or failed workers are restarted automatically.
    pub auto_restart: bool,
    /// Factory used by `WorkerPool::new`.
    pub worker_factory: WorkerFactory,
}

impl std::fmt::Debug for WorkerPoolConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerPoolConfig")
            .field("min_workers", &self.min_workers)
            .field("max_workers", &self.max_workers)
            .field("health_check_interval", &self.health_check_interval)
            .field("unhealthy_threshold", &self.unhealthy_threshold)
            .field("auto_restart", &self.auto_restart)
            .finish()
    }
}

impl Default for WorkerPoolConfig {
    fn default() -> Self {
        Self {
            min_workers: 2,
            max_workers: 10,
            health_check_interval: Duration::from_secs(30),
            unhealthy_threshold: 3,
            auto_restart: true,
            worker_factory: Arc::new(|| {
                Box::new(BaseAgent::new("Worker", "Worker Pool")) as Box<dyn Agent>
            }),
        }
    }
}

/// Current worker-pool statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PoolStats {
    /// Total workers in the pool.
    pub total_workers: usize,
    /// Workers ready for work.
    pub idle_workers: usize,
    /// Workers currently processing tasks.
    pub busy_workers: usize,
    /// Workers marked unhealthy.
    pub unhealthy_workers: usize,
    /// Workers marked failed.
    pub failed_workers: usize,
}

/// Unified worker pool composed from lifecycle, health, spawn, and dispatch components.
pub struct WorkerPool {
    /// Shared worker registry.
    pub workers: SharedWorkers,
    /// Lifecycle manager for worker creation and restart.
    pub lifecycle: WorkerLifecycleManager,
    /// Health monitor for heartbeat checks.
    pub health_monitor: HealthMonitor,
    /// Capacity-aware spawner.
    pub spawner: WorkerSpawner,
    /// Task dispatcher and result registry.
    pub dispatcher: TaskDispatcher,
    /// Runtime configuration.
    pub config: WorkerPoolConfig,
}

impl std::fmt::Debug for WorkerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkerPool")
            .field("worker_count", &self.workers.len())
            .field("config", &self.config)
            .finish()
    }
}

impl WorkerPool {
    /// Create a new pool using the configured worker factory.
    pub fn new(config: WorkerPoolConfig) -> Result<Self> {
        Self::build(config.clone(), config.worker_factory.clone())
    }

    /// Create a new pool using an explicit worker factory.
    pub fn with_factory<F>(config: WorkerPoolConfig, factory: F) -> Result<Self>
    where
        F: Fn() -> Box<dyn Agent> + Send + Sync + 'static,
    {
        Self::build(config, Arc::new(factory))
    }

    fn build(config: WorkerPoolConfig, factory: WorkerFactory) -> Result<Self> {
        if config.min_workers == 0 {
            return Err(WorkerPoolError::InvalidConfig(
                "min_workers must be greater than zero".to_string(),
            ));
        }
        if config.min_workers > config.max_workers {
            return Err(WorkerPoolError::InvalidConfig(
                "min_workers cannot exceed max_workers".to_string(),
            ));
        }
        if config.unhealthy_threshold == 0 {
            return Err(WorkerPoolError::InvalidConfig(
                "unhealthy_threshold must be greater than zero".to_string(),
            ));
        }

        let workers = Arc::new(DashMap::new());
        let lifecycle = WorkerLifecycleManager::new(workers.clone(), factory, config.max_workers);
        let health_monitor = HealthMonitor::new(
            workers.clone(),
            config.health_check_interval,
            config.unhealthy_threshold,
        );
        let spawner = WorkerSpawner::new(
            workers.clone(),
            lifecycle.clone(),
            config.clone(),
            Arc::new(AtomicU64::new(0)),
        );
        let dispatcher = TaskDispatcher::new(workers.clone());

        let pool = Self {
            workers,
            lifecycle,
            health_monitor,
            spawner,
            dispatcher,
            config,
        };

        for _ in 0..pool.config.min_workers {
            pool.lifecycle.spawn_worker()?;
        }

        Ok(pool)
    }

    /// Return an idle worker if one exists.
    pub fn get_available_worker(&self) -> Option<WorkerId> {
        self.workers.iter().find_map(|entry| {
            let worker = entry.value().read();
            worker.is_idle().then(|| entry.key().clone())
        })
    }

    /// Dispatch a task to a worker.
    pub fn dispatch_task(&self, worker_id: impl AsRef<str>, task: Task) -> Result<()> {
        let worker_id = worker_id.as_ref().to_string();
        self.dispatcher.register_task(task.clone());
        self.dispatcher.dispatch(task, worker_id)
    }

    /// Return the status of the task currently assigned to the worker.
    pub fn get_task_status(&self, worker_id: impl AsRef<str>) -> Result<TaskStatus> {
        let worker_id = worker_id.as_ref().to_string();
        let worker = self
            .workers
            .get(&worker_id)
            .ok_or_else(|| WorkerPoolError::WorkerNotFound(worker_id.clone()))?;
        let task_id =
            worker
                .read()
                .current_task
                .clone()
                .ok_or_else(|| WorkerPoolError::WorkerHasNoTask {
                    worker_id: worker_id.clone(),
                })?;
        self.dispatcher.task_status(&task_id)
    }

    /// Run a health-check pass and return workers flagged unhealthy.
    pub fn health_check(&self) -> Result<Vec<WorkerId>> {
        let unhealthy = self.health_monitor.scan_unhealthy();
        if self.config.auto_restart {
            for worker_id in &unhealthy {
                let state = self.health_monitor.check_health(worker_id)?;
                if matches!(
                    state,
                    WorkerHealth::Unhealthy { .. } | WorkerHealth::Failed { .. }
                ) {
                    let _ = self.lifecycle.restart_worker(worker_id)?;
                }
            }
        }
        Ok(unhealthy)
    }

    /// Apply autoscaling based on dispatcher queue depth.
    pub fn rebalance(&self) -> Result<()> {
        self.spawner.auto_scale(self.dispatcher.pending_depth())?;
        Ok(())
    }

    /// Record a fresh worker heartbeat.
    pub fn record_heartbeat(&self, worker_id: &WorkerId) -> Result<()> {
        self.health_monitor.record_heartbeat(worker_id)
    }

    /// Return current pool statistics.
    pub fn get_pool_stats(&self) -> PoolStats {
        let mut stats = PoolStats::default();
        for entry in self.workers.iter() {
            let worker = entry.value().read();
            stats.total_workers += 1;
            match &worker.status {
                WorkerStatus::Idle => stats.idle_workers += 1,
                WorkerStatus::Busy(_) => stats.busy_workers += 1,
                WorkerStatus::Unhealthy { .. } => stats.unhealthy_workers += 1,
                WorkerStatus::Failed { .. } => stats.failed_workers += 1,
                WorkerStatus::Terminated => {}
            }
        }
        stats
    }

    #[cfg(test)]
    pub(crate) fn dispatcher_for_tests(&self) -> &TaskDispatcher {
        &self.dispatcher
    }

    #[cfg(test)]
    pub(crate) fn workers_for_tests(&self) -> &SharedWorkers {
        &self.workers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Priority;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[derive(Default)]
    struct CountingFactory {
        created: Arc<AtomicUsize>,
    }

    impl CountingFactory {
        fn factory(&self) -> WorkerFactory {
            let created = self.created.clone();
            Arc::new(move || {
                created.fetch_add(1, Ordering::SeqCst);
                Box::new(BaseAgent::new("Worker", "Pool Test")) as Box<dyn Agent>
            })
        }
    }

    fn test_pool(config: WorkerPoolConfig) -> WorkerPool {
        let factory = CountingFactory::default();
        WorkerPool::build(config, factory.factory()).unwrap()
    }

    #[test]
    fn creates_minimum_workers_on_startup() {
        let pool = test_pool(WorkerPoolConfig {
            min_workers: 3,
            max_workers: 6,
            ..WorkerPoolConfig::default()
        });
        assert_eq!(pool.get_pool_stats().total_workers, 3);
    }

    #[test]
    fn rejects_invalid_config() {
        let err = WorkerPool::new(WorkerPoolConfig {
            min_workers: 4,
            max_workers: 2,
            ..WorkerPoolConfig::default()
        })
        .unwrap_err();
        assert!(matches!(err, WorkerPoolError::InvalidConfig(_)));
    }

    #[test]
    fn returns_idle_worker_when_available() {
        let pool = test_pool(WorkerPoolConfig::default());
        assert!(pool.get_available_worker().is_some());
    }

    #[test]
    fn dispatch_task_runs_and_records_result() {
        let pool = test_pool(WorkerPoolConfig::default());
        let worker_id = pool.get_available_worker().unwrap();
        let task = Task::new("index repository").with_priority(Priority::High);
        let task_id = task.id.clone();

        pool.dispatch_task(&worker_id, task).unwrap();

        let result = pool.dispatcher_for_tests().get_result(&task_id).unwrap();
        assert!(result.result.success);
        assert_eq!(
            pool.dispatcher_for_tests().task_status(&task_id).unwrap(),
            TaskStatus::Completed
        );
    }

    #[test]
    fn health_check_restarts_unhealthy_workers() {
        let pool = test_pool(WorkerPoolConfig {
            health_check_interval: Duration::from_millis(10),
            unhealthy_threshold: 1,
            ..WorkerPoolConfig::default()
        });
        let worker_id = pool.get_available_worker().unwrap();
        {
            let worker = pool.workers_for_tests().get(&worker_id).unwrap();
            worker.write().last_heartbeat = Instant::now() - Duration::from_millis(50);
        }

        let unhealthy = pool.health_check().unwrap();

        assert_eq!(unhealthy, vec![worker_id]);
        assert_eq!(pool.get_pool_stats().total_workers, pool.config.min_workers);
    }

    #[test]
    fn rebalance_spawns_when_queue_outgrows_idle_workers() {
        let pool = test_pool(WorkerPoolConfig {
            min_workers: 1,
            max_workers: 4,
            ..WorkerPoolConfig::default()
        });
        for idx in 0..3 {
            pool.dispatcher_for_tests()
                .register_task(Task::new(&format!("queued-{idx}")));
        }

        pool.rebalance().unwrap();
        assert_eq!(pool.get_pool_stats().total_workers, 2);
    }

    #[test]
    fn rebalance_trims_idle_workers_down_to_minimum() {
        let pool = test_pool(WorkerPoolConfig {
            min_workers: 1,
            max_workers: 5,
            ..WorkerPoolConfig::default()
        });
        pool.spawner.spawn_if_capacity().unwrap();
        pool.spawner.spawn_if_capacity().unwrap();

        pool.spawner.auto_scale(0).unwrap();
        pool.spawner.auto_scale(0).unwrap();

        assert_eq!(pool.get_pool_stats().total_workers, 1);
    }

    #[test]
    fn concurrent_spawning_caps_at_max_workers() {
        let pool = Arc::new(test_pool(WorkerPoolConfig {
            min_workers: 1,
            max_workers: 100,
            ..WorkerPoolConfig::default()
        }));

        let mut handles = Vec::new();
        for _ in 0..200 {
            let pool = pool.clone();
            handles.push(thread::spawn(move || {
                let _ = pool.spawner.spawn_if_capacity();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(pool.get_pool_stats().total_workers, 100);
    }

    #[test]
    fn record_heartbeat_recovers_unhealthy_worker() {
        let pool = test_pool(WorkerPoolConfig {
            health_check_interval: Duration::from_millis(10),
            unhealthy_threshold: 1,
            auto_restart: false,
            ..WorkerPoolConfig::default()
        });
        let worker_id = pool.get_available_worker().unwrap();
        {
            let worker = pool.workers_for_tests().get(&worker_id).unwrap();
            worker.write().last_heartbeat = Instant::now() - Duration::from_millis(50);
        }

        let unhealthy = pool.health_check().unwrap();
        assert_eq!(unhealthy, vec![worker_id.clone()]);

        pool.record_heartbeat(&worker_id).unwrap();
        let state = pool.health_monitor.check_health(&worker_id).unwrap();
        assert!(matches!(state, WorkerHealth::Healthy { .. }));
    }
}
