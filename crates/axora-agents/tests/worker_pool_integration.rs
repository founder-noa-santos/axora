use axora_agents::agent::{Agent, TaskResult};
use axora_agents::task::{Task, TaskStatus};
use axora_agents::worker_pool::{WorkerPool, WorkerPoolConfig, WorkerStatus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct CountingAgent {
    id: String,
}

impl CountingAgent {
    fn new(id: String) -> Self {
        Self { id }
    }
}

impl Agent for CountingAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.id
    }

    fn role(&self) -> &str {
        "integration"
    }

    fn execute(&mut self, task: Task) -> axora_agents::Result<TaskResult> {
        Ok(TaskResult {
            success: true,
            output: format!("processed {}", task.id),
            error: None,
        })
    }
}

fn integration_config(max_workers: usize) -> WorkerPoolConfig {
    let counter = Arc::new(AtomicUsize::new(0));
    WorkerPoolConfig {
        min_workers: 2,
        max_workers,
        health_check_interval: Duration::from_millis(10),
        unhealthy_threshold: 3,
        auto_restart: true,
        worker_factory: {
            let counter = counter.clone();
            Arc::new(move || {
                let id = counter.fetch_add(1, Ordering::SeqCst);
                Box::new(CountingAgent::new(format!("worker-{id}")))
            })
        },
    }
}

#[test]
fn concurrent_spawning_scales_to_capacity() {
    let pool = Arc::new(
        WorkerPool::with_factory(integration_config(100), || {
            Box::new(CountingAgent::new("worker".to_string()))
        })
        .unwrap(),
    );
    let handles: Vec<_> = (0..150)
        .map(|_| {
            let pool = pool.clone();
            std::thread::spawn(move || pool.spawner.spawn_if_capacity())
        })
        .collect();

    for handle in handles {
        let _ = handle.join().unwrap();
    }

    assert_eq!(pool.workers.len(), 100);
}

#[test]
fn health_monitor_detects_failures() {
    let pool = WorkerPool::with_factory(integration_config(4), || {
        Box::new(CountingAgent::new("worker".to_string()))
    })
    .unwrap();
    let worker_id = pool.get_available_worker().unwrap();

    {
        let worker = pool.workers.get(&worker_id).unwrap();
        let mut worker = worker.write();
        worker.last_heartbeat = Instant::now() - Duration::from_secs(60);
        worker.missed_heartbeats = 2;
    }

    let unhealthy = pool.health_check().unwrap();
    assert_eq!(unhealthy, vec![worker_id]);
}

#[test]
fn auto_restart_replaces_failed_worker_state() {
    let pool = WorkerPool::with_factory(integration_config(4), || {
        Box::new(CountingAgent::new("worker".to_string()))
    })
    .unwrap();
    let worker_id = pool.get_available_worker().unwrap();

    {
        let worker = pool.workers.get(&worker_id).unwrap();
        let mut worker = worker.write();
        worker.status = WorkerStatus::Failed {
            error: "boom".to_string(),
        };
        worker.lifecycle = axora_agents::WorkerLifecycle::Failed;
    }

    pool.lifecycle.restart_worker(&worker_id).unwrap();
    let worker = pool.workers.get(&worker_id).unwrap();
    assert!(matches!(worker.read().status, WorkerStatus::Idle));
}

#[test]
fn resource_limits_are_enforced() {
    let pool = WorkerPool::with_factory(integration_config(3), || {
        Box::new(CountingAgent::new("worker".to_string()))
    })
    .unwrap();
    assert_eq!(pool.workers.len(), 2);
    assert!(pool.spawner.spawn_if_capacity().is_ok());
    assert!(pool.spawner.spawn_if_capacity().is_err());
    assert_eq!(pool.workers.len(), 3);
}

#[test]
fn dispatch_and_status_round_trip() {
    let pool = WorkerPool::with_factory(integration_config(4), || {
        Box::new(CountingAgent::new("worker".to_string()))
    })
    .unwrap();
    let worker_id = pool.get_available_worker().unwrap();
    let task = Task::new("integration-task");
    let task_id = task.id.clone();

    pool.dispatch_task(&worker_id, task).unwrap();

    assert_eq!(
        pool.get_task_status(&worker_id).unwrap(),
        TaskStatus::Completed
    );
    let result = pool.dispatcher.get_result(&task_id).unwrap();
    assert!(result.result.success);
}
