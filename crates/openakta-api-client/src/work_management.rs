use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandEnvelope {
    pub client_command_id: Uuid,
    #[serde(default)]
    pub base_seq: i64,
    pub command_type: String,
    #[serde(default)]
    pub payload: Value,
    #[serde(default)]
    pub actor_context: Option<Value>,
}

/// Payload for `record_clarification_answers` — persists answers to Postgres (canonical path).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordClarificationAnswersPayload {
    pub session_id: Uuid,
    pub answers: Vec<RecordClarificationAnswerItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordClarificationAnswerItem {
    pub clarification_item_id: Uuid,
    pub answer_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResponse {
    pub status: String,
    pub resulting_seq: i64,
    pub event_ids: Vec<Uuid>,
    pub read_model_etag: String,
    #[serde(default)]
    pub conflict_snapshot: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<WorkEvent>,
    pub next_seq: i64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceView {
    pub id: Uuid,
    pub tenant_id: String,
    pub slug: String,
    pub name: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningCycleView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub cadence_mode: String,
    pub planning_mode: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub status: String,
    pub global_wip_limit: Option<i32>,
    pub replanning_interval_secs: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CyclePhaseView {
    pub id: Uuid,
    pub cycle_id: Uuid,
    pub phase_key: String,
    pub ordinal: i32,
    pub strict_barrier: bool,
    pub phase_wip_limit: Option<i32>,
    pub exit_criteria_json: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub cycle_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub item_type: String,
    pub execution_profile: String,
    pub title: String,
    pub description_md: Option<String>,
    pub tracker_state: String,
    pub run_state: String,
    pub priority: i32,
    pub assignee_user_id: Option<Uuid>,
    pub external_master: bool,
    pub wave_rank: Option<i32>,
    pub wave_label: Option<String>,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub owner_persona_id: Option<String>,
    pub requirement_slice_json: Option<Value>,
    pub handoff_contract_state: Option<String>,
    pub claim_state: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdgeView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub from_item_id: Uuid,
    pub to_item_id: Uuid,
    pub edge_type: String,
    pub strength: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersionView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub cycle_id: Option<Uuid>,
    pub story_id: Option<Uuid>,
    pub base_seq: i64,
    pub plan_hash: String,
    pub snapshot_json: Value,
    pub status: String,
    pub created_by: Uuid,
    pub approved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub approved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionRecordView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub cycle_id: Option<Uuid>,
    pub subject_type: String,
    pub subject_id: Option<Uuid>,
    pub requirement_id: Option<Uuid>,
    pub profile_decision_id: Option<Uuid>,
    pub title: String,
    pub decision_json: Value,
    pub rationale_md: Option<String>,
    pub clarification_session_id: Option<Uuid>,
    pub status: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationItemView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub cycle_id: Option<Uuid>,
    pub work_item_id: Option<Uuid>,
    pub story_id: Option<Uuid>,
    pub requirement_id: Option<Uuid>,
    pub mission_id: Option<String>,
    pub task_id: Option<String>,
    pub question_kind: String,
    pub prompt_text: String,
    pub schema_json: Option<Value>,
    pub options_json: Option<Value>,
    pub dedupe_fingerprint: String,
    pub status: String,
    pub raised_by_agent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub answered_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceLinkView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub subject_type: String,
    pub subject_id: Option<Uuid>,
    pub artifact_kind: String,
    pub locator_json: Value,
    pub content_hash: String,
    pub storage_scope: String,
    pub preview_redacted: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryIntakeView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub external_ref: Option<String>,
    pub title: String,
    pub raw_request_md: String,
    pub source_kind: String,
    pub status: String,
    pub urgency: String,
    pub priority_band: String,
    pub affected_surfaces_json: Option<Value>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryPreparationView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub story_id: Uuid,
    pub status: String,
    pub mission_card_json: Value,
    pub execution_card_json: Option<Value>,
    pub dependency_summary_json: Option<Value>,
    pub readiness_blockers_json: Option<Value>,
    pub primary_execution_profile: String,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ready_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub plan_version_id: Option<Uuid>,
    pub parent_requirement_id: Option<Uuid>,
    pub title: String,
    pub statement: String,
    pub kind: String,
    pub criticality: String,
    pub source: String,
    pub ambiguity_state: String,
    pub owner_persona_id: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementEdgeView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub requirement_id: Uuid,
    pub related_requirement_id: Uuid,
    pub edge_type: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCheckView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub requirement_id: Uuid,
    pub check_kind: String,
    pub title: String,
    pub status: String,
    pub evidence_required: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Payload for `SubmitCommand` with `command_type` `upsert_acceptance_checks` (matches `work.v1.UpsertAcceptanceChecksPayload`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertAcceptanceChecksPayload {
    pub checks: Vec<AcceptanceCheckUpsertItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCheckUpsertItem {
    #[serde(default)]
    pub id: Option<Uuid>,
    pub requirement_id: Uuid,
    pub check_kind: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub evidence_required: bool,
}

/// Payload for `SubmitCommand` with `command_type` `delete_acceptance_check` (matches `work.v1.DeleteAcceptanceCheckPayload`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteAcceptanceCheckPayload {
    pub acceptance_check_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementCoverageView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub requirement_id: Uuid,
    pub work_item_id: Uuid,
    pub coverage_kind: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffContractView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub prepared_story_id: Option<Uuid>,
    pub from_work_item_id: Option<Uuid>,
    pub to_work_item_id: Option<Uuid>,
    pub contract_kind: String,
    pub expected_artifact_json: Option<Value>,
    pub acceptance_signal_json: Option<Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProfileDecisionView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub profile_name: String,
    pub policy_json: Value,
    pub inferred_from_json: Option<Value>,
    pub override_reason_md: Option<String>,
    pub escalation_level: String,
    pub decided_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRunView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub status: String,
    pub verification_stage: String,
    pub run_kind: String,
    pub initiated_by_persona_id: Option<String>,
    pub summary_json: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationFindingView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub verification_run_id: Uuid,
    pub requirement_id: Option<Uuid>,
    pub severity: String,
    pub finding_type: String,
    pub title: String,
    pub detail_md: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureClaimView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub work_item_id: Option<Uuid>,
    pub requirement_id: Option<Uuid>,
    pub claim_type: String,
    pub status: String,
    pub claimed_by_persona_id: Option<String>,
    pub claim_json: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureGateView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub gate_type: String,
    pub status: String,
    pub decided_by_persona_id: Option<String>,
    pub rationale_md: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaView {
    pub id: String,
    pub workspace_id: Uuid,
    pub display_name: String,
    pub accountability_md: String,
    pub tool_scope_json: Option<Value>,
    pub memory_scope_json: Option<Value>,
    pub autonomy_policy_json: Option<Value>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaAssignmentView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub persona_id: String,
    pub subject_type: String,
    pub subject_id: Option<Uuid>,
    pub assignment_role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeArtifactView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub persona_id: Option<String>,
    pub title: String,
    pub artifact_kind: String,
    pub body_md: Option<String>,
    pub source_refs_json: Option<Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPromotionEventView {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub knowledge_artifact_id: Option<Uuid>,
    pub source_kind: String,
    pub source_ref: Option<String>,
    pub outcome: String,
    pub summary_json: Option<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkEvent {
    pub workspace_id: Uuid,
    pub event_seq: i64,
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Option<Uuid>,
    pub event_type: String,
    pub actor_user_id: Uuid,
    pub actor_kind: String,
    pub client_command_id: Uuid,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Option<Uuid>,
    pub payload_json: Value,
    pub content_hash: String,
    pub occurred_at: DateTime<Utc>,
    pub privacy_class: String,
}

/// Requirement graph and related artifacts for a story or prepared story (`work.v1.GetRequirementGraph`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementGraphView {
    pub requirements: Vec<RequirementView>,
    pub requirement_edges: Vec<RequirementEdgeView>,
    pub acceptance_checks: Vec<AcceptanceCheckView>,
    pub requirement_coverage: Vec<RequirementCoverageView>,
    pub handoff_contracts: Vec<HandoffContractView>,
}

/// Aggregated closure snapshot (`work.v1.GetClosureReport`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosureReportView {
    pub workspace_id: Uuid,
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub requirements: Vec<RequirementView>,
    pub closure_claims: Vec<ClosureClaimView>,
    pub closure_gates: Vec<ClosureGateView>,
    pub verification_findings: Vec<VerificationFindingView>,
}

/// Personas and their assignments for a workspace (`work.v1.ListPersonaAssignments`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaAssignmentsListView {
    pub personas: Vec<PersonaView>,
    pub assignments: Vec<PersonaAssignmentView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadModelResponse {
    pub workspace: WorkspaceView,
    pub cycles: Vec<PlanningCycleView>,
    pub phases: Vec<CyclePhaseView>,
    pub work_items: Vec<WorkItemView>,
    pub dependencies: Vec<DependencyEdgeView>,
    pub clarifications: Vec<ClarificationItemView>,
    pub decisions: Vec<DecisionRecordView>,
    pub plan_versions: Vec<PlanVersionView>,
    pub story_intakes: Vec<StoryIntakeView>,
    pub story_preparations: Vec<StoryPreparationView>,
    pub requirements: Vec<RequirementView>,
    pub requirement_edges: Vec<RequirementEdgeView>,
    pub acceptance_checks: Vec<AcceptanceCheckView>,
    pub requirement_coverage: Vec<RequirementCoverageView>,
    pub handoff_contracts: Vec<HandoffContractView>,
    pub execution_profile_decisions: Vec<ExecutionProfileDecisionView>,
    pub verification_runs: Vec<VerificationRunView>,
    pub verification_findings: Vec<VerificationFindingView>,
    pub closure_claims: Vec<ClosureClaimView>,
    pub closure_gates: Vec<ClosureGateView>,
    pub personas: Vec<PersonaView>,
    pub persona_assignments: Vec<PersonaAssignmentView>,
    pub knowledge_artifacts: Vec<KnowledgeArtifactView>,
    pub memory_promotion_events: Vec<MemoryPromotionEventView>,
    pub checkpoint_seq: i64,
}
