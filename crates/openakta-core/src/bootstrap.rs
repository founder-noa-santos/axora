//! Batteries-included runtime bootstrap for OPENAKTA CLI flows.

#![allow(clippy::items_after_test_module)]

use openakta_agents::{
    default_local_transport, transport_for_instance, BlackboardV2, CloudModelRef, Coordinator,
    CoordinatorConfig, FallbackPolicy, HitlConfig, LocalModelRef, MissionHitlGate, MissionResult,
    ModelRegistrySnapshot, ProviderInstanceId, ProviderRegistry, ProviderRuntimeBundle,
    RuntimeBlackboard,
};
use openakta_mcp_server::{McpService, McpServiceConfig};
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
use openakta_proto::mcp::v1::tool_service_server::ToolServiceServer;
use openakta_storage::{Database, DatabaseConfig};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

use crate::config_resolve::{
    build_model_registry_snapshot, build_provider_bundle, load_project_config,
    load_workspace_overlay, merge_config_layers, resolve_secrets,
};
use crate::{CoreConfig, DocSyncService, MemoryServices};
use std::collections::HashMap;

/// Runtime bootstrap options for CLI entrypoints.
#[derive(Debug, Clone)]
pub struct RuntimeBootstrapOptions {
    /// Workspace root containing the codebase to operate on.
    pub workspace_root: std::path::PathBuf,
    /// Optional cloud instance override.
    pub cloud_instance: Option<ProviderInstanceId>,
    /// Optional cloud model override.
    pub cloud_model: Option<String>,
    /// Optional local instance override.
    pub local_instance: Option<ProviderInstanceId>,
    /// Optional local model override.
    pub local_model: Option<String>,
    /// Optional local runtime URL override.
    pub local_base_url: Option<String>,
    /// Optional fallback policy override.
    pub fallback_policy: Option<FallbackPolicy>,
    /// Optional routing toggle override.
    pub routing_enabled: Option<bool>,
    /// Optional local validation retry budget override.
    pub local_validation_retry_budget: Option<u32>,
    /// Whether to start background memory/doc services.
    pub start_background_services: bool,
}

impl Default for RuntimeBootstrapOptions {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".")),
            cloud_instance: None,
            cloud_model: None,
            local_instance: None,
            local_model: None,
            local_base_url: None,
            fallback_policy: None,
            routing_enabled: None,
            local_validation_retry_budget: None,
            start_background_services: true,
        }
    }
}

/// Running batteries-included OPENAKTA runtime.
pub struct RuntimeBootstrap {
    config: CoreConfig,
    blackboard: Arc<BlackboardV2>,
    /// Shared HITL gate (MCP `request_user_input` + coordinator lifecycle).
    pub hitl_gate: Arc<MissionHitlGate>,
    _mcp_task: JoinHandle<Result<(), tonic::transport::Error>>,
    _memory_handles: Vec<std::thread::JoinHandle<()>>,
    _doc_sync_handle: Option<std::thread::JoinHandle<()>>,
}

impl RuntimeBootstrap {
    /// Bootstrap a ready-to-run runtime for the given options.
    pub async fn new(options: RuntimeBootstrapOptions) -> anyhow::Result<Self> {
        let mut config = resolve_workspace_config(&options.workspace_root)?;
        apply_runtime_overrides(&mut config, &options);
        config.workspace_root = options.workspace_root.clone();
        config.ensure_runtime_layout()?;

        if config.providers.instances.is_empty() {
            panic!(
                "FATAL: No provider instances configured. OPENAKTA requires at least one provider \
                 (cloud or local) to function. Update openakta.toml with provider configuration."
            );
        }

        let secrets = resolve_secrets(&config.workspace_root, &config.providers)?;
        let _provider_bundle = Arc::new(build_provider_bundle(&config, &secrets)?);
        let _model_registry = Arc::new(build_model_registry_snapshot(&config).await?);

        let db = Database::new(DatabaseConfig {
            path: config.database_path.to_string_lossy().to_string(),
            ..Default::default()
        });
        let _conn = db.init()?;

        let memory_services = MemoryServices::new(&config).await?;
        let memory_handles = if options.start_background_services {
            memory_services.start(&config)
        } else {
            Vec::new()
        };
        let doc_sync_handle = if options.start_background_services {
            Some(DocSyncService::start(config.clone()))
        } else {
            None
        };

        let (message_bus, hitl_bus_rx) = tokio::sync::broadcast::channel(1024);
        let hitl_gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                checkpoint_dir: config.workspace_root.join(".openakta/checkpoints"),
                ..Default::default()
            },
            Some((message_bus.clone(), hitl_bus_rx)),
        ));
        let (mcp_addr, mcp_task) =
            start_embedded_mcp_server(&config, Arc::clone(&hitl_gate)).await?;
        std::env::set_var("OPENAKTA_MCP_ENDPOINT", format!("http://{}", mcp_addr));

        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        Ok(Self {
            config,
            blackboard,
            hitl_gate,
            _mcp_task: mcp_task,
            _memory_handles: memory_handles,
            _doc_sync_handle: doc_sync_handle,
        })
    }

    /// Bootstrap the runtime and execute a mission immediately.
    pub async fn run_mission(
        options: RuntimeBootstrapOptions,
        mission: &str,
    ) -> anyhow::Result<MissionResult> {
        let runtime = Self::new(options.clone()).await?;
        let secrets = resolve_secrets(&runtime.config.workspace_root, &runtime.config.providers)?;
        let provider_bundle = Arc::new(build_provider_bundle(&runtime.config, &secrets)?);
        let model_registry = Arc::new(build_model_registry_snapshot(&runtime.config).await?);
        let provider_registry = Arc::new(build_provider_registry(
            &runtime.config,
            Arc::clone(&provider_bundle),
            Arc::clone(&model_registry),
        )?);
        let mut coordinator = Coordinator::new(
            CoordinatorConfig {
                default_cloud: default_cloud_ref(&runtime.config, provider_bundle.as_ref()),
                default_local: default_local_ref(&runtime.config, provider_bundle.as_ref()),
                model_instance_priority: runtime.config.providers.model_instance_priority.clone(),
                provider_bundle,
                registry: model_registry,
                fallback_policy: runtime.config.fallback_policy,
                routing_enabled: runtime.config.routing_enabled
                    || (runtime.config.providers.default_cloud_instance.is_some()
                        && runtime.config.providers.default_local_instance.is_some()),
                local_validation_retry_budget: runtime.config.local_validation_retry_budget,
                local_enabled_for: vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
                workspace_root: runtime.config.workspace_root.clone(),
                hitl_gate: Some(Arc::clone(&runtime.hitl_gate)),
                context_use_ratio: runtime.config.provider_context_use_ratio,
                context_margin_tokens: runtime.config.provider_context_margin_tokens,
                retrieval_share: runtime.config.provider_retrieval_share,
                ..Default::default()
            },
            Arc::clone(&runtime.blackboard),
        )
        .map_err(anyhow::Error::msg)?;
        coordinator = Coordinator::new_with_provider_registry(
            coordinator.config.clone(),
            Arc::clone(&runtime.blackboard),
            provider_registry,
        )
        .map_err(anyhow::Error::msg)?;

        coordinator
            .execute_mission(mission)
            .await
            .map_err(anyhow::Error::msg)
    }
}

fn resolve_workspace_config(workspace_root: &std::path::Path) -> anyhow::Result<CoreConfig> {
    let config_path = workspace_root.join("openakta.toml");
    let defaults = CoreConfig::for_workspace(workspace_root.to_path_buf());
    let project = load_project_config(&config_path)?;
    let workspace = load_workspace_overlay()?;
    merge_config_layers(defaults, workspace, project)
}

fn apply_runtime_overrides(config: &mut CoreConfig, options: &RuntimeBootstrapOptions) {
    if let Some(instance_id) = options.cloud_instance.clone() {
        config.providers.default_cloud_instance = Some(instance_id.clone());
        if let Some(model) = &options.cloud_model {
            if let Some(instance) = config.providers.instances.get_mut(&instance_id) {
                instance.default_model = Some(model.clone());
            }
        }
    }
    if let Some(instance_id) = options.local_instance.clone() {
        config.providers.default_local_instance = Some(instance_id.clone());
        if let Some(instance) = config.providers.instances.get_mut(&instance_id) {
            if let Some(model) = &options.local_model {
                instance.default_model = Some(model.clone());
            }
            if let Some(url) = &options.local_base_url {
                instance.base_url = url.clone();
            }
        }
    }
    if let Some(policy) = options.fallback_policy {
        config.fallback_policy = policy;
    }
    if let Some(routing_enabled) = options.routing_enabled {
        config.routing_enabled = routing_enabled;
    }
    if let Some(retry_budget) = options.local_validation_retry_budget {
        config.local_validation_retry_budget = retry_budget;
    }
}

fn build_provider_registry(
    config: &CoreConfig,
    bundle: Arc<ProviderRuntimeBundle>,
    model_registry: Arc<ModelRegistrySnapshot>,
) -> anyhow::Result<ProviderRegistry> {
    let mut cloud = HashMap::new();
    let mut local = HashMap::new();
    for (instance_id, instance) in &bundle.instances {
        if instance.is_local {
            let local_config = openakta_agents::LocalProviderConfig {
                provider: openakta_agents::LocalProviderKind::Ollama,
                base_url: instance.base_url.clone(),
                default_model: instance
                    .default_model
                    .clone()
                    .unwrap_or_else(|| "qwen2.5-coder:7b".to_string()),
                enabled_for: vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
            };
            local.insert(
                instance_id.clone(),
                Arc::from(default_local_transport(
                    &local_config,
                    config.provider_runtime.timeout,
                )?),
            );
        } else {
            cloud.insert(
                instance_id.clone(),
                Arc::from(transport_for_instance(instance, &config.provider_runtime)?),
            );
        }
    }

    Ok(ProviderRegistry::new(
        cloud,
        local,
        default_cloud_ref(config, bundle.as_ref()),
        default_local_ref(config, bundle.as_ref()),
        config.fallback_policy,
        bundle,
        model_registry,
    ))
}

fn default_cloud_ref(config: &CoreConfig, bundle: &ProviderRuntimeBundle) -> Option<CloudModelRef> {
    let instance_id = config.providers.default_cloud_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    let model = instance.default_model.clone()?;
    Some(CloudModelRef {
        instance_id,
        model,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

fn default_local_ref(config: &CoreConfig, bundle: &ProviderRuntimeBundle) -> Option<LocalModelRef> {
    let instance_id = config.providers.default_local_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    let model = instance.default_model.clone()?;
    Some(LocalModelRef {
        instance_id,
        model,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

#[allow(dead_code)]
fn parse_fallback_policy(value: &str) -> FallbackPolicy {
    match value.to_ascii_lowercase().as_str() {
        "never" => FallbackPolicy::Never,
        "automatic" => FallbackPolicy::Automatic,
        _ => FallbackPolicy::Explicit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_agents::{ProviderInstanceConfig, ProviderInstanceId, ProviderProfileId};
    use std::sync::Arc;

    #[tokio::test]
    #[allow(clippy::field_reassign_with_default)]
    async fn hitl_gate_is_shared_between_mcp_and_coordinator() {
        let tmp = tempfile::tempdir().unwrap();
        let config = CoreConfig::for_workspace(tmp.path().to_path_buf());
        let (bus, bus_rx) = tokio::sync::broadcast::channel(8);
        let gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                checkpoint_dir: tmp.path().join(".openakta/cp"),
                ..Default::default()
            },
            Some((bus.clone(), bus_rx)),
        ));
        let (_addr, _jh) = start_embedded_mcp_server(&config, Arc::clone(&gate))
            .await
            .unwrap();

        let mut coord_cfg = CoordinatorConfig::default();
        coord_cfg.provider_bundle = Arc::new(ProviderRuntimeBundle {
            instances: [(
                ProviderInstanceId("local".to_string()),
                openakta_agents::ResolvedProviderInstance {
                    id: ProviderInstanceId("local".to_string()),
                    profile: ProviderProfileId::OpenAiCompatible,
                    base_url: "http://127.0.0.1:11434".to_string(),
                    api_key: None,
                    is_local: true,
                    default_model: Some("llama3".to_string()),
                    label: None,
                },
            )]
            .into_iter()
            .collect(),
            http: openakta_agents::ProviderRuntimeConfig::default(),
        });
        coord_cfg.default_local = Some(LocalModelRef {
            instance_id: ProviderInstanceId("local".to_string()),
            model: "llama3".to_string(),
            wire_profile: openakta_agents::WireProfile::OpenAiChatCompletions,
            telemetry_kind: openakta_agents::ProviderKind::OpenAi,
        });
        coord_cfg.workspace_root = tmp.path().to_path_buf();
        coord_cfg.hitl_gate = Some(Arc::clone(&gate));
        let bb = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        let coord = Coordinator::new(coord_cfg, bb).unwrap();
        assert!(Arc::ptr_eq(coord.config.hitl_gate.as_ref().unwrap(), &gate));
    }

    #[test]
    fn runtime_overrides_apply_local_lane_without_forcing_cloud() {
        let mut config = CoreConfig::file_defaults();
        config.providers.instances.insert(
            ProviderInstanceId("local".to_string()),
            ProviderInstanceConfig {
                profile: ProviderProfileId::OpenAiCompatible,
                base_url: "http://127.0.0.1:11434".to_string(),
                secret: openakta_agents::SecretRef::default(),
                is_local: true,
                default_model: Some("qwen2.5-coder:7b".to_string()),
                label: None,
            },
        );
        let options = RuntimeBootstrapOptions {
            workspace_root: std::path::PathBuf::from("."),
            local_instance: Some(ProviderInstanceId("local".to_string())),
            local_model: Some("qwen2.5-coder:7b".to_string()),
            local_base_url: Some("http://127.0.0.1:11434".to_string()),
            fallback_policy: Some(FallbackPolicy::Automatic),
            routing_enabled: Some(true),
            ..RuntimeBootstrapOptions::default()
        };

        apply_runtime_overrides(&mut config, &options);

        assert!(config.providers.default_cloud_instance.is_none());
        assert_eq!(config.fallback_policy, FallbackPolicy::Automatic);
        assert!(config.routing_enabled);
        assert_eq!(
            config
                .providers
                .instances
                .get(&ProviderInstanceId("local".to_string()))
                .and_then(|local| local.default_model.as_deref()),
            Some("qwen2.5-coder:7b")
        );
    }

    #[test]
    fn runtime_overrides_can_select_cloud_lane_explicitly() {
        let mut config = CoreConfig::default();
        config.providers.instances.insert(
            ProviderInstanceId("cloud".to_string()),
            ProviderInstanceConfig {
                profile: ProviderProfileId::OpenAiChatCompletions,
                base_url: "https://api.openai.com".to_string(),
                secret: openakta_agents::SecretRef::default(),
                is_local: false,
                default_model: Some("gpt-5.4-mini".to_string()),
                label: None,
            },
        );
        let options = RuntimeBootstrapOptions {
            workspace_root: std::path::PathBuf::from("."),
            cloud_instance: Some(ProviderInstanceId("cloud".to_string())),
            cloud_model: Some("gpt-5.4".to_string()),
            ..RuntimeBootstrapOptions::default()
        };

        apply_runtime_overrides(&mut config, &options);

        assert_eq!(
            config
                .providers
                .instances
                .get(&ProviderInstanceId("cloud".to_string()))
                .and_then(|cloud| cloud.default_model.as_deref()),
            Some("gpt-5.4")
        );
    }

    #[test]
    fn fallback_policy_parser_is_case_insensitive() {
        assert_eq!(
            parse_fallback_policy("automatic"),
            FallbackPolicy::Automatic
        );
        assert_eq!(parse_fallback_policy("NEVER"), FallbackPolicy::Never);
        assert_eq!(
            parse_fallback_policy("anything-else"),
            FallbackPolicy::Explicit
        );
    }

    #[test]
    fn runtime_overrides_apply_local_retry_budget() {
        let mut config = CoreConfig::file_defaults();
        let options = RuntimeBootstrapOptions {
            workspace_root: std::path::PathBuf::from("."),
            local_validation_retry_budget: Some(4),
            ..RuntimeBootstrapOptions::default()
        };

        apply_runtime_overrides(&mut config, &options);

        assert_eq!(config.local_validation_retry_budget, 4);
    }
}

async fn start_embedded_mcp_server(
    config: &CoreConfig,
    hitl_gate: Arc<MissionHitlGate>,
) -> anyhow::Result<(SocketAddr, JoinHandle<Result<(), tonic::transport::Error>>)> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
    let local_addr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);
    let service = McpService::with_config(McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
        execution_mode: config.execution_mode,
        container_executor: config.container_executor.clone(),
        wasi_executor: config.wasi_executor.clone(),
        dense_backend: config.retrieval.backend,
        dense_qdrant_url: config.retrieval.qdrant_url.clone(),
        dense_store_path: config.retrieval.sqlite_path.clone(),
        code_collection: config.retrieval.code.collection_spec(),
        code_embedding: config.retrieval.code.embedding_config(),
        code_retrieval_budget_tokens: config.retrieval.code.token_budget,
        skill_config: openakta_memory::SkillRetrievalConfig {
            corpus_root: config.retrieval.skills.corpus_root.clone(),
            catalog_db_path: config.retrieval.skills.catalog_db_path.clone(),
            dense_backend: config.retrieval.backend,
            dense_store_path: config.retrieval.sqlite_path.clone(),
            qdrant_url: config.retrieval.qdrant_url.clone(),
            dense_collection: config.retrieval.skills.collection_spec(),
            embedding: config.retrieval.skills.embedding_config(),
            bm25_dir: config.retrieval.skills.bm25_dir.clone(),
            skill_token_budget: config.retrieval.skills.token_budget,
            dense_limit: 64,
            bm25_limit: 64,
        },
        hitl_gate: Some(hitl_gate),
    });
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GraphRetrievalServiceServer::new(service.clone()))
            .add_service(ToolServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
    });

    Ok((local_addr, task))
}
