use std::sync::Arc;
use std::time::Duration;

use axora_agents::coordinator::v2::{BlackboardV2, Coordinator, CoordinatorConfig};
use axora_agents::provider_transport::SyntheticTransport;
use axora_agents::Task;

fn blackboard() -> Arc<BlackboardV2> {
    Arc::new(BlackboardV2::default())
}

fn test_coordinator(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Coordinator {
    let workspace_root = config.workspace_root.clone();
    Coordinator::new_with_provider_transport(
        config,
        blackboard,
        Arc::new(SyntheticTransport::new(workspace_root)),
    )
    .expect("coordinator should be created")
}

#[tokio::test]
async fn coordinator_creation_registers_worker_slots() {
    let coordinator = test_coordinator(
        CoordinatorConfig {
            max_workers: 4,
            ..CoordinatorConfig::default()
        },
        blackboard(),
    );

    assert_eq!(coordinator.worker_registry.len(), 4);
}

#[tokio::test]
async fn get_available_worker_returns_idle_worker() {
    let coordinator = test_coordinator(CoordinatorConfig::default(), blackboard());

    assert!(coordinator.get_available_worker().is_some());
}

#[tokio::test]
async fn assign_task_marks_worker_busy() {
    let mut coordinator = test_coordinator(CoordinatorConfig::default(), blackboard());
    let worker_id = coordinator
        .get_available_worker()
        .expect("idle worker should exist");
    let task = Task::new("dispatch a task");

    coordinator
        .assign_task(worker_id.clone(), task.clone())
        .expect("assignment should succeed");

    let worker = coordinator
        .worker_registry
        .workers
        .get(&worker_id)
        .expect("worker should exist");
    assert_eq!(worker.current_task.as_deref(), Some(task.id.as_str()));
}

#[tokio::test]
async fn execute_mission_completes_single_task_workflow() {
    let mut coordinator = test_coordinator(CoordinatorConfig::default(), blackboard());

    let result = coordinator
        .execute_mission("simple workflow")
        .await
        .expect("mission should execute");

    assert!(result.success);
    assert!(result.tasks_completed >= 1);
    assert_eq!(result.tasks_failed, 0);
    assert!(result.duration <= Duration::from_secs(5));
}

#[tokio::test]
async fn execute_mission_publishes_result_to_blackboard() {
    let blackboard = blackboard();
    let mut coordinator = test_coordinator(CoordinatorConfig::default(), Arc::clone(&blackboard));

    let result = coordinator
        .execute_mission("simple workflow")
        .await
        .expect("mission should execute");

    let blackboard = blackboard.lock().await;
    let memories = blackboard.get_accessible("coordinator");

    assert_eq!(memories.len(), result.tasks_completed);
    assert!(memories
        .iter()
        .all(|memory| memory.content.contains("completed")));
}
