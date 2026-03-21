use super::{SharedWorkers, WorkerId, WorkerPoolError, WorkerStatus};
use std::time::{Duration, Instant};

/// Health outcome for a worker.
#[derive(Debug, Clone, PartialEq)]
pub enum WorkerHealth {
    /// Worker heartbeat is within the expected interval.
    Healthy {
        /// Consecutive missed heartbeat windows.
        missed_heartbeats: usize,
        /// Current health score.
        health_score: f32,
    },
    /// Worker exceeded the missed-heartbeat threshold.
    Unhealthy {
        /// Consecutive missed heartbeat windows.
        missed_heartbeats: usize,
        /// Failure reason.
        reason: String,
    },
    /// Worker task execution already failed.
    Failed {
        /// Failure message.
        error: String,
    },
    /// Worker has been terminated.
    Terminated,
}

/// Evaluates worker health from heartbeat timing.
pub struct HealthMonitor {
    workers: SharedWorkers,
    heartbeat_interval: Duration,
    unhealthy_threshold: usize,
}

impl HealthMonitor {
    /// Create a health monitor for the shared worker registry.
    pub fn new(
        workers: SharedWorkers,
        heartbeat_interval: Duration,
        unhealthy_threshold: usize,
    ) -> Self {
        Self {
            workers,
            heartbeat_interval,
            unhealthy_threshold,
        }
    }

    /// Return the configured heartbeat interval.
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Record a fresh heartbeat for a worker.
    pub fn record_heartbeat(&self, worker_id: &WorkerId) -> Result<(), WorkerPoolError> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| WorkerPoolError::WorkerNotFound(worker_id.clone()))?;
        let mut worker = worker.write();
        worker.last_heartbeat = Instant::now();
        worker.missed_heartbeats = 0;
        worker.health_score = 1.0;
        if matches!(worker.status, WorkerStatus::Unhealthy { .. }) {
            worker.status = WorkerStatus::Idle;
        }
        Ok(())
    }

    /// Evaluate the health of a single worker.
    pub fn check_health(&self, worker_id: &WorkerId) -> Result<WorkerHealth, WorkerPoolError> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| WorkerPoolError::WorkerNotFound(worker_id.clone()))?;
        let mut worker = worker.write();

        match &worker.status {
            WorkerStatus::Failed { error } => {
                return Ok(WorkerHealth::Failed {
                    error: error.clone(),
                })
            }
            WorkerStatus::Terminated => return Ok(WorkerHealth::Terminated),
            _ => {}
        }

        let elapsed = Instant::now().saturating_duration_since(worker.last_heartbeat);
        let interval_ms = self.heartbeat_interval.as_millis().max(1);
        let missed = (elapsed.as_millis() / interval_ms) as usize;
        worker.missed_heartbeats = missed;

        if missed >= self.unhealthy_threshold {
            return Ok(self.mark_unhealthy_locked(&mut worker));
        }

        worker.health_score = (1.0 - missed as f32 / self.unhealthy_threshold as f32).max(0.0);
        Ok(WorkerHealth::Healthy {
            missed_heartbeats: missed,
            health_score: worker.health_score,
        })
    }

    /// Explicitly mark a worker unhealthy.
    pub fn mark_unhealthy(&self, worker_id: &WorkerId) -> Result<WorkerHealth, WorkerPoolError> {
        let worker = self
            .workers
            .get(worker_id)
            .ok_or_else(|| WorkerPoolError::WorkerNotFound(worker_id.clone()))?;
        let mut worker = worker.write();
        Ok(self.mark_unhealthy_locked(&mut worker))
    }

    /// Return all workers that are unhealthy after a scan.
    pub fn scan_unhealthy(&self) -> Vec<WorkerId> {
        self.workers
            .iter()
            .filter_map(|entry| {
                let worker_id = entry.key().clone();
                match self.check_health(&worker_id) {
                    Ok(WorkerHealth::Unhealthy { .. } | WorkerHealth::Failed { .. }) => {
                        Some(worker_id)
                    }
                    _ => None,
                }
            })
            .collect()
    }

    fn mark_unhealthy_locked(
        &self,
        worker: &mut parking_lot::RwLockWriteGuard<'_, super::Worker>,
    ) -> WorkerHealth {
        worker.health_score = 0.0;
        let reason = format!("missed {} heartbeats", worker.missed_heartbeats);
        worker.status = WorkerStatus::Unhealthy {
            reason: reason.clone(),
        };
        WorkerHealth::Unhealthy {
            missed_heartbeats: worker.missed_heartbeats,
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{Agent, BaseAgent};
    use crate::worker_pool::{Worker, WorkerLifecycle};
    use dashmap::DashMap;
    use parking_lot::RwLock;
    use std::sync::Arc;

    fn monitor() -> (HealthMonitor, SharedWorkers, WorkerId) {
        let workers = Arc::new(DashMap::new());
        let worker_id = "worker-1".to_string();
        let mut worker = Worker::new(
            worker_id.clone(),
            Box::new(BaseAgent::new("Worker", "Health")) as Box<dyn Agent>,
        );
        worker.lifecycle = WorkerLifecycle::Ready;
        worker.status = WorkerStatus::Idle;
        workers.insert(worker_id.clone(), Arc::new(RwLock::new(worker)));
        (
            HealthMonitor::new(workers.clone(), Duration::from_secs(30), 3),
            workers,
            worker_id,
        )
    }

    #[test]
    fn returns_default_heartbeat_interval() {
        let (monitor, _, _) = monitor();
        assert_eq!(monitor.heartbeat_interval(), Duration::from_secs(30));
    }

    #[test]
    fn check_health_returns_healthy_for_recent_heartbeat() {
        let (monitor, _, worker_id) = monitor();
        let state = monitor.check_health(&worker_id).unwrap();
        assert!(matches!(state, WorkerHealth::Healthy { .. }));
    }

    #[test]
    fn marks_worker_unhealthy_after_threshold() {
        let (monitor, workers, worker_id) = monitor();
        workers.get(&worker_id).unwrap().write().last_heartbeat =
            Instant::now() - Duration::from_secs(95);

        let state = monitor.check_health(&worker_id).unwrap();
        assert!(matches!(state, WorkerHealth::Unhealthy { .. }));
    }

    #[test]
    fn mark_unhealthy_sets_zero_health_score() {
        let (monitor, workers, worker_id) = monitor();
        let state = monitor.mark_unhealthy(&worker_id).unwrap();
        assert!(matches!(state, WorkerHealth::Unhealthy { .. }));
        assert_eq!(workers.get(&worker_id).unwrap().read().health_score, 0.0);
    }

    #[test]
    fn record_heartbeat_resets_health_state() {
        let (monitor, workers, worker_id) = monitor();
        {
            let worker_entry = workers.get(&worker_id).unwrap();
            let mut worker = worker_entry.write();
            worker.status = WorkerStatus::Unhealthy {
                reason: "missed heartbeats".to_string(),
            };
            worker.last_heartbeat = Instant::now() - Duration::from_secs(95);
        }

        monitor.record_heartbeat(&worker_id).unwrap();

        let state = monitor.check_health(&worker_id).unwrap();
        assert!(matches!(state, WorkerHealth::Healthy { .. }));
    }

    #[test]
    fn scan_unhealthy_collects_only_bad_workers() {
        let (monitor, workers, worker_id) = monitor();
        workers.get(&worker_id).unwrap().write().last_heartbeat =
            Instant::now() - Duration::from_secs(95);

        let unhealthy = monitor.scan_unhealthy();

        assert_eq!(unhealthy, vec![worker_id]);
    }
}
