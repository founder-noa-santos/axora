use std::collections::{HashMap, HashSet};
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
    ApiClientPool, ApiError, ClientConfig, CommandEnvelope, EnvAuthProvider, EvidenceLinkView,
    ReadModelResponse,
};
use openakta_core::config_resolve::{
    build_model_registry_snapshot, build_provider_bundle, resolve_secrets,
};
use openakta_core::CoreConfig;
use openakta_proto::work::v1::work_management_service_server::WorkManagementService;
use openakta_proto::work::v1::{
    AcceptanceCheck, ClarificationAnswer, ClarificationItem, ClosureClaim, ClosureGate,
    ClosureReport, CyclePhase, DependencyEdge, EvidenceLink, ExecutionProfileDecision,
    GetBoardRequest, GetBoardResponse, GetClarificationQueueRequest, GetClarificationQueueResponse,
    GetClosureReportRequest, GetClosureReportResponse, GetExecutionProfileDecisionRequest,
    GetExecutionProfileDecisionResponse, GetPlanVersionRequest, GetPlanVersionResponse,
    GetPreparedStoryRequest, GetPreparedStoryResponse, GetRequirementGraphRequest,
    GetRequirementGraphResponse, GetStoryIntakeRequest, GetStoryIntakeResponse, HandoffContract,
    ListEvidenceRequest, ListEvidenceResponse, ListPersonaAssignmentsRequest,
    ListPersonaAssignmentsResponse, ListPreparedStoriesRequest, ListPreparedStoriesResponse,
    ListVerificationFindingsRequest, ListVerificationFindingsResponse, ListVerificationRunsRequest,
    ListVerificationRunsResponse, ListWorkItemsRequest, ListWorkItemsResponse, Persona,
    PersonaAssignment, PlanVersion, PlanningCycle, PreparedStory, Requirement, RequirementCoverage,
    RequirementEdge, ResolveClarificationsRequest, ResolveClarificationsResponse,
    StartExecutionRequest, StartExecutionResponse, StoryIntake, SubmitCommandRequest,
    SubmitCommandResponse, VerificationCard, VerificationExpectation, VerificationFinding,
    VerificationRun, WorkItem, Workspace,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};
use tracing::{error, warn};
use uuid::Uuid;

use crate::background::execution_card_json::proto_execution_card_from_json;
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
    pub(crate) fn mol_verification_automation_enabled(&self) -> bool {
        self.config.mol.verification_automation_enabled
    }

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

    pub(crate) async fn synced_read_model(
        &self,
        workspace_id: Uuid,
    ) -> Result<StoredReadModel, Status> {
        match self
            .api_client_pool
            .completion_client
            .get_work_read_model(workspace_id)
            .await
        {
            Ok(read_model) => {
                let etag = hash_read_model(&read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .upsert_read_model(workspace_id, &etag, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .upsert_verification_index(workspace_id, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .refresh_persona_memory_index(workspace_id, &read_model)
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
                if let Err(err) = self.record_execution_success(workspace_id, &compiled).await {
                    error!(
                        workspace_id = %workspace_id,
                        mission_id = %compiled.mission_id,
                        error = %err,
                        "failed to record mission closure scaffolding"
                    );
                }
                self.mirror
                    .mark_items_execution_succeeded(workspace_id, &compiled.work_item_ids)
                    .context("reapply local work item success state")?;
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
                if let Err(record_err) =
                    self.record_execution_failure(workspace_id, &compiled).await
                {
                    error!(
                        workspace_id = %workspace_id,
                        mission_id = %compiled.mission_id,
                        error = %record_err,
                        "failed to record mission failure state"
                    );
                }
                self.mirror
                    .mark_items_execution_failed(workspace_id, &compiled.work_item_ids)
                    .context("reapply local work item failure state")?;
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
                mol: self.config.mol,
                ..Default::default()
            },
            Arc::new(Mutex::new(RuntimeBlackboard::new())),
            provider_registry,
        )
        .map_err(anyhow::Error::msg)
    }

    pub(crate) async fn submit_system_command(
        &self,
        workspace_id: Uuid,
        command_type: &str,
        payload: Value,
    ) -> Result<()> {
        for _ in 0..3 {
            let snapshot = self
                .synced_read_model(workspace_id)
                .await
                .map_err(|status| anyhow!(status.to_string()))?;
            let command = CommandEnvelope {
                client_command_id: Uuid::new_v4(),
                base_seq: snapshot.checkpoint_seq,
                command_type: command_type.to_string(),
                payload: payload.clone(),
                actor_context: Some(json!({
                    "actor_kind": "daemon",
                    "actor_source": "work_management_service",
                })),
            };

            let response = self
                .api_client_pool
                .completion_client
                .submit_work_command(workspace_id, &command)
                .await?;
            match response.status.as_str() {
                "accepted" | "duplicate" => {
                    let refreshed = self
                        .api_client_pool
                        .completion_client
                        .get_work_read_model(workspace_id)
                        .await?;
                    let etag = hash_read_model(&refreshed)?;
                    self.mirror
                        .upsert_read_model(workspace_id, &etag, &refreshed)
                        .context("refresh mirrored read model after system command")?;
                    self.mirror
                        .upsert_verification_index(workspace_id, &refreshed)
                        .context("refresh local verification index after system command")?;
                    self.mirror
                        .refresh_persona_memory_index(workspace_id, &refreshed)
                        .context("refresh local persona memory index after system command")?;
                    return Ok(());
                }
                "conflict" => continue,
                other => {
                    return Err(anyhow!(
                        "system command {} rejected with status {}",
                        command_type,
                        other
                    ))
                }
            }
        }

        Err(anyhow!(
            "system command {} exhausted conflict retries",
            command_type
        ))
    }

    async fn record_execution_started(
        &self,
        workspace_id: Uuid,
        compiled: &CompiledWorkPlan,
    ) -> Result<()> {
        let Some(prepared_story_id) = compiled.contract.prepared_story_id else {
            return Ok(());
        };

        self.submit_system_command(
            workspace_id,
            "transition_story_preparation",
            json!({
                "prepared_story_id": prepared_story_id,
                "status": "executing",
            }),
        )
        .await
    }

    /// After a successful mission run: `closure_pending`, claims, gates, verification drain, then
    /// a best-effort `closed` transition when the API closure engine (ABC1) accepts it.
    /// Open findings may block unless `MOL_CLOSURE_ALLOW_OPEN_FINDINGS=true` (see `MolFeatureFlags`).
    async fn record_execution_success(
        &self,
        workspace_id: Uuid,
        compiled: &CompiledWorkPlan,
    ) -> Result<()> {
        let Some(prepared_story_id) = compiled.contract.prepared_story_id else {
            return Ok(());
        };
        let snapshot = self
            .synced_read_model(workspace_id)
            .await
            .map_err(|status| anyhow!(status.to_string()))?;

        self.submit_system_command(
            workspace_id,
            "transition_story_preparation",
            json!({
                "prepared_story_id": prepared_story_id,
                "status": "closure_pending",
            }),
        )
        .await?;

        for requirement_id in &compiled.contract.claimed_requirement_ids {
            let work_item_id = snapshot
                .model
                .requirement_coverage
                .iter()
                .find(|coverage| {
                    coverage.requirement_id == *requirement_id
                        && compiled.work_item_ids.contains(&coverage.work_item_id)
                })
                .map(|coverage| coverage.work_item_id);
            let claimed_by_persona_id = work_item_id
                .and_then(|id| {
                    snapshot
                        .model
                        .work_items
                        .iter()
                        .find(|item| item.id == id)
                        .and_then(|item| item.owner_persona_id.clone())
                })
                .unwrap_or_else(|| persona_id(workspace_id, "implementation_steward"));
            self.submit_system_command(
                workspace_id,
                "record_completion_claim",
                json!({
                    "id": claim_id(prepared_story_id, *requirement_id, work_item_id),
                    "work_item_id": work_item_id,
                    "requirement_id": requirement_id,
                    "claim_type": "implemented",
                    "status": "recorded",
                    "claimed_by_persona_id": claimed_by_persona_id,
                    "claim_json": {
                        "mission_id": compiled.mission_id,
                        "profile_name": compiled.contract.profile_name,
                    }
                }),
            )
            .await?;
        }

        self.submit_system_command(
            workspace_id,
            "advance_closure_gate",
            json!({
                "id": closure_gate_id(prepared_story_id, compiled.contract.story_id, "coverage"),
                "story_id": compiled.contract.story_id,
                "prepared_story_id": prepared_story_id,
                "gate_type": "coverage",
                "status": "passed",
                "decided_by_persona_id": persona_id(workspace_id, "planning_steward"),
                "rationale_md": "Requirement coverage was satisfied before execution and completion claims were recorded.",
            }),
        )
        .await?;

        if compiled.contract.review_required {
            self.submit_system_command(
                workspace_id,
                "advance_closure_gate",
                json!({
                    "id": closure_gate_id(prepared_story_id, compiled.contract.story_id, "review"),
                    "story_id": compiled.contract.story_id,
                    "prepared_story_id": prepared_story_id,
                    "gate_type": "review",
                    "status": "pending",
                    "decided_by_persona_id": persona_id(workspace_id, "review_steward"),
                    "rationale_md": "Review Steward approval is required before closure.",
                }),
            )
            .await?;
        }

        if compiled.contract.verification_required {
            self.submit_system_command(
                workspace_id,
                "start_verification_run",
                json!({
                    "id": Uuid::new_v4(),
                    "story_id": compiled.contract.story_id,
                    "prepared_story_id": prepared_story_id,
                    "status": "pending",
                    "verification_stage": "post_implementation",
                    "run_kind": "independent",
                    "initiated_by_persona_id": persona_id(workspace_id, "verification_steward"),
                    "summary_json": {
                        "mission_id": compiled.mission_id,
                        "requirement_ids": compiled.contract.claimed_requirement_ids,
                        "independence_required": true,
                    }
                }),
            )
            .await?;
            self.submit_system_command(
                workspace_id,
                "advance_closure_gate",
                json!({
                    "id": closure_gate_id(prepared_story_id, compiled.contract.story_id, "verification"),
                    "story_id": compiled.contract.story_id,
                    "prepared_story_id": prepared_story_id,
                    "gate_type": "verification",
                    "status": "pending",
                    "decided_by_persona_id": persona_id(workspace_id, "verification_steward"),
                    "rationale_md": "Independent verification is required before closure.",
                }),
            )
            .await?;
        }

        if compiled.contract.reliability_required {
            self.submit_system_command(
                workspace_id,
                "advance_closure_gate",
                json!({
                    "id": closure_gate_id(prepared_story_id, compiled.contract.story_id, "reliability"),
                    "story_id": compiled.contract.story_id,
                    "prepared_story_id": prepared_story_id,
                    "gate_type": "reliability",
                    "status": "pending",
                    "decided_by_persona_id": persona_id(workspace_id, "reliability_steward"),
                    "rationale_md": "Reliability approval is required for the active profile.",
                }),
            )
            .await?;
        }

        if compiled.contract.documentation_required {
            self.submit_system_command(
                workspace_id,
                "mark_documentation_alignment",
                json!({
                    "gate_id": closure_gate_id(prepared_story_id, compiled.contract.story_id, "documentation"),
                    "story_id": compiled.contract.story_id,
                    "prepared_story_id": prepared_story_id,
                    "status": "pending",
                    "decided_by_persona_id": persona_id(workspace_id, "knowledge_steward"),
                    "rationale_md": "Documentation alignment must be reviewed before closure.",
                }),
            )
            .await?;
        }

        if let Err(err) =
            crate::background::verification_run_worker::process_pending_verification_runs(
                self,
                workspace_id,
            )
            .await
        {
            error!(
                workspace_id = %workspace_id,
                error = %err,
                "verification automation drain failed"
            );
        }

        if let Err(err) = self
            .submit_close_story_preparation_if_eligible(workspace_id, prepared_story_id)
            .await
        {
            warn!(
                workspace_id = %workspace_id,
                prepared_story_id = %prepared_story_id,
                error = %err,
                "could not transition prepared story to closed after mission success"
            );
        }
        Ok(())
    }

    /// Best-effort `closed` after gates/verification. The API returns structured `CLOSURE_*` codes
    /// when the closure engine rejects the transition (ABC1/ABC2); those are expected until the
    /// story is fully ready — treat as success so mission success still completes.
    async fn submit_close_story_preparation_if_eligible(
        &self,
        workspace_id: Uuid,
        prepared_story_id: Uuid,
    ) -> Result<()> {
        match self
            .submit_system_command(
                workspace_id,
                "transition_story_preparation",
                json!({
                    "prepared_story_id": prepared_story_id,
                    "status": "closed",
                }),
            )
            .await
        {
            Ok(()) => Ok(()),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("CLOSURE_NOT_READY")
                    || msg.contains("CLOSURE_BLOCKED_")
                    || msg.contains("CLOSURE_GATE_")
                    || msg.contains("CLOSURE_CLAIM_")
                    || msg.contains("CLOSURE_HANDOFF_")
                    || msg.contains("CLOSURE_ACCEPTANCE_")
                {
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn record_execution_failure(
        &self,
        workspace_id: Uuid,
        compiled: &CompiledWorkPlan,
    ) -> Result<()> {
        let Some(prepared_story_id) = compiled.contract.prepared_story_id else {
            return Ok(());
        };

        self.submit_system_command(
            workspace_id,
            "transition_story_preparation",
            json!({
                "prepared_story_id": prepared_story_id,
                "status": "blocked",
                "readiness_blockers_json": ["Execution failed before closure verification completed."],
            }),
        )
        .await
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
            items: snapshot
                .model
                .work_items
                .iter()
                .map(proto_work_item)
                .collect(),
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
            items: snapshot
                .model
                .work_items
                .iter()
                .map(proto_work_item)
                .collect(),
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

    async fn get_story_intake(
        &self,
        request: Request<GetStoryIntakeRequest>,
    ) -> Result<Response<GetStoryIntakeResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let story_id = parse_uuid(&request.get_ref().story_id, "story_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let story = snapshot
            .model
            .story_intakes
            .iter()
            .find(|story| story.id == story_id)
            .ok_or_else(|| Status::not_found("story intake not found"))?;
        Ok(Response::new(GetStoryIntakeResponse {
            story_intake: Some(proto_story_intake(story)),
        }))
    }

    async fn list_prepared_stories(
        &self,
        request: Request<ListPreparedStoriesRequest>,
    ) -> Result<Response<ListPreparedStoriesResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(ListPreparedStoriesResponse {
            items: snapshot
                .model
                .story_preparations
                .iter()
                .map(proto_prepared_story)
                .collect(),
            checkpoint_seq: snapshot.checkpoint_seq,
        }))
    }

    async fn get_prepared_story(
        &self,
        request: Request<GetPreparedStoryRequest>,
    ) -> Result<Response<GetPreparedStoryResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let prepared_story_id =
            parse_uuid(&request.get_ref().prepared_story_id, "prepared_story_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let prepared_story = snapshot
            .model
            .story_preparations
            .iter()
            .find(|story| story.id == prepared_story_id)
            .ok_or_else(|| Status::not_found("prepared story not found"))?;
        Ok(Response::new(GetPreparedStoryResponse {
            prepared_story: Some(proto_prepared_story(prepared_story)),
        }))
    }

    async fn get_requirement_graph(
        &self,
        request: Request<GetRequirementGraphRequest>,
    ) -> Result<Response<GetRequirementGraphResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let story_id = parse_optional_uuid(&request.get_ref().story_id)?;
        let prepared_story_id = parse_optional_uuid(&request.get_ref().prepared_story_id)?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let scoped_req_ids = scoped_requirement_ids(&snapshot.model, story_id, prepared_story_id);
        let prepared_ids_for_story = story_scoped_prepared_story_ids(&snapshot.model, story_id);
        Ok(Response::new(GetRequirementGraphResponse {
            requirements: snapshot
                .model
                .requirements
                .iter()
                .filter(|requirement| {
                    filter_story_scoped(
                        requirement.story_id,
                        requirement.prepared_story_id,
                        story_id,
                        prepared_story_id,
                    )
                })
                .map(proto_requirement)
                .collect(),
            edges: snapshot
                .model
                .requirement_edges
                .iter()
                .filter(|edge| {
                    scoped_req_ids.contains(&edge.requirement_id)
                        && scoped_req_ids.contains(&edge.related_requirement_id)
                })
                .map(proto_requirement_edge)
                .collect(),
            acceptance_checks: snapshot
                .model
                .acceptance_checks
                .iter()
                .filter(|check| scoped_req_ids.contains(&check.requirement_id))
                .map(proto_acceptance_check)
                .collect(),
            coverage: snapshot
                .model
                .requirement_coverage
                .iter()
                .filter(|coverage| scoped_req_ids.contains(&coverage.requirement_id))
                .map(proto_requirement_coverage)
                .collect(),
            handoff_contracts: snapshot
                .model
                .handoff_contracts
                .iter()
                .filter(|contract| {
                    filter_handoff_for_scope(
                        contract,
                        story_id,
                        prepared_story_id,
                        prepared_ids_for_story.as_ref(),
                    )
                })
                .map(proto_handoff_contract)
                .collect(),
        }))
    }

    async fn list_verification_runs(
        &self,
        request: Request<ListVerificationRunsRequest>,
    ) -> Result<Response<ListVerificationRunsResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let story_id = parse_optional_uuid(&request.get_ref().story_id)?;
        let prepared_story_id = parse_optional_uuid(&request.get_ref().prepared_story_id)?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(ListVerificationRunsResponse {
            items: snapshot
                .model
                .verification_runs
                .iter()
                .filter(|run| {
                    filter_story_scoped(
                        run.story_id,
                        run.prepared_story_id,
                        story_id,
                        prepared_story_id,
                    )
                })
                .map(proto_verification_run)
                .collect(),
        }))
    }

    async fn list_verification_findings(
        &self,
        request: Request<ListVerificationFindingsRequest>,
    ) -> Result<Response<ListVerificationFindingsResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let verification_run_id = parse_uuid(
            &request.get_ref().verification_run_id,
            "verification_run_id",
        )?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(ListVerificationFindingsResponse {
            items: snapshot
                .model
                .verification_findings
                .iter()
                .filter(|finding| finding.verification_run_id == verification_run_id)
                .map(proto_verification_finding)
                .collect(),
        }))
    }

    async fn get_closure_report(
        &self,
        request: Request<GetClosureReportRequest>,
    ) -> Result<Response<GetClosureReportResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let story_id = parse_optional_uuid(&request.get_ref().story_id)?;
        let prepared_story_id = parse_optional_uuid(&request.get_ref().prepared_story_id)?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let scoped_req_ids = scoped_requirement_ids(&snapshot.model, story_id, prepared_story_id);
        let scoped_run_ids =
            scoped_verification_run_ids(&snapshot.model, story_id, prepared_story_id);
        Ok(Response::new(GetClosureReportResponse {
            report: Some(ClosureReport {
                workspace_id: workspace_id.to_string(),
                story_id: story_id.map(|value| value.to_string()).unwrap_or_default(),
                prepared_story_id: prepared_story_id
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
                requirements: snapshot
                    .model
                    .requirements
                    .iter()
                    .filter(|requirement| {
                        filter_story_scoped(
                            requirement.story_id,
                            requirement.prepared_story_id,
                            story_id,
                            prepared_story_id,
                        )
                    })
                    .map(proto_requirement)
                    .collect(),
                closure_claims: snapshot
                    .model
                    .closure_claims
                    .iter()
                    .filter(|claim| {
                        closure_claim_matches_scope(
                            claim,
                            &scoped_req_ids,
                            story_id,
                            prepared_story_id,
                            &snapshot.model.work_items,
                        )
                    })
                    .map(proto_closure_claim)
                    .collect(),
                closure_gates: snapshot
                    .model
                    .closure_gates
                    .iter()
                    .filter(|gate| {
                        filter_story_scoped(
                            gate.story_id,
                            gate.prepared_story_id,
                            story_id,
                            prepared_story_id,
                        )
                    })
                    .map(proto_closure_gate)
                    .collect(),
                verification_findings: snapshot
                    .model
                    .verification_findings
                    .iter()
                    .filter(|finding| {
                        story_id.is_none() && prepared_story_id.is_none()
                            || scoped_run_ids.contains(&finding.verification_run_id)
                    })
                    .map(proto_verification_finding)
                    .collect(),
            }),
        }))
    }

    async fn list_persona_assignments(
        &self,
        request: Request<ListPersonaAssignmentsRequest>,
    ) -> Result<Response<ListPersonaAssignmentsResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        Ok(Response::new(ListPersonaAssignmentsResponse {
            items: snapshot.model.personas.iter().map(proto_persona).collect(),
            assignments: snapshot
                .model
                .persona_assignments
                .iter()
                .map(proto_persona_assignment)
                .collect(),
        }))
    }

    async fn get_execution_profile_decision(
        &self,
        request: Request<GetExecutionProfileDecisionRequest>,
    ) -> Result<Response<GetExecutionProfileDecisionResponse>, Status> {
        let workspace_id = parse_uuid(&request.get_ref().workspace_id, "workspace_id")?;
        let story_id = parse_optional_uuid(&request.get_ref().story_id)?;
        let prepared_story_id = parse_optional_uuid(&request.get_ref().prepared_story_id)?;
        let snapshot = self.synced_read_model(workspace_id).await?;
        let decision = snapshot
            .model
            .execution_profile_decisions
            .iter()
            .find(|decision| {
                filter_story_scoped(
                    decision.story_id,
                    decision.prepared_story_id,
                    story_id,
                    prepared_story_id,
                )
            })
            .ok_or_else(|| Status::not_found("execution profile decision not found"))?;
        Ok(Response::new(GetExecutionProfileDecisionResponse {
            decision: Some(proto_execution_profile_decision(decision)),
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
        let actor_context =
            parse_optional_json_field(&request.actor_context_json, "actor_context_json")?;
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
                    hash_read_model(&read_model).map_err(|err| Status::internal(err.to_string()))?
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
                    event_ids: response
                        .event_ids
                        .into_iter()
                        .map(|id| id.to_string())
                        .collect(),
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
        let session_id_str = request.session_id.clone();
        if request.answers.is_empty() {
            return Err(Status::invalid_argument("answers must not be empty"));
        }
        let session_id = Uuid::parse_str(session_id_str.trim())
            .map_err(|_| Status::invalid_argument("session_id must be a valid UUID string"))?;

        let answers: Vec<(String, String)> = request
            .answers
            .iter()
            .map(|answer: &ClarificationAnswer| {
                (
                    answer.clarification_item_id.clone(),
                    answer.answer_json.clone(),
                )
            })
            .collect();

        let answer_json: Vec<Value> = answers
            .iter()
            .map(|(id_str, raw)| {
                let cid = Uuid::parse_str(id_str.trim()).map_err(|_| {
                    Status::invalid_argument("clarification_item_id must be a valid UUID string")
                })?;
                let aj: Value = serde_json::from_str(raw).unwrap_or_else(|_| json!(raw));
                Ok(json!({
                    "clarification_item_id": cid,
                    "answer_json": aj,
                }))
            })
            .collect::<Result<Vec<_>, Status>>()?;

        let snapshot = self.synced_read_model(workspace_id).await?;

        let command = CommandEnvelope {
            client_command_id: Uuid::new_v4(),
            base_seq: snapshot.checkpoint_seq,
            command_type: "record_clarification_answers".to_string(),
            payload: json!({
                "session_id": session_id,
                "answers": answer_json,
            }),
            actor_context: None,
        };

        match self
            .api_client_pool
            .completion_client
            .submit_work_command(workspace_id, &command)
            .await
        {
            Ok(response) => {
                let read_model = self
                    .api_client_pool
                    .completion_client
                    .get_work_read_model(workspace_id)
                    .await
                    .map_err(|err| Status::unavailable(err.to_string()))?;
                let etag = if response.read_model_etag.is_empty() {
                    hash_read_model(&read_model).map_err(|err| Status::internal(err.to_string()))?
                } else {
                    response.read_model_etag.clone()
                };
                self.mirror
                    .upsert_read_model(workspace_id, &etag, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .upsert_verification_index(workspace_id, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                self.mirror
                    .refresh_persona_memory_index(workspace_id, &read_model)
                    .map_err(|err| Status::internal(err.to_string()))?;
                Ok(Response::new(ResolveClarificationsResponse {
                    resolved_count: answers.len() as i32,
                }))
            }
            Err(err) => {
                if clarification_mirror_fallback_allowed(&err) {
                    warn!(
                        error = %err,
                        "resolve_clarifications: hosted API unavailable; applying answers to local mirror only (not canonical until API sync)"
                    );
                    let resolved_count = self
                        .mirror
                        .resolve_clarifications(workspace_id, &session_id_str, &answers)
                        .map_err(|e| Status::internal(e.to_string()))?;
                    Ok(Response::new(ResolveClarificationsResponse {
                        resolved_count: resolved_count as i32,
                    }))
                } else {
                    Err(map_clarification_api_error(err))
                }
            }
        }
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
        let prepared_story_id = parse_optional_uuid(&request.prepared_story_id)?;
        let compiled = compile_work_plan(
            &snapshot.model,
            &selected_work_item_ids,
            selected_cycle_id,
            prepared_story_id,
            self.config.mol.raw_execution_allowed,
        )
        .map_err(|err| Status::invalid_argument(err.to_string()))?;
        let compiled_plan_json = serde_json::to_string_pretty(&compiled.mission)
            .map_err(|err| Status::internal(err.to_string()))?;

        if !request.dry_run {
            self.record_execution_started(workspace_id, &compiled)
                .await
                .map_err(|err| Status::failed_precondition(err.to_string()))?;
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

fn persona_id(workspace_id: Uuid, role: &str) -> String {
    format!("{workspace_id}:{role}")
}

fn closure_gate_id(prepared_story_id: Uuid, story_id: Option<Uuid>, gate_type: &str) -> Uuid {
    deterministic_uuid(&format!(
        "closure-gate:{}:{}:{}",
        story_id.unwrap_or_else(Uuid::nil),
        prepared_story_id,
        gate_type
    ))
}

fn claim_id(prepared_story_id: Uuid, requirement_id: Uuid, work_item_id: Option<Uuid>) -> Uuid {
    deterministic_uuid(&format!(
        "closure-claim:{}:{}:{}",
        prepared_story_id,
        requirement_id,
        work_item_id.unwrap_or_else(Uuid::nil)
    ))
}

fn deterministic_uuid(seed: &str) -> Uuid {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

fn redact_preview(value: &str) -> String {
    const MAX_CHARS: usize = 240;
    value.chars().take(MAX_CHARS).collect()
}

fn parse_uuid(value: &str, field_name: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(value)
        .map_err(|_| Status::invalid_argument(format!("{field_name} must be a UUID")))
}

fn parse_optional_uuid(value: &str) -> Result<Option<Uuid>, Status> {
    if value.is_empty() {
        Ok(None)
    } else {
        Uuid::parse_str(value)
            .map(Some)
            .map_err(|_| Status::invalid_argument("value must be a UUID"))
    }
}

fn parse_json_field(value: &str, field_name: &str) -> Result<serde_json::Value, Status> {
    if value.is_empty() {
        Ok(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::from_str(value)
            .map_err(|_| Status::invalid_argument(format!("{field_name} must be valid JSON")))
    }
}

fn parse_optional_json_field(
    value: &str,
    field_name: &str,
) -> Result<Option<serde_json::Value>, Status> {
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
        start_at: cycle
            .start_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        end_at: cycle
            .end_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
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
        cycle_id: item
            .cycle_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        parent_id: item
            .parent_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
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
        story_id: item
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        prepared_story_id: item
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        owner_persona_id: item.owner_persona_id.clone().unwrap_or_default(),
        requirement_slice_json: item
            .requirement_slice_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        handoff_contract_state: item.handoff_contract_state.clone().unwrap_or_default(),
        claim_state: item.claim_state.clone().unwrap_or_default(),
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
        cycle_id: item
            .cycle_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        work_item_id: item
            .work_item_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        mission_id: item.mission_id.clone().unwrap_or_default(),
        task_id: item.task_id.clone().unwrap_or_default(),
        question_kind: item.question_kind.clone(),
        prompt_text: item.prompt_text.clone(),
        schema_json: item
            .schema_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        options_json: item
            .options_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        dedupe_fingerprint: item.dedupe_fingerprint.clone(),
        status: item.status.clone(),
        raised_by_agent_id: item.raised_by_agent_id.clone().unwrap_or_default(),
        created_at: item.created_at.to_rfc3339(),
        answered_at: item
            .answered_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        story_id: item
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        requirement_id: item
            .requirement_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
    }
}

fn proto_plan_version(plan: &openakta_api_client::PlanVersionView) -> PlanVersion {
    PlanVersion {
        id: plan.id.to_string(),
        workspace_id: plan.workspace_id.to_string(),
        cycle_id: plan
            .cycle_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        base_seq: plan.base_seq,
        plan_hash: plan.plan_hash.clone(),
        snapshot_json: plan.snapshot_json.to_string(),
        status: plan.status.clone(),
        created_by: plan.created_by.to_string(),
        approved_by: plan
            .approved_by
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_at: plan.created_at.to_rfc3339(),
        approved_at: plan
            .approved_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        story_id: plan
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
    }
}

fn proto_story_intake(story: &openakta_api_client::StoryIntakeView) -> StoryIntake {
    StoryIntake {
        id: story.id.to_string(),
        workspace_id: story.workspace_id.to_string(),
        external_ref: story.external_ref.clone().unwrap_or_default(),
        title: story.title.clone(),
        raw_request_md: story.raw_request_md.clone(),
        source_kind: story.source_kind.clone(),
        status: story.status.clone(),
        urgency: story.urgency.clone(),
        priority_band: story.priority_band.clone(),
        affected_surfaces_json: story
            .affected_surfaces_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_by: story.created_by.to_string(),
        created_at: story.created_at.to_rfc3339(),
        updated_at: story.updated_at.to_rfc3339(),
    }
}

/// Builds a [`VerificationCard`] from optional nested `verification_card` in execution JSON, with profile defaults (aligned with `MissionProfile::requires_verification`).
fn proto_verification_card_from_story(
    story: &openakta_api_client::StoryPreparationView,
    execution_json: &Value,
) -> VerificationCard {
    let mut card = VerificationCard {
        schema_version: 1,
        verification_required: story.primary_execution_profile != "Fast Iterate",
        ..Default::default()
    };
    let Some(vc) = execution_json.get("verification_card") else {
        return card;
    };
    let Some(obj) = vc.as_object() else {
        return card;
    };
    if let Some(b) = obj.get("verification_required").and_then(|x| x.as_bool()) {
        card.verification_required = b;
    }
    if let Some(s) = obj.get("steward_persona_id").and_then(|x| x.as_str()) {
        card.steward_persona_id = s.to_string();
    }
    if let Some(n) = obj.get("notes_md").and_then(|x| x.as_str()) {
        card.notes_md = n.to_string();
    }
    if let Some(arr) = obj.get("verification_stages").and_then(|x| x.as_array()) {
        for s in arr {
            if let Some(st) = s.as_str() {
                card.verification_stages.push(st.to_string());
            }
        }
    }
    if let Some(arr) = obj.get("closure_expectations").and_then(|x| x.as_array()) {
        for e in arr {
            let Some(o) = e.as_object() else {
                continue;
            };
            card.closure_expectations.push(VerificationExpectation {
                gate_type: o
                    .get("gate_type")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                expected_status: o
                    .get("expected_status")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
            });
        }
    }
    if let Some(ext) = obj.get("extension_json") {
        card.extension_json = ext.to_string();
    }
    card
}

fn proto_prepared_story(story: &openakta_api_client::StoryPreparationView) -> PreparedStory {
    let (mol_schema_version, execution_card, verification_card) =
        if let Some(ref json) = story.execution_card_json {
            (
                1u32,
                Some(proto_execution_card_from_json(
                    json,
                    &story.primary_execution_profile,
                )),
                Some(proto_verification_card_from_story(story, json)),
            )
        } else {
            (0u32, None, None)
        };

    PreparedStory {
        id: story.id.to_string(),
        workspace_id: story.workspace_id.to_string(),
        story_id: story.story_id.to_string(),
        status: story.status.clone(),
        mission_card_json: story.mission_card_json.to_string(),
        execution_card_json: story
            .execution_card_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        dependency_summary_json: story
            .dependency_summary_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        readiness_blockers_json: story
            .readiness_blockers_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        primary_execution_profile: story.primary_execution_profile.clone(),
        created_by: story.created_by.to_string(),
        created_at: story.created_at.to_rfc3339(),
        updated_at: story.updated_at.to_rfc3339(),
        ready_at: story
            .ready_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
        mol_schema_version,
        execution_card,
        verification_card,
    }
}

fn proto_requirement(requirement: &openakta_api_client::RequirementView) -> Requirement {
    Requirement {
        id: requirement.id.to_string(),
        workspace_id: requirement.workspace_id.to_string(),
        story_id: requirement
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        prepared_story_id: requirement
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        plan_version_id: requirement
            .plan_version_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        parent_requirement_id: requirement
            .parent_requirement_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        title: requirement.title.clone(),
        statement: requirement.statement.clone(),
        kind: requirement.kind.clone(),
        criticality: requirement.criticality.clone(),
        source: requirement.source.clone(),
        ambiguity_state: requirement.ambiguity_state.clone(),
        owner_persona_id: requirement.owner_persona_id.clone().unwrap_or_default(),
        status: requirement.status.clone(),
        created_at: requirement.created_at.to_rfc3339(),
        updated_at: requirement.updated_at.to_rfc3339(),
    }
}

fn proto_requirement_edge(edge: &openakta_api_client::RequirementEdgeView) -> RequirementEdge {
    RequirementEdge {
        id: edge.id.to_string(),
        workspace_id: edge.workspace_id.to_string(),
        requirement_id: edge.requirement_id.to_string(),
        related_requirement_id: edge.related_requirement_id.to_string(),
        edge_type: edge.edge_type.clone(),
        created_at: edge.created_at.to_rfc3339(),
    }
}

fn proto_acceptance_check(check: &openakta_api_client::AcceptanceCheckView) -> AcceptanceCheck {
    AcceptanceCheck {
        id: check.id.to_string(),
        workspace_id: check.workspace_id.to_string(),
        requirement_id: check.requirement_id.to_string(),
        check_kind: check.check_kind.clone(),
        title: check.title.clone(),
        status: check.status.clone(),
        evidence_required: check.evidence_required,
        created_at: check.created_at.to_rfc3339(),
        updated_at: check.updated_at.to_rfc3339(),
    }
}

fn proto_requirement_coverage(
    coverage: &openakta_api_client::RequirementCoverageView,
) -> RequirementCoverage {
    RequirementCoverage {
        id: coverage.id.to_string(),
        workspace_id: coverage.workspace_id.to_string(),
        requirement_id: coverage.requirement_id.to_string(),
        work_item_id: coverage.work_item_id.to_string(),
        coverage_kind: coverage.coverage_kind.clone(),
        status: coverage.status.clone(),
        created_at: coverage.created_at.to_rfc3339(),
        updated_at: coverage.updated_at.to_rfc3339(),
    }
}

fn proto_handoff_contract(contract: &openakta_api_client::HandoffContractView) -> HandoffContract {
    HandoffContract {
        id: contract.id.to_string(),
        workspace_id: contract.workspace_id.to_string(),
        prepared_story_id: contract
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        from_work_item_id: contract
            .from_work_item_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        to_work_item_id: contract
            .to_work_item_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        contract_kind: contract.contract_kind.clone(),
        expected_artifact_json: contract
            .expected_artifact_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        acceptance_signal_json: contract
            .acceptance_signal_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        status: contract.status.clone(),
        created_at: contract.created_at.to_rfc3339(),
        updated_at: contract.updated_at.to_rfc3339(),
    }
}

fn proto_execution_profile_decision(
    decision: &openakta_api_client::ExecutionProfileDecisionView,
) -> ExecutionProfileDecision {
    ExecutionProfileDecision {
        id: decision.id.to_string(),
        workspace_id: decision.workspace_id.to_string(),
        story_id: decision
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        prepared_story_id: decision
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        profile_name: decision.profile_name.clone(),
        policy_json: decision.policy_json.to_string(),
        inferred_from_json: decision
            .inferred_from_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        override_reason_md: decision.override_reason_md.clone().unwrap_or_default(),
        escalation_level: decision.escalation_level.clone(),
        decided_by: decision.decided_by.clone(),
        created_at: decision.created_at.to_rfc3339(),
        updated_at: decision.updated_at.to_rfc3339(),
    }
}

fn proto_verification_run(run: &openakta_api_client::VerificationRunView) -> VerificationRun {
    VerificationRun {
        id: run.id.to_string(),
        workspace_id: run.workspace_id.to_string(),
        story_id: run
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        prepared_story_id: run
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        status: run.status.clone(),
        verification_stage: run.verification_stage.clone(),
        run_kind: run.run_kind.clone(),
        initiated_by_persona_id: run.initiated_by_persona_id.clone().unwrap_or_default(),
        summary_json: run
            .summary_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_at: run.created_at.to_rfc3339(),
        completed_at: run
            .completed_at
            .map(|value| value.to_rfc3339())
            .unwrap_or_default(),
    }
}

fn proto_verification_finding(
    finding: &openakta_api_client::VerificationFindingView,
) -> VerificationFinding {
    VerificationFinding {
        id: finding.id.to_string(),
        workspace_id: finding.workspace_id.to_string(),
        verification_run_id: finding.verification_run_id.to_string(),
        requirement_id: finding
            .requirement_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        severity: finding.severity.clone(),
        finding_type: finding.finding_type.clone(),
        title: finding.title.clone(),
        detail_md: finding.detail_md.clone().unwrap_or_default(),
        status: finding.status.clone(),
        created_at: finding.created_at.to_rfc3339(),
        updated_at: finding.updated_at.to_rfc3339(),
    }
}

fn proto_closure_claim(claim: &openakta_api_client::ClosureClaimView) -> ClosureClaim {
    ClosureClaim {
        id: claim.id.to_string(),
        workspace_id: claim.workspace_id.to_string(),
        work_item_id: claim
            .work_item_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        requirement_id: claim
            .requirement_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        claim_type: claim.claim_type.clone(),
        status: claim.status.clone(),
        claimed_by_persona_id: claim.claimed_by_persona_id.clone().unwrap_or_default(),
        claim_json: claim
            .claim_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        created_at: claim.created_at.to_rfc3339(),
        updated_at: claim.updated_at.to_rfc3339(),
    }
}

fn proto_closure_gate(gate: &openakta_api_client::ClosureGateView) -> ClosureGate {
    ClosureGate {
        id: gate.id.to_string(),
        workspace_id: gate.workspace_id.to_string(),
        story_id: gate
            .story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        prepared_story_id: gate
            .prepared_story_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        gate_type: gate.gate_type.clone(),
        status: gate.status.clone(),
        decided_by_persona_id: gate.decided_by_persona_id.clone().unwrap_or_default(),
        rationale_md: gate.rationale_md.clone().unwrap_or_default(),
        created_at: gate.created_at.to_rfc3339(),
        updated_at: gate.updated_at.to_rfc3339(),
    }
}

fn proto_persona(persona: &openakta_api_client::PersonaView) -> Persona {
    Persona {
        id: persona.id.clone(),
        workspace_id: persona.workspace_id.to_string(),
        display_name: persona.display_name.clone(),
        accountability_md: persona.accountability_md.clone(),
        tool_scope_json: persona
            .tool_scope_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        memory_scope_json: persona
            .memory_scope_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        autonomy_policy_json: persona
            .autonomy_policy_json
            .clone()
            .map(|value| value.to_string())
            .unwrap_or_default(),
        active: persona.active,
        created_at: persona.created_at.to_rfc3339(),
        updated_at: persona.updated_at.to_rfc3339(),
    }
}

fn proto_persona_assignment(
    assignment: &openakta_api_client::PersonaAssignmentView,
) -> PersonaAssignment {
    PersonaAssignment {
        id: assignment.id.to_string(),
        workspace_id: assignment.workspace_id.to_string(),
        persona_id: assignment.persona_id.clone(),
        subject_type: assignment.subject_type.clone(),
        subject_id: assignment
            .subject_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        assignment_role: assignment.assignment_role.clone(),
        status: assignment.status.clone(),
        created_at: assignment.created_at.to_rfc3339(),
        updated_at: assignment.updated_at.to_rfc3339(),
    }
}

fn filter_story_scoped(
    item_story_id: Option<Uuid>,
    item_prepared_story_id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
) -> bool {
    (story_id.is_none() || item_story_id == story_id)
        && (prepared_story_id.is_none() || item_prepared_story_id == prepared_story_id)
}

fn scoped_requirement_ids(
    model: &ReadModelResponse,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
) -> HashSet<Uuid> {
    model
        .requirements
        .iter()
        .filter(|r| {
            filter_story_scoped(r.story_id, r.prepared_story_id, story_id, prepared_story_id)
        })
        .map(|r| r.id)
        .collect()
}

fn story_scoped_prepared_story_ids(
    model: &ReadModelResponse,
    story_id: Option<Uuid>,
) -> Option<HashSet<Uuid>> {
    let sid = story_id?;
    Some(
        model
            .story_preparations
            .iter()
            .filter(|p| p.story_id == sid)
            .map(|p| p.id)
            .collect(),
    )
}

fn filter_handoff_for_scope(
    contract: &openakta_api_client::HandoffContractView,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    prepared_ids_for_story: Option<&HashSet<Uuid>>,
) -> bool {
    if let Some(psid) = prepared_story_id {
        return contract.prepared_story_id == Some(psid);
    }
    if story_id.is_some() {
        return contract
            .prepared_story_id
            .map(|ps| prepared_ids_for_story.map_or(false, |s| s.contains(&ps)))
            .unwrap_or(false);
    }
    true
}

fn scoped_verification_run_ids(
    model: &ReadModelResponse,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
) -> HashSet<Uuid> {
    model
        .verification_runs
        .iter()
        .filter(|run| {
            filter_story_scoped(
                run.story_id,
                run.prepared_story_id,
                story_id,
                prepared_story_id,
            )
        })
        .map(|r| r.id)
        .collect()
}

fn closure_claim_matches_scope(
    claim: &openakta_api_client::ClosureClaimView,
    scoped_req_ids: &HashSet<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    work_items: &[openakta_api_client::WorkItemView],
) -> bool {
    if story_id.is_none() && prepared_story_id.is_none() {
        return true;
    }
    if let Some(rid) = claim.requirement_id {
        if scoped_req_ids.contains(&rid) {
            return true;
        }
    }
    if let Some(wid) = claim.work_item_id {
        if let Some(item) = work_items.iter().find(|i| i.id == wid) {
            return filter_story_scoped(
                item.story_id,
                item.prepared_story_id,
                story_id,
                prepared_story_id,
            );
        }
    }
    false
}

fn proto_evidence(item: &EvidenceLinkView) -> EvidenceLink {
    EvidenceLink {
        id: item.id.to_string(),
        workspace_id: item.workspace_id.to_string(),
        subject_type: item.subject_type.clone(),
        subject_id: item
            .subject_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        artifact_kind: item.artifact_kind.clone(),
        locator_json: item.locator_json.to_string(),
        content_hash: item.content_hash.clone(),
        storage_scope: item.storage_scope.clone(),
        preview_redacted: item.preview_redacted.clone().unwrap_or_default(),
        created_at: item.created_at.to_rfc3339(),
    }
}

/// When the hosted API cannot be reached, [`WorkManagementGrpc::resolve_clarifications`] falls back
/// to [`WorkMirror::resolve_clarifications`] (local-only). All other API failures surface to the
/// client so base-seq conflicts and validation errors are not silently mirrored.
fn clarification_mirror_fallback_allowed(err: &ApiError) -> bool {
    matches!(
        err,
        ApiError::Unavailable(_)
            | ApiError::ConnectionRefused(_)
            | ApiError::Timeout(_)
            | ApiError::CircuitOpen
    )
}

fn map_clarification_api_error(err: ApiError) -> Status {
    match err {
        ApiError::InvalidRequest(msg) => Status::invalid_argument(msg),
        ApiError::Unauthenticated(msg) | ApiError::AuthFailed(msg) => Status::unauthenticated(msg),
        ApiError::Unavailable(msg) => Status::unavailable(msg),
        ApiError::Timeout(msg) => Status::deadline_exceeded(msg),
        ApiError::RateLimited(msg) => Status::resource_exhausted(msg),
        other => Status::failed_precondition(other.to_string()),
    }
}
