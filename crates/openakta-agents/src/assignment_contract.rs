//! Canonical worker assignment contract for runtime handoff.

use crate::task::TaskType;
use serde::{Deserialize, Serialize};

pub const DEFAULT_MAX_TOOL_TURNS: u32 = 6;
pub const DEFAULT_MAX_TOOL_CALLS: u32 = 8;
pub const DEFAULT_MAX_MUTATING_TOOL_CALLS: u32 = 2;

/// Stable execution lane for worker handoff.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerAssignmentLane {
    Planning,
    Search,
    Execution,
    Validation,
}

impl WorkerAssignmentLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Search => "search",
            Self::Execution => "execution",
            Self::Validation => "validation",
        }
    }
}

/// Runtime budget carried on the assignment contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerExecutionBudget {
    pub token_budget: u32,
    pub max_tool_turns: u32,
    pub max_tool_calls: u32,
    pub max_mutating_tool_calls: u32,
}

impl WorkerExecutionBudget {
    pub fn compat_defaults(token_budget: u32) -> Self {
        Self {
            token_budget,
            max_tool_turns: DEFAULT_MAX_TOOL_TURNS,
            max_tool_calls: DEFAULT_MAX_TOOL_CALLS,
            max_mutating_tool_calls: DEFAULT_MAX_MUTATING_TOOL_CALLS,
        }
    }
}

/// Minimal stop condition exposed to workers.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTerminationCondition {
    SummaryRequired,
    ValidatedPatchRequired,
    VerificationSummaryRequired,
    ContextArtifactRequired,
}

impl WorkerTerminationCondition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SummaryRequired => "summary_required",
            Self::ValidatedPatchRequired => "validated_patch_required",
            Self::VerificationSummaryRequired => "verification_summary_required",
            Self::ContextArtifactRequired => "context_artifact_required",
        }
    }
}

/// Whether the assignment originated in direct or plan-backed flow.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanningOriginMode {
    Direct,
    Planned,
}

impl PlanningOriginMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Planned => "planned",
        }
    }
}

/// Durable provenance for the assignment origin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanningOriginRef {
    pub mode: PlanningOriginMode,
    pub prepared_story_id: Option<String>,
    pub work_item_id: Option<String>,
    pub plan_version_id: Option<String>,
}

impl PlanningOriginRef {
    pub fn direct() -> Self {
        Self {
            mode: PlanningOriginMode::Direct,
            prepared_story_id: None,
            work_item_id: None,
            plan_version_id: None,
        }
    }

    pub fn planned(
        prepared_story_id: Option<String>,
        work_item_id: Option<String>,
        plan_version_id: Option<String>,
    ) -> Self {
        Self {
            mode: PlanningOriginMode::Planned,
            prepared_story_id,
            work_item_id,
            plan_version_id,
        }
    }
}

/// Canonical runtime handoff to a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerAssignmentContract {
    pub session_id: String,
    pub story_id: Option<String>,
    pub task_id: String,
    pub task_type: TaskType,
    pub lane: WorkerAssignmentLane,
    pub goal: String,
    pub requirement_refs: Vec<String>,
    pub context_artifact_refs: Vec<String>,
    pub target_files: Vec<String>,
    pub target_symbols: Vec<String>,
    pub expected_artifacts: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub budget: WorkerExecutionBudget,
    pub termination_condition: WorkerTerminationCondition,
    pub verification_required: bool,
    pub workspace_revision_token: Option<String>,
    pub planning_origin_ref: PlanningOriginRef,
}

impl WorkerAssignmentContract {
    pub fn legacy_title(&self) -> &str {
        &self.goal
    }

    pub fn legacy_description(&self) -> &str {
        &self.goal
    }
}

pub fn default_lane_for_task_type(task_type: &TaskType) -> WorkerAssignmentLane {
    match task_type {
        TaskType::Retrieval => WorkerAssignmentLane::Search,
        TaskType::Review => WorkerAssignmentLane::Validation,
        TaskType::General | TaskType::CodeModification => WorkerAssignmentLane::Execution,
    }
}

pub fn default_worker_role(task_type: &TaskType) -> &'static str {
    match task_type {
        TaskType::CodeModification => "coder",
        TaskType::Review => "reviewer",
        TaskType::Retrieval => "architect",
        TaskType::General => "executor",
    }
}

pub fn default_expected_artifacts(lane: WorkerAssignmentLane, task_type: &TaskType) -> Vec<String> {
    match lane {
        WorkerAssignmentLane::Planning => vec!["decomposition_plan".to_string()],
        WorkerAssignmentLane::Search => vec!["workspace_context".to_string()],
        WorkerAssignmentLane::Validation => vec!["validation_summary".to_string()],
        WorkerAssignmentLane::Execution => match task_type {
            TaskType::CodeModification => vec!["validated_patch".to_string()],
            TaskType::Retrieval => vec!["workspace_context".to_string()],
            TaskType::Review => vec!["review_summary".to_string()],
            TaskType::General => vec!["execution_summary".to_string()],
        },
    }
}

pub fn default_termination_condition(
    lane: WorkerAssignmentLane,
    task_type: &TaskType,
) -> WorkerTerminationCondition {
    match lane {
        WorkerAssignmentLane::Planning => WorkerTerminationCondition::SummaryRequired,
        WorkerAssignmentLane::Search => WorkerTerminationCondition::ContextArtifactRequired,
        WorkerAssignmentLane::Validation => WorkerTerminationCondition::VerificationSummaryRequired,
        WorkerAssignmentLane::Execution => match task_type {
            TaskType::CodeModification => WorkerTerminationCondition::ValidatedPatchRequired,
            TaskType::Retrieval => WorkerTerminationCondition::ContextArtifactRequired,
            TaskType::Review => WorkerTerminationCondition::VerificationSummaryRequired,
            TaskType::General => WorkerTerminationCondition::SummaryRequired,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_origin_defaults_are_stable() {
        let origin = PlanningOriginRef::direct();
        assert_eq!(origin.mode, PlanningOriginMode::Direct);
        assert!(origin.prepared_story_id.is_none());
        assert!(origin.work_item_id.is_none());
        assert!(origin.plan_version_id.is_none());
    }

    #[test]
    fn planned_origin_keeps_provenance_fields() {
        let origin = PlanningOriginRef::planned(
            Some("prepared-1".to_string()),
            Some("work-2".to_string()),
            Some("plan-3".to_string()),
        );
        assert_eq!(origin.mode, PlanningOriginMode::Planned);
        assert_eq!(origin.prepared_story_id.as_deref(), Some("prepared-1"));
        assert_eq!(origin.work_item_id.as_deref(), Some("work-2"));
        assert_eq!(origin.plan_version_id.as_deref(), Some("plan-3"));
    }

    #[test]
    fn compatibility_budget_uses_bounded_defaults() {
        let budget = WorkerExecutionBudget::compat_defaults(512);
        assert_eq!(budget.token_budget, 512);
        assert_eq!(budget.max_tool_turns, DEFAULT_MAX_TOOL_TURNS);
        assert_eq!(budget.max_tool_calls, DEFAULT_MAX_TOOL_CALLS);
        assert_eq!(
            budget.max_mutating_tool_calls,
            DEFAULT_MAX_MUTATING_TOOL_CALLS
        );
    }
}
