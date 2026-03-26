use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use openakta_agents::hitl::MissionHitlGate;
use openakta_agents::{
    default_local_transport, local_provider_config_from_instance, CloudModelRef, Coordinator,
    CoordinatorConfig, ExecutionTraceRegistry, ExecutionTraceService, LocalModelRef,
    ModelRegistrySnapshot, ProviderRegistry, ProviderRuntimeBundle, RuntimeBlackboard,
};
use openakta_api_client::{
    ApiClientPool, ClientConfig, CommandEnvelope, EnvAuthProvider, EvidenceLinkView,
    ReadModelResponse,
};
use openakta_core::config_resolve::{
    build_model_registry_snapshot, build_provider_bundle, resolve_secrets,
};
use openakta_core::CoreConfig;
use openakta_proto::work::v1::work_management_service_server::WorkManagementService;
use openakta_proto::work::v1::{
    ClarificationAnswer, ClarificationItem, CyclePhase, DependencyEdge, EvidenceLink,
    GetBoardRequest, GetBoardResponse, GetClarificationQueueRequest, GetClarificationQueueResponse,
    GetPlanVersionRequest, GetPlanVersionResponse, ListEvidenceRequest, ListEvidenceResponse,
    ListWorkItemsRequest, ListWorkItemsResponse, PlanningCycle, PlanVersion,
    ResolveClarificationsRequest, ResolveClarificationsResponse, StartExecutionRequest,
    StartExecutionResponse, SubmitCommandRequest, SubmitCommandResponse, WorkItem, Workspace,
};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::error;
use uuid::Uuid;

use crate::background::work_mirror::{StoredReadModel, WorkMirror};
use crate::background::work_plan_compiler::{compile_work_plan, CompiledWorkPlan};

#[derive(Clone)]
pub struct WorkManagementGrpc {
    mirror: WorkMirror,
    api_client_pool: &'static ApiClientPool,
    config: CoreConfig,
    trace_registry: Arc<ExecutionTraceRegistry>,
    hitl_gate: Arc<MissionHitlGate>,
    #[allow(dead_code)]
    workspace_root: PathBuf,
}

impl WorkManagementGrpc {
    pub fn open(
        mirror: WorkMirror,
        api_client_pool: &'static ApiClientPool,
        config: CoreConfig,
        trace_registry: Arc<ExecutionTraceRegistry>,
        hitl_gate: Arc<MissionHitlGate>,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            mirror,
            api_client_pool,
            config,
            trace_registry,
            hitl_gate,
            workspace_root,
        }
    }

    async fn synced_read_model(&self, workspace_id: Uuid) -> Result<StoredReadModel, Status> {
        match self.api_client_pool.completion_client.get_work_read_model(workspace_id).await {
            Ok(read_model) => {
                let etag = hash_read_model(&read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .upsert_read_model(workspace_id, &etag, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                Ok(StoredReadModel {
                    checkpoint_seq: read_model.checkpoint_seq,
                    model: read_model,
                    etag,
                })
            }
            Err(err) => self
                .mirror
                .read_model(workspace_id)
                .map_err(|mirror_err| Status::internal(mirror_err.to_string()))?
                .ok_or_else(|| Status::unavailable(err.to_string())),
        }
    }

    async fn execute_compiled_plan(
        &self,
        workspace_id: Uuid,
        compiled: CompiledWorkPlan,
    ) -> Result<()> {
        self.mirror
            .mark_items_executing(workspace_id, &compiled.work_item_ids)
            .context("mark work items executing")?;

        let session_id = format!("work-{}", compiled.mission_id);
        let trace_service = self
            .trace_registry
            .create_session(session_id.clone(), true)
            .context("create execution trace session")?;
        let trace_log_path = trace_service.log_path().to_path_buf();

        let mut coordinator = self
            .build_coordinator(trace_service)
            .await
            .context("build coordinator for compiled work plan")?;
        let mission_result = coordinator
            .execute_decomposed_mission(compiled.mission.clone())
            .await
            .map_err(|err| anyhow!(err.to_string()));

        match mission_result {
            Ok(result) => {
                self.mirror
                    .mark_items_execution_succeeded(workspace_id, &compiled.work_item_ids)
                    .context("mark work items succeeded")?;
                self.mirror
                    .append_evidence(&EvidenceLinkView {
                        id: Uuid::new_v4(),
                        workspace_id,
                        subject_type: "mission".to_string(),
                        subject_id: None,
                        artifact_kind: "execution_trace".to_string(),
                        locator_json: serde_json::json!({
                            "mission_id": result.mission_id,
                            "session_id": session_id,
                            "trace_log_path": trace_log_path,
                        }),
                        content_hash: hash_bytes(trace_log_path.to_string_lossy().as_bytes()),
                        storage_scope: "local".to_string(),
                        preview_redacted: Some("Execution trace stored locally".to_string()),
                        created_at: Utc::now(),
                    })
                    .context("store execution trace evidence")?;
                self.mirror
                    .append_evidence(&EvidenceLinkView {
                        id: Uuid::new_v4(),
                        workspace_id,
                        subject_type: "mission".to_string(),
                        subject_id: None,
                        artifact_kind: "mission_result".to_string(),
                        locator_json: serde_json::json!({
                            "mission_id": result.mission_id,
                            "success": result.success,
                            "tasks_completed": result.tasks_completed,
                            "tasks_failed": result.tasks_failed,
                            "duration_ms": result.duration.as_millis(),
                            "trace_event_count": result.trace_events.len(),
                        }),
                        content_hash: hash_bytes(result.output.as_bytes()),
                        storage_scope: "local".to_string(),
                        preview_redacted: Some(redact_preview(&result.output)),
                        created_at: Utc::now(),
                    })
                    .context("store mission result evidence")?;
                Ok(())
            }
            Err(err) => {
                self.mirror
                    .mark_items_execution_failed(workspace_id, &compiled.work_item_ids)
                    .context("mark work items failed")?;
                self.mirror
                    .append_evidence(&EvidenceLinkView {
                        id: Uuid::new_v4(),
                        workspace_id,
                        subject_type: "mission".to_string(),
                        subject_id: None,
                        artifact_kind: "mission_failure".to_string(),
                        locator_json: serde_json::json!({
                            "mission_id": compiled.mission_id,
                            "session_id": session_id,
                            "trace_log_path": trace_log_path,
                        }),
                        content_hash: hash_bytes(err.to_string().as_bytes()),
                        storage_scope: "local".to_string(),
                        preview_redacted: Some(err.to_string()),
                        created_at: Utc::now(),
                    })
                    .context("store mission failure evidence")?;
                Err(err)
            }
        }
    }

    async fn build_coordinator(
        &self,
        trace_service: Arc<ExecutionTraceService>,
    ) -> Result<Coordinator> {
        let local_only = self.config.providers.default_cloud_instance.is_none();
        let local_task_timeout = self
            .config
            .provider_runtime
            .timeout
            .max(Duration::from_secs(90));
        let secrets = resolve_secrets(&self.config.workspace_root, &self.config.providers)
            .context("resolve provider secrets for work execution")?;
        let provider_bundle = Arc::new(
            build_provider_bundle(&self.config, &secrets)
                .context("build provider bundle for work execution")?,
        );
        let model_registry = Arc::new(
            build_model_registry_snapshot(&self.config)
                .await
                .context("build model registry for work execution")?,
        );
        let provider_registry = Arc::new(build_provider_registry(
            &self.config,
            Arc::clone(&provider_bundle),
            Arc::clone(&model_registry),
        )?);

        Coordinator::new_with_provider_registry(
            CoordinatorConfig {
                max_workers: if local_only { 1 } else { 5 },
                default_cloud: default_cloud_ref(&self.config, provider_bundle.as_ref()),
                default_local: default_local_ref(&self.config, provider_bundle.as_ref()),
                model_instance_priority: self.config.providers.model_instance_priority.clone(),
                provider_bundle,
                registry: model_registry,
                fallback_policy: self.config.fallback_policy,
                routing_enabled: self.config.routing_enabled
                    || (self.config.providers.default_cloud_instance.is_some()
                        && self.config.providers.default_local_instance.is_some()),
                local_validation_retry_budget: self.config.local_validation_retry_budget,
                local_enabled_for: vec![
                    "syntax_fix".to_string(),
                    "docstring".to_string(),
                    "autocomplete".to_string(),
                    "small_edit".to_string(),
                ],
                workspace_root: self.workspace_root.clone(),
                task_timeout: if local_only {
                    local_task_timeout
                } else {
                    Duration::from_secs(5)
                },
                hitl_gate: Some(Arc::clone(&self.hitl_gate)),
                mcp_endpoint: std::env::var("OPENAKTA_MCP_ENDPOINT").ok(),
                context_use_ratio: self.config.provider_context_use_ratio,
                context_margin_tokens: self.config.provider_context_margin_tokens,
                retrieval_share: self.config.provider_retrieval_share,
                execution_tracer: Some(trace_service),
                execution_trace_registry: Some(Arc::clone(&self.trace_registry)),
                ..Default::default()
            },
            Arc::new(Mutex::new(RuntimeBlackboard::new())),
            provider_registry,
        )
        .map_err(anyhow::Error::msg)
    }
}

#[tonic::async_trait]
impl WorkManagementService for WorkManagementGrpc {
    async fn list_work_items(
        &self,
        request: Request<ListWorkItemsRequest>,
    ) -> Result<Response<ListWorkItemsResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(ListWorkItemsResponse {
            items: snapshot.model.work_items.iter().map(proto_work_item).collect(),
            checkpoint_seq: snapshot.checkpoint_seq,
        }))
    }

    async fn get_board(
        &self,
        request: Request<GetBoardRequest>,
    ) -> Result<Response<GetBoardResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(GetBoardResponse {
            workspace: Some(proto_workspace(&snapshot.model.workspace)),
            cycles: snapshot.model.cycles.iter().map(proto_cycle).collect(),
            phases: snapshot.model.phases.iter().map(proto_phase).collect(),
            items: snapshot.model.work_items.iter().map(proto_work_item).collect(),
            dependencies: snapshot
                .model
                .dependencies
                .iter()
                .map(proto_dependency)
                .collect(),
            checkpoint_seq: snapshot.checkpoint_seq,
            etag: snapshot.etag,
        }))
    }

    async fn get_plan_version(
        &self,
        request: Request<GetPlanVersionRequest>,
    ) -> Result<Response<GetPlanVersionResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let plan_version_id = parse_uuid(&request.get_ref().plan_version_id, "plan_version_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let plan_version = snapshot
            .model
            .plan_versions
            .iter()
            .find(|plan| plan.id == plan_version_id)
            .ok_or_else(|| Status::not_found("plan version not found"))?;
        Ok(Response::new(GetPlanVersionResponse {
            plan_version: Some(proto_plan_version(plan_version)),
        }))
    }

    async fn submit_command(
        &self,
        request: Request<SubmitCommandRequest>,
    ) -> Result<Response<SubmitCommandResponse>, Status> {
        let request = request.into_inner();
        let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
        let client_command_id = if request.client_command_id.is_empty() {
            Uuid::new_v4()
        } else {
            parse_uuid(&request.client_command_id, "client_command_id")?
        };
        let payload = parse_json_field(&request.payload_json, "payload_json")?;
        let actor_context = parse_optional_json_field(&request.actor_context_json, "actor_context_json")?;
        let command = CommandEnvelope {
            client_command_id,
            base_seq: request.base_seq,
            command_type: request.command_type,
            payload,
            actor_context,
        };

        self.mirror
            .record_pending_command(workspace_id, &command)
            .map_err(|err| Status::internal(err.to_string()))?;

        let response = self
            .api_client_pool
            .completion_client
            .submit_work_command(workspace_id, &command)
            .await
            .map_err(|err| Status::unavailable(err.to_string()));

        match response {
            Ok(response) => {
                let read_model = self
                    .api_client_pool
                    .completion_client
                    .get_work_read_model(workspace_id)
                    .await
                    .map_err(|err| Status::unavailable(err.to_string()))?;
                let etag = if response.read_model_etag.is_empty() {
                    hash_read_model(&read_model)
                        .map_err(|err| Status::internal(err.to_string()))?
                } else {
                    response.read_model_etag.clone()
                };
                self.mirror
                    .upsert_read_model(workspace_id, &etag, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .mark_command_applied(client_command_id)
                    .map_err(|err| Status::internal(err.to_string()))?;
                Ok(Response::new(SubmitCommandResponse {
                    status: response.status,
                    resulting_seq: response.resulting_seq,
                    event_ids: response.event_ids.into_iter().map(|id| id.to_string()).collect(),
                    read_model_etag: etag,
                    conflict_snapshot_json: response
                        .conflict_snapshot
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                }))
            }
            Err(status) => {
                self.mirror
                    .mark_command_failed(client_command_id, status.message())
                    .map_err(|err| Status::internal(err.to_string()))?;
                Err(status)
            }
        }
    }

    async fn get_clarification_queue(
        &self,
        request: Request<GetClarificationQueueRequest>,
    ) -> Result<Response<GetClarificationQueueResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(GetClarificationQueueResponse {
            items: snapshot
                .model
                .clarifications
                .iter()
                .map(proto_clarification)
                .collect(),
            checkpoint_seq: snapshot.checkpoint_seq,
        }))
    }

    async fn resolve_clarifications(
        &self,
        request: Request<ResolveClarificationsRequest>,
    ) -> Result<Response<ResolveClarificationsResponse>, Status> {
        let request = request.into_inner();
        let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
        let answers = request
            .answers
            .iter()
            .map(|answer: &ClarificationAnswer| {
                (answer.clarification_item_id.clone(), answer.answer_json.clone())
            })
            .collect::<Vec<_>>();
        let resolved_count = self
            .mirror
            .resolve_clarifications(workspace_id, &request.session_id, &answers)
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(ResolveClarificationsResponse {
            resolved_count: resolved_count as i32,
        }))
    }

    async fn start_execution(
        &self,
        request: Request<StartExecutionRequest>,
    ) -> Result<Response<StartExecutionResponse>, Status> {
        let request = request.into_inner();
        let workspace_id = parse_uuid(&request.workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let selected_work_item_ids = request
            .work_item_ids
            .iter()
            .map(|value| parse_uuid(value, "work_item_ids"))
            .collect::<Result<Vec<_>, _>>()?;
        let selected_cycle_id = if request.cycle_id.is_empty() {
            None
        } else {
            Some(parse_uuid(&request.cycle_id, "cycle_id")?)
        };
        let compiled = compile_work_plan(
            &snapshot.model,
            &selected_work_item_ids,
            selected_cycle_id,
        )
        .map_err(|err| Status::invalid_argument(err.to_string()))?;
        let compiled_plan_json = serde_json::to_string_pretty(&compiled.mission)
            .map_err(|err| Status::internal(err.to_string()))?;

        if !request.dry_run {
            self.mirror
                .mark_items_queued_for_execution(workspace_id, &compiled.work_item_ids)
                .map_err(|err| Status::internal(err.to_string()))?;
            let evidence = EvidenceLinkView {
                id: Uuid::new_v4(),
                workspace_id,
                subject_type: "mission".to_string(),
                subject_id: None,
                artifact_kind: "compiled_plan".to_string(),
                locator_json: serde_json::json!({
                    "mission_id": compiled.mission_id,
                    "workspace_id": workspace_id,
                }),
                content_hash: format!("{:x}", Sha256::digest(compiled_plan_json.as_bytes())),
                storage_scope: "local".to_string(),
                preview_redacted: Some("Compiled mission preview stored locally".to_string()),
                created_at: Utc::now(),
            };
            self.mirror
                .append_evidence(&evidence)
                .map_err(|err| Status::internal(err.to_string()))?;

            let execution_service = self.clone();
            let compiled_for_execution = compiled.clone();
            let mission_id = compiled_for_execution.mission_id.clone();
            tokio::spawn(async move {
                if let Err(err) = execution_service
                    .execute_compiled_plan(workspace_id, compiled_for_execution)
                    .await
                {
                    error!(
                        workspace_id = %workspace_id,
                        mission_id = %mission_id,
                        error = %err,
                        "work plan execution failed"
                    );
                }
            });
        }

        Ok(Response::new(StartExecutionResponse {
            started: !request.dry_run,
            mission_id: compiled.mission_id,
            status: if request.dry_run {
                "compiled".to_string()
            } else {
                "queued_for_llm".to_string()
            },
            compiled_plan_json,
            error: String::new(),
        }))
    }

    async fn list_evidence(
        &self,
        request: Request<ListEvidenceRequest>,
    ) -> Result<Response<ListEvidenceResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let items = self
            .mirror
            .list_evidence(workspace_id)
            .map_err(|err| Status::internal(err.to_string()))?;
        Ok(Response::new(ListEvidenceResponse {
            items: items.iter().map(proto_evidence).collect(),
        }))
    }
}

fn build_provider_registry(
    config: &CoreConfig,
    bundle: Arc<ProviderRuntimeBundle>,
    model_registry: Arc<ModelRegistrySnapshot>,
) -> Result<ProviderRegistry> {
    let mut local = HashMap::new();
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
                Arc::from(
                    default_local_transport(&local_config, Duration::from_secs(30))
                        .map_err(|err| anyhow!(err.to_string()))?,
                ),
            );
        }
    }

    let client_config = ClientConfig::load_from_file("openakta.toml").unwrap_or_default();
    let api_client_pool = Arc::new(ApiClientPool::with_auth_provider(
        client_config,
        Some(Arc::new(EnvAuthProvider)),
    )?);

    Ok(ProviderRegistry::new_with_api_client(
        local,
        default_cloud_ref(config, bundle.as_ref()),
        default_local_ref(config, bundle.as_ref()),
        config.fallback_policy,
        bundle,
        model_registry,
        api_client_pool,
    ))
}

fn default_cloud_ref(config: &CoreConfig, bundle: &ProviderRuntimeBundle) -> Option<CloudModelRef> {
    let instance_id = config.providers.default_cloud_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    Some(CloudModelRef {
        instance_id,
        model: instance.default_model.clone()?,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

fn default_local_ref(config: &CoreConfig, bundle: &ProviderRuntimeBundle) -> Option<LocalModelRef> {
    let instance_id = config.providers.default_local_instance.clone()?;
    let instance = bundle.instances.get(&instance_id)?;
    Some(LocalModelRef {
        instance_id,
        model: instance.default_model.clone()?,
        wire_profile: instance.wire_profile(),
        telemetry_kind: instance.provider_kind(),
    })
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn redact_preview(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    value.chars().take(MAX_CHARS).collect()
}

fn parse_uuid(value: &str, field_name: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value).map_err(|_| Status::invalid_argument(format!("{field_name} must be a UUID")))
}

fn parse_json_field(value: &str, field_name: &str) -> Result<serde_json::Value, Status> {
    if value.is_empty() {
        Ok(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::from_str(value)
            .map_err(|_| Status::invalid_argument(format!("{field_name} must be valid JSON")))
    }
}

fn parse_optional_json_field(value: &str, field_name: &str) -> Result<Option<serde_json::Value>, Status> {
    if value.is_empty() {
        Ok(None)
    } else {
        parse_json_field(value, field_name).map(Some)
    }
}

fn hash_read_model(read_model: &ReadModelResponse) -> Result<String> {
    let bytes = serde_json::to_vec(read_model)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn proto_workspace(workspace: &openakta_api_client::WorkspaceView) -> Workspace {
    Workspace {
        id: workspace.id.to_string(),
        tenant_id: workspace.tenant_id.clone(),
        slug: workspace.slug.clone(),
        name: workspace.name.clone(),
        created_by: workspace.created_by.to_string(),
        created_at: workspace.created_at.to_rfc3339(),
    }
}

fn proto_cycle(cycle: &openakta_api_client::PlanningCycleView) -> PlanningCycle {
    PlanningCycle {
        id: cycle.id.to_string(),
        workspace_id: cycle.workspace_id.to_string(),
        cadence_mode: cycle.cadence_mode.clone(),
        planning_mode: cycle.planning_mode.clone(),
        start_at: cycle.start_at.map(|value| value.to_rfc3339()).unwrap_or_default(),
        end_at: cycle.end_at.map(|value| value.to_rfc3339()).unwrap_or_default(),
        status: cycle.status.clone(),
        global_wip_limit: cycle.global_wip_limit.unwrap_or_default(),
        replanning_interval_secs: cycle.replanning_interval_secs.unwrap_or_default(),
        created_at: cycle.created_at.to_rfc3339(),
    }
}

fn proto_phase(phase: &openakta_api_client::CyclePhaseView) -> CyclePhase {
    CyclePhase {
        id: phase.id.to_string(),
        cycle_id: phase.cycle_id.to_string(),
        phase_key: phase.phase_key.clone(),
        ordinal: phase.ordinal,
        strict_barrier: phase.strict_barrier,
        phase_wip_limit: phase.phase_wip_limit.unwrap_or_default(),
        exit_criteria_json: phase
            .exit_criteria_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_at: phase.created_at.to_rfc3339(),
    }
}

fn proto_work_item(item: &openakta_api_client::WorkItemView) -> WorkItem {
    WorkItem {
        id: item.id.to_string(),
        workspace_id: item.workspace_id.to_string(),
        cycle_id: item.cycle_id.map(|value| value.to_string()).unwrap_or_default(),
        parent_id: item.parent_id.map(|value| value.to_string()).unwrap_or_default(),
        item_type: item.item_type.clone(),
        execution_profile: item.execution_profile.clone(),
        title: item.title.clone(),
        description_md: item.description_md.clone().unwrap_or_default(),
        tracker_state: item.tracker_state.clone(),
        run_state: item.run_state.clone(),
        priority: item.priority,
        assignee_user_id: item
            .assignee_user_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        external_master: item.external_master,
        wave_rank: item.wave_rank.unwrap_or_default(),
        wave_label: item.wave_label.clone().unwrap_or_default(),
        updated_at: item.updated_at.to_rfc3339(),
        created_at: item.created_at.to_rfc3339(),
    }
}

fn proto_dependency(edge: &openakta_api_client::DependencyEdgeView) -> DependencyEdge {
    DependencyEdge {
        id: edge.id.to_string(),
        workspace_id: edge.workspace_id.to_string(),
        from_item_id: edge.from_item_id.to_string(),
        to_item_id: edge.to_item_id.to_string(),
        edge_type: edge.edge_type.clone(),
        strength: edge.strength.clone(),
        created_at: edge.created_at.to_rfc3339(),
    }
}

fn proto_clarification(item: &openakta_api_client::ClarificationItemView) -> ClarificationItem {
    ClarificationItem {
        id: item.id.to_string(),
        workspace_id: item.workspace_id.to_string(),
        cycle_id: item.cycle_id.map(|value| value.to_string()).unwrap_or_default(),
        work_item_id: item.work_item_id.map(|value| value.to_string()).unwrap_or_default(),
        mission_id: item.mission_id.clone().unwrap_or_default(),
        task_id: item.task_id.clone().unwrap_or_default(),
        question_kind: item.question_kind.clone(),
        prompt_text: item.prompt_text.clone(),
        schema_json: item.schema_json.clone().map(|value| value.to_string()).unwrap_or_default(),
        options_json: item.options_json.clone().map(|value| value.to_string()).unwrap_or_default(),
        dedupe_fingerprint: item.dedupe_fingerprint.clone(),
        status: item.status.clone(),
        raised_by_agent_id: item.raised_by_agent_id.clone().unwrap_or_default(),
        created_at: item.created_at.to_rfc3339(),
        answered_at: item.answered_at.map(|value| value.to_rfc3339()).unwrap_or_default(),
    }
}

fn proto_plan_version(plan: &openakta_api_client::PlanVersionView) -> PlanVersion {
    PlanVersion {
        id: plan.id.to_string(),
        workspace_id: plan.workspace_id.to_string(),
        cycle_id: plan.cycle_id.map(|value| value.to_string()).unwrap_or_default(),
        base_seq: plan.base_seq,
        plan_hash: plan.plan_hash.clone(),
        snapshot_json: plan.snapshot_json.to_string(),
        status: plan.status.clone(),
        created_by: plan.created_by.to_string(),
        approved_by: plan.approved_by.map(|value| value.to_string()).unwrap_or_default(),
        created_at: plan.created_at.to_rfc3339(),
        approved_at: plan.approved_at.map(|value| value.to_rfc3339()).unwrap_or_default(),
    }
}

fn proto_evidence(item: &EvidenceLinkView) -> EvidenceLink {
    EvidenceLink {
        id: item.id.to_string(),
        workspace_id: item.workspace_id.to_string(),
        subject_type: item.subject_type.clone(),
        subject_id: item.subject_id.map(|value| value.to_string()).unwrap_or_default(),
        artifact_kind: item.artifact_kind.clone(),
        locator_json: item.locator_json.to_string(),
        content_hash: item.content_hash.clone(),
        storage_scope: item.storage_scope.clone(),
        preview_redacted: item.preview_redacted.clone().unwrap_or_default(),
        created_at: item.created_at.to_rfc3339(),
    }
}
