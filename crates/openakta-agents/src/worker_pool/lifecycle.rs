//! Worker lifecycle management.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use parking_lot::{Mutex, RwLock};

use super::{
    Result, SharedWorkers, Worker, WorkerFactory, WorkerId, WorkerPoolError, WorkerStatus,
};

/// Worker lifecycle stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerLifecycle {
    /// Worker is being created.
    Spawning,
    /// Worker is ready to execute tasks.
    Ready,
    /// Worker is assigned to a task.
    Busy,
    /// Worker has failed and requires replacement.
    Failed,
    /// Worker was terminated and removed.
    Terminated,
}

/// Coordinates worker creation, termination, and restart.
#[derive(Clone)]
pub struct WorkerLifecycleManager {
    workers: SharedWorkers,
    worker_factory: WorkerFactory,
    next_id: Arc<AtomicUsize>,
    max_workers: usize,
    mutation_lock: Arc<Mutex<()>>,
}

impl WorkerLifecycleManager {
    /// Create a new lifecycle manager for the shared worker registry.
    pub fn new(workers: SharedWorkers, worker_factory: WorkerFactory, max_workers: usize) -> Self {
        Self {
            workers,
            worker_factory,
            next_id: Arc::new(AtomicUsize::new(0)),
            max_workers,
            mutation_lock: Arc::new(Mutex::new(())),
        }
    }

    /// Spawn a new worker.
    pub fn spawn_worker(&self) -> Result<WorkerId> {
        let _guard = self.mutation_lock.lock();
        if self.workers.len() >= self.max_workers {
            return Err(WorkerPoolError::CapacityReached);
        }

        let id = format!("worker-{}", self.next_id.fetch_add(1, Ordering::SeqCst));
        let mut worker = Worker::new(id.clone(), (self.worker_factory)());
        worker.lifecycle = WorkerLifecycle::Ready;
        worker.status = WorkerStatus::Idle;
        self.workers
            .insert(id.clone(), Arc::new(RwLock::new(worker)));
        Ok(id)
    }

    /// Gracefully terminate a worker.
    pub fn terminate_worker(&self, id: &WorkerId) -> Result<()> {
        let _guard = self.mutation_lock.lock();
        let Some((_, worker)) = self.workers.remove(id) else {
            return Err(WorkerPoolError::WorkerNotFound(id.clone()));
        };
        let mut worker = worker.write();
        worker.current_task = None;
        worker.lifecycle = WorkerLifecycle::Terminated;
        worker.status = WorkerStatus::Terminated;
        Ok(())
    }

    /// Restart a worker in place.
    pub fn restart_worker(&self, id: &WorkerId) -> Result<WorkerId> {
        let _guard = self.mutation_lock.lock();
        if !self.workers.contains_key(id) {
            return Err(WorkerPoolError::WorkerNotFound(id.clone()));
        }

        let mut worker = Worker::new(id.clone(), (self.worker_factory)());
        worker.lifecycle = WorkerLifecycle::Ready;
        worker.status = WorkerStatus::Idle;
        self.workers
            .insert(id.clone(), Arc::new(RwLock::new(worker)));
        Ok(id.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, BaseAgent};
    use dashmap::DashMap;
    use std::thread;

    fn manager(max_workers: usize) -> (SharedWorkers, WorkerLifecycleManager) {
        let workers = Arc::new(DashMap::new());
        let manager = WorkerLifecycleManager::new(
            workers.clone(),
            Arc::new(|| Box::new(BaseAgent::new("Worker", "Lifecycle")) as Box<dyn Agent>),
            max_workers,
        );
        (workers, manager)
    }

    #[test]
    fn spawn_worker_creates_ready_idle_worker() {
        let (workers, manager) = manager(2);

        let worker_id = manager.spawn_worker().unwrap();
        let worker = workers.get(&worker_id).unwrap();
        let worker = worker.read();

        assert_eq!(worker.lifecycle, WorkerLifecycle::Ready);
        assert_eq!(worker.status, WorkerStatus::Idle);
    }

    #[test]
    fn spawn_worker_respects_capacity() {
        let (_, manager) = manager(1);

        manager.spawn_worker().unwrap();
        let err = manager.spawn_worker().unwrap_err();

        assert!(matches!(err, WorkerPoolError::CapacityReached));
    }

    #[test]
    fn terminate_worker_removes_worker() {
        let (workers, manager) = manager(2);
        let worker_id = manager.spawn_worker().unwrap();

        manager.terminate_worker(&worker_id).unwrap();

        assert!(!workers.contains_key(&worker_id));
    }

    #[test]
    fn restart_worker_resets_failed_worker() {
        let (workers, manager) = manager(2);
        let worker_id = manager.spawn_worker().unwrap();
        {
            let worker = workers.get(&worker_id).unwrap();
            let mut worker = worker.write();
            worker.lifecycle = WorkerLifecycle::Failed;
            worker.status = WorkerStatus::Failed {
                error: "boom".to_string(),
            };
        }

        manager.restart_worker(&worker_id).unwrap();

        let worker = workers.get(&worker_id).unwrap();
        let worker = worker.read();
        assert_eq!(worker.lifecycle, WorkerLifecycle::Ready);
        assert_eq!(worker.status, WorkerStatus::Idle);
    }

    #[test]
    fn concurrent_spawning_caps_at_max_workers() {
        let (workers, manager) = manager(100);
        let manager = Arc::new(manager);
        let handles: Vec<_> = (0..200)
            .map(|_| {
                let manager = manager.clone();
                thread::spawn(move || {
                    let _ = manager.spawn_worker();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(workers.len(), 100);
    }
}
