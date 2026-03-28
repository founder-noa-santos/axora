use std::collections::{HashMap, HashSet};

use chrono::Utc;
use openakta_workflow::{
    check_legacy_create_work_item, check_legacy_patch_work_item, dedup_ids,
    evaluate_closure_with_mol, validate_preparation_transition,
    validate_story_intake_capture_status, validate_story_preparation_capture_status,
    AcceptanceCheckUpsertItem, AcceptanceCheckView, ClosureClaimView, ClosureEngineError,
    ClosureGateView, ClosureSnapshot, CommandEnvelope, CyclePhaseView,
    DeleteAcceptanceCheckPayload, DependencyEdgeView, ExecutionProfileDecisionView,
    HandoffContractView, KnowledgeArtifactView, MemoryPromotionEventView, MolError,
    MolFeatureFlags, PersonaAssignmentView, PersonaView, PlanVersionView, PlanningCycleView,
    ReadModelResponse, RecordClarificationAnswersPayload, RequirementCoverageView,
    RequirementEdgeView, RequirementView, StoryIntakeView, StoryPreparationView,
    UpsertAcceptanceChecksPayload, VerificationFindingView, VerificationRunView, WorkItemView,
    WorkspaceView,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AppliedCommand {
    pub read_model: ReadModelResponse,
    pub event_type: String,
    pub aggregate_id: Option<Uuid>,
    pub result_json: Value,
}

#[derive(Debug, Error)]
pub enum ApplyCommandError {
    #[error("workspace not found")]
    WorkspaceNotFound,
    #[error("{0} not found")]
    NotFound(&'static str),
    #[error("{0}")]
    BadRequest(String),
    #[error(transparent)]
    Mol(#[from] MolError),
    #[error(transparent)]
    Closure(#[from] ClosureEngineError),
}

impl ApplyCommandError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest(message.into())
    }
}

struct DefaultPersonaSpec {
    role: &'static str,
    display_name: &'static str,
    accountability_md: &'static str,
}

const DEFAULT_PERSONAS: &[DefaultPersonaSpec] = &[
    DefaultPersonaSpec {
        role: "backlog_steward",
        display_name: "Backlog Steward",
        accountability_md: "Owns intake classification, value framing, prioritization, urgency, non-goals, and clarification debt.",
    },
    DefaultPersonaSpec {
        role: "planning_steward",
        display_name: "Planning Steward",
        accountability_md: "Owns preparation workflow, decomposition boundaries, execution cards, and readiness gating.",
    },
    DefaultPersonaSpec {
        role: "architecture_steward",
        display_name: "Architecture Steward",
        accountability_md: "Owns cross-component design, contracts, invariants, compatibility, and architecture acceptance checks.",
    },
    DefaultPersonaSpec {
        role: "implementation_steward",
        display_name: "Implementation Steward",
        accountability_md: "Owns code change execution and requirement-scoped completion claims within mission guardrails.",
    },
    DefaultPersonaSpec {
        role: "review_steward",
        display_name: "Review Steward",
        accountability_md: "Owns defect-prevention review, change-risk review, and merge-quality contracts.",
    },
    DefaultPersonaSpec {
        role: "verification_steward",
        display_name: "Verification Steward",
        accountability_md: "Owns independent proof, acceptance checks, verification runs, and closure evidence sufficiency.",
    },
    DefaultPersonaSpec {
        role: "reliability_steward",
        display_name: "Reliability Steward",
        accountability_md: "Owns operational risk, rollout brakes, incident-linked strictness escalation, and safety throttles.",
    },
    DefaultPersonaSpec {
        role: "knowledge_steward",
        display_name: "Knowledge Steward",
        accountability_md: "Owns canonical project truth, glossary, playbooks, documentation alignment, and memory promotion decisions.",
    },
];

#[derive(Debug, Deserialize)]
struct CreateWorkspacePayload {
    slug: String,
    name: String,
}

#[derive(Debug, Deserialize)]
struct CreatePlanningCyclePayload {
    id: Option<Uuid>,
    cadence_mode: String,
    planning_mode: String,
    start_at: Option<chrono::DateTime<chrono::Utc>>,
    end_at: Option<chrono::DateTime<chrono::Utc>>,
    status: String,
    global_wip_limit: Option<i32>,
    replanning_interval_secs: Option<i32>,
    #[serde(default)]
    phases: Vec<CreateCyclePhasePayload>,
}

#[derive(Debug, Deserialize)]
struct CreateCyclePhasePayload {
    id: Option<Uuid>,
    phase_key: String,
    ordinal: i32,
    #[serde(default)]
    strict_barrier: bool,
    phase_wip_limit: Option<i32>,
    exit_criteria_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CreateWorkItemPayload {
    id: Option<Uuid>,
    cycle_id: Option<Uuid>,
    parent_id: Option<Uuid>,
    #[serde(rename = "type")]
    item_type: String,
    execution_profile: String,
    title: String,
    description_md: Option<String>,
    tracker_state: String,
    run_state: String,
    priority: Option<i32>,
    assignee_user_id: Option<Uuid>,
    external_master: Option<bool>,
    wave_label: Option<String>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    owner_persona_id: Option<String>,
    requirement_slice_json: Option<Value>,
    handoff_contract_state: Option<String>,
    claim_state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PatchWorkItemPayload {
    work_item_id: Uuid,
    title: Option<String>,
    description_md: Option<String>,
    priority: Option<i32>,
    tracker_state: Option<String>,
    run_state: Option<String>,
    wave_label: Option<String>,
    assignee_user_id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    owner_persona_id: Option<String>,
    requirement_slice_json: Option<Value>,
    handoff_contract_state: Option<String>,
    claim_state: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AddDependencyPayload {
    from_item_id: Uuid,
    to_item_id: Uuid,
    edge_type: String,
    strength: String,
}

#[derive(Debug, Deserialize)]
struct PublishPlanVersionPayload {
    id: Option<Uuid>,
    cycle_id: Option<Uuid>,
    story_id: Option<Uuid>,
    snapshot_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CaptureStoryIntakePayload {
    id: Option<Uuid>,
    external_ref: Option<String>,
    title: String,
    raw_request_md: String,
    source_kind: String,
    status: String,
    urgency: String,
    priority_band: String,
    affected_surfaces_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct PrepareStoryPayload {
    id: Option<Uuid>,
    story_id: Uuid,
    status: String,
    mission_card_json: Value,
    execution_card_json: Option<Value>,
    dependency_summary_json: Option<Value>,
    readiness_blockers_json: Option<Value>,
    primary_execution_profile: String,
    ready_at: Option<chrono::DateTime<chrono::Utc>>,
}

fn default_reconcile_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct UpsertRequirementGraphPayload {
    requirements: Vec<RequirementUpsertPayload>,
    #[serde(default)]
    edges: Vec<RequirementEdgeUpsertPayload>,
    #[serde(default = "default_reconcile_true")]
    reconcile: bool,
    #[serde(default)]
    reconcile_prepared_story_id: Option<Uuid>,
    #[serde(default)]
    reconcile_story_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RequirementUpsertPayload {
    id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    plan_version_id: Option<Uuid>,
    parent_requirement_id: Option<Uuid>,
    title: String,
    statement: String,
    kind: String,
    criticality: String,
    source: String,
    ambiguity_state: String,
    owner_persona_id: Option<String>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct RequirementEdgeUpsertPayload {
    id: Option<Uuid>,
    requirement_id: Uuid,
    related_requirement_id: Uuid,
    edge_type: String,
}

#[derive(Debug, Clone, Copy)]
enum RequirementGraphScope {
    PreparedStory(Uuid),
    StoryOnly(Uuid),
}

#[derive(Debug, Deserialize)]
struct LinkRequirementCoveragePayload {
    id: Option<Uuid>,
    requirement_id: Uuid,
    work_item_id: Uuid,
    coverage_kind: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct RecordProfileDecisionPayload {
    id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    profile_name: String,
    policy_json: Value,
    inferred_from_json: Option<Value>,
    override_reason_md: Option<String>,
    escalation_level: String,
    decided_by: String,
}

#[derive(Debug, Deserialize)]
struct CreateHandoffContractPayload {
    id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    from_work_item_id: Option<Uuid>,
    to_work_item_id: Option<Uuid>,
    contract_kind: String,
    expected_artifact_json: Option<Value>,
    acceptance_signal_json: Option<Value>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct RecordCompletionClaimPayload {
    id: Option<Uuid>,
    work_item_id: Option<Uuid>,
    requirement_id: Option<Uuid>,
    claim_type: String,
    status: String,
    claimed_by_persona_id: Option<String>,
    claim_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct StartVerificationRunPayload {
    id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    status: String,
    verification_stage: String,
    run_kind: String,
    initiated_by_persona_id: Option<String>,
    summary_json: Option<Value>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct UpdateVerificationRunPayload {
    id: Uuid,
    status: String,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    summary_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct RecordVerificationFindingPayload {
    id: Option<Uuid>,
    verification_run_id: Uuid,
    requirement_id: Option<Uuid>,
    severity: String,
    finding_type: String,
    title: String,
    detail_md: Option<String>,
    status: String,
}

#[derive(Debug, Deserialize)]
struct AdvanceClosureGatePayload {
    id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    gate_type: String,
    status: String,
    decided_by_persona_id: Option<String>,
    rationale_md: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AssignPersonaPayload {
    persona_id: String,
    display_name: Option<String>,
    accountability_md: Option<String>,
    tool_scope_json: Option<Value>,
    memory_scope_json: Option<Value>,
    autonomy_policy_json: Option<Value>,
    active: Option<bool>,
    assignment_id: Option<Uuid>,
    subject_type: Option<String>,
    subject_id: Option<Uuid>,
    assignment_role: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PromoteKnowledgeArtifactPayload {
    id: Option<Uuid>,
    persona_id: Option<String>,
    title: String,
    artifact_kind: String,
    body_md: Option<String>,
    source_refs_json: Option<Value>,
    status: String,
    promotion_event_id: Option<Uuid>,
    source_kind: Option<String>,
    source_ref: Option<String>,
    outcome: Option<String>,
    summary_json: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct MarkDocumentationAlignmentPayload {
    gate_id: Option<Uuid>,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    status: String,
    decided_by_persona_id: Option<String>,
    rationale_md: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TransitionStoryPreparationPayload {
    prepared_story_id: Uuid,
    status: String,
    readiness_blockers_json: Option<Value>,
    ready_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
struct ApprovePlanVersionPayload {
    plan_version_id: Uuid,
}

pub fn apply_command(
    current: Option<ReadModelResponse>,
    workspace_id: Uuid,
    command: &CommandEnvelope,
    mol: MolFeatureFlags,
) -> Result<AppliedCommand, ApplyCommandError> {
    let now = Utc::now();
    let actor_user_id = parse_actor_user_id(command.actor_context.as_ref());
    let actor_tenant_id = parse_actor_tenant_id(command.actor_context.as_ref());

    let mut model = if command.command_type == "create_workspace" {
        current
    } else {
        Some(current.ok_or(ApplyCommandError::WorkspaceNotFound)?)
    };

    match command.command_type.as_str() {
        "create_workspace" => {
            let payload: CreateWorkspacePayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let mut rm = model.take().unwrap_or_else(|| {
                empty_workspace(
                    workspace_id,
                    actor_tenant_id,
                    actor_user_id,
                    &payload.slug,
                    &payload.name,
                    now,
                )
            });
            rm.workspace.slug = payload.slug;
            rm.workspace.name = payload.name;
            ensure_default_personas(&mut rm, workspace_id, now);
            Ok(success(
                rm,
                "workspace.created",
                Some(workspace_id),
                json!({ "workspace_id": workspace_id }),
            ))
        }
        "create_planning_cycle" => {
            let mut rm = model.unwrap();
            let payload: CreatePlanningCyclePayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_cycle_status(&payload.status)?;
            let cycle_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.cycles.push(PlanningCycleView {
                id: cycle_id,
                workspace_id,
                cadence_mode: payload.cadence_mode,
                planning_mode: payload.planning_mode,
                start_at: payload.start_at,
                end_at: payload.end_at,
                status: payload.status,
                global_wip_limit: payload.global_wip_limit,
                replanning_interval_secs: payload.replanning_interval_secs,
                created_at: now,
            });
            for phase in payload.phases {
                rm.phases.push(CyclePhaseView {
                    id: phase.id.unwrap_or_else(Uuid::new_v4),
                    cycle_id,
                    phase_key: phase.phase_key,
                    ordinal: phase.ordinal,
                    strict_barrier: phase.strict_barrier,
                    phase_wip_limit: phase.phase_wip_limit,
                    exit_criteria_json: phase.exit_criteria_json,
                    created_at: now,
                });
            }
            Ok(success(
                rm,
                "planning_cycle.created",
                Some(cycle_id),
                json!({ "cycle_id": cycle_id }),
            ))
        }
        "create_work_item" => {
            let mut rm = model.unwrap();
            let payload: CreateWorkItemPayload = serde_json::from_value(command.payload.clone())
                .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            check_legacy_create_work_item(
                mol,
                payload.prepared_story_id,
                payload.owner_persona_id.as_ref(),
                payload.requirement_slice_json.as_ref(),
                payload.handoff_contract_state.as_ref(),
                payload.claim_state.as_ref(),
            )?;
            validate_work_item_type(&payload.item_type)?;
            validate_execution_profile(&payload.execution_profile)?;
            validate_tracker_state(&payload.tracker_state, None)?;
            validate_run_state(&payload.run_state, None)?;
            ensure_cycle_belongs(&rm, payload.cycle_id)?;
            validate_parent_chain(&rm.work_items, payload.parent_id, None)?;
            let wave_rank = normalize_wave_label_rank(payload.wave_label.as_deref())?;
            let item_id = payload.id.unwrap_or_else(Uuid::new_v4);
            if rm.work_items.iter().any(|item| item.id == item_id) {
                return Err(ApplyCommandError::bad_request("work item already exists"));
            }
            rm.work_items.push(WorkItemView {
                id: item_id,
                workspace_id,
                cycle_id: payload.cycle_id,
                parent_id: payload.parent_id,
                item_type: payload.item_type,
                execution_profile: payload.execution_profile,
                title: payload.title,
                description_md: payload.description_md,
                tracker_state: payload.tracker_state,
                run_state: payload.run_state,
                priority: payload.priority.unwrap_or(50).clamp(0, 100),
                assignee_user_id: payload.assignee_user_id,
                external_master: payload.external_master.unwrap_or(false),
                wave_rank,
                wave_label: payload.wave_label,
                story_id: payload.story_id,
                prepared_story_id: payload.prepared_story_id,
                owner_persona_id: payload.owner_persona_id,
                requirement_slice_json: payload.requirement_slice_json,
                handoff_contract_state: payload.handoff_contract_state,
                claim_state: payload.claim_state,
                updated_at: now,
                created_at: now,
            });
            Ok(success(
                rm,
                "work_item.created",
                Some(item_id),
                json!({ "work_item_id": item_id }),
            ))
        }
        "patch_item" => {
            let mut rm = model.unwrap();
            let payload: PatchWorkItemPayload = serde_json::from_value(command.payload.clone())
                .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let item = rm
                .work_items
                .iter_mut()
                .find(|item| item.id == payload.work_item_id)
                .ok_or(ApplyCommandError::NotFound("work item"))?;
            let effective_prepared_story_id = payload.prepared_story_id.or(item.prepared_story_id);
            check_legacy_patch_work_item(
                mol,
                effective_prepared_story_id,
                payload.owner_persona_id.as_ref(),
                payload.requirement_slice_json.as_ref(),
                payload.handoff_contract_state.as_ref(),
                payload.claim_state.as_ref(),
            )?;
            let tracker_state = match payload.tracker_state {
                Some(next) => {
                    validate_tracker_state(&next, Some(&item.tracker_state))?;
                    next
                }
                None => item.tracker_state.clone(),
            };
            let run_state = match payload.run_state {
                Some(next) => {
                    validate_run_state(&next, Some(&item.run_state))?;
                    next
                }
                None => item.run_state.clone(),
            };
            let wave_label = payload.wave_label.or(item.wave_label.clone());
            let wave_rank = normalize_wave_label_rank(wave_label.as_deref())?;
            item.title = payload.title.unwrap_or_else(|| item.title.clone());
            item.description_md = payload.description_md.or(item.description_md.clone());
            item.priority = payload.priority.unwrap_or(item.priority).clamp(0, 100);
            item.tracker_state = tracker_state;
            item.run_state = run_state;
            item.wave_rank = wave_rank;
            item.wave_label = wave_label;
            item.assignee_user_id = payload.assignee_user_id.or(item.assignee_user_id);
            item.story_id = payload.story_id.or(item.story_id);
            item.prepared_story_id = payload.prepared_story_id.or(item.prepared_story_id);
            item.owner_persona_id = payload.owner_persona_id.or(item.owner_persona_id.clone());
            item.requirement_slice_json = payload
                .requirement_slice_json
                .or(item.requirement_slice_json.clone());
            item.handoff_contract_state = payload
                .handoff_contract_state
                .or(item.handoff_contract_state.clone());
            item.claim_state = payload.claim_state.or(item.claim_state.clone());
            item.updated_at = now;
            Ok(success(
                rm,
                "work_item.updated",
                Some(payload.work_item_id),
                json!({ "work_item_id": payload.work_item_id }),
            ))
        }
        "add_dependency" => {
            let mut rm = model.unwrap();
            let payload: AddDependencyPayload = serde_json::from_value(command.payload.clone())
                .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_dependency_edge_type(&payload.edge_type)?;
            validate_dependency_strength(&payload.strength)?;
            let from = rm
                .work_items
                .iter()
                .find(|item| item.id == payload.from_item_id)
                .ok_or(ApplyCommandError::NotFound("work item"))?;
            let to = rm
                .work_items
                .iter()
                .find(|item| item.id == payload.to_item_id)
                .ok_or(ApplyCommandError::NotFound("work item"))?;
            validate_wave_dependency(from.wave_rank, to.wave_rank)?;
            validate_dependency_acyclic(
                &rm.dependencies,
                payload.from_item_id,
                payload.to_item_id,
                &payload.edge_type,
            )?;
            let edge_id = Uuid::new_v4();
            rm.dependencies.push(DependencyEdgeView {
                id: edge_id,
                workspace_id,
                from_item_id: payload.from_item_id,
                to_item_id: payload.to_item_id,
                edge_type: payload.edge_type,
                strength: payload.strength,
                created_at: now,
            });
            Ok(success(
                rm,
                "dependency.added",
                Some(edge_id),
                json!({ "edge_id": edge_id }),
            ))
        }
        "publish_plan_version" => {
            let mut rm = model.unwrap();
            let payload: PublishPlanVersionPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            ensure_cycle_belongs(&rm, payload.cycle_id)?;
            let snapshot_json = payload.snapshot_json.unwrap_or_else(
                || json!({ "workspace_id": workspace_id, "cycle_id": payload.cycle_id }),
            );
            let plan_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.plan_versions.push(PlanVersionView {
                id: plan_id,
                workspace_id,
                cycle_id: payload.cycle_id,
                story_id: payload.story_id,
                base_seq: command.base_seq,
                plan_hash: hash_json(&snapshot_json)?,
                snapshot_json,
                status: "draft".to_string(),
                created_by: actor_user_id,
                approved_by: None,
                created_at: now,
                approved_at: None,
            });
            Ok(success(
                rm,
                "plan_version.published",
                Some(plan_id),
                json!({ "plan_version_id": plan_id }),
            ))
        }
        "approve_plan_version" => {
            let mut rm = model.unwrap();
            let payload: ApprovePlanVersionPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let plan = rm
                .plan_versions
                .iter_mut()
                .find(|plan| plan.id == payload.plan_version_id)
                .ok_or(ApplyCommandError::NotFound("plan version"))?;
            plan.status = "approved".to_string();
            plan.approved_by = Some(actor_user_id);
            plan.approved_at = Some(now);
            Ok(success(
                rm,
                "plan_version.approved",
                Some(payload.plan_version_id),
                json!({ "plan_version_id": payload.plan_version_id }),
            ))
        }
        "capture_story_intake" => {
            let mut rm = model.unwrap();
            let payload: CaptureStoryIntakePayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            ensure_default_personas(&mut rm, workspace_id, now);
            validate_story_intake_status(&payload.status)?;
            validate_story_intake_capture_status(&payload.status)?;
            let story_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.story_intakes.push(StoryIntakeView {
                id: story_id,
                workspace_id,
                external_ref: payload.external_ref,
                title: payload.title,
                raw_request_md: payload.raw_request_md,
                source_kind: payload.source_kind,
                status: payload.status,
                urgency: payload.urgency,
                priority_band: payload.priority_band,
                affected_surfaces_json: payload.affected_surfaces_json,
                created_by: actor_user_id,
                created_at: now,
                updated_at: now,
            });
            Ok(success(
                rm,
                "story_intake.captured",
                Some(story_id),
                json!({ "story_id": story_id }),
            ))
        }
        "prepare_story" => {
            let mut rm = model.unwrap();
            let payload: PrepareStoryPayload = serde_json::from_value(command.payload.clone())
                .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            ensure_default_personas(&mut rm, workspace_id, now);
            validate_story_preparation_status(&payload.status)?;
            validate_story_preparation_capture_status(&payload.status)?;
            validate_profile_name(&payload.primary_execution_profile)?;
            if !rm
                .story_intakes
                .iter()
                .any(|story| story.id == payload.story_id)
            {
                return Err(ApplyCommandError::NotFound("story intake"));
            }
            let preparation_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.story_preparations.push(StoryPreparationView {
                id: preparation_id,
                workspace_id,
                story_id: payload.story_id,
                status: payload.status,
                mission_card_json: payload.mission_card_json,
                execution_card_json: payload.execution_card_json,
                dependency_summary_json: payload.dependency_summary_json,
                readiness_blockers_json: payload.readiness_blockers_json,
                primary_execution_profile: payload.primary_execution_profile,
                created_by: actor_user_id,
                created_at: now,
                updated_at: now,
                ready_at: payload.ready_at,
            });
            Ok(success(
                rm,
                "story_preparation.created",
                Some(preparation_id),
                json!({ "prepared_story_id": preparation_id, "story_id": payload.story_id }),
            ))
        }
        "record_profile_decision" => {
            let mut rm = model.unwrap();
            let payload: RecordProfileDecisionPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            ensure_default_personas(&mut rm, workspace_id, now);
            validate_profile_name(&payload.profile_name)?;
            validate_escalation_level(&payload.escalation_level)?;
            let profile_decision_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.execution_profile_decisions
                .push(ExecutionProfileDecisionView {
                    id: profile_decision_id,
                    workspace_id,
                    story_id: payload.story_id,
                    prepared_story_id: payload.prepared_story_id,
                    profile_name: payload.profile_name,
                    policy_json: payload.policy_json,
                    inferred_from_json: payload.inferred_from_json,
                    override_reason_md: payload.override_reason_md,
                    escalation_level: payload.escalation_level,
                    decided_by: payload.decided_by,
                    created_at: now,
                    updated_at: now,
                });
            Ok(success(
                rm,
                "execution_profile.decision_recorded",
                Some(profile_decision_id),
                json!({ "profile_decision_id": profile_decision_id }),
            ))
        }
        "upsert_requirement_graph" => {
            let mut rm = model.unwrap();
            let payload: UpsertRequirementGraphPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            ensure_default_personas(&mut rm, workspace_id, now);
            let scope = resolve_requirement_graph_scope(&payload)?;
            let mut kept_ids = HashSet::new();
            let mut touched = Vec::new();
            for requirement in &payload.requirements {
                validate_requirement_ambiguity_state(&requirement.ambiguity_state)?;
                validate_requirement_status(&requirement.status)?;
                let requirement_id = requirement.id.unwrap_or_else(Uuid::new_v4);
                kept_ids.insert(requirement_id);
                upsert_requirement(
                    &mut rm,
                    RequirementView {
                        id: requirement_id,
                        workspace_id,
                        story_id: requirement.story_id,
                        prepared_story_id: requirement.prepared_story_id,
                        plan_version_id: requirement.plan_version_id,
                        parent_requirement_id: requirement.parent_requirement_id,
                        title: requirement.title.clone(),
                        statement: requirement.statement.clone(),
                        kind: requirement.kind.clone(),
                        criticality: requirement.criticality.clone(),
                        source: requirement.source.clone(),
                        ambiguity_state: requirement.ambiguity_state.clone(),
                        owner_persona_id: requirement.owner_persona_id.clone(),
                        status: requirement.status.clone(),
                        created_at: now,
                        updated_at: now,
                    },
                );
                touched.push(requirement_id);
            }
            for edge in &payload.edges {
                if !kept_ids.contains(&edge.requirement_id)
                    || !kept_ids.contains(&edge.related_requirement_id)
                {
                    return Err(ApplyCommandError::bad_request(
                        "requirement edge references unknown requirement id",
                    ));
                }
            }
            if payload.reconcile {
                if let Some(scope) = scope {
                    let to_remove = requirement_ids_for_scope(&rm, scope)
                        .into_iter()
                        .filter(|id| !kept_ids.contains(id))
                        .collect::<Vec<_>>();
                    for id in to_remove {
                        remove_requirement_by_id(&mut rm, id);
                    }
                }
                prune_stale_requirement_edges(&mut rm, &kept_ids, &payload.edges);
            }
            for edge in &payload.edges {
                upsert_requirement_edge(
                    &mut rm,
                    RequirementEdgeView {
                        id: edge.id.unwrap_or_else(Uuid::new_v4),
                        workspace_id,
                        requirement_id: edge.requirement_id,
                        related_requirement_id: edge.related_requirement_id,
                        edge_type: edge.edge_type.clone(),
                        created_at: now,
                    },
                );
            }
            Ok(success(
                rm,
                "requirement_graph.upserted",
                touched.first().copied(),
                json!({ "requirement_ids": touched }),
            ))
        }
        "link_requirement_coverage" => {
            let mut rm = model.unwrap();
            let payload: LinkRequirementCoveragePayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let wi = rm
                .work_items
                .iter()
                .find(|item| item.id == payload.work_item_id)
                .cloned()
                .ok_or(ApplyCommandError::NotFound("work item"))?;
            let req = rm
                .requirements
                .iter()
                .find(|req| req.id == payload.requirement_id)
                .cloned()
                .ok_or(ApplyCommandError::NotFound("requirement"))?;
            let prep_story = wi
                .prepared_story_id
                .and_then(|prepared_story_id| story_id_for_prepared_story(&rm, prepared_story_id));
            if !requirement_aligned_with_work_item_pure(
                req.prepared_story_id,
                req.story_id,
                wi.prepared_story_id,
                wi.story_id,
                prep_story,
            ) {
                return Err(ApplyCommandError::bad_request(
                    "requirement is not in the same story scope as the work item",
                ));
            }
            let coverage_id = upsert_requirement_coverage(
                &mut rm,
                RequirementCoverageView {
                    id: payload.id.unwrap_or_else(Uuid::new_v4),
                    workspace_id,
                    requirement_id: payload.requirement_id,
                    work_item_id: payload.work_item_id,
                    coverage_kind: payload.coverage_kind,
                    status: payload.status,
                    created_at: now,
                    updated_at: now,
                },
            );
            let pruned =
                prune_orphan_requirement_coverage_for_work_item(&mut rm, payload.work_item_id);
            Ok(success(
                rm,
                "requirement_coverage.linked",
                Some(coverage_id),
                json!({ "coverage_id": coverage_id, "pruned_rows": pruned }),
            ))
        }
        "upsert_acceptance_checks" => {
            let mut rm = model.unwrap();
            let payload: UpsertAcceptanceChecksPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            if payload.checks.is_empty() {
                return Err(ApplyCommandError::bad_request("checks must not be empty"));
            }
            let mut touched = Vec::with_capacity(payload.checks.len());
            for check in payload.checks {
                validate_acceptance_check(&rm, &check)?;
                let check_id = check.id.unwrap_or_else(Uuid::new_v4);
                upsert_acceptance_check(
                    &mut rm,
                    AcceptanceCheckView {
                        id: check_id,
                        workspace_id,
                        requirement_id: check.requirement_id,
                        check_kind: check.check_kind,
                        title: check.title,
                        status: check.status,
                        evidence_required: check.evidence_required,
                        created_at: now,
                        updated_at: now,
                    },
                );
                touched.push(check_id);
            }
            Ok(success(
                rm,
                "acceptance_check.upserted",
                touched.first().copied(),
                json!({ "acceptance_check_ids": touched }),
            ))
        }
        "delete_acceptance_check" => {
            let mut rm = model.unwrap();
            let payload: DeleteAcceptanceCheckPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let before = rm.acceptance_checks.len();
            rm.acceptance_checks
                .retain(|check| check.id != payload.acceptance_check_id);
            if rm.acceptance_checks.len() == before {
                return Err(ApplyCommandError::NotFound("acceptance check"));
            }
            Ok(success(
                rm,
                "acceptance_check.deleted",
                Some(payload.acceptance_check_id),
                json!({ "acceptance_check_id": payload.acceptance_check_id }),
            ))
        }
        "record_clarification_answers" => {
            let mut rm = model.unwrap();
            let payload: RecordClarificationAnswersPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            if payload.answers.is_empty() {
                return Err(ApplyCommandError::bad_request("answers must not be empty"));
            }
            let answer_ids: HashSet<Uuid> = payload
                .answers
                .iter()
                .map(|answer| answer.clarification_item_id)
                .collect();
            let mut resolved = 0usize;
            for item in &mut rm.clarifications {
                if answer_ids.contains(&item.id) {
                    item.status = "answered".to_string();
                    item.answered_at = Some(now);
                    resolved += 1;
                }
            }
            if resolved != answer_ids.len() {
                return Err(ApplyCommandError::NotFound("clarification item"));
            }
            Ok(success(
                rm,
                "clarification.answers_recorded",
                Some(payload.session_id),
                json!({
                    "session_id": payload.session_id,
                    "resolved_count": resolved,
                }),
            ))
        }
        "create_handoff_contract" => {
            let mut rm = model.unwrap();
            let payload: CreateHandoffContractPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            let contract_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.handoff_contracts.push(HandoffContractView {
                id: contract_id,
                workspace_id,
                prepared_story_id: payload.prepared_story_id,
                from_work_item_id: payload.from_work_item_id,
                to_work_item_id: payload.to_work_item_id,
                contract_kind: payload.contract_kind,
                expected_artifact_json: payload.expected_artifact_json,
                acceptance_signal_json: payload.acceptance_signal_json,
                status: payload.status,
                created_at: now,
                updated_at: now,
            });
            Ok(success(
                rm,
                "handoff_contract.created",
                Some(contract_id),
                json!({ "handoff_contract_id": contract_id }),
            ))
        }
        "record_completion_claim" => {
            let mut rm = model.unwrap();
            let payload: RecordCompletionClaimPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_closure_claim_status(&payload.status)?;
            if let Some(pid) = payload.claimed_by_persona_id.as_deref() {
                ensure_persona_active(&rm, workspace_id, pid)?;
            }
            let claim_id = payload.id.unwrap_or_else(Uuid::new_v4);
            upsert_closure_claim(
                &mut rm,
                ClosureClaimView {
                    id: claim_id,
                    workspace_id,
                    work_item_id: payload.work_item_id,
                    requirement_id: payload.requirement_id,
                    claim_type: payload.claim_type,
                    status: payload.status,
                    claimed_by_persona_id: payload.claimed_by_persona_id,
                    claim_json: payload.claim_json,
                    created_at: now,
                    updated_at: now,
                },
            );
            Ok(success(
                rm,
                "closure_claim.recorded",
                Some(claim_id),
                json!({ "closure_claim_id": claim_id }),
            ))
        }
        "start_verification_run" => {
            let mut rm = model.unwrap();
            let payload: StartVerificationRunPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_verification_status(&payload.status)?;
            validate_verification_stage(&payload.verification_stage)?;
            if let Some(pid) = payload.initiated_by_persona_id.as_deref() {
                ensure_persona_active(&rm, workspace_id, pid)?;
            }
            let run_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.verification_runs.push(VerificationRunView {
                id: run_id,
                workspace_id,
                story_id: payload.story_id,
                prepared_story_id: payload.prepared_story_id,
                status: payload.status,
                verification_stage: payload.verification_stage,
                run_kind: payload.run_kind,
                initiated_by_persona_id: payload.initiated_by_persona_id,
                summary_json: payload.summary_json,
                created_at: now,
                completed_at: payload.completed_at,
            });
            Ok(success(
                rm,
                "verification_run.started",
                Some(run_id),
                json!({ "verification_run_id": run_id }),
            ))
        }
        "record_verification_finding" => {
            let mut rm = model.unwrap();
            let payload: RecordVerificationFindingPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_finding_severity(&payload.severity)?;
            validate_finding_status(&payload.status)?;
            if !rm
                .verification_runs
                .iter()
                .any(|run| run.id == payload.verification_run_id)
            {
                return Err(ApplyCommandError::NotFound("verification run"));
            }
            let finding_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.verification_findings.push(VerificationFindingView {
                id: finding_id,
                workspace_id,
                verification_run_id: payload.verification_run_id,
                requirement_id: payload.requirement_id,
                severity: payload.severity,
                finding_type: payload.finding_type,
                title: payload.title,
                detail_md: payload.detail_md,
                status: payload.status,
                created_at: now,
                updated_at: now,
            });
            Ok(success(
                rm,
                "verification_finding.recorded",
                Some(finding_id),
                json!({ "verification_finding_id": finding_id }),
            ))
        }
        "update_verification_run" => {
            let mut rm = model.unwrap();
            let payload: UpdateVerificationRunPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_verification_status(&payload.status)?;
            let run = rm
                .verification_runs
                .iter_mut()
                .find(|run| run.id == payload.id)
                .ok_or(ApplyCommandError::NotFound("verification run"))?;
            let completed_at = match payload.status.as_str() {
                "passed" | "failed" | "blocked" => Some(payload.completed_at.unwrap_or(now)),
                "running" | "pending" => None,
                _ => payload.completed_at,
            };
            run.status = payload.status;
            run.completed_at = completed_at;
            run.summary_json = merge_optional_json(run.summary_json.take(), payload.summary_json);
            Ok(success(
                rm,
                "verification_run.updated",
                Some(payload.id),
                json!({ "verification_run_id": payload.id }),
            ))
        }
        "advance_closure_gate" => {
            let mut rm = model.unwrap();
            let payload: AdvanceClosureGatePayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_closure_gate_type(&payload.gate_type)?;
            validate_closure_gate_status(&payload.status)?;
            if let Some(pid) = payload.decided_by_persona_id.as_deref() {
                ensure_persona_active(&rm, workspace_id, pid)?;
            }
            let gate_id = payload.id.unwrap_or_else(Uuid::new_v4);
            upsert_closure_gate(
                &mut rm,
                ClosureGateView {
                    id: gate_id,
                    workspace_id,
                    story_id: payload.story_id,
                    prepared_story_id: payload.prepared_story_id,
                    gate_type: payload.gate_type,
                    status: payload.status,
                    decided_by_persona_id: payload.decided_by_persona_id,
                    rationale_md: payload.rationale_md,
                    created_at: now,
                    updated_at: now,
                },
            );
            Ok(success(
                rm,
                "closure_gate.advanced",
                Some(gate_id),
                json!({ "closure_gate_id": gate_id }),
            ))
        }
        "transition_story_preparation" => {
            let mut rm = model.unwrap();
            let payload: TransitionStoryPreparationPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_story_preparation_status(&payload.status)?;
            let current_status = rm
                .story_preparations
                .iter()
                .find(|story| story.id == payload.prepared_story_id)
                .map(|story| story.status.clone())
                .ok_or(ApplyCommandError::NotFound("prepared story"))?;
            validate_preparation_transition(&current_status, &payload.status)?;
            if payload.status == "closed" {
                let snapshot = build_closure_snapshot(&rm, payload.prepared_story_id)?;
                let verification_required =
                    resolve_verification_required(&rm, payload.prepared_story_id)?;
                evaluate_closure_with_mol(&snapshot, mol, verification_required)?;
            }
            let ready_at = if payload.status == "ready" {
                Some(payload.ready_at.unwrap_or(now))
            } else {
                payload.ready_at
            };
            let prep = rm
                .story_preparations
                .iter_mut()
                .find(|story| story.id == payload.prepared_story_id)
                .ok_or(ApplyCommandError::NotFound("prepared story"))?;
            prep.status = payload.status;
            if payload.readiness_blockers_json.is_some() {
                prep.readiness_blockers_json = payload.readiness_blockers_json;
            }
            if ready_at.is_some() {
                prep.ready_at = ready_at;
            }
            prep.updated_at = now;
            Ok(success(
                rm,
                "story_preparation.transitioned",
                Some(payload.prepared_story_id),
                json!({ "prepared_story_id": payload.prepared_story_id }),
            ))
        }
        "assign_persona" => {
            let mut rm = model.unwrap();
            let payload: AssignPersonaPayload = serde_json::from_value(command.payload.clone())
                .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            upsert_persona(
                &mut rm,
                PersonaView {
                    id: payload.persona_id.clone(),
                    workspace_id,
                    display_name: payload
                        .display_name
                        .unwrap_or_else(|| payload.persona_id.clone()),
                    accountability_md: payload.accountability_md.unwrap_or_default(),
                    tool_scope_json: payload.tool_scope_json,
                    memory_scope_json: payload.memory_scope_json,
                    autonomy_policy_json: payload.autonomy_policy_json,
                    active: payload.active.unwrap_or(true),
                    created_at: now,
                    updated_at: now,
                },
            );
            let assignment_id =
                if payload.subject_type.is_some() || payload.assignment_role.is_some() {
                    let assignment_id = payload.assignment_id.unwrap_or_else(Uuid::new_v4);
                    upsert_persona_assignment(
                        &mut rm,
                        PersonaAssignmentView {
                            id: assignment_id,
                            workspace_id,
                            persona_id: payload.persona_id.clone(),
                            subject_type: payload
                                .subject_type
                                .unwrap_or_else(|| "workspace".to_string()),
                            subject_id: payload.subject_id,
                            assignment_role: payload
                                .assignment_role
                                .unwrap_or_else(|| "owner".to_string()),
                            status: payload.status.unwrap_or_else(|| "active".to_string()),
                            created_at: now,
                            updated_at: now,
                        },
                    );
                    Some(assignment_id)
                } else {
                    None
                };
            Ok(success(
                rm,
                "persona.assigned",
                assignment_id,
                json!({ "persona_id": payload.persona_id, "assignment_id": assignment_id }),
            ))
        }
        "promote_knowledge_artifact" => {
            let mut rm = model.unwrap();
            let payload: PromoteKnowledgeArtifactPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            if let Some(pid) = payload.persona_id.as_deref() {
                ensure_persona_active(&rm, workspace_id, pid)?;
            }
            let artifact_id = payload.id.unwrap_or_else(Uuid::new_v4);
            rm.knowledge_artifacts.push(KnowledgeArtifactView {
                id: artifact_id,
                workspace_id,
                persona_id: payload.persona_id,
                title: payload.title,
                artifact_kind: payload.artifact_kind,
                body_md: payload.body_md,
                source_refs_json: payload.source_refs_json,
                status: payload.status,
                created_at: now,
                updated_at: now,
            });
            let promotion_event_id = if payload.source_kind.is_some() || payload.outcome.is_some() {
                let event_id = payload.promotion_event_id.unwrap_or_else(Uuid::new_v4);
                rm.memory_promotion_events.push(MemoryPromotionEventView {
                    id: event_id,
                    workspace_id,
                    knowledge_artifact_id: Some(artifact_id),
                    source_kind: payload.source_kind.unwrap_or_else(|| "manual".to_string()),
                    source_ref: payload.source_ref,
                    outcome: payload.outcome.unwrap_or_else(|| "promoted".to_string()),
                    summary_json: payload.summary_json,
                    created_at: now,
                });
                Some(event_id)
            } else {
                None
            };
            Ok(success(
                rm,
                "knowledge_artifact.promoted",
                Some(artifact_id),
                json!({ "knowledge_artifact_id": artifact_id, "memory_promotion_event_id": promotion_event_id }),
            ))
        }
        "mark_documentation_alignment" => {
            let mut rm = model.unwrap();
            let payload: MarkDocumentationAlignmentPayload =
                serde_json::from_value(command.payload.clone())
                    .map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
            validate_closure_gate_status(&payload.status)?;
            if let Some(pid) = payload.decided_by_persona_id.as_deref() {
                ensure_persona_active(&rm, workspace_id, pid)?;
            }
            let gate_id = payload.gate_id.unwrap_or_else(Uuid::new_v4);
            upsert_closure_gate(
                &mut rm,
                ClosureGateView {
                    id: gate_id,
                    workspace_id,
                    story_id: payload.story_id,
                    prepared_story_id: payload.prepared_story_id,
                    gate_type: "documentation".to_string(),
                    status: payload.status,
                    decided_by_persona_id: payload.decided_by_persona_id,
                    rationale_md: payload.rationale_md,
                    created_at: now,
                    updated_at: now,
                },
            );
            Ok(success(
                rm,
                "documentation_alignment.marked",
                Some(gate_id),
                json!({ "closure_gate_id": gate_id, "gate_type": "documentation" }),
            ))
        }
        other => Err(ApplyCommandError::bad_request(format!(
            "unsupported command_type: {other}"
        ))),
    }
}

pub fn aggregate_type_for_event_type(event_type: &str) -> &'static str {
    if event_type.starts_with("workspace.") {
        "workspace"
    } else if event_type.starts_with("story_intake.") {
        "story_intake"
    } else if event_type.starts_with("story_preparation.") {
        "story_preparation"
    } else if event_type.starts_with("requirement_graph.") {
        "requirement"
    } else if event_type.starts_with("requirement_coverage.") {
        "requirement_coverage"
    } else if event_type.starts_with("acceptance_check.") {
        "acceptance_check"
    } else if event_type.starts_with("handoff_contract.") {
        "handoff_contract"
    } else if event_type.starts_with("execution_profile.") {
        "execution_profile_decision"
    } else if event_type.starts_with("verification_run.") {
        "verification_run"
    } else if event_type.starts_with("verification_finding.") {
        "verification_finding"
    } else if event_type.starts_with("closure_claim.") {
        "closure_claim"
    } else if event_type.starts_with("closure_gate.")
        || event_type.starts_with("documentation_alignment.")
    {
        "closure_gate"
    } else if event_type.starts_with("persona.") {
        "persona_assignment"
    } else if event_type.starts_with("knowledge_artifact.") {
        "knowledge_artifact"
    } else if event_type.starts_with("planning_cycle.") {
        "planning_cycle"
    } else if event_type.starts_with("plan_version.") {
        "plan_version"
    } else if event_type.starts_with("work_item.") {
        "work_item"
    } else if event_type.starts_with("dependency.") {
        "dependency"
    } else if event_type.starts_with("clarification.") {
        "clarification"
    } else {
        "workspace"
    }
}

fn success(
    read_model: ReadModelResponse,
    event_type: &str,
    aggregate_id: Option<Uuid>,
    result_json: Value,
) -> AppliedCommand {
    AppliedCommand {
        read_model,
        event_type: event_type.to_string(),
        aggregate_id,
        result_json,
    }
}

fn empty_workspace(
    workspace_id: Uuid,
    tenant_id: String,
    created_by: Uuid,
    slug: &str,
    name: &str,
    now: chrono::DateTime<Utc>,
) -> ReadModelResponse {
    ReadModelResponse {
        workspace: WorkspaceView {
            id: workspace_id,
            tenant_id,
            slug: slug.to_string(),
            name: name.to_string(),
            created_by,
            created_at: now,
        },
        cycles: Vec::new(),
        phases: Vec::new(),
        work_items: Vec::new(),
        dependencies: Vec::new(),
        clarifications: Vec::new(),
        decisions: Vec::new(),
        plan_versions: Vec::new(),
        story_intakes: Vec::new(),
        story_preparations: Vec::new(),
        requirements: Vec::new(),
        requirement_edges: Vec::new(),
        acceptance_checks: Vec::new(),
        requirement_coverage: Vec::new(),
        handoff_contracts: Vec::new(),
        execution_profile_decisions: Vec::new(),
        verification_runs: Vec::new(),
        verification_findings: Vec::new(),
        closure_claims: Vec::new(),
        closure_gates: Vec::new(),
        personas: Vec::new(),
        persona_assignments: Vec::new(),
        knowledge_artifacts: Vec::new(),
        memory_promotion_events: Vec::new(),
        checkpoint_seq: 0,
    }
}

fn ensure_default_personas(
    read_model: &mut ReadModelResponse,
    workspace_id: Uuid,
    now: chrono::DateTime<Utc>,
) {
    for spec in DEFAULT_PERSONAS {
        let persona_id = format!("{workspace_id}:{}", spec.role);
        if read_model
            .personas
            .iter()
            .any(|persona| persona.id == persona_id)
        {
            continue;
        }
        read_model.personas.push(PersonaView {
            id: persona_id,
            workspace_id,
            display_name: spec.display_name.to_string(),
            accountability_md: spec.accountability_md.to_string(),
            tool_scope_json: None,
            memory_scope_json: None,
            autonomy_policy_json: None,
            active: true,
            created_at: now,
            updated_at: now,
        });
    }
}

fn parse_actor_user_id(actor_context: Option<&Value>) -> Uuid {
    actor_context
        .and_then(|ctx| ctx.get("user_id").or_else(|| ctx.get("actor_user_id")))
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::nil)
}

fn parse_actor_tenant_id(actor_context: Option<&Value>) -> String {
    actor_context
        .and_then(|ctx| ctx.get("tenant_id"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| "local".to_string())
}

fn ensure_cycle_belongs(
    read_model: &ReadModelResponse,
    cycle_id: Option<Uuid>,
) -> Result<(), ApplyCommandError> {
    let Some(cycle_id) = cycle_id else {
        return Ok(());
    };
    if read_model.cycles.iter().any(|cycle| cycle.id == cycle_id) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "cycle_id does not belong to workspace",
        ))
    }
}

fn validate_parent_chain(
    work_items: &[WorkItemView],
    parent_id: Option<Uuid>,
    self_id: Option<Uuid>,
) -> Result<(), ApplyCommandError> {
    let Some(mut current) = parent_id else {
        return Ok(());
    };
    let mut visited = HashSet::new();
    if let Some(self_id) = self_id {
        visited.insert(self_id);
    }
    loop {
        if !visited.insert(current) {
            return Err(ApplyCommandError::bad_request("parent cycle detected"));
        }
        let Some(item) = work_items.iter().find(|item| item.id == current) else {
            return Err(ApplyCommandError::bad_request(
                "parent_id does not belong to workspace",
            ));
        };
        match item.parent_id {
            Some(next) => current = next,
            None => return Ok(()),
        }
    }
}

fn validate_dependency_acyclic(
    dependencies: &[DependencyEdgeView],
    from_item_id: Uuid,
    to_item_id: Uuid,
    edge_type: &str,
) -> Result<(), ApplyCommandError> {
    if from_item_id == to_item_id {
        return Err(ApplyCommandError::bad_request(
            "dependency cannot target the same work item",
        ));
    }
    if !matches!(edge_type, "blocks" | "hard" | "data") {
        return Ok(());
    }
    let mut graph: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for edge in dependencies
        .iter()
        .filter(|edge| matches!(edge.edge_type.as_str(), "blocks" | "hard" | "data"))
    {
        graph
            .entry(edge.from_item_id)
            .or_default()
            .push(edge.to_item_id);
    }
    graph.entry(from_item_id).or_default().push(to_item_id);
    let mut stack = vec![to_item_id];
    let mut seen = HashSet::new();
    while let Some(node) = stack.pop() {
        if node == from_item_id {
            return Err(ApplyCommandError::bad_request(
                "dependency would introduce a cycle",
            ));
        }
        if !seen.insert(node) {
            continue;
        }
        if let Some(next) = graph.get(&node) {
            stack.extend(next.iter().copied());
        }
    }
    Ok(())
}

fn resolve_requirement_graph_scope(
    payload: &UpsertRequirementGraphPayload,
) -> Result<Option<RequirementGraphScope>, ApplyCommandError> {
    if let Some(pid) = payload.reconcile_prepared_story_id {
        return Ok(Some(RequirementGraphScope::PreparedStory(pid)));
    }
    if let Some(sid) = payload.reconcile_story_id {
        return Ok(Some(RequirementGraphScope::StoryOnly(sid)));
    }
    infer_requirement_graph_scope_from_requirements(&payload.requirements)
}

fn infer_requirement_graph_scope_from_requirements(
    requirements: &[RequirementUpsertPayload],
) -> Result<Option<RequirementGraphScope>, ApplyCommandError> {
    if requirements.is_empty() {
        return Ok(None);
    }
    let mut prepared: Option<Option<Uuid>> = None;
    let mut story: Option<Option<Uuid>> = None;
    for r in requirements {
        if prepared.is_none() {
            prepared = Some(r.prepared_story_id);
        } else if prepared != Some(r.prepared_story_id) {
            return Err(ApplyCommandError::bad_request(
                "inconsistent prepared_story_id across requirements in graph payload",
            ));
        }
        if story.is_none() {
            story = Some(r.story_id);
        } else if story != Some(r.story_id) {
            return Err(ApplyCommandError::bad_request(
                "inconsistent story_id across requirements in graph payload",
            ));
        }
    }
    let prepared = prepared.unwrap_or(None);
    let story = story.unwrap_or(None);
    if let Some(pid) = prepared {
        return Ok(Some(RequirementGraphScope::PreparedStory(pid)));
    }
    if let Some(sid) = story {
        return Ok(Some(RequirementGraphScope::StoryOnly(sid)));
    }
    Ok(None)
}

fn requirement_ids_for_scope(
    read_model: &ReadModelResponse,
    scope: RequirementGraphScope,
) -> Vec<Uuid> {
    read_model
        .requirements
        .iter()
        .filter(|requirement| match scope {
            RequirementGraphScope::PreparedStory(prepared_story_id) => {
                requirement.prepared_story_id == Some(prepared_story_id)
            }
            RequirementGraphScope::StoryOnly(story_id) => {
                requirement.story_id == Some(story_id) && requirement.prepared_story_id.is_none()
            }
        })
        .map(|requirement| requirement.id)
        .collect()
}

fn remove_requirement_by_id(read_model: &mut ReadModelResponse, requirement_id: Uuid) {
    read_model
        .requirements
        .retain(|requirement| requirement.id != requirement_id);
    read_model.requirement_edges.retain(|edge| {
        edge.requirement_id != requirement_id && edge.related_requirement_id != requirement_id
    });
    read_model
        .acceptance_checks
        .retain(|check| check.requirement_id != requirement_id);
    read_model
        .requirement_coverage
        .retain(|coverage| coverage.requirement_id != requirement_id);
    read_model
        .closure_claims
        .retain(|claim| claim.requirement_id != Some(requirement_id));
    read_model
        .verification_findings
        .retain(|finding| finding.requirement_id != Some(requirement_id));
}

fn prune_stale_requirement_edges(
    read_model: &mut ReadModelResponse,
    kept_ids: &HashSet<Uuid>,
    edges: &[RequirementEdgeUpsertPayload],
) {
    let expected: HashSet<(Uuid, Uuid, &str)> = edges
        .iter()
        .map(|edge| {
            (
                edge.requirement_id,
                edge.related_requirement_id,
                edge.edge_type.as_str(),
            )
        })
        .collect();
    read_model.requirement_edges.retain(|edge| {
        !(kept_ids.contains(&edge.requirement_id)
            && kept_ids.contains(&edge.related_requirement_id)
            && !expected.contains(&(
                edge.requirement_id,
                edge.related_requirement_id,
                edge.edge_type.as_str(),
            )))
    });
}

fn upsert_requirement(read_model: &mut ReadModelResponse, requirement: RequirementView) {
    if let Some(existing) = read_model
        .requirements
        .iter_mut()
        .find(|existing| existing.id == requirement.id)
    {
        *existing = requirement;
    } else {
        read_model.requirements.push(requirement);
    }
}

fn upsert_requirement_edge(read_model: &mut ReadModelResponse, edge: RequirementEdgeView) {
    if let Some(existing) = read_model.requirement_edges.iter_mut().find(|existing| {
        existing.requirement_id == edge.requirement_id
            && existing.related_requirement_id == edge.related_requirement_id
            && existing.edge_type == edge.edge_type
    }) {
        *existing = edge;
    } else {
        read_model.requirement_edges.push(edge);
    }
}

fn story_id_for_prepared_story(
    read_model: &ReadModelResponse,
    prepared_story_id: Uuid,
) -> Option<Uuid> {
    read_model
        .story_preparations
        .iter()
        .find(|story| story.id == prepared_story_id)
        .map(|story| story.story_id)
}

fn requirement_aligned_with_work_item_pure(
    req_prepared_story_id: Option<Uuid>,
    req_story_id: Option<Uuid>,
    wi_prepared_story_id: Option<Uuid>,
    wi_story_id: Option<Uuid>,
    preparation_intake_story_id: Option<Uuid>,
) -> bool {
    if wi_prepared_story_id.is_none() && wi_story_id.is_none() {
        return true;
    }
    if let Some(ps) = wi_prepared_story_id {
        return req_prepared_story_id == Some(ps)
            || (req_prepared_story_id.is_none()
                && preparation_intake_story_id.is_some()
                && req_story_id == preparation_intake_story_id);
    }
    if let Some(s) = wi_story_id {
        return req_story_id == Some(s) && req_prepared_story_id.is_none();
    }
    true
}

fn upsert_requirement_coverage(
    read_model: &mut ReadModelResponse,
    coverage: RequirementCoverageView,
) -> Uuid {
    if let Some(existing) = read_model.requirement_coverage.iter_mut().find(|existing| {
        existing.requirement_id == coverage.requirement_id
            && existing.work_item_id == coverage.work_item_id
    }) {
        existing.coverage_kind = coverage.coverage_kind;
        existing.status = coverage.status;
        existing.updated_at = coverage.updated_at;
        existing.id
    } else {
        let id = coverage.id;
        read_model.requirement_coverage.push(coverage);
        id
    }
}

fn prune_orphan_requirement_coverage_for_work_item(
    read_model: &mut ReadModelResponse,
    work_item_id: Uuid,
) -> u64 {
    let Some(item) = read_model
        .work_items
        .iter()
        .find(|item| item.id == work_item_id)
        .cloned()
    else {
        return 0;
    };
    if item.prepared_story_id.is_none() && item.story_id.is_none() {
        return 0;
    }
    let prep_story = item
        .prepared_story_id
        .and_then(|prepared_story_id| story_id_for_prepared_story(read_model, prepared_story_id));
    let before = read_model.requirement_coverage.len();
    read_model.requirement_coverage.retain(|coverage| {
        if coverage.work_item_id != work_item_id {
            return true;
        }
        let Some(requirement) = read_model
            .requirements
            .iter()
            .find(|requirement| requirement.id == coverage.requirement_id)
        else {
            return false;
        };
        requirement_aligned_with_work_item_pure(
            requirement.prepared_story_id,
            requirement.story_id,
            item.prepared_story_id,
            item.story_id,
            prep_story,
        )
    });
    (before - read_model.requirement_coverage.len()) as u64
}

fn validate_acceptance_check(
    read_model: &ReadModelResponse,
    check: &AcceptanceCheckUpsertItem,
) -> Result<(), ApplyCommandError> {
    validate_acceptance_check_kind(&check.check_kind)?;
    validate_acceptance_check_status(&check.status)?;
    if check.title.trim().is_empty() {
        return Err(ApplyCommandError::bad_request(
            "acceptance check title must not be empty",
        ));
    }
    if !read_model
        .requirements
        .iter()
        .any(|requirement| requirement.id == check.requirement_id)
    {
        return Err(ApplyCommandError::NotFound("requirement"));
    }
    Ok(())
}

fn upsert_acceptance_check(read_model: &mut ReadModelResponse, check: AcceptanceCheckView) {
    if let Some(existing) = read_model
        .acceptance_checks
        .iter_mut()
        .find(|existing| existing.id == check.id)
    {
        *existing = check;
    } else {
        read_model.acceptance_checks.push(check);
    }
}

fn ensure_persona_active(
    read_model: &ReadModelResponse,
    workspace_id: Uuid,
    persona_id: &str,
) -> Result<(), ApplyCommandError> {
    if read_model.personas.iter().any(|persona| {
        persona.workspace_id == workspace_id && persona.id == persona_id && persona.active
    }) {
        Ok(())
    } else {
        Err(MolError::UnknownPersona {
            persona_id: persona_id.to_string(),
        }
        .into())
    }
}

fn upsert_closure_claim(read_model: &mut ReadModelResponse, claim: ClosureClaimView) {
    if let Some(existing) = read_model
        .closure_claims
        .iter_mut()
        .find(|existing| existing.id == claim.id)
    {
        existing.status = claim.status;
        existing.claimed_by_persona_id = claim.claimed_by_persona_id;
        existing.claim_json = claim.claim_json;
        existing.updated_at = claim.updated_at;
    } else {
        read_model.closure_claims.push(claim);
    }
}

fn merge_optional_json(current: Option<Value>, incoming: Option<Value>) -> Option<Value> {
    match (current, incoming) {
        (None, None) => None,
        (Some(existing), None) => Some(existing),
        (None, Some(next)) => Some(next),
        (Some(Value::Object(mut existing)), Some(Value::Object(next))) => {
            for (key, value) in next {
                existing.insert(key, value);
            }
            Some(Value::Object(existing))
        }
        (_, Some(next)) => Some(next),
    }
}

fn upsert_closure_gate(read_model: &mut ReadModelResponse, gate: ClosureGateView) {
    if let Some(existing) = read_model
        .closure_gates
        .iter_mut()
        .find(|existing| existing.id == gate.id)
    {
        existing.status = gate.status;
        existing.decided_by_persona_id = gate.decided_by_persona_id;
        existing.rationale_md = gate.rationale_md;
        existing.updated_at = gate.updated_at;
    } else {
        read_model.closure_gates.push(gate);
    }
}

fn upsert_persona(read_model: &mut ReadModelResponse, persona: PersonaView) {
    if let Some(existing) = read_model
        .personas
        .iter_mut()
        .find(|existing| existing.id == persona.id)
    {
        existing.display_name = persona.display_name;
        existing.accountability_md = persona.accountability_md;
        existing.tool_scope_json = persona.tool_scope_json;
        existing.memory_scope_json = persona.memory_scope_json;
        existing.autonomy_policy_json = persona.autonomy_policy_json;
        existing.active = persona.active;
        existing.updated_at = persona.updated_at;
    } else {
        read_model.personas.push(persona);
    }
}

fn upsert_persona_assignment(
    read_model: &mut ReadModelResponse,
    assignment: PersonaAssignmentView,
) {
    if let Some(existing) = read_model
        .persona_assignments
        .iter_mut()
        .find(|existing| existing.id == assignment.id)
    {
        *existing = assignment;
    } else {
        read_model.persona_assignments.push(assignment);
    }
}

fn build_closure_snapshot(
    read_model: &ReadModelResponse,
    prepared_story_id: Uuid,
) -> Result<ClosureSnapshot, ApplyCommandError> {
    let prep_story_story_id = story_id_for_prepared_story(read_model, prepared_story_id)
        .ok_or(ApplyCommandError::NotFound("prepared story"))?;

    let required_requirement_ids = dedup_ids(
        read_model
            .requirement_coverage
            .iter()
            .filter(|coverage| {
                read_model
                    .work_items
                    .iter()
                    .find(|item| item.id == coverage.work_item_id)
                    .map(|item| item.prepared_story_id == Some(prepared_story_id))
                    .unwrap_or(false)
            })
            .map(|coverage| coverage.requirement_id)
            .collect(),
    );

    let gates = read_model
        .closure_gates
        .iter()
        .filter(|gate| gate.prepared_story_id == Some(prepared_story_id))
        .map(|gate| (gate.gate_type.clone(), gate.status.clone()))
        .collect();

    let claims = read_model
        .closure_claims
        .iter()
        .filter_map(|claim| {
            claim
                .requirement_id
                .filter(|requirement_id| required_requirement_ids.contains(requirement_id))
                .map(|requirement_id| (requirement_id, claim.status.clone()))
        })
        .collect();

    let verification_runs: Vec<(Uuid, String)> = read_model
        .verification_runs
        .iter()
        .filter(|run| run.prepared_story_id == Some(prepared_story_id))
        .map(|run| (run.id, run.status.clone()))
        .collect();
    let verification_run_ids: HashSet<Uuid> = verification_runs
        .iter()
        .map(|(run_id, _)| *run_id)
        .collect();

    let findings = read_model
        .verification_findings
        .iter()
        .filter(|finding| verification_run_ids.contains(&finding.verification_run_id))
        .map(|finding| (finding.id, finding.status.clone()))
        .collect();

    let handoffs = read_model
        .handoff_contracts
        .iter()
        .filter(|contract| contract.prepared_story_id == Some(prepared_story_id))
        .map(|contract| (contract.id, contract.status.clone()))
        .collect();

    let acceptance_checks = read_model
        .acceptance_checks
        .iter()
        .filter(|check| {
            read_model
                .requirements
                .iter()
                .find(|requirement| requirement.id == check.requirement_id)
                .map(|requirement| {
                    requirement.prepared_story_id == Some(prepared_story_id)
                        || (requirement.prepared_story_id.is_none()
                            && requirement.story_id == Some(prep_story_story_id))
                })
                .unwrap_or(false)
        })
        .map(|check| (check.id, check.status.clone()))
        .collect();

    Ok(ClosureSnapshot {
        gates,
        required_requirement_ids,
        claims,
        verification_runs,
        findings,
        handoffs,
        acceptance_checks,
    })
}

#[derive(Debug, Clone, Default)]
struct PolicyOverrides {
    verification_required: Option<bool>,
}

impl PolicyOverrides {
    fn merge_layer(&mut self, layer: PolicyOverrides) {
        if layer.verification_required.is_some() {
            self.verification_required = layer.verification_required;
        }
    }
}

fn parse_policy_overrides(value: &Value) -> PolicyOverrides {
    let Some(obj) = value.as_object() else {
        return PolicyOverrides::default();
    };
    PolicyOverrides {
        verification_required: obj.get("verification_required").and_then(|v| v.as_bool()),
    }
}

fn merge_policy_override_layers(layers: &[PolicyOverrides]) -> PolicyOverrides {
    let mut merged = PolicyOverrides::default();
    for layer in layers {
        merged.merge_layer(layer.clone());
    }
    merged
}

fn extract_profile_name_override(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    obj.get("profile_name")
        .or_else(|| obj.get("primary_execution_profile"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn base_profile_requires_verification(profile_name: &str) -> bool {
    !matches!(profile_name.trim(), "Fast Iterate")
}

fn resolve_verification_required(
    read_model: &ReadModelResponse,
    prepared_story_id: Uuid,
) -> Result<bool, ApplyCommandError> {
    let prep = read_model
        .story_preparations
        .iter()
        .find(|prep| prep.id == prepared_story_id)
        .ok_or(ApplyCommandError::NotFound("prepared story"))?;
    let mut profile_name = prep.primary_execution_profile.clone();
    let decision = read_model
        .execution_profile_decisions
        .iter()
        .filter(|decision| decision.prepared_story_id == Some(prepared_story_id))
        .max_by_key(|decision| decision.created_at);

    let mut policy_layers: Vec<Value> = Vec::new();
    if let Some(decision) = decision {
        profile_name = decision.profile_name.clone();
        policy_layers.push(decision.policy_json.clone());
    }
    if let Some(card) = prep.execution_card_json.as_ref() {
        if let Some(policy_json) = card.get("policy_json") {
            policy_layers.push(policy_json.clone());
        }
    }
    for layer in &policy_layers {
        if let Some(name) = extract_profile_name_override(layer) {
            profile_name = name;
        }
    }
    let parsed_layers: Vec<PolicyOverrides> =
        policy_layers.iter().map(parse_policy_overrides).collect();
    let merged = merge_policy_override_layers(&parsed_layers);
    Ok(merged
        .verification_required
        .unwrap_or_else(|| base_profile_requires_verification(&profile_name)))
}

fn hash_json(value: &Value) -> Result<String, ApplyCommandError> {
    let bytes =
        serde_json::to_vec(value).map_err(|err| ApplyCommandError::bad_request(err.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_work_item_type(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "story" | "bug" | "task" | "spike" | "chore") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid work item type"))
    }
}

fn validate_execution_profile(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "general" | "code_modification" | "review" | "retrieval"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid execution_profile"))
    }
}

fn validate_cycle_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "draft" | "active" | "completed" | "archived") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid cycle status"))
    }
}

fn validate_story_intake_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "captured"
            | "classified"
            | "clarification_pending"
            | "triaged"
            | "preparing"
            | "prepared"
            | "ready"
            | "executing"
            | "closure_pending"
            | "closed"
            | "blocked"
            | "abandoned"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid story intake status",
        ))
    }
}

fn validate_story_preparation_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "preparing" | "prepared" | "ready" | "executing" | "closure_pending" | "closed" | "blocked"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid story preparation status",
        ))
    }
}

fn validate_profile_name(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "Fast Iterate" | "Balanced" | "High Assurance" | "Critical Change"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid execution profile name",
        ))
    }
}

fn validate_escalation_level(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "none" | "advisory" | "raised" | "hard_minimum") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid escalation level"))
    }
}

fn validate_requirement_ambiguity_state(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "clear" | "needs_clarification" | "waived") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid requirement ambiguity state",
        ))
    }
}

fn validate_requirement_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "draft"
            | "active"
            | "clarification_needed"
            | "implemented_claimed"
            | "verification_pending"
            | "satisfied"
            | "partial"
            | "waived"
            | "failed"
            | "superseded"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid requirement status"))
    }
}

fn validate_acceptance_check_kind(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "unit" | "contract" | "integration" | "review" | "docs" | "e2e" | "manual" | "other"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid acceptance check kind",
        ))
    }
}

fn validate_acceptance_check_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "pending" | "in_progress" | "passed" | "failed" | "waived" | "skipped"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid acceptance check status",
        ))
    }
}

fn validate_verification_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "pending" | "running" | "passed" | "failed" | "blocked"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid verification status",
        ))
    }
}

fn validate_verification_stage(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "readiness" | "in_flight" | "post_implementation" | "mission_closure"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid verification stage"))
    }
}

fn validate_finding_severity(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "info" | "low" | "medium" | "high" | "critical") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid finding severity"))
    }
}

fn validate_finding_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "open" | "accepted" | "resolved" | "waived") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid finding status"))
    }
}

fn validate_closure_claim_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "pending" | "recorded" | "accepted" | "rejected") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid closure claim status",
        ))
    }
}

fn validate_closure_gate_type(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "coverage" | "verification" | "handoff" | "review" | "reliability" | "documentation"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request("invalid closure gate type"))
    }
}

fn validate_closure_gate_status(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "pending" | "passed" | "failed" | "waived") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid closure gate status",
        ))
    }
}

fn validate_dependency_edge_type(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(
        value,
        "blocks" | "relates_to" | "duplicates" | "parent_of" | "hard" | "data"
    ) {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid dependency edge type",
        ))
    }
}

fn validate_dependency_strength(value: &str) -> Result<(), ApplyCommandError> {
    if matches!(value, "hard" | "soft" | "data") {
        Ok(())
    } else {
        Err(ApplyCommandError::bad_request(
            "invalid dependency strength",
        ))
    }
}

fn validate_tracker_state(next: &str, current: Option<&str>) -> Result<(), ApplyCommandError> {
    const ALLOWED: &[&str] = &[
        "backlog",
        "triage",
        "in_progress",
        "blocked",
        "needs_review",
        "done",
    ];
    if !ALLOWED.contains(&next) {
        return Err(ApplyCommandError::bad_request("invalid tracker_state"));
    }
    if matches!(current, Some("done")) && next != "done" {
        return Err(ApplyCommandError::bad_request(
            "cannot transition tracker_state from done",
        ));
    }
    Ok(())
}

fn validate_run_state(next: &str, current: Option<&str>) -> Result<(), ApplyCommandError> {
    const ALLOWED: &[&str] = &[
        "idle",
        "queued_for_llm",
        "executing",
        "awaiting_hitl",
        "failed_retryable",
        "failed_terminal",
        "done",
    ];
    if !ALLOWED.contains(&next) {
        return Err(ApplyCommandError::bad_request("invalid run_state"));
    }
    if matches!(current, Some("done")) && next != "done" {
        return Err(ApplyCommandError::bad_request(
            "cannot transition run_state from done",
        ));
    }
    Ok(())
}

fn normalize_wave_label_rank(label: Option<&str>) -> Result<Option<i32>, ApplyCommandError> {
    let Some(label) = label else {
        return Ok(None);
    };
    let normalized = label.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    let bytes = normalized.as_bytes();
    if bytes.first().copied() != Some(b'A') {
        return Err(ApplyCommandError::bad_request("wave_label must start at A"));
    }
    for (idx, byte) in bytes.iter().copied().enumerate() {
        let expected = b'A' + idx as u8;
        if byte != expected {
            return Err(ApplyCommandError::bad_request(
                "wave_label must be contiguous like A, AB, ABC",
            ));
        }
    }
    Ok(Some(bytes.len() as i32))
}

fn validate_wave_dependency(
    from_rank: Option<i32>,
    to_rank: Option<i32>,
) -> Result<(), ApplyCommandError> {
    if let (Some(from_rank), Some(to_rank)) = (from_rank, to_rank) {
        if from_rank > to_rank {
            return Err(ApplyCommandError::bad_request(
                "dependency conflicts with wave ordering: later wave cannot depend on earlier wave label ordering",
            ));
        }
    }
    Ok(())
}
