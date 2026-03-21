use std::sync::Arc;
use std::time::Duration;

use openakta_agents::coordinator::v2::{BlackboardV2, Coordinator, CoordinatorConfig};
use openakta_agents::provider_transport::{
    CloudModelRef, ModelRegistryEntry, ModelRegistrySnapshot, ProviderInstanceId,
    ProviderProfileId, ProviderRuntimeBundle, ProviderRuntimeConfig, ResolvedProviderInstance,
    SyntheticTransport,
};
use openakta_agents::Task;
use openakta_agents::{ProviderKind, WireProfile};

fn blackboard() -> Arc<BlackboardV2> {
    Arc::new(BlackboardV2::default())
}

fn test_registry() -> ModelRegistrySnapshot {
    let mut models = std::collections::HashMap::new();
    models.insert(
        "claude-sonnet-4-5".to_string(),
        ModelRegistryEntry {
            name: "claude-sonnet-4-5".to_string(),
            max_context_window: 200_000,
            max_output_tokens: 8_192,
            preferred_instance: Some(ProviderInstanceId("cloud".to_string())),
        },
    );
    ModelRegistrySnapshot {
        models,
        sources: Default::default(),
    }
}

fn base_config() -> CoordinatorConfig {
    CoordinatorConfig {
        default_cloud: Some(CloudModelRef {
            instance_id: ProviderInstanceId("cloud".to_string()),
            model: "claude-sonnet-4-5".to_string(),
            wire_profile: WireProfile::AnthropicMessagesV1,
            telemetry_kind: ProviderKind::Anthropic,
        }),
        provider_bundle: Arc::new(ProviderRuntimeBundle {
            instances: [(
                ProviderInstanceId("cloud".to_string()),
                ResolvedProviderInstance {
                    id: ProviderInstanceId("cloud".to_string()),
                    profile: ProviderProfileId::AnthropicMessagesV1,
                    base_url: "https://api.anthropic.com".to_string(),
                    api_key: None,
                    is_local: false,
                    default_model: Some("claude-sonnet-4-5".to_string()),
                    label: None,
                },
            )]
            .into_iter()
            .collect(),
            http: ProviderRuntimeConfig::default(),
        }),
        registry: Arc::new(test_registry()),
        ..CoordinatorConfig::default()
    }
}

fn test_coordinator(config: CoordinatorConfig, blackboard: Arc<BlackboardV2>) -> Coordinator {
    let workspace_root = config.workspace_root.clone();
    Coordinator::new_with_provider_transport(
        config,
        blackboard,
        ProviderInstanceId("cloud".to_string()),
        Arc::new(SyntheticTransport::new(workspace_root)),
    )
    .expect("coordinator should be created")
}

#[tokio::test]
async fn coordinator_creation_registers_worker_slots() {
    let coordinator = test_coordinator(
        CoordinatorConfig {
            max_workers: 4,
            ..base_config()
        },
        blackboard(),
    );

    assert_eq!(coordinator.worker_registry.len(), 4);
}

#[tokio::test]
async fn get_available_worker_returns_idle_worker() {
    let coordinator = test_coordinator(base_config(), blackboard());

    assert!(coordinator.get_available_worker().is_some());
}

#[tokio::test]
async fn assign_task_marks_worker_busy() {
    let mut coordinator = test_coordinator(base_config(), blackboard());
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
    let mut coordinator = test_coordinator(base_config(), blackboard());

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
    let mut coordinator = test_coordinator(base_config(), Arc::clone(&blackboard));

    let result = coordinator
        .execute_mission("simple workflow")
        .await
        .expect("mission should execute");

    let blackboard = blackboard.lock().await;
    let memories = blackboard.get_accessible("coordinator");

    assert_eq!(memories.len(), result.tasks_completed);
    assert!(memories
        .iter()
        .all(|memory| { memory.content.to_lowercase().contains("completed") }));
}
