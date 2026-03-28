//! Batteries-included runtime bootstrap for OPENAKTA CLI flows.

#![allow(clippy::items_after_test_module)]

use anyhow::Context;
use openakta_agents::{
    default_local_transport, local_provider_config_from_instance, BlackboardV2, CloudModelRef,
    Coordinator, CoordinatorConfig, DecomposerConfig, ExecutionTraceEvent, ExecutionTraceRegistry,
    FallbackPolicy, HitlConfig, LocalModelRef, MessageExecutionMode, MessageSurface,
    MissionDecision, MissionDecomposer, MissionGate, MissionGateRequest, MissionHitlGate,
    MissionResult, ModelRegistrySnapshot, ProviderInstanceConfig, ProviderInstanceId,
    ProviderProfileId, ProviderRegistry, ProviderRuntimeBundle, ResponsePreference,
    RuntimeBlackboard, SecretRef, Task, TaskTargetHints,
};
use openakta_indexing::InfluenceGraph;
use openakta_mcp_server::{McpService, McpServiceConfig};
use openakta_proto::mcp::v1::graph_retrieval_service_server::GraphRetrievalServiceServer;
use openakta_proto::mcp::v1::retrieval_service_server::RetrievalServiceServer;
use openakta_proto::mcp::v1::tool_service_server::ToolServiceServer;
use openakta_storage::{Database, DatabaseConfig};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::TcpListenerStream;

use crate::config_resolve::{
    build_model_registry_snapshot, build_provider_bundle, load_project_config,
    load_workspace_overlay, merge_config_layers, resolve_secrets,
};
use crate::control_plane::{
    task_shells_for_decomposed_mission, task_shells_for_direct_task, ControlPlaneRuntime,
    TaskShellSeed, WorkSessionInit, WorkSessionStatus, WorkTaskLane, WorkTaskStatus,
};
use crate::{CoreConfig, DocSyncService, MemoryServices};
use std::collections::HashMap;
use tracing::{info, warn};

/// Runtime bootstrap options for CLI entrypoints.
#[derive(Clone)]
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
    /// Whether hosted API-backed execution is allowed for this runtime.
    pub remote_enabled: bool,
    /// Auth provider used by hosted API clients.
    pub auth_provider: Option<std::sync::Arc<dyn openakta_api_client::AuthProvider>>,
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
            remote_enabled: true,
            auth_provider: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MessageRequest {
    pub message: String,
    pub workspace_root: std::path::PathBuf,
    pub surface: MessageSurface,
    pub response_preference: ResponsePreference,
    pub allow_code_context: bool,
    pub side_effects_allowed: bool,
    pub remote_enabled: bool,
    pub workspace_context_override: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MessageResult {
    pub work_session_id: String,
    pub mission_id: String,
    pub success: bool,
    pub output: String,
    pub mode: MessageExecutionMode,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub duration: Duration,
    pub trace_events: Vec<ExecutionTraceEvent>,
}

/// Running batteries-included OPENAKTA runtime.
pub struct RuntimeBootstrap {
    config: CoreConfig,
    blackboard: Arc<BlackboardV2>,
    trace_registry: Arc<ExecutionTraceRegistry>,
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

        if !options.remote_enabled && config.providers.default_local_instance.is_none() {
            anyhow::bail!(
                "--no-auth requires a configured default local provider instance in openakta.toml"
            );
        }

        if config.providers.instances.is_empty() {
            anyhow::bail!(
                "No provider instances configured. OPENAKTA requires at least one provider \
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
        info!(
            "Initializing main database at: {}",
            config.database_path.display()
        );
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
        let trace_registry = Arc::new(ExecutionTraceRegistry::new(config.execution_log_dir()));
        let hitl_gate = Arc::new(MissionHitlGate::new(
            HitlConfig {
                checkpoint_dir: config.workspace_root.join(".openakta/checkpoints"),
                execution_trace_registry: Some(Arc::clone(&trace_registry)),
                ..Default::default()
            },
            Some((message_bus.clone(), hitl_bus_rx)),
        ));
        let (mcp_addr, mcp_task) =
            start_embedded_mcp_server(&config, Some(&memory_services), Arc::clone(&hitl_gate))
                .await?;
        std::env::set_var("OPENAKTA_MCP_ENDPOINT", format!("http://{}", mcp_addr));

        let blackboard = Arc::new(Mutex::new(RuntimeBlackboard::new()));
        Ok(Self {
            config,
            blackboard,
            trace_registry,
            hitl_gate,
            _mcp_task: mcp_task,
            _memory_handles: memory_handles,
            _doc_sync_handle: doc_sync_handle,
        })
    }

    /// Bootstrap the runtime and handle a user message through the unified intake layer.
    pub async fn handle_message(
        mut options: RuntimeBootstrapOptions,
        request: MessageRequest,
    ) -> anyhow::Result<MessageResult> {
        options.workspace_root = request.workspace_root.clone();
        options.remote_enabled = request.remote_enabled;

        let decision = MissionGate::analyze(&MissionGateRequest {
            message: &request.message,
            workspace_root: &request.workspace_root,
            surface: request.surface,
            response_preference: request.response_preference,
            allow_code_context: request.allow_code_context,
            side_effects_allowed: request.side_effects_allowed,
            workspace_context_override: request.workspace_context_override.clone(),
        })?;

        let work_session_id = uuid::Uuid::new_v4().to_string();
        let control_plane = ControlPlaneRuntime::open(&request.workspace_root)?;
        control_plane.admit_session(&WorkSessionInit {
            session_id: work_session_id.clone(),
            workspace_root: request.workspace_root.clone(),
            request_text: request.message.clone(),
            surface: request.surface,
            response_preference: request.response_preference,
            allow_code_context: request.allow_code_context,
            side_effects_allowed: request.side_effects_allowed,
            remote_enabled: request.remote_enabled,
            decision: decision.clone(),
        })?;
        control_plane.materialize_initial_retrieval(
            &work_session_id,
            &request.message,
            &decision,
        )?;

        let (runtime, mut coordinator) =
            match Self::build_runtime_and_coordinator(options, &work_session_id).await {
                Ok(result) => result,
                Err(err) => {
                    if let Err(store_err) = control_plane.finalize_failure(
                        &work_session_id,
                        None,
                        Some(work_session_id.as_str()),
                        &err.to_string(),
                        &[],
                    ) {
                        warn!(
                            work_session_id = %work_session_id,
                            error = %store_err,
                            "failed to persist control-plane bootstrap failure"
                        );
                    }
                    return Err(err);
                }
            };

        let mission_result = Self::execute_with_control_plane(
            &control_plane,
            &mut coordinator,
            &request,
            &decision,
            &work_session_id,
        )
        .await;

        match mission_result {
            Ok(mission_result) => {
                let trace_events = trace_snapshot_for_session(&runtime, &work_session_id);
                control_plane.reconcile_trace_events(&work_session_id, &trace_events)?;
                let finalized = control_plane.finalize_outcome(
                    &work_session_id,
                    Some(mission_result.mission_id.as_str()),
                    Some(work_session_id.as_str()),
                    &mission_result.output,
                    &trace_events,
                    mission_result.duration.as_millis(),
                    mission_result.success,
                )?;
                let snapshot = control_plane
                    .snapshot_session(&work_session_id)?
                    .context("work session disappeared before result rendering")?;
                let (tasks_completed, tasks_failed) = execution_task_counts(&snapshot);

                Ok(MessageResult {
                    work_session_id,
                    mission_id: mission_result.mission_id,
                    success: finalized.status == WorkSessionStatus::Completed,
                    output: mission_result.output,
                    mode: decision.mode,
                    tasks_completed,
                    tasks_failed,
                    duration: mission_result.duration,
                    trace_events,
                })
            }
            Err(err) => {
                let trace_events = trace_snapshot_for_session(&runtime, &work_session_id);
                let mission_id = mission_id_from_trace_events(&trace_events);
                if let Err(store_err) = control_plane.finalize_failure(
                    &work_session_id,
                    mission_id.as_deref(),
                    Some(work_session_id.as_str()),
                    &err.to_string(),
                    &trace_events,
                ) {
                    warn!(
                        work_session_id = %work_session_id,
                        error = %store_err,
                        "failed to persist control-plane execution failure"
                    );
                }
                Err(err)
            }
        }
    }

    /// Bootstrap the runtime and execute a mission immediately.
    pub async fn run_mission(
        options: RuntimeBootstrapOptions,
        mission: &str,
    ) -> anyhow::Result<MissionResult> {
        let result = Self::handle_message(
            options.clone(),
            MessageRequest {
                message: mission.to_string(),
                workspace_root: options.workspace_root.clone(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: options.remote_enabled,
                workspace_context_override: None,
            },
        )
        .await?;

        Ok(MissionResult {
            mission_id: result.mission_id,
            success: result.success,
            output: result.output,
            tasks_completed: result.tasks_completed,
            tasks_failed: result.tasks_failed,
            duration: result.duration,
            trace_events: result.trace_events,
        })
    }

    pub async fn ask_local(
        options: RuntimeBootstrapOptions,
        prompt: &str,
        workspace_context: Option<String>,
    ) -> anyhow::Result<String> {
        let result = Self::handle_message(
            options.clone(),
            MessageRequest {
                message: prompt.to_string(),
                workspace_root: options.workspace_root.clone(),
                surface: MessageSurface::CliAsk,
                response_preference: ResponsePreference::PreferDirectReply,
                allow_code_context: workspace_context.is_some(),
                side_effects_allowed: false,
                remote_enabled: options.remote_enabled,
                workspace_context_override: workspace_context,
            },
        )
        .await?;
        Ok(result.output)
    }

    async fn build_runtime_and_coordinator(
        options: RuntimeBootstrapOptions,
        session_id: &str,
    ) -> anyhow::Result<(Self, Coordinator)> {
        let runtime = Self::new(options.clone()).await?;
        let trace_service = runtime
            .trace_registry
            .create_session(session_id.to_string(), true)?;
        let local_only = !options.remote_enabled;
        let local_task_timeout = runtime
            .config
            .provider_runtime
            .timeout
            .max(Duration::from_secs(90));
        let secrets = resolve_secrets(&runtime.config.workspace_root, &runtime.config.providers)?;
        let provider_bundle = Arc::new(build_provider_bundle(&runtime.config, &secrets)?);
        let model_registry = Arc::new(build_model_registry_snapshot(&runtime.config).await?);
        let provider_registry = Arc::new(build_provider_registry(
            &runtime.config,
            Arc::clone(&provider_bundle),
            Arc::clone(&model_registry),
            options.auth_provider.clone(),
        )?);
        let mut coordinator = Coordinator::new(
            CoordinatorConfig {
                max_workers: if local_only { 1 } else { 5 },
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
                task_timeout: if local_only {
                    local_task_timeout
                } else {
                    Duration::from_secs(5)
                },
                hitl_gate: Some(Arc::clone(&runtime.hitl_gate)),
                mcp_endpoint: std::env::var("OPENAKTA_MCP_ENDPOINT").ok(),
                context_use_ratio: runtime.config.provider_context_use_ratio,
                context_margin_tokens: runtime.config.provider_context_margin_tokens,
                retrieval_share: runtime.config.provider_retrieval_share,
                execution_tracer: Some(trace_service),
                execution_trace_registry: Some(Arc::clone(&runtime.trace_registry)),
                mol: runtime.config.mol,
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
        Ok((runtime, coordinator))
    }

    async fn execute_with_control_plane(
        control_plane: &ControlPlaneRuntime,
        coordinator: &mut Coordinator,
        request: &MessageRequest,
        decision: &MissionDecision,
        work_session_id: &str,
    ) -> anyhow::Result<MissionResult> {
        match decision.mode {
            MessageExecutionMode::DirectReply => {
                let task = Task::new(&request.message).with_task_type(decision.task_type.clone());
                let task_shells = task_shells_for_direct_task(
                    work_session_id,
                    &task,
                    &decision.target_hints,
                    decision.retrieval_plan.workspace_context.is_some(),
                );
                control_plane.register_task_shells(work_session_id, &task_shells)?;
                control_plane.mark_session_executing(work_session_id)?;
                control_plane.mark_task_state(
                    work_session_id,
                    &task.id,
                    WorkTaskStatus::Running,
                    None,
                )?;
                coordinator
                    .execute_direct_reply_for_task(
                        task,
                        &request.message,
                        &decision.target_hints,
                        decision.retrieval_plan.workspace_context.clone(),
                    )
                    .await
                    .map_err(anyhow::Error::msg)
            }
            MessageExecutionMode::DirectAction | MessageExecutionMode::SingleAgent => {
                let task = Task::new(&request.message).with_task_type(decision.task_type.clone());
                let task_shells = task_shells_for_direct_task(
                    work_session_id,
                    &task,
                    &decision.target_hints,
                    decision.retrieval_plan.workspace_context.is_some(),
                );
                control_plane.register_task_shells(work_session_id, &task_shells)?;
                control_plane.mark_session_executing(work_session_id)?;
                execute_authoritative_task_graph(control_plane, coordinator, work_session_id).await
            }
            MessageExecutionMode::MultiStep | MessageExecutionMode::Delegated => {
                let mut planning_task = TaskShellSeed::planning(
                    work_session_id,
                    format!("Plan request: {}", request.message),
                );
                if decision.retrieval_plan.workspace_context.is_some() {
                    planning_task
                        .depends_on_task_ids
                        .push(crate::control_plane::search_task_id(work_session_id));
                }
                control_plane.register_task_shells(work_session_id, &[planning_task.clone()])?;
                control_plane.plan_started(work_session_id)?;
                control_plane.mark_task_state(
                    work_session_id,
                    &planning_task.task_id,
                    WorkTaskStatus::Running,
                    None,
                )?;

                let decomposed = build_decomposed_mission(
                    &request.message,
                    decision.decomposition_budget.clone(),
                )
                .await
                .map_err(anyhow::Error::msg);
                let decomposed = match decomposed {
                    Ok(decomposed) => decomposed,
                    Err(err) => {
                        let err_message = err.to_string();
                        control_plane.mark_task_state(
                            work_session_id,
                            &planning_task.task_id,
                            WorkTaskStatus::FailedTerminal,
                            Some(err_message.as_str()),
                        )?;
                        return Err(err);
                    }
                };

                control_plane.record_decomposition(work_session_id, &decomposed)?;
                control_plane.mark_task_state(
                    work_session_id,
                    &planning_task.task_id,
                    WorkTaskStatus::Done,
                    None,
                )?;
                control_plane.register_task_shells(
                    work_session_id,
                    &task_shells_for_decomposed_mission(work_session_id, &decomposed),
                )?;
                control_plane.mark_session_executing(work_session_id)?;
                execute_authoritative_task_graph(control_plane, coordinator, work_session_id).await
            }
        }
    }
}

async fn execute_authoritative_task_graph(
    control_plane: &ControlPlaneRuntime,
    coordinator: &mut Coordinator,
    work_session_id: &str,
) -> anyhow::Result<MissionResult> {
    let started_at = Instant::now();
    let mut outputs = Vec::new();
    let mut last_mission_id: Option<String> = None;
    let mut task_failures = 0usize;
    let mut task_successes = 0usize;

    loop {
        let Some(task_record) = control_plane.reserve_next_dispatchable_task(work_session_id)?
        else {
            break;
        };

        let task = task_from_record(&task_record);
        let hints = TaskTargetHints {
            target_files: task_record.target_files.clone(),
            target_symbols: task_record.target_symbols.clone(),
        };

        match coordinator.execute_single_task(task, &hints).await {
            Ok(task_result) => {
                last_mission_id = Some(task_result.mission_id.clone());
                if !task_result.output.trim().is_empty() {
                    outputs.push(task_result.output.clone());
                }
                if task_result.success {
                    task_successes += 1;
                    control_plane.finalize_runtime_task_success(
                        work_session_id,
                        &task_record.task_id,
                        &task_result.output,
                        Some(task_result.mission_id.as_str()),
                        Some(work_session_id),
                    )?;
                } else {
                    task_failures += 1;
                    control_plane.finalize_runtime_task_failure(
                        work_session_id,
                        &task_record.task_id,
                        &task_failure_message(&task_record, Some(&task_result), None),
                        Some(task_result.mission_id.as_str()),
                        Some(work_session_id),
                    )?;
                }
            }
            Err(err) => {
                task_failures += 1;
                outputs.push(format!("{} failed: {}", task_record.title, err));
                control_plane.finalize_runtime_task_failure(
                    work_session_id,
                    &task_record.task_id,
                    &task_failure_message(&task_record, None, Some(&err)),
                    last_mission_id.as_deref(),
                    Some(work_session_id),
                )?;
            }
        }
    }

    Ok(MissionResult {
        mission_id: last_mission_id.unwrap_or_else(|| work_session_id.to_string()),
        success: task_failures == 0,
        output: outputs.join("\n"),
        tasks_completed: task_successes,
        tasks_failed: task_failures,
        duration: started_at.elapsed(),
        trace_events: Vec::new(),
    })
}

async fn build_decomposed_mission(
    mission: &str,
    decomposition_budget: openakta_agents::DecompositionBudget,
) -> openakta_agents::Result<openakta_agents::DecomposedMission> {
    MissionDecomposer::new_with_config(
        Arc::new(InfluenceGraph::new()),
        DecomposerConfig {
            max_tasks: decomposition_budget.max_tasks,
            max_parallelism: decomposition_budget.max_parallelism,
            ..DecomposerConfig::default()
        },
    )
    .decompose_async(mission)
    .await
}

fn trace_snapshot_for_session(
    runtime: &RuntimeBootstrap,
    session_id: &str,
) -> Vec<ExecutionTraceEvent> {
    runtime
        .trace_registry
        .service(session_id)
        .map(|service| service.snapshot())
        .unwrap_or_default()
}

fn mission_id_from_trace_events(trace_events: &[ExecutionTraceEvent]) -> Option<String> {
    trace_events
        .iter()
        .rev()
        .find(|event| !event.mission_id.is_empty())
        .map(|event| event.mission_id.clone())
}

fn task_from_record(task: &crate::control_plane::WorkSessionTaskRecord) -> Task {
    Task {
        id: task.task_id.clone(),
        description: task.title.clone(),
        priority: match task.lane {
            WorkTaskLane::Validation => openakta_agents::Priority::High,
            WorkTaskLane::Execution => openakta_agents::Priority::Normal,
            WorkTaskLane::Search | WorkTaskLane::Planning => openakta_agents::Priority::Low,
        },
        status: openakta_agents::TaskStatus::Pending,
        assigned_to: None,
        parent_task: task.parent_task_id.clone(),
        task_type: task.task_type.clone(),
    }
}

fn task_failure_message(
    task: &crate::control_plane::WorkSessionTaskRecord,
    mission_result: Option<&MissionResult>,
    error: Option<&openakta_agents::CoordinatorV2Error>,
) -> String {
    if let Some(mission_result) = mission_result {
        if !mission_result.output.trim().is_empty() {
            return mission_result.output.clone();
        }
    }
    if let Some(error) = error {
        return error.to_string();
    }
    format!("{} did not complete successfully", task.title)
}

fn execution_task_counts(snapshot: &crate::control_plane::WorkSessionSnapshot) -> (usize, usize) {
    let completed = snapshot
        .tasks
        .iter()
        .filter(|task| task.lane == WorkTaskLane::Execution && task.status == WorkTaskStatus::Done)
        .count();
    let failed = snapshot
        .tasks
        .iter()
        .filter(|task| task.lane == WorkTaskLane::Execution && task.status != WorkTaskStatus::Done)
        .count();
    (completed, failed)
}

fn resolve_workspace_config(workspace_root: &std::path::Path) -> anyhow::Result<CoreConfig> {
    let config_path = workspace_root.join("openakta.toml");
    let defaults = CoreConfig::for_workspace(workspace_root.to_path_buf());
    let workspace = load_workspace_overlay()?;
    if config_path.exists() {
        let project = load_project_config(&config_path)?;
        merge_config_layers(defaults, workspace, project)
    } else {
        match workspace {
            Some(workspace) => merge_config_layers(defaults.clone(), Some(workspace), defaults),
            None => Ok(defaults),
        }
    }
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
        let default_base_url = "http://127.0.0.1:11434".to_string();
        let inferred_secret = infer_override_secret(options.local_base_url.as_deref());
        let instance = config
            .providers
            .instances
            .entry(instance_id.clone())
            .or_insert_with(|| ProviderInstanceConfig {
                profile: ProviderProfileId::OpenAiCompatible,
                base_url: options
                    .local_base_url
                    .clone()
                    .unwrap_or_else(|| default_base_url.clone()),
                secret: inferred_secret.clone(),
                is_local: true,
                default_model: options.local_model.clone(),
                label: None,
            });
        instance.is_local = true;
        if let Some(model) = &options.local_model {
            instance.default_model = Some(model.clone());
        }
        if let Some(url) = &options.local_base_url {
            instance.base_url = url.clone();
        }
        if instance.secret == SecretRef::default() {
            instance.secret = inferred_secret;
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
    if !options.remote_enabled {
        config.providers.default_cloud_instance = None;
        config.routing_enabled = false;
        config.fallback_policy = FallbackPolicy::Never;
    }
}

fn infer_override_secret(base_url: Option<&str>) -> SecretRef {
    let Some(base_url) = base_url else {
        return SecretRef::default();
    };

    if base_url.contains("openrouter.ai") {
        if let Ok(api_key) = std::env::var("OPENROUTER_API_KEY") {
            if !api_key.trim().is_empty() {
                return SecretRef {
                    api_key: Some(api_key),
                    api_key_file: None,
                };
            }
        }

        if let Ok(current_dir) = std::env::current_dir() {
            let candidate = current_dir.join(".openakta/secrets/openrouter.key");
            if candidate.exists() {
                return SecretRef {
                    api_key: None,
                    api_key_file: Some(candidate),
                };
            }
        }
    }

    SecretRef::default()
}

fn build_provider_registry(
    config: &CoreConfig,
    bundle: Arc<ProviderRuntimeBundle>,
    model_registry: Arc<ModelRegistrySnapshot>,
    auth_provider: Option<Arc<dyn openakta_api_client::AuthProvider>>,
) -> anyhow::Result<ProviderRegistry> {
    let mut local = HashMap::new();
    // Phase 5+: Build local transports only (cloud execution uses API client)
    for (instance_id, instance) in &bundle.instances {
        if instance.is_local {
            let local_config = local_provider_config_from_instance(
                instance,
                vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
            );
            local.insert(
                instance_id.clone(),
                Arc::from(default_local_transport(
                    &local_config,
                    config.provider_runtime.timeout,
                )?),
            );
        }
        // Phase 5+: Cloud instances no longer create direct transports
        // Cloud execution now uses API client pool instead
    }

    // Phase 5+: Use new constructor with API client pool
    Ok(ProviderRegistry::new_with_api_client(
        local,
        default_cloud_ref(config, bundle.as_ref()),
        default_local_ref(config, bundle.as_ref()),
        config.fallback_policy,
        bundle,
        model_registry,
        Arc::new(openakta_api_client::ApiClientPool::with_auth_provider(
            openakta_api_client::ClientConfig::default(),
            auth_provider,
        )?),
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
        let (_addr, _jh) = start_embedded_mcp_server(&config, None, Arc::clone(&gate))
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

    #[tokio::test]
    async fn bootstrap_errors_when_no_provider_instances_configured() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("openakta.toml"),
            r#"fallback_policy = "automatic"
routing_enabled = true
"#,
        )
        .unwrap();

        let result = RuntimeBootstrap::new(RuntimeBootstrapOptions {
            workspace_root: tmp.path().to_path_buf(),
            start_background_services: false,
            ..RuntimeBootstrapOptions::default()
        })
        .await;

        let err = match result {
            Ok(_) => panic!("expected error when providers.instances is empty"),
            Err(e) => e,
        };

        let msg = err.to_string();
        assert!(
            msg.contains("No provider instances configured"),
            "unexpected error: {msg}"
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
    memory_services: Option<&MemoryServices>,
    hitl_gate: Arc<MissionHitlGate>,
) -> anyhow::Result<(SocketAddr, JoinHandle<Result<(), tonic::transport::Error>>)> {
    let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))).await?;
    let local_addr = listener.local_addr()?;
    let incoming = TcpListenerStream::new(listener);
    let service_config = McpServiceConfig {
        workspace_root: config.workspace_root.clone(),
        allowed_commands: config.mcp_allowed_commands.clone(),
        default_max_execution_seconds: config.mcp_command_timeout_secs as u32,
        execution_mode: config.execution_mode,
        container_executor: config.container_executor.clone(),
        wasi_executor: config.wasi_executor.clone(),
        mass_refactor_executor: config.mass_refactor_executor.clone(),
        dense_backend: config.retrieval.backend,
        dense_qdrant_url: config.retrieval.qdrant_url.clone(),
        dense_store_path: config.retrieval.sqlite_path.clone(),
        code_collection: config.retrieval.code.collection_spec(),
        code_embedding: config.retrieval.code.embedding_config(),
        code_bm25_dir: config.retrieval.code.bm25_dir.clone(),
        code_index_state_path: config.retrieval.code.index_state_path.clone(),
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
    };
    let service = if let Some(memory_services) = memory_services {
        McpService::with_runtime_retrievers(
            service_config,
            Arc::clone(&memory_services.skill_retrieval),
            Arc::clone(&memory_services.code_retrieval),
        )
    } else {
        McpService::with_config(service_config)
    };
    let task = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(GraphRetrievalServiceServer::new(service.clone()))
            .add_service(RetrievalServiceServer::new(service.clone()))
            .add_service(ToolServiceServer::new(service))
            .serve_with_incoming(incoming)
            .await
    });

    Ok((local_addr, task))
}
