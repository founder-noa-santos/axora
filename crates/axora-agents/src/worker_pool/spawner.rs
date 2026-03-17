//! Dynamic worker spawning and autoscaling.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use super::{Result, SharedWorkers, WorkerLifecycleManager, WorkerPoolConfig, WorkerStatus};

/// Result of an autoscaling operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScaleAction {
    /// No scaling change was applied.
    None,
    /// A worker was spawned.
    Spawned,
    /// A worker was terminated.
    Terminated,
}

/// Handles capacity-aware worker creation and downscaling.
pub struct WorkerSpawner {
    workers: SharedWorkers,
    lifecycle: WorkerLifecycleManager,
    config: WorkerPoolConfig,
    queue_depth: Arc<AtomicU64>,
}

impl WorkerSpawner {
    /// Create a spawner for the shared worker registry.
    pub fn new(
        workers: SharedWorkers,
        lifecycle: WorkerLifecycleManager,
        config: WorkerPoolConfig,
        queue_depth: Arc<AtomicU64>,
    ) -> Self {
        Self {
            workers,
            lifecycle,
            config,
            queue_depth,
        }
    }

    /// Return the remaining worker capacity.
    pub fn get_available_capacity(&self) -> usize {
        self.config.max_workers.saturating_sub(self.workers.len())
    }

    /// Spawn a worker if the pool is below max capacity.
    pub fn spawn_if_capacity(&self) -> Result<String> {
        self.lifecycle.spawn_worker()
    }

    /// Scale the worker count up or down from queue pressure.
    pub fn scale_for_queue_depth(&self, queue_depth: usize) -> Result<ScaleAction> {
        self.queue_depth.store(queue_depth as u64, Ordering::SeqCst);
        let total_workers = self.workers.len();
        let idle_workers = self
            .workers
            .iter()
            .filter(|entry| matches!(entry.value().read().status, WorkerStatus::Idle))
            .count();

        if queue_depth > idle_workers.saturating_mul(2) && total_workers < self.config.max_workers {
            self.lifecycle.spawn_worker()?;
            return Ok(ScaleAction::Spawned);
        }

        if idle_workers > queue_depth.saturating_mul(2) && total_workers > self.config.min_workers {
            if let Some(worker_id) = self
                .workers
                .iter()
                .find(|entry| matches!(entry.value().read().status, WorkerStatus::Idle))
                .map(|entry| entry.key().clone())
            {
                self.lifecycle.terminate_worker(&worker_id)?;
                return Ok(ScaleAction::Terminated);
            }
        }

        Ok(ScaleAction::None)
    }

    /// Compatibility wrapper used by older callers.
    pub fn auto_scale(&self, queue_depth: usize) -> Result<ScaleAction> {
        self.scale_for_queue_depth(queue_depth)
    }
}
