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
    pub checkpoint_seq: i64,
}
