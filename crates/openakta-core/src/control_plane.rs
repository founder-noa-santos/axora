use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use uuid::Uuid;

use openakta_agents::intake::extract_target_hints;
use openakta_agents::{
    DecomposedMission, ExecutionEventKind, ExecutionTraceEvent, ExecutionTracePhase,
    MessageExecutionMode, MessageSurface, MissionDecision, ResponsePreference, RiskLevel, Task,
    TaskTargetHints, TaskType,
};

const DB_BUSY_TIMEOUT_SECS: u64 = 5;

#[derive(Debug, Clone)]
pub struct WorkSessionInit {
    pub session_id: String,
    pub workspace_root: PathBuf,
    pub request_text: String,
    pub surface: MessageSurface,
    pub response_preference: ResponsePreference,
    pub allow_code_context: bool,
    pub side_effects_allowed: bool,
    pub remote_enabled: bool,
    pub decision: MissionDecision,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkSessionStatus {
    Admitted,
    Planned,
    Executing,
    AwaitingValidation,
    Completed,
    Failed,
    Blocked,
}

impl WorkSessionStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::Planned => "planned",
            Self::Executing => "executing",
            Self::AwaitingValidation => "awaiting_validation",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
        }
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "admitted" => Some(Self::Admitted),
            "planned" => Some(Self::Planned),
            "executing" => Some(Self::Executing),
            "awaiting_validation" => Some(Self::AwaitingValidation),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkTaskLane {
    Planning,
    Search,
    Execution,
    Validation,
}

impl WorkTaskLane {
    fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Search => "search",
            Self::Execution => "execution",
            Self::Validation => "validation",
        }
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "planning" => Some(Self::Planning),
            "search" => Some(Self::Search),
            "execution" => Some(Self::Execution),
            "validation" => Some(Self::Validation),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkTaskStatus {
    Ready,
    Running,
    AwaitingValidation,
    Done,
    FailedTerminal,
    Blocked,
    Cancelled,
}

impl WorkTaskStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Running => "running",
            Self::AwaitingValidation => "awaiting_validation",
            Self::Done => "done",
            Self::FailedTerminal => "failed_terminal",
            Self::Blocked => "blocked",
            Self::Cancelled => "cancelled",
        }
    }

    fn from_str(raw: &str) -> Option<Self> {
        match raw {
            "ready" => Some(Self::Ready),
            "running" => Some(Self::Running),
            "awaiting_validation" => Some(Self::AwaitingValidation),
            "done" => Some(Self::Done),
            "failed_terminal" => Some(Self::FailedTerminal),
            "blocked" => Some(Self::Blocked),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkSessionRecord {
    pub session_id: String,
    pub workspace_root: String,
    pub request_text: String,
    pub status: WorkSessionStatus,
    pub admitted_mode: MessageExecutionMode,
    pub task_type: TaskType,
    pub risk: RiskLevel,
    pub mission_id: Option<String>,
    pub trace_session_id: Option<String>,
    pub error_message: Option<String>,
    pub decision_json: Value,
    pub outcome_json: Option<Value>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub completed_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkSessionTaskRecord {
    pub session_id: String,
    pub task_id: String,
    pub parent_task_id: Option<String>,
    pub lane: WorkTaskLane,
    pub title: String,
    pub task_type: TaskType,
    pub status: WorkTaskStatus,
    pub depends_on_task_ids: Vec<String>,
    pub requirement_refs: Vec<String>,
    pub expected_artifacts: Vec<String>,
    pub target_files: Vec<String>,
    pub target_symbols: Vec<String>,
    pub verification_required: bool,
    pub error_message: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkSessionArtifactRecord {
    pub artifact_id: String,
    pub session_id: String,
    pub task_id: Option<String>,
    pub artifact_kind: String,
    pub requirement_refs: Vec<String>,
    pub verification_task_id: Option<String>,
    pub payload_json: Value,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone)]
pub struct TaskShellSeed {
    pub task_id: String,
    pub parent_task_id: Option<String>,
    pub lane: WorkTaskLane,
    pub title: String,
    pub task_type: TaskType,
    pub depends_on_task_ids: Vec<String>,
    pub requirement_refs: Vec<String>,
    pub expected_artifacts: Vec<String>,
    pub target_files: Vec<String>,
    pub target_symbols: Vec<String>,
    pub verification_required: bool,
}

impl TaskShellSeed {
    pub fn planning(session_id: &str, title: impl Into<String>) -> Self {
        Self {
            task_id: planning_task_id(session_id),
            parent_task_id: None,
            lane: WorkTaskLane::Planning,
            title: title.into(),
            task_type: TaskType::General,
            depends_on_task_ids: Vec::new(),
            requirement_refs: vec![request_requirement_ref(session_id)],
            expected_artifacts: vec!["decomposition_plan".to_string()],
            target_files: Vec::new(),
            target_symbols: Vec::new(),
            verification_required: false,
        }
    }

    pub fn search_initial(
        session_id: &str,
        title: impl Into<String>,
        hints: &TaskTargetHints,
    ) -> Self {
        Self {
            task_id: search_task_id(session_id),
            parent_task_id: None,
            lane: WorkTaskLane::Search,
            title: title.into(),
            task_type: TaskType::Retrieval,
            depends_on_task_ids: Vec::new(),
            requirement_refs: vec![request_requirement_ref(session_id)],
            expected_artifacts: vec!["workspace_context".to_string()],
            target_files: hints.target_files.clone(),
            target_symbols: hints.target_symbols.clone(),
            verification_required: false,
        }
    }

    pub fn execution(task: &Task, hints: &TaskTargetHints) -> Self {
        Self {
            task_id: task.id.clone(),
            parent_task_id: task.parent_task.clone(),
            lane: WorkTaskLane::Execution,
            title: task.description.clone(),
            task_type: task.task_type.clone(),
            depends_on_task_ids: Vec::new(),
            requirement_refs: Vec::new(),
            expected_artifacts: expected_artifacts_for_task_type(&task.task_type),
            target_files: hints.target_files.clone(),
            target_symbols: hints.target_symbols.clone(),
            verification_required: task.task_type == TaskType::CodeModification,
        }
    }

    pub fn validation(parent: &TaskShellSeed) -> Option<Self> {
        if !parent.verification_required || parent.lane != WorkTaskLane::Execution {
            return None;
        }
        Some(Self {
            task_id: validation_task_id(&parent.task_id),
            parent_task_id: Some(parent.task_id.clone()),
            lane: WorkTaskLane::Validation,
            title: format!(
                "Validate that '{}' is complete and report pass/fail with concrete findings",
                parent.title
            ),
            task_type: TaskType::Review,
            depends_on_task_ids: vec![parent.task_id.clone()],
            requirement_refs: parent.requirement_refs.clone(),
            expected_artifacts: vec!["validation_summary".to_string()],
            target_files: parent.target_files.clone(),
            target_symbols: parent.target_symbols.clone(),
            verification_required: false,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionLink {
    pub subject_ref: String,
    pub artifact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionClosureSummary {
    pub request_satisfied: bool,
    pub uncovered_deliverables: Vec<String>,
    pub blockers: Vec<String>,
    pub requirement_closure_state: String,
    pub remaining_requirements: Vec<String>,
    pub requirement_artifact_links: Vec<CompletionLink>,
    pub requirement_verification_links: Vec<CompletionLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FinalizedSessionOutcome {
    pub status: WorkSessionStatus,
    pub completion: CompletionClosureSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkSessionSnapshot {
    pub session: WorkSessionRecord,
    pub tasks: Vec<WorkSessionTaskRecord>,
    pub artifacts: Vec<WorkSessionArtifactRecord>,
}

#[derive(Debug, Clone)]
pub struct ControlPlaneRuntime {
    store: WorkSessionStore,
}

impl ControlPlaneRuntime {
    pub fn open(workspace_root: &Path) -> Result<Self> {
        Ok(Self {
            store: WorkSessionStore::open(WorkSessionStore::path_for_workspace(workspace_root))?,
        })
    }

    pub fn admit_session(&self, init: &WorkSessionInit) -> Result<()> {
        self.store.create_session(init)?;

        self.store.append_artifact(
            &init.session_id,
            None,
            "admission",
            json!({
                "surface": init.surface,
                "response_preference": init.response_preference,
                "allow_code_context": init.allow_code_context,
                "side_effects_allowed": init.side_effects_allowed,
                "remote_enabled": init.remote_enabled,
                "decision": init.decision,
            }),
            &[],
            None,
        )?;
        Ok(())
    }

    pub fn register_task_shells(&self, session_id: &str, seeds: &[TaskShellSeed]) -> Result<()> {
        self.store.upsert_tasks(session_id, seeds)
    }

    pub fn snapshot_session(&self, session_id: &str) -> Result<Option<WorkSessionSnapshot>> {
        let Some(session) = self.store.get_session(session_id)? else {
            return Ok(None);
        };
        Ok(Some(WorkSessionSnapshot {
            tasks: self.store.list_tasks(session_id)?,
            artifacts: self.store.list_artifacts(session_id)?,
            session,
        }))
    }

    pub fn materialize_initial_retrieval(
        &self,
        session_id: &str,
        request_text: &str,
        decision: &MissionDecision,
    ) -> Result<()> {
        let Some(workspace_context) = decision.retrieval_plan.workspace_context.as_deref() else {
            return Ok(());
        };

        let retrieval_task = TaskShellSeed::search_initial(
            session_id,
            format!("Retrieve workspace context for: {}", request_text),
            &decision.target_hints,
        );
        self.store
            .upsert_tasks(session_id, &[retrieval_task.clone()])?;
        self.store.update_task_state(
            session_id,
            &retrieval_task.task_id,
            WorkTaskStatus::Running,
            None,
        )?;
        self.store.append_artifact(
            session_id,
            Some(retrieval_task.task_id.as_str()),
            "workspace_context",
            json!({
                "retrieval_source": "mission_gate_workspace_context",
                "repo_context_requested": decision.retrieval_plan.repo_context_requested,
                "max_hits": decision.retrieval_plan.max_hits,
                "workspace_context": workspace_context,
            }),
            &retrieval_task.requirement_refs,
            None,
        )?;
        self.store.update_task_state(
            session_id,
            &retrieval_task.task_id,
            WorkTaskStatus::Done,
            None,
        )?;
        Ok(())
    }

    pub fn plan_started(&self, session_id: &str) -> Result<()> {
        self.store
            .update_session_state(session_id, WorkSessionStatus::Planned, None, None, None)
    }

    pub fn mark_task_state(
        &self,
        session_id: &str,
        task_id: &str,
        status: WorkTaskStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.store
            .update_task_state(session_id, task_id, status, error_message)
    }

    pub fn mark_session_executing(&self, session_id: &str) -> Result<()> {
        self.store
            .update_session_state(session_id, WorkSessionStatus::Executing, None, None, None)
    }

    pub fn record_decomposition(
        &self,
        session_id: &str,
        decomposed: &DecomposedMission,
    ) -> Result<()> {
        let requirement_refs = vec![request_requirement_ref(session_id)];
        self.store.append_artifact(
            session_id,
            Some(planning_task_id(session_id).as_str()),
            "decomposition_plan",
            serde_json::to_value(decomposed).context("serialize decomposed mission")?,
            &requirement_refs,
            None,
        )?;
        Ok(())
    }

    pub fn reconcile_trace_events(
        &self,
        session_id: &str,
        trace_events: &[ExecutionTraceEvent],
    ) -> Result<()> {
        let tasks = self.store.list_tasks(session_id)?;
        let task_index = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task))
            .collect::<HashMap<_, _>>();
        let mut task_events: HashMap<String, Vec<&ExecutionTraceEvent>> = HashMap::new();
        for event in trace_events {
            if event.event_kind == ExecutionEventKind::Task && !event.task_id.is_empty() {
                task_events
                    .entry(event.task_id.clone())
                    .or_default()
                    .push(event);
            }
        }

        for (task_id, events) in task_events {
            let Some(record) = task_index.get(&task_id) else {
                continue;
            };
            let (status, error_message) = derive_task_state(record, &events);
            self.store
                .update_task_state(session_id, &task_id, status, error_message.as_deref())?;
        }
        Ok(())
    }

    pub fn reserve_next_dispatchable_task(
        &self,
        session_id: &str,
    ) -> Result<Option<WorkSessionTaskRecord>> {
        let tasks = self.store.list_tasks(session_id)?;
        self.block_stale_validation_tasks(session_id, &tasks)?;
        let tasks = self.store.list_tasks(session_id)?;
        let task_index = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task.clone()))
            .collect::<HashMap<_, _>>();

        let mut candidates = tasks
            .into_iter()
            .filter(|task| task.status == WorkTaskStatus::Ready)
            .filter(|task| {
                task.depends_on_task_ids.iter().all(|dependency_id| {
                    task_index
                        .get(dependency_id)
                        .map(|dependency| dependency_satisfied_for_dispatch(task, dependency))
                        .unwrap_or(false)
                })
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| {
            dispatch_lane_rank(left.lane)
                .cmp(&dispatch_lane_rank(right.lane))
                .then(left.created_at_ms.cmp(&right.created_at_ms))
                .then(left.task_id.cmp(&right.task_id))
        });

        let Some(mut task) = candidates.into_iter().next() else {
            return Ok(None);
        };

        self.store
            .update_task_state(session_id, &task.task_id, WorkTaskStatus::Running, None)?;
        self.store.update_session_state(
            session_id,
            if task.lane == WorkTaskLane::Validation {
                WorkSessionStatus::AwaitingValidation
            } else {
                WorkSessionStatus::Executing
            },
            None,
            None,
            None,
        )?;
        task.status = WorkTaskStatus::Running;
        Ok(Some(task))
    }

    pub fn finalize_runtime_task_success(
        &self,
        session_id: &str,
        task_id: &str,
        summary: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
    ) -> Result<()> {
        let tasks = self.store.list_tasks(session_id)?;
        let task_index = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task.clone()))
            .collect::<HashMap<_, _>>();
        let task = task_index
            .get(task_id)
            .cloned()
            .with_context(|| format!("task {task_id} not found for session {session_id}"))?;

        match task.lane {
            WorkTaskLane::Planning | WorkTaskLane::Search => {
                self.store
                    .update_task_state(session_id, task_id, WorkTaskStatus::Done, None)?;
            }
            WorkTaskLane::Execution => {
                if task.verification_required {
                    self.store.update_task_state(
                        session_id,
                        task_id,
                        WorkTaskStatus::AwaitingValidation,
                        None,
                    )?;
                    self.store.update_session_state(
                        session_id,
                        WorkSessionStatus::AwaitingValidation,
                        mission_id,
                        trace_session_id,
                        None,
                    )?;
                    self.store.append_artifact(
                        session_id,
                        Some(task_id),
                        "task_completion_candidate",
                        json!({
                            "mission_id": mission_id,
                            "trace_session_id": trace_session_id,
                            "task_id": task.task_id,
                            "title": task.title,
                            "task_type": task.task_type,
                            "status": "awaiting_validation",
                            "summary": summary,
                            "file_revisions": self.file_revision_snapshot_for_task(session_id, &task)?,
                        }),
                        &[],
                        None,
                    )?;
                } else {
                    self.store.update_task_state(
                        session_id,
                        task_id,
                        WorkTaskStatus::Done,
                        None,
                    )?;
                    self.append_task_completion_artifact(
                        session_id,
                        &task,
                        mission_id,
                        trace_session_id,
                        summary,
                    )?;
                    self.store.update_session_state(
                        session_id,
                        WorkSessionStatus::Executing,
                        mission_id,
                        trace_session_id,
                        None,
                    )?;
                }
            }
            WorkTaskLane::Validation => {
                let Some(parent_task_id) = task.parent_task_id.as_deref() else {
                    return Err(anyhow::anyhow!(
                        "validation task {} is missing parent execution task",
                        task.task_id
                    ));
                };
                let parent_task = task_index.get(parent_task_id).cloned().with_context(|| {
                    format!(
                        "parent execution task {parent_task_id} not found for validation {}",
                        task.task_id
                    )
                })?;
                self.store
                    .update_task_state(session_id, task_id, WorkTaskStatus::Done, None)?;
                self.store.update_task_state(
                    session_id,
                    parent_task_id,
                    WorkTaskStatus::Done,
                    None,
                )?;
                self.append_validation_summary_artifact(
                    session_id,
                    &task,
                    &parent_task,
                    summary,
                    mission_id,
                    trace_session_id,
                )?;
                self.append_task_completion_artifact(
                    session_id,
                    &parent_task,
                    mission_id,
                    trace_session_id,
                    summary,
                )?;
                self.store.update_session_state(
                    session_id,
                    WorkSessionStatus::Executing,
                    mission_id,
                    trace_session_id,
                    None,
                )?;
            }
        }

        Ok(())
    }

    pub fn finalize_runtime_task_failure(
        &self,
        session_id: &str,
        task_id: &str,
        error_message: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
    ) -> Result<()> {
        let tasks = self.store.list_tasks(session_id)?;
        let task_index = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task.clone()))
            .collect::<HashMap<_, _>>();
        let task = task_index
            .get(task_id)
            .cloned()
            .with_context(|| format!("task {task_id} not found for session {session_id}"))?;

        match task.lane {
            WorkTaskLane::Planning => {
                self.store.update_task_state(
                    session_id,
                    task_id,
                    WorkTaskStatus::FailedTerminal,
                    Some(error_message),
                )?;
                self.append_blocker_artifact(
                    session_id,
                    &task,
                    "planning_failed",
                    error_message,
                    mission_id,
                    trace_session_id,
                )?;
            }
            WorkTaskLane::Search => {
                self.store.update_task_state(
                    session_id,
                    task_id,
                    WorkTaskStatus::Blocked,
                    Some(error_message),
                )?;
                self.append_blocker_artifact(
                    session_id,
                    &task,
                    "search_failed",
                    error_message,
                    mission_id,
                    trace_session_id,
                )?;
            }
            WorkTaskLane::Execution => {
                self.store.update_task_state(
                    session_id,
                    task_id,
                    WorkTaskStatus::Blocked,
                    Some(error_message),
                )?;
                self.append_blocker_artifact(
                    session_id,
                    &task,
                    "execution_failed",
                    error_message,
                    mission_id,
                    trace_session_id,
                )?;
                for validation in tasks.iter().filter(|candidate| {
                    candidate.lane == WorkTaskLane::Validation
                        && candidate.parent_task_id.as_deref() == Some(task_id)
                        && matches!(
                            candidate.status,
                            WorkTaskStatus::Ready
                                | WorkTaskStatus::Running
                                | WorkTaskStatus::AwaitingValidation
                        )
                }) {
                    self.store.update_task_state(
                        session_id,
                        &validation.task_id,
                        WorkTaskStatus::Cancelled,
                        Some("parent execution task did not complete"),
                    )?;
                }
            }
            WorkTaskLane::Validation => {
                let validation_error = format!("validation failed: {error_message}");
                self.store.update_task_state(
                    session_id,
                    task_id,
                    WorkTaskStatus::Blocked,
                    Some(error_message),
                )?;
                self.append_blocker_artifact(
                    session_id,
                    &task,
                    "validation_failed",
                    error_message,
                    mission_id,
                    trace_session_id,
                )?;
                if let Some(parent_task_id) = task.parent_task_id.as_deref() {
                    self.store.update_task_state(
                        session_id,
                        parent_task_id,
                        WorkTaskStatus::Blocked,
                        Some(validation_error.as_str()),
                    )?;
                }
            }
        }

        self.store.update_session_state(
            session_id,
            WorkSessionStatus::Blocked,
            mission_id,
            trace_session_id,
            None,
        )?;
        self.store.set_session_error(session_id, error_message)?;
        Ok(())
    }

    pub fn finalize_success(
        &self,
        session_id: &str,
        mission_id: &str,
        trace_session_id: &str,
        mission_output: &str,
        trace_events: &[ExecutionTraceEvent],
        duration_ms: u128,
    ) -> Result<()> {
        self.finalize_outcome(
            session_id,
            Some(mission_id),
            Some(trace_session_id),
            mission_output,
            trace_events,
            duration_ms,
            true,
        )?;
        Ok(())
    }

    pub fn finalize_outcome(
        &self,
        session_id: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
        mission_output: &str,
        trace_events: &[ExecutionTraceEvent],
        duration_ms: u128,
        mission_success: bool,
    ) -> Result<FinalizedSessionOutcome> {
        self.store.append_artifact(
            session_id,
            None,
            "mission_result",
            json!({
                "mission_id": mission_id,
                "success": mission_success,
                "output": mission_output,
                "trace_event_count": trace_events.len(),
                "duration_ms": duration_ms,
            }),
            &[],
            None,
        )?;
        if let Some(trace_session_id) = trace_session_id {
            self.store.append_artifact(
                session_id,
                None,
                "trace_ref",
                json!({
                    "trace_session_id": trace_session_id,
                }),
                &[],
                None,
            )?;
        }
        if !mission_success {
            self.store.append_artifact(
                session_id,
                None,
                "failure",
                json!({
                    "error": mission_output,
                    "trace_event_count": trace_events.len(),
                }),
                &[],
                None,
            )?;
        }

        self.record_completion_artifacts(session_id, mission_id, trace_session_id)?;
        let completion = self.build_completion_summary(session_id)?;
        let tasks = self.store.list_tasks(session_id)?;
        let status = determine_terminal_status(mission_success, &completion, &tasks);
        self.store.update_session_state(
            session_id,
            status,
            mission_id,
            trace_session_id,
            Some(serde_json::to_value(&completion).context("serialize completion summary")?),
        )?;
        if status != WorkSessionStatus::Completed {
            let error_message = completion
                .blockers
                .first()
                .cloned()
                .unwrap_or_else(|| "request not fully satisfied".to_string());
            self.store.set_session_error(session_id, &error_message)?;
        }
        Ok(FinalizedSessionOutcome { status, completion })
    }

    pub fn finalize_failure(
        &self,
        session_id: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
        error_message: &str,
        trace_events: &[ExecutionTraceEvent],
    ) -> Result<()> {
        self.reconcile_trace_events(session_id, trace_events)?;
        self.fail_non_terminal_tasks(session_id, error_message)?;
        self.cancel_pending_validation_tasks(session_id)?;
        self.store.append_artifact(
            session_id,
            None,
            "failure",
            json!({
                "error": error_message,
                "trace_event_count": trace_events.len(),
            }),
            &[],
            None,
        )?;
        let completion = self.build_completion_summary(session_id)?;
        let status = if has_blocked_tasks(&self.store.list_tasks(session_id)?) {
            WorkSessionStatus::Blocked
        } else {
            WorkSessionStatus::Failed
        };
        self.store.update_session_state(
            session_id,
            status,
            mission_id,
            trace_session_id,
            Some(serde_json::to_value(completion).context("serialize failure completion")?),
        )?;
        self.store.set_session_error(session_id, error_message)?;
        Ok(())
    }

    fn fail_non_terminal_tasks(&self, session_id: &str, error_message: &str) -> Result<()> {
        for task in self.store.list_tasks(session_id)? {
            if matches!(
                task.status,
                WorkTaskStatus::Ready
                    | WorkTaskStatus::Running
                    | WorkTaskStatus::AwaitingValidation
            ) {
                let status = if task.lane == WorkTaskLane::Planning {
                    WorkTaskStatus::FailedTerminal
                } else {
                    WorkTaskStatus::Blocked
                };
                self.store.update_task_state(
                    session_id,
                    &task.task_id,
                    status,
                    Some(error_message),
                )?;
            }
        }
        Ok(())
    }

    fn record_completion_artifacts(
        &self,
        session_id: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
    ) -> Result<()> {
        for task in self
            .store
            .list_tasks(session_id)?
            .into_iter()
            .filter(|task| {
                task.lane == WorkTaskLane::Execution && task.status == WorkTaskStatus::Done
            })
        {
            self.append_task_completion_artifact(
                session_id,
                &task,
                mission_id,
                trace_session_id,
                "task closed through completion gate",
            )?;
        }
        Ok(())
    }

    fn cancel_pending_validation_tasks(&self, session_id: &str) -> Result<()> {
        for task in self
            .store
            .list_tasks(session_id)?
            .into_iter()
            .filter(|task| task.lane == WorkTaskLane::Validation)
        {
            if matches!(task.status, WorkTaskStatus::Ready | WorkTaskStatus::Running) {
                self.store.update_task_state(
                    session_id,
                    &task.task_id,
                    WorkTaskStatus::Cancelled,
                    Some("session did not complete successfully"),
                )?;
            }
        }
        Ok(())
    }

    fn build_completion_summary(&self, session_id: &str) -> Result<CompletionClosureSummary> {
        let tasks = self.store.list_tasks(session_id)?;
        let artifacts = self.store.list_artifacts(session_id)?;
        let mut required_requirement_titles = HashMap::new();
        let mut verification_required_requirements = HashSet::new();
        let mut remaining_requirements = Vec::new();
        let mut uncovered_deliverables = Vec::new();
        let mut blockers = Vec::new();
        let mut requirement_artifact_links = Vec::new();
        let mut requirement_verification_links = Vec::new();
        let mut artifact_backed_requirements = HashSet::new();
        let mut verified_requirements = HashSet::new();

        for task in tasks
            .iter()
            .filter(|task| task.lane == WorkTaskLane::Execution)
        {
            for requirement_ref in &task.requirement_refs {
                required_requirement_titles
                    .entry(requirement_ref.clone())
                    .or_insert_with(|| task.title.clone());
                if task.verification_required {
                    verification_required_requirements.insert(requirement_ref.clone());
                }
            }
            if task.status != WorkTaskStatus::Done {
                uncovered_deliverables.push(task.title.clone());
                if let Some(error_message) = task.error_message.clone() {
                    blockers.push(format!("{}: {}", task.title, error_message));
                } else {
                    blockers.push(format!("{}: not completed", task.title));
                }
                remaining_requirements.extend(task.requirement_refs.clone());
            }
        }

        for task in tasks
            .iter()
            .filter(|task| task.lane == WorkTaskLane::Validation)
        {
            if task.status != WorkTaskStatus::Done {
                blockers.push(
                    task.error_message
                        .clone()
                        .map(|error| format!("{}: {}", task.title, error))
                        .unwrap_or_else(|| format!("{}: {}", task.title, task.status.as_str())),
                );
                remaining_requirements.extend(task.requirement_refs.clone());
            }
        }

        for artifact in &artifacts {
            for requirement_ref in &artifact.requirement_refs {
                artifact_backed_requirements.insert(requirement_ref.clone());
                requirement_artifact_links.push(CompletionLink {
                    subject_ref: requirement_ref.clone(),
                    artifact_id: artifact.artifact_id.clone(),
                });
            }
            if let Some(validation_task_id) = artifact.verification_task_id.clone() {
                if artifact.requirement_refs.is_empty() {
                    requirement_verification_links.push(CompletionLink {
                        subject_ref: validation_task_id,
                        artifact_id: artifact.artifact_id.clone(),
                    });
                } else {
                    for requirement_ref in &artifact.requirement_refs {
                        verified_requirements.insert(requirement_ref.clone());
                        requirement_verification_links.push(CompletionLink {
                            subject_ref: requirement_ref.clone(),
                            artifact_id: artifact.artifact_id.clone(),
                        });
                    }
                }
            }
        }

        for (requirement_ref, title) in &required_requirement_titles {
            if !artifact_backed_requirements.contains(requirement_ref) {
                remaining_requirements.push(requirement_ref.clone());
                blockers.push(format!("{title}: missing requirement evidence"));
            }
        }

        for requirement_ref in verification_required_requirements {
            if !verified_requirements.contains(&requirement_ref) {
                remaining_requirements.push(requirement_ref.clone());
                if let Some(title) = required_requirement_titles.get(&requirement_ref) {
                    blockers.push(format!("{title}: missing verification evidence"));
                }
            }
        }

        remaining_requirements.sort();
        remaining_requirements.dedup();
        uncovered_deliverables.sort();
        uncovered_deliverables.dedup();
        blockers.sort();
        blockers.dedup();

        let requirement_closure_state =
            if required_requirement_titles.is_empty() && requirement_artifact_links.is_empty() {
                "not_applicable".to_string()
            } else if remaining_requirements.is_empty() {
                "satisfied".to_string()
            } else {
                "partial".to_string()
            };

        Ok(CompletionClosureSummary {
            request_satisfied: uncovered_deliverables.is_empty()
                && blockers.is_empty()
                && remaining_requirements.is_empty(),
            uncovered_deliverables,
            blockers,
            requirement_closure_state,
            remaining_requirements,
            requirement_artifact_links,
            requirement_verification_links,
        })
    }

    #[cfg(test)]
    pub fn store(&self) -> &WorkSessionStore {
        &self.store
    }

    fn append_task_completion_artifact(
        &self,
        session_id: &str,
        task: &WorkSessionTaskRecord,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
        summary: &str,
    ) -> Result<()> {
        if self.artifact_exists(session_id, Some(task.task_id.as_str()), "task_completion")? {
            return Ok(());
        }
        self.store.append_artifact(
            session_id,
            Some(task.task_id.as_str()),
            "task_completion",
            json!({
                "mission_id": mission_id,
                "trace_session_id": trace_session_id,
                "task_id": task.task_id,
                "title": task.title,
                "task_type": task.task_type,
                "status": task.status,
                "summary": summary,
                "file_revisions": self.file_revision_snapshot_for_task(session_id, task)?,
            }),
            &task.requirement_refs,
            None,
        )?;
        Ok(())
    }

    fn append_validation_summary_artifact(
        &self,
        session_id: &str,
        validation_task: &WorkSessionTaskRecord,
        parent_task: &WorkSessionTaskRecord,
        summary: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
    ) -> Result<()> {
        if self.artifact_exists(
            session_id,
            Some(validation_task.task_id.as_str()),
            "validation_summary",
        )? {
            return Ok(());
        }
        self.store.append_artifact(
            session_id,
            Some(validation_task.task_id.as_str()),
            "validation_summary",
            json!({
                "mission_id": mission_id,
                "trace_session_id": trace_session_id,
                "validation_scope": "independent_validation_task",
                "validated_task_id": parent_task.task_id,
                "validated_title": parent_task.title,
                "summary": summary,
                "status": "passed",
                "file_revisions": self.file_revision_snapshot_for_task(session_id, parent_task)?,
            }),
            &validation_task.requirement_refs,
            Some(validation_task.task_id.as_str()),
        )?;
        Ok(())
    }

    fn append_blocker_artifact(
        &self,
        session_id: &str,
        task: &WorkSessionTaskRecord,
        blocker_kind: &str,
        reason: &str,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
    ) -> Result<()> {
        self.store.append_artifact(
            session_id,
            Some(task.task_id.as_str()),
            "blocker",
            json!({
                "mission_id": mission_id,
                "trace_session_id": trace_session_id,
                "task_id": task.task_id,
                "lane": task.lane,
                "blocker_kind": blocker_kind,
                "reason": reason,
            }),
            &task.requirement_refs,
            None,
        )?;
        Ok(())
    }

    fn artifact_exists(
        &self,
        session_id: &str,
        task_id: Option<&str>,
        artifact_kind: &str,
    ) -> Result<bool> {
        Ok(self
            .store
            .list_artifacts(session_id)?
            .iter()
            .any(|artifact| {
                artifact.artifact_kind == artifact_kind && artifact.task_id.as_deref() == task_id
            }))
    }

    fn block_stale_validation_tasks(
        &self,
        session_id: &str,
        tasks: &[WorkSessionTaskRecord],
    ) -> Result<()> {
        let session = self
            .store
            .get_session(session_id)?
            .with_context(|| format!("session {session_id} not found"))?;
        let workspace_root = PathBuf::from(session.workspace_root);
        let artifacts = self.store.list_artifacts(session_id)?;

        for validation in tasks.iter().filter(|task| {
            task.lane == WorkTaskLane::Validation && task.status == WorkTaskStatus::Ready
        }) {
            let Some(parent_task_id) = validation.parent_task_id.as_deref() else {
                continue;
            };
            let expected = artifacts
                .iter()
                .rev()
                .find(|artifact| {
                    artifact.artifact_kind == "task_completion_candidate"
                        && artifact.task_id.as_deref() == Some(parent_task_id)
                })
                .and_then(|artifact| artifact.payload_json.get("file_revisions"))
                .cloned();
            let Some(expected) = expected else {
                continue;
            };
            let current = file_revision_snapshot(&workspace_root, &validation.target_files);
            if current != expected {
                let reason =
                    "validation context is stale; workspace changed after execution candidate";
                self.store.update_task_state(
                    session_id,
                    &validation.task_id,
                    WorkTaskStatus::Blocked,
                    Some(reason),
                )?;
                self.store.update_task_state(
                    session_id,
                    parent_task_id,
                    WorkTaskStatus::Blocked,
                    Some(reason),
                )?;
                self.append_blocker_artifact(
                    session_id,
                    validation,
                    "stale_context",
                    reason,
                    None,
                    None,
                )?;
            }
        }

        Ok(())
    }

    fn file_revision_snapshot_for_task(
        &self,
        session_id: &str,
        task: &WorkSessionTaskRecord,
    ) -> Result<Value> {
        let session = self
            .store
            .get_session(session_id)?
            .with_context(|| format!("session {session_id} not found"))?;
        Ok(file_revision_snapshot(
            &PathBuf::from(session.workspace_root),
            &task.target_files,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct WorkSessionStore {
    db_path: PathBuf,
    busy_timeout: Duration,
}

impl WorkSessionStore {
    pub fn path_for_workspace(workspace_root: &Path) -> PathBuf {
        workspace_root.join(".openakta").join("work-management.db")
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let store = Self {
            db_path: path.into(),
            busy_timeout: Duration::from_secs(DB_BUSY_TIMEOUT_SECS),
        };
        store.init_schema()?;
        Ok(store)
    }

    pub fn create_session(&self, init: &WorkSessionInit) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        conn.execute(
            r#"
            INSERT INTO wm_work_sessions
                (session_id, workspace_root, request_text, surface, response_preference,
                 allow_code_context, side_effects_allowed, remote_enabled, status, admitted_mode,
                 task_type, risk, decision_json, outcome_json, mission_id, trace_session_id,
                 error_message, created_at_ms, updated_at_ms, completed_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, NULL, ?14, NULL, ?15, ?15, NULL)
            ON CONFLICT(session_id) DO UPDATE SET
                request_text = excluded.request_text,
                surface = excluded.surface,
                response_preference = excluded.response_preference,
                allow_code_context = excluded.allow_code_context,
                side_effects_allowed = excluded.side_effects_allowed,
                remote_enabled = excluded.remote_enabled,
                status = excluded.status,
                admitted_mode = excluded.admitted_mode,
                task_type = excluded.task_type,
                risk = excluded.risk,
                decision_json = excluded.decision_json,
                trace_session_id = excluded.trace_session_id,
                updated_at_ms = excluded.updated_at_ms
            "#,
            params![
                init.session_id.as_str(),
                init.workspace_root.display().to_string(),
                init.request_text.as_str(),
                serde_json::to_string(&init.surface).context("serialize message surface")?,
                serde_json::to_string(&init.response_preference)
                    .context("serialize response preference")?,
                init.allow_code_context,
                init.side_effects_allowed,
                init.remote_enabled,
                WorkSessionStatus::Admitted.as_str(),
                serde_json::to_string(&init.decision.mode).context("serialize execution mode")?,
                serde_json::to_string(&init.decision.task_type).context("serialize task type")?,
                serde_json::to_string(&init.decision.risk).context("serialize risk")?,
                serde_json::to_string(&init.decision).context("serialize mission decision")?,
                init.session_id.as_str(),
                now,
            ],
        )?;
        Ok(())
    }

    pub fn upsert_tasks(&self, session_id: &str, seeds: &[TaskShellSeed]) -> Result<()> {
        let mut conn = self.connect()?;
        let tx = conn.transaction()?;
        for seed in seeds {
            tx.execute(
                r#"
                INSERT INTO wm_work_session_tasks
                    (session_id, task_id, parent_task_id, lane, title, task_type, status,
                     depends_on_task_ids_json, requirement_refs_json, expected_artifacts_json,
                     target_files_json, target_symbols_json, verification_required, error_message,
                     created_at_ms, updated_at_ms)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, ?14, ?14)
                ON CONFLICT(session_id, task_id) DO UPDATE SET
                    parent_task_id = excluded.parent_task_id,
                    lane = excluded.lane,
                    title = excluded.title,
                    task_type = excluded.task_type,
                    depends_on_task_ids_json = excluded.depends_on_task_ids_json,
                    requirement_refs_json = excluded.requirement_refs_json,
                    expected_artifacts_json = excluded.expected_artifacts_json,
                    target_files_json = excluded.target_files_json,
                    target_symbols_json = excluded.target_symbols_json,
                    verification_required = excluded.verification_required,
                    updated_at_ms = excluded.updated_at_ms
                "#,
                params![
                    session_id,
                    seed.task_id.as_str(),
                    seed.parent_task_id.as_deref(),
                    seed.lane.as_str(),
                    seed.title.as_str(),
                    serde_json::to_string(&seed.task_type).context("serialize task type")?,
                    WorkTaskStatus::Ready.as_str(),
                    serde_json::to_string(&seed.depends_on_task_ids)
                        .context("serialize task dependencies")?,
                    serde_json::to_string(&seed.requirement_refs)
                        .context("serialize requirement refs")?,
                    serde_json::to_string(&seed.expected_artifacts)
                        .context("serialize expected artifacts")?,
                    serde_json::to_string(&seed.target_files).context("serialize target files")?,
                    serde_json::to_string(&seed.target_symbols)
                        .context("serialize target symbols")?,
                    seed.verification_required,
                    now_ms(),
                ],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn update_session_state(
        &self,
        session_id: &str,
        status: WorkSessionStatus,
        mission_id: Option<&str>,
        trace_session_id: Option<&str>,
        outcome_json: Option<Value>,
    ) -> Result<()> {
        let conn = self.connect()?;
        let now = now_ms();
        conn.execute(
            r#"
            UPDATE wm_work_sessions
            SET status = ?2,
                mission_id = COALESCE(?3, mission_id),
                trace_session_id = COALESCE(?4, trace_session_id),
                outcome_json = COALESCE(?5, outcome_json),
                updated_at_ms = ?6,
                completed_at_ms = CASE
                    WHEN ?2 IN ('completed', 'failed', 'blocked') THEN ?6
                    ELSE completed_at_ms
                END
            WHERE session_id = ?1
            "#,
            params![
                session_id,
                status.as_str(),
                mission_id,
                trace_session_id,
                outcome_json
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .context("serialize session outcome")?,
                now,
            ],
        )?;
        Ok(())
    }

    pub fn set_session_error(&self, session_id: &str, error_message: &str) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            "UPDATE wm_work_sessions SET error_message = ?2, updated_at_ms = ?3 WHERE session_id = ?1",
            params![session_id, error_message, now_ms()],
        )?;
        Ok(())
    }

    pub fn update_task_state(
        &self,
        session_id: &str,
        task_id: &str,
        status: WorkTaskStatus,
        error_message: Option<&str>,
    ) -> Result<()> {
        let conn = self.connect()?;
        conn.execute(
            r#"
            UPDATE wm_work_session_tasks
            SET status = ?3,
                error_message = CASE
                    WHEN ?4 IS NOT NULL THEN ?4
                    ELSE error_message
                END,
                updated_at_ms = ?5
            WHERE session_id = ?1 AND task_id = ?2
            "#,
            params![
                session_id,
                task_id,
                status.as_str(),
                error_message,
                now_ms()
            ],
        )?;
        Ok(())
    }

    pub fn append_artifact(
        &self,
        session_id: &str,
        task_id: Option<&str>,
        artifact_kind: &str,
        payload_json: Value,
        requirement_refs: &[String],
        verification_task_id: Option<&str>,
    ) -> Result<String> {
        let artifact_id = Uuid::new_v4().to_string();
        let conn = self.connect()?;
        conn.execute(
            r#"
            INSERT INTO wm_work_session_artifacts
                (artifact_id, session_id, task_id, artifact_kind, requirement_refs_json,
                 verification_task_id, payload_json, created_at_ms)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                artifact_id.as_str(),
                session_id,
                task_id,
                artifact_kind,
                serde_json::to_string(requirement_refs)
                    .context("serialize artifact requirement refs")?,
                verification_task_id,
                serde_json::to_string(&payload_json).context("serialize artifact payload")?,
                now_ms(),
            ],
        )?;
        Ok(artifact_id)
    }

    pub fn get_session(&self, session_id: &str) -> Result<Option<WorkSessionRecord>> {
        use rusqlite::OptionalExtension;

        let conn = self.connect()?;
        conn.query_row(
            r#"
            SELECT session_id, workspace_root, request_text, status, admitted_mode, task_type,
                   risk, mission_id, trace_session_id, error_message, decision_json,
                   outcome_json, created_at_ms, updated_at_ms, completed_at_ms
            FROM wm_work_sessions
            WHERE session_id = ?1
            "#,
            params![session_id],
            row_to_session_record,
        )
        .optional()
        .map_err(Into::into)
    }

    pub fn list_tasks(&self, session_id: &str) -> Result<Vec<WorkSessionTaskRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT session_id, task_id, parent_task_id, lane, title, task_type, status,
                   depends_on_task_ids_json, requirement_refs_json, expected_artifacts_json,
                   target_files_json, target_symbols_json, verification_required, error_message,
                   created_at_ms, updated_at_ms
            FROM wm_work_session_tasks
            WHERE session_id = ?1
            ORDER BY created_at_ms ASC, task_id ASC
            "#,
        )?;
        let rows = stmt
            .query_map(params![session_id], row_to_task_record)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn list_artifacts(&self, session_id: &str) -> Result<Vec<WorkSessionArtifactRecord>> {
        let conn = self.connect()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT artifact_id, session_id, task_id, artifact_kind, requirement_refs_json,
                   verification_task_id, payload_json, created_at_ms
            FROM wm_work_session_artifacts
            WHERE session_id = ?1
            ORDER BY created_at_ms ASC, artifact_id ASC
            "#,
        )?;
        let rows = stmt
            .query_map(params![session_id], row_to_artifact_record)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn init_schema(&self) -> Result<()> {
        if let Some(parent) = self.db_path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("create control-plane db directory {}", parent.display())
            })?;
        }
        let conn = self.connect()?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS wm_work_sessions (
                session_id TEXT PRIMARY KEY,
                workspace_root TEXT NOT NULL,
                request_text TEXT NOT NULL,
                surface TEXT NOT NULL,
                response_preference TEXT NOT NULL,
                allow_code_context INTEGER NOT NULL,
                side_effects_allowed INTEGER NOT NULL,
                remote_enabled INTEGER NOT NULL,
                status TEXT NOT NULL,
                admitted_mode TEXT NOT NULL,
                task_type TEXT NOT NULL,
                risk TEXT NOT NULL,
                decision_json TEXT NOT NULL,
                outcome_json TEXT,
                mission_id TEXT,
                trace_session_id TEXT,
                error_message TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                completed_at_ms INTEGER
            );

            CREATE INDEX IF NOT EXISTS idx_wm_work_sessions_status
            ON wm_work_sessions(status, updated_at_ms DESC);

            CREATE TABLE IF NOT EXISTS wm_work_session_tasks (
                session_id TEXT NOT NULL,
                task_id TEXT NOT NULL,
                parent_task_id TEXT,
                lane TEXT NOT NULL,
                title TEXT NOT NULL,
                task_type TEXT NOT NULL,
                status TEXT NOT NULL,
                depends_on_task_ids_json TEXT NOT NULL,
                requirement_refs_json TEXT NOT NULL,
                expected_artifacts_json TEXT NOT NULL,
                target_files_json TEXT NOT NULL,
                target_symbols_json TEXT NOT NULL,
                verification_required INTEGER NOT NULL,
                error_message TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL,
                PRIMARY KEY (session_id, task_id)
            );

            CREATE INDEX IF NOT EXISTS idx_wm_work_session_tasks_lane
            ON wm_work_session_tasks(session_id, lane, status, updated_at_ms DESC);

            CREATE TABLE IF NOT EXISTS wm_work_session_artifacts (
                artifact_id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                task_id TEXT,
                artifact_kind TEXT NOT NULL,
                requirement_refs_json TEXT NOT NULL,
                verification_task_id TEXT,
                payload_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_wm_work_session_artifacts_session
            ON wm_work_session_artifacts(session_id, created_at_ms DESC);
            "#,
        )?;
        self.migrate_schema(&conn)?;
        Ok(())
    }

    fn migrate_schema(&self, conn: &Connection) -> Result<()> {
        if !Self::column_exists(conn, "wm_work_session_tasks", "depends_on_task_ids_json")? {
            conn.execute(
                "ALTER TABLE wm_work_session_tasks ADD COLUMN depends_on_task_ids_json TEXT NOT NULL DEFAULT '[]'",
                [],
            )?;
        }
        Ok(())
    }

    fn connect(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open control-plane db {}", self.db_path.display()))?;
        conn.busy_timeout(self.busy_timeout)?;
        Ok(conn)
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let pragma = format!("PRAGMA table_info({table})");
        let mut stmt = conn.prepare(&pragma)?;
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            let name: String = row.get(1)?;
            if name == column {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn row_to_session_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkSessionRecord> {
    Ok(WorkSessionRecord {
        session_id: row.get(0)?,
        workspace_root: row.get(1)?,
        request_text: row.get(2)?,
        status: WorkSessionStatus::from_str(&row.get::<_, String>(3)?)
            .unwrap_or(WorkSessionStatus::Failed),
        admitted_mode: serde_json::from_str(&row.get::<_, String>(4)?)
            .unwrap_or(MessageExecutionMode::SingleAgent),
        task_type: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(TaskType::General),
        risk: serde_json::from_str(&row.get::<_, String>(6)?).unwrap_or(RiskLevel::Low),
        mission_id: row.get(7)?,
        trace_session_id: row.get(8)?,
        error_message: row.get(9)?,
        decision_json: parse_json_column(&row.get::<_, String>(10)?),
        outcome_json: row
            .get::<_, Option<String>>(11)?
            .map(|value| parse_json_column(&value)),
        created_at_ms: row.get(12)?,
        updated_at_ms: row.get(13)?,
        completed_at_ms: row.get(14)?,
    })
}

fn row_to_task_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkSessionTaskRecord> {
    Ok(WorkSessionTaskRecord {
        session_id: row.get(0)?,
        task_id: row.get(1)?,
        parent_task_id: row.get(2)?,
        lane: WorkTaskLane::from_str(&row.get::<_, String>(3)?).unwrap_or(WorkTaskLane::Execution),
        title: row.get(4)?,
        task_type: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(TaskType::General),
        status: WorkTaskStatus::from_str(&row.get::<_, String>(6)?)
            .unwrap_or(WorkTaskStatus::Blocked),
        depends_on_task_ids: parse_vec_string(&row.get::<_, String>(7)?),
        requirement_refs: parse_vec_string(&row.get::<_, String>(8)?),
        expected_artifacts: parse_vec_string(&row.get::<_, String>(9)?),
        target_files: parse_vec_string(&row.get::<_, String>(10)?),
        target_symbols: parse_vec_string(&row.get::<_, String>(11)?),
        verification_required: row.get(12)?,
        error_message: row.get(13)?,
        created_at_ms: row.get(14)?,
        updated_at_ms: row.get(15)?,
    })
}

fn row_to_artifact_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkSessionArtifactRecord> {
    Ok(WorkSessionArtifactRecord {
        artifact_id: row.get(0)?,
        session_id: row.get(1)?,
        task_id: row.get(2)?,
        artifact_kind: row.get(3)?,
        requirement_refs: parse_vec_string(&row.get::<_, String>(4)?),
        verification_task_id: row.get(5)?,
        payload_json: parse_json_column(&row.get::<_, String>(6)?),
        created_at_ms: row.get(7)?,
    })
}

fn derive_task_state(
    record: &WorkSessionTaskRecord,
    events: &[&ExecutionTraceEvent],
) -> (WorkTaskStatus, Option<String>) {
    if events
        .iter()
        .any(|event| event.phase == ExecutionTracePhase::Failed)
    {
        if record.status == WorkTaskStatus::Blocked {
            return (WorkTaskStatus::Blocked, record.error_message.clone());
        }
        let error_message = events
            .iter()
            .rev()
            .find_map(|event| event.error.clone())
            .or_else(|| Some("task failed".to_string()));
        return (WorkTaskStatus::FailedTerminal, error_message);
    }
    if events
        .iter()
        .any(|event| event.phase == ExecutionTracePhase::Completed)
    {
        return (
            if record.verification_required {
                if matches!(
                    record.status,
                    WorkTaskStatus::Done
                        | WorkTaskStatus::Blocked
                        | WorkTaskStatus::FailedTerminal
                        | WorkTaskStatus::Cancelled
                ) {
                    record.status
                } else {
                    WorkTaskStatus::AwaitingValidation
                }
            } else {
                WorkTaskStatus::Done
            },
            None,
        );
    }
    if events
        .iter()
        .any(|event| event.phase == ExecutionTracePhase::Started)
    {
        return (WorkTaskStatus::Running, None);
    }
    (record.status, record.error_message.clone())
}

fn expected_artifacts_for_task_type(task_type: &TaskType) -> Vec<String> {
    match task_type {
        TaskType::General => vec!["mission_result".to_string()],
        TaskType::CodeModification => vec![
            "patch_receipt".to_string(),
            "mission_result".to_string(),
            "validation_summary".to_string(),
        ],
        TaskType::Review => vec!["mission_result".to_string()],
        TaskType::Retrieval => vec!["mission_result".to_string()],
    }
}

pub fn task_shells_for_direct_task(
    session_id: &str,
    task: &Task,
    hints: &TaskTargetHints,
    has_initial_retrieval: bool,
) -> Vec<TaskShellSeed> {
    let mut execution = TaskShellSeed::execution(task, hints);
    execution.requirement_refs = vec![request_requirement_ref(session_id)];
    if has_initial_retrieval {
        execution
            .depends_on_task_ids
            .push(search_task_id(session_id));
    }
    let mut seeds = vec![execution];
    if let Some(validation) = TaskShellSeed::validation(&seeds[0]) {
        seeds.push(validation);
    }
    seeds
}

pub fn task_shells_for_decomposed_mission(
    session_id: &str,
    decomposed: &DecomposedMission,
) -> Vec<TaskShellSeed> {
    let task_ids = decomposed
        .tasks
        .iter()
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let mut seeds = Vec::new();
    for (index, task) in decomposed.tasks.iter().enumerate() {
        let hints = extract_target_hints(&task.description);
        let mut execution = TaskShellSeed::execution(task, &hints);
        execution.requirement_refs = vec![deliverable_requirement_ref(&task.id)];
        execution
            .depends_on_task_ids
            .push(planning_task_id(session_id));
        for dependency in decomposed
            .dependencies
            .iter()
            .filter(|dependency| dependency.from == index)
        {
            if let Some(task_id) = task_ids.get(dependency.to) {
                push_dependency_once(&mut execution.depends_on_task_ids, task_id.clone());
            }
        }
        seeds.push(execution.clone());
        if let Some(validation) = TaskShellSeed::validation(&execution) {
            seeds.push(validation);
        }
    }
    seeds
}

pub fn planning_task_id(session_id: &str) -> String {
    format!("planning:{session_id}")
}

pub fn search_task_id(session_id: &str) -> String {
    format!("search:{session_id}")
}

pub fn validation_task_id(parent_task_id: &str) -> String {
    format!("validate:{parent_task_id}")
}

fn request_requirement_ref(session_id: &str) -> String {
    format!("request:{session_id}")
}

fn deliverable_requirement_ref(task_id: &str) -> String {
    format!("deliverable:{task_id}")
}

fn push_dependency_once(dependencies: &mut Vec<String>, dependency: String) {
    if !dependencies.iter().any(|existing| existing == &dependency) {
        dependencies.push(dependency);
    }
}

fn has_blocked_tasks(tasks: &[WorkSessionTaskRecord]) -> bool {
    tasks
        .iter()
        .any(|task| task.status == WorkTaskStatus::Blocked)
}

fn dependency_satisfied_for_dispatch(
    task: &WorkSessionTaskRecord,
    dependency: &WorkSessionTaskRecord,
) -> bool {
    match task.lane {
        WorkTaskLane::Validation => matches!(
            dependency.status,
            WorkTaskStatus::AwaitingValidation | WorkTaskStatus::Done
        ),
        _ => dependency.status == WorkTaskStatus::Done,
    }
}

fn dispatch_lane_rank(lane: WorkTaskLane) -> u8 {
    match lane {
        WorkTaskLane::Planning => 0,
        WorkTaskLane::Search => 1,
        WorkTaskLane::Execution => 2,
        WorkTaskLane::Validation => 3,
    }
}

fn has_open_runtime_tasks(tasks: &[WorkSessionTaskRecord]) -> bool {
    tasks.iter().any(|task| {
        matches!(
            task.status,
            WorkTaskStatus::Ready | WorkTaskStatus::Running | WorkTaskStatus::AwaitingValidation
        )
    })
}

fn determine_terminal_status(
    mission_success: bool,
    completion: &CompletionClosureSummary,
    tasks: &[WorkSessionTaskRecord],
) -> WorkSessionStatus {
    if completion.request_satisfied {
        return WorkSessionStatus::Completed;
    }

    if has_blocked_tasks(tasks)
        || has_open_runtime_tasks(tasks)
        || !completion.remaining_requirements.is_empty()
        || !completion.uncovered_deliverables.is_empty()
    {
        return WorkSessionStatus::Blocked;
    }

    if mission_success {
        WorkSessionStatus::Blocked
    } else {
        WorkSessionStatus::Failed
    }
}

fn parse_json_column(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or(Value::Null)
}

fn parse_vec_string(raw: &str) -> Vec<String> {
    serde_json::from_str(raw).unwrap_or_default()
}

fn file_revision_snapshot(workspace_root: &Path, files: &[String]) -> Value {
    Value::Array(
        files
            .iter()
            .map(|file| {
                let resolved = workspace_root.join(file);
                let metadata = std::fs::metadata(&resolved).ok();
                json!({
                    "path": file,
                    "size_bytes": metadata.as_ref().map(|item| item.len()),
                    "modified_at_ms": metadata
                        .and_then(|item| item.modified().ok())
                        .and_then(|timestamp| {
                            timestamp
                                .duration_since(std::time::UNIX_EPOCH)
                                .ok()
                                .map(|duration| duration.as_millis() as u64)
                        }),
                })
            })
            .collect(),
    )
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_agents::{DecompositionBudget, DelegationBudget, RetrievalPlan};

    #[test]
    fn control_plane_persists_session_tasks_and_artifacts() {
        let tempdir = tempfile::tempdir().unwrap();
        let runtime = ControlPlaneRuntime::open(tempdir.path()).unwrap();
        let session_id = Uuid::new_v4().to_string();
        let decision = MissionDecision {
            mode: MessageExecutionMode::DirectAction,
            retrieval_plan: RetrievalPlan {
                repo_context_requested: true,
                max_hits: 4,
                workspace_context: Some("ctx".to_string()),
            },
            target_hints: TaskTargetHints {
                target_files: vec!["src/lib.rs".to_string()],
                target_symbols: vec!["crate::run".to_string()],
            },
            decomposition_budget: DecompositionBudget {
                max_tasks: 1,
                max_parallelism: 1,
            },
            delegation_budget: DelegationBudget {
                max_agents: 0,
                max_depth: 0,
                allow_delegation: false,
            },
            task_type: TaskType::CodeModification,
            risk: RiskLevel::Medium,
        };
        runtime
            .admit_session(&WorkSessionInit {
                session_id: session_id.clone(),
                workspace_root: tempdir.path().to_path_buf(),
                request_text: "patch lib".to_string(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: false,
                decision: decision.clone(),
            })
            .unwrap();
        runtime
            .materialize_initial_retrieval(&session_id, "patch lib", &decision)
            .unwrap();

        let task = Task::new("patch lib").with_task_type(TaskType::CodeModification);
        runtime
            .register_task_shells(
                &session_id,
                &task_shells_for_direct_task(&session_id, &task, &decision.target_hints, true),
            )
            .unwrap();

        let session = runtime.store().get_session(&session_id).unwrap().unwrap();
        assert_eq!(session.status, WorkSessionStatus::Admitted);
        assert_eq!(session.admitted_mode, MessageExecutionMode::DirectAction);

        let tasks = runtime.store().list_tasks(&session_id).unwrap();
        assert_eq!(tasks.len(), 3);
        assert!(tasks.iter().any(|task| {
            task.lane == WorkTaskLane::Search
                && task.status == WorkTaskStatus::Done
                && task.task_id == search_task_id(&session_id)
        }));
        let search_id = search_task_id(&session_id);
        assert!(tasks.iter().any(|task| {
            task.lane == WorkTaskLane::Execution
                && task.depends_on_task_ids == vec![search_id.clone()]
        }));
        assert!(tasks.iter().any(|record| {
            record.lane == WorkTaskLane::Validation
                && record.depends_on_task_ids == vec![task.id.clone()]
        }));
        assert!(tasks
            .iter()
            .any(|task| task.lane == WorkTaskLane::Execution));
        assert!(tasks
            .iter()
            .any(|task| task.lane == WorkTaskLane::Validation));
        assert!(tasks
            .iter()
            .any(|task| task.requirement_refs == vec![request_requirement_ref(&session_id)]));

        let artifacts = runtime.store().list_artifacts(&session_id).unwrap();
        assert_eq!(artifacts.len(), 2);
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == "admission"));
        assert!(artifacts.iter().any(|artifact| {
            artifact.artifact_kind == "workspace_context"
                && artifact.task_id.as_deref() == Some(search_task_id(&session_id).as_str())
        }));
    }

    #[test]
    fn control_plane_success_closes_requirement_evidence() {
        let tempdir = tempfile::tempdir().unwrap();
        let runtime = ControlPlaneRuntime::open(tempdir.path()).unwrap();
        let session_id = Uuid::new_v4().to_string();
        let decision = MissionDecision {
            mode: MessageExecutionMode::DirectAction,
            retrieval_plan: RetrievalPlan {
                repo_context_requested: true,
                max_hits: 4,
                workspace_context: Some("ctx".to_string()),
            },
            target_hints: TaskTargetHints {
                target_files: vec!["src/lib.rs".to_string()],
                target_symbols: vec!["crate::run".to_string()],
            },
            decomposition_budget: DecompositionBudget {
                max_tasks: 1,
                max_parallelism: 1,
            },
            delegation_budget: DelegationBudget {
                max_agents: 0,
                max_depth: 0,
                allow_delegation: false,
            },
            task_type: TaskType::CodeModification,
            risk: RiskLevel::Medium,
        };
        runtime
            .admit_session(&WorkSessionInit {
                session_id: session_id.clone(),
                workspace_root: tempdir.path().to_path_buf(),
                request_text: "patch lib".to_string(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: false,
                decision: decision.clone(),
            })
            .unwrap();
        runtime
            .materialize_initial_retrieval(&session_id, "patch lib", &decision)
            .unwrap();

        let task = Task::new("patch lib").with_task_type(TaskType::CodeModification);
        runtime
            .register_task_shells(
                &session_id,
                &task_shells_for_direct_task(&session_id, &task, &TaskTargetHints::default(), true),
            )
            .unwrap();
        let execution = runtime
            .reserve_next_dispatchable_task(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(execution.task_id, task.id);
        assert_eq!(execution.lane, WorkTaskLane::Execution);
        runtime
            .finalize_runtime_task_success(
                &session_id,
                &task.id,
                "diff applied",
                Some("mission-1"),
                Some("trace-1"),
            )
            .unwrap();

        let validation = runtime
            .reserve_next_dispatchable_task(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(validation.lane, WorkTaskLane::Validation);
        runtime
            .finalize_runtime_task_success(
                &session_id,
                &validation.task_id,
                "validation passed",
                Some("mission-1"),
                Some("trace-1"),
            )
            .unwrap();

        runtime
            .finalize_outcome(
                &session_id,
                Some("mission-1"),
                Some("trace-1"),
                "done",
                &[],
                42,
                true,
            )
            .unwrap();

        let session = runtime.store().get_session(&session_id).unwrap().unwrap();
        assert_eq!(session.status, WorkSessionStatus::Completed);
        let summary: CompletionClosureSummary =
            serde_json::from_value(session.outcome_json.unwrap()).unwrap();
        assert!(summary.request_satisfied);
        assert_eq!(summary.requirement_closure_state, "satisfied");
        assert!(summary.remaining_requirements.is_empty());
        assert!(!summary.requirement_artifact_links.is_empty());
        assert!(!summary.requirement_verification_links.is_empty());

        let artifacts = runtime.store().list_artifacts(&session_id).unwrap();
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == "task_completion"));
        assert!(artifacts
            .iter()
            .any(|artifact| artifact.artifact_kind == "validation_summary"));
    }

    #[test]
    fn control_plane_blocks_completion_until_validation_runs() {
        let tempdir = tempfile::tempdir().unwrap();
        let runtime = ControlPlaneRuntime::open(tempdir.path()).unwrap();
        let session_id = Uuid::new_v4().to_string();
        let decision = MissionDecision {
            mode: MessageExecutionMode::DirectAction,
            retrieval_plan: RetrievalPlan {
                repo_context_requested: false,
                max_hits: 2,
                workspace_context: None,
            },
            target_hints: TaskTargetHints::default(),
            decomposition_budget: DecompositionBudget {
                max_tasks: 1,
                max_parallelism: 1,
            },
            delegation_budget: DelegationBudget {
                max_agents: 0,
                max_depth: 0,
                allow_delegation: false,
            },
            task_type: TaskType::CodeModification,
            risk: RiskLevel::Medium,
        };
        runtime
            .admit_session(&WorkSessionInit {
                session_id: session_id.clone(),
                workspace_root: tempdir.path().to_path_buf(),
                request_text: "patch lib".to_string(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: false,
                decision,
            })
            .unwrap();

        let task = Task::new("patch lib").with_task_type(TaskType::CodeModification);
        runtime
            .register_task_shells(
                &session_id,
                &task_shells_for_direct_task(
                    &session_id,
                    &task,
                    &TaskTargetHints::default(),
                    false,
                ),
            )
            .unwrap();

        let execution = runtime
            .reserve_next_dispatchable_task(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(execution.task_id, task.id);
        runtime
            .finalize_runtime_task_success(
                &session_id,
                &task.id,
                "diff applied",
                Some("mission-1"),
                Some("trace-1"),
            )
            .unwrap();

        let finalized = runtime
            .finalize_outcome(
                &session_id,
                Some("mission-1"),
                Some("trace-1"),
                "done",
                &[],
                42,
                true,
            )
            .unwrap();

        assert_eq!(finalized.status, WorkSessionStatus::Blocked);
        assert!(!finalized.completion.request_satisfied);
        assert!(finalized
            .completion
            .blockers
            .iter()
            .any(|blocker| blocker.contains("missing verification evidence")));
    }

    #[test]
    fn stale_workspace_blocks_validation_dispatch() {
        let tempdir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tempdir.path().join("src")).unwrap();
        std::fs::write(tempdir.path().join("src/lib.rs"), "fn old() {}\n").unwrap();

        let runtime = ControlPlaneRuntime::open(tempdir.path()).unwrap();
        let session_id = Uuid::new_v4().to_string();
        let decision = MissionDecision {
            mode: MessageExecutionMode::DirectAction,
            retrieval_plan: RetrievalPlan {
                repo_context_requested: false,
                max_hits: 2,
                workspace_context: None,
            },
            target_hints: TaskTargetHints {
                target_files: vec!["src/lib.rs".to_string()],
                target_symbols: Vec::new(),
            },
            decomposition_budget: DecompositionBudget {
                max_tasks: 1,
                max_parallelism: 1,
            },
            delegation_budget: DelegationBudget {
                max_agents: 0,
                max_depth: 0,
                allow_delegation: false,
            },
            task_type: TaskType::CodeModification,
            risk: RiskLevel::Medium,
        };
        runtime
            .admit_session(&WorkSessionInit {
                session_id: session_id.clone(),
                workspace_root: tempdir.path().to_path_buf(),
                request_text: "patch lib".to_string(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: false,
                decision,
            })
            .unwrap();

        let task = Task::new("patch src/lib.rs").with_task_type(TaskType::CodeModification);
        runtime
            .register_task_shells(
                &session_id,
                &task_shells_for_direct_task(
                    &session_id,
                    &task,
                    &TaskTargetHints {
                        target_files: vec!["src/lib.rs".to_string()],
                        target_symbols: Vec::new(),
                    },
                    false,
                ),
            )
            .unwrap();

        let execution = runtime
            .reserve_next_dispatchable_task(&session_id)
            .unwrap()
            .unwrap();
        assert_eq!(execution.task_id, task.id);
        runtime
            .finalize_runtime_task_success(
                &session_id,
                &task.id,
                "diff applied",
                Some("mission-1"),
                Some("trace-1"),
            )
            .unwrap();

        std::fs::write(tempdir.path().join("src/lib.rs"), "fn drifted() {}\n").unwrap();

        assert!(runtime
            .reserve_next_dispatchable_task(&session_id)
            .unwrap()
            .is_none());

        let snapshot = runtime.snapshot_session(&session_id).unwrap().unwrap();
        assert!(snapshot.tasks.iter().any(|task| {
            task.lane == WorkTaskLane::Validation && task.status == WorkTaskStatus::Blocked
        }));
        assert!(snapshot.tasks.iter().any(|task| {
            task.task_id == execution.task_id && task.status == WorkTaskStatus::Blocked
        }));
    }

    #[test]
    fn control_plane_failure_leaves_requirement_closure_open() {
        let tempdir = tempfile::tempdir().unwrap();
        let runtime = ControlPlaneRuntime::open(tempdir.path()).unwrap();
        let session_id = Uuid::new_v4().to_string();
        let decision = MissionDecision {
            mode: MessageExecutionMode::DirectAction,
            retrieval_plan: RetrievalPlan {
                repo_context_requested: false,
                max_hits: 2,
                workspace_context: None,
            },
            target_hints: TaskTargetHints::default(),
            decomposition_budget: DecompositionBudget {
                max_tasks: 1,
                max_parallelism: 1,
            },
            delegation_budget: DelegationBudget {
                max_agents: 0,
                max_depth: 0,
                allow_delegation: false,
            },
            task_type: TaskType::CodeModification,
            risk: RiskLevel::Medium,
        };
        runtime
            .admit_session(&WorkSessionInit {
                session_id: session_id.clone(),
                workspace_root: tempdir.path().to_path_buf(),
                request_text: "patch lib".to_string(),
                surface: MessageSurface::CliDo,
                response_preference: ResponsePreference::PreferMission,
                allow_code_context: true,
                side_effects_allowed: true,
                remote_enabled: false,
                decision,
            })
            .unwrap();

        let task = Task::new("patch lib").with_task_type(TaskType::CodeModification);
        runtime
            .register_task_shells(
                &session_id,
                &task_shells_for_direct_task(
                    &session_id,
                    &task,
                    &TaskTargetHints::default(),
                    false,
                ),
            )
            .unwrap();
        runtime
            .mark_task_state(&session_id, &task.id, WorkTaskStatus::Running, None)
            .unwrap();
        runtime
            .finalize_failure(
                &session_id,
                Some("mission-1"),
                Some("trace-1"),
                "patch failed",
                &[],
            )
            .unwrap();

        let session = runtime.store().get_session(&session_id).unwrap().unwrap();
        assert_eq!(session.status, WorkSessionStatus::Blocked);
        let summary: CompletionClosureSummary =
            serde_json::from_value(session.outcome_json.unwrap()).unwrap();
        assert!(!summary.request_satisfied);
        assert_eq!(summary.requirement_closure_state, "partial");
        assert!(!summary.remaining_requirements.is_empty());
        assert!(!summary.uncovered_deliverables.is_empty());
        assert!(summary
            .blockers
            .iter()
            .any(|blocker| blocker.contains("patch failed")));
    }

    #[test]
    fn decomposed_task_shells_capture_planning_and_graph_dependencies() {
        let session_id = Uuid::new_v4().to_string();
        let task_one = Task::new("edit src/lib.rs").with_task_type(TaskType::CodeModification);
        let task_two = Task::new("review src/lib.rs").with_task_type(TaskType::Review);
        let mut decomposed = DecomposedMission::new("ship feature");
        decomposed.tasks = vec![task_one.clone(), task_two.clone()];
        decomposed
            .dependencies
            .push(openakta_agents::Dependency::hard(1, 0));

        let shells = task_shells_for_decomposed_mission(&session_id, &decomposed);
        let execution_shells = shells
            .iter()
            .filter(|seed| seed.lane == WorkTaskLane::Execution)
            .collect::<Vec<_>>();
        let validation_shells = shells
            .iter()
            .filter(|seed| seed.lane == WorkTaskLane::Validation)
            .collect::<Vec<_>>();

        assert_eq!(execution_shells.len(), 2);
        assert_eq!(validation_shells.len(), 1);

        let first = execution_shells
            .iter()
            .find(|seed| seed.task_id == task_one.id)
            .unwrap();
        assert_eq!(
            first.depends_on_task_ids,
            vec![planning_task_id(&session_id)]
        );

        let second = execution_shells
            .iter()
            .find(|seed| seed.task_id == task_two.id)
            .unwrap();
        assert!(second
            .depends_on_task_ids
            .contains(&planning_task_id(&session_id)));
        assert!(second.depends_on_task_ids.contains(&task_one.id));
    }
}
