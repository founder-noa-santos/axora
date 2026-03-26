use std::collections::{HashMap, HashSet};
use std::time::Duration;

use anyhow::{anyhow, Result};
use openakta_agents::{
    CoordinatorTaskQueue, DecomposedMission, Dependency, DependencyType, ExecutionMode,
    ParallelGroupIdentifier, Priority, Task, TaskDAG, TaskType,
};
use openakta_api_client::{
    ClarificationItemView, ClosureGateView, DependencyEdgeView, ReadModelResponse, RequirementView,
    WorkItemView,
};
use openakta_proto::work::v1::ExecutionTaskOutline;
use serde_json::Value;
use uuid::Uuid;

use crate::background::execution_card_json::execution_compile_task_outlines;

const ROLE_ARCHITECTURE_STEWARD: &str = "architecture_steward";
const ROLE_REVIEW_STEWARD: &str = "review_steward";
const ROLE_VERIFICATION_STEWARD: &str = "verification_steward";
const ROLE_RELIABILITY_STEWARD: &str = "reliability_steward";
const ROLE_KNOWLEDGE_STEWARD: &str = "knowledge_steward";
#[cfg(test)]
const ROLE_PLANNING_STEWARD: &str = "planning_steward";
#[cfg(test)]
const ROLE_IMPLEMENTATION_STEWARD: &str = "implementation_steward";

#[derive(Debug, Clone)]
pub struct CompiledWorkPlan {
    pub mission_id: String,
    pub work_item_ids: Vec<Uuid>,
    pub mission: DecomposedMission,
    pub contract: MissionOperatingContract,
}

#[derive(Debug, Clone)]
pub struct MissionOperatingContract {
    pub story_id: Option<Uuid>,
    pub prepared_story_id: Option<Uuid>,
    pub profile_name: String,
    pub claimed_requirement_ids: Vec<Uuid>,
    pub review_required: bool,
    pub verification_required: bool,
    pub reliability_required: bool,
    pub documentation_required: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MissionProfile {
    FastIterate,
    Balanced,
    HighAssurance,
    CriticalChange,
}

impl MissionProfile {
    fn name(self) -> &'static str {
        match self {
            Self::FastIterate => "Fast Iterate",
            Self::Balanced => "Balanced",
            Self::HighAssurance => "High Assurance",
            Self::CriticalChange => "Critical Change",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "Fast Iterate" => Some(Self::FastIterate),
            "Balanced" => Some(Self::Balanced),
            "High Assurance" => Some(Self::HighAssurance),
            "Critical Change" => Some(Self::CriticalChange),
            _ => None,
        }
    }

    fn requires_prepared_story(self) -> bool {
        !matches!(self, Self::FastIterate)
    }

    fn requires_review(self) -> bool {
        !matches!(self, Self::FastIterate)
    }

    fn requires_verification(self) -> bool {
        !matches!(self, Self::FastIterate)
    }

    fn requires_documentation(self) -> bool {
        matches!(self, Self::HighAssurance | Self::CriticalChange)
    }

    fn requires_reliability(self) -> bool {
        matches!(self, Self::HighAssurance | Self::CriticalChange)
    }

    fn execution_mode(self) -> ExecutionMode {
        match self {
            Self::FastIterate | Self::Balanced => ExecutionMode::Parallel,
            Self::HighAssurance | Self::CriticalChange => ExecutionMode::Sequential,
        }
    }
}

/// Optional overrides from `wm_execution_profile_decisions.policy_json` and nested
/// `execution_card_json.policy_json`. Unset keys fall back to [`MissionProfile`] defaults.
#[derive(Debug, Clone, Default)]
struct PolicyOverrides {
    review_required: Option<bool>,
    verification_required: Option<bool>,
    documentation_required: Option<bool>,
    reliability_required: Option<bool>,
    requires_prepared_story: Option<bool>,
    execution_mode: Option<ExecutionMode>,
}

impl PolicyOverrides {
    fn merge_layer(&mut self, layer: PolicyOverrides) {
        if layer.review_required.is_some() {
            self.review_required = layer.review_required;
        }
        if layer.verification_required.is_some() {
            self.verification_required = layer.verification_required;
        }
        if layer.documentation_required.is_some() {
            self.documentation_required = layer.documentation_required;
        }
        if layer.reliability_required.is_some() {
            self.reliability_required = layer.reliability_required;
        }
        if layer.requires_prepared_story.is_some() {
            self.requires_prepared_story = layer.requires_prepared_story;
        }
        if layer.execution_mode.is_some() {
            self.execution_mode = layer.execution_mode;
        }
    }
}

fn parse_policy_overrides(value: &Value) -> PolicyOverrides {
    let Some(obj) = value.as_object() else {
        return PolicyOverrides::default();
    };
    let mut o = PolicyOverrides::default();
    o.review_required = bool_field(obj, "review_required");
    o.verification_required = bool_field(obj, "verification_required");
    o.documentation_required = bool_field(obj, "documentation_required");
    o.reliability_required = bool_field(obj, "reliability_required");
    o.requires_prepared_story = bool_field(obj, "requires_prepared_story")
        .or_else(|| bool_field(obj, "prepared_story_required"));
    o.execution_mode = obj
        .get("execution_mode")
        .and_then(|v| v.as_str())
        .and_then(parse_execution_mode);
    o
}

fn bool_field(obj: &serde_json::Map<String, Value>, key: &str) -> Option<bool> {
    obj.get(key).and_then(|v| v.as_bool())
}

fn parse_execution_mode(raw: &str) -> Option<ExecutionMode> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "parallel" => Some(ExecutionMode::Parallel),
        "sequential" => Some(ExecutionMode::Sequential),
        _ => None,
    }
}

/// If present, overrides the profile name from the decision / preparation before boolean overrides apply.
fn extract_profile_name_override(value: &Value) -> Option<String> {
    let obj = value.as_object()?;
    obj.get("profile_name")
        .or_else(|| obj.get("primary_execution_profile"))
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn merge_policy_override_layers(layers: &[PolicyOverrides]) -> PolicyOverrides {
    let mut merged = PolicyOverrides::default();
    for layer in layers {
        merged.merge_layer(layer.clone());
    }
    merged
}

#[derive(Debug, Clone)]
struct EffectiveMissionPolicy {
    profile: MissionProfile,
    /// Display name for contracts; matches [`MissionProfile::name`] when the profile is known.
    profile_display_name: String,
    review_required: bool,
    verification_required: bool,
    documentation_required: bool,
    reliability_required: bool,
    requires_prepared_story: bool,
    execution_mode: ExecutionMode,
}

impl EffectiveMissionPolicy {
    fn from_profile_and_overrides(
        base: MissionProfile,
        merged: PolicyOverrides,
        profile_display_name: String,
    ) -> Self {
        Self {
            profile: base,
            profile_display_name,
            review_required: merged
                .review_required
                .unwrap_or_else(|| base.requires_review()),
            verification_required: merged
                .verification_required
                .unwrap_or_else(|| base.requires_verification()),
            documentation_required: merged
                .documentation_required
                .unwrap_or_else(|| base.requires_documentation()),
            reliability_required: merged
                .reliability_required
                .unwrap_or_else(|| base.requires_reliability()),
            requires_prepared_story: merged
                .requires_prepared_story
                .unwrap_or_else(|| base.requires_prepared_story()),
            execution_mode: merged
                .execution_mode
                .unwrap_or_else(|| base.execution_mode()),
        }
    }

    fn requires_prepared_story(&self) -> bool {
        self.requires_prepared_story
    }

    fn review_required(&self) -> bool {
        self.review_required
    }

    fn verification_required(&self) -> bool {
        self.verification_required
    }

    fn reliability_required(&self) -> bool {
        self.reliability_required
    }

    fn documentation_required(&self) -> bool {
        self.documentation_required
    }
}

/// Resolves profile name from decision row, optional `policy_json` / `execution_card_json.policy_json`
/// (`profile_name`, `primary_execution_profile`), then applies merged policy layers.
fn resolve_effective_mission_policy(
    preparation: Option<&openakta_api_client::StoryPreparationView>,
    decision: Option<&openakta_api_client::ExecutionProfileDecisionView>,
) -> EffectiveMissionPolicy {
    let mut profile_name_str = decision
        .map(|d| d.profile_name.clone())
        .or_else(|| preparation.map(|p| p.primary_execution_profile.clone()))
        .unwrap_or_else(|| MissionProfile::FastIterate.name().to_string());

    let mut policy_layers: Vec<Value> = Vec::new();
    if let Some(d) = decision {
        policy_layers.push(d.policy_json.clone());
    }
    if let Some(p) = preparation {
        if let Some(exec) = p.execution_card_json.as_ref() {
            if let Some(pol) = exec.get("policy_json") {
                policy_layers.push(pol.clone());
            }
        }
    }

    for layer in &policy_layers {
        if let Some(name) = extract_profile_name_override(layer) {
            profile_name_str = name;
        }
    }

    let base_profile =
        MissionProfile::from_name(&profile_name_str).unwrap_or(MissionProfile::Balanced);
    let display_name = if MissionProfile::from_name(&profile_name_str).is_some() {
        base_profile.name().to_string()
    } else {
        profile_name_str.clone()
    };

    let parsed_layers: Vec<PolicyOverrides> =
        policy_layers.iter().map(parse_policy_overrides).collect();
    let merged = merge_policy_override_layers(&parsed_layers);
    EffectiveMissionPolicy::from_profile_and_overrides(base_profile, merged, display_name)
}

#[derive(Debug, Clone)]
struct MissionCompilationContext {
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    effective_policy: EffectiveMissionPolicy,
    mission_label: String,
    readiness_blockers: Vec<String>,
    claimed_requirement_ids: Vec<Uuid>,
    handoff_contract_count: usize,
    documentation_gate_present: bool,
    reliability_gate_present: bool,
    /// Parsed `execution_card_json.tasks` (valid UUID `work_item_id` only); drives order and overrides.
    execution_task_outlines: Vec<ExecutionTaskOutline>,
}

pub fn compile_work_plan(
    read_model: &ReadModelResponse,
    selected_work_item_ids: &[Uuid],
    selected_cycle_id: Option<Uuid>,
    selected_prepared_story_id: Option<Uuid>,
    raw_execution_allowed: bool,
) -> Result<CompiledWorkPlan> {
    let selected_items = select_items(
        read_model,
        selected_work_item_ids,
        selected_cycle_id,
        selected_prepared_story_id,
    );
    let context = build_context(read_model, &selected_items, selected_prepared_story_id)?;
    enforce_mol_raw_execution_policy(&context, raw_execution_allowed)?;
    let ordered_items =
        order_items_by_execution_card(&selected_items, &context.execution_task_outlines);
    let outline_by_item: HashMap<Uuid, &ExecutionTaskOutline> = context
        .execution_task_outlines
        .iter()
        .filter_map(|o| {
            Uuid::parse_str(o.work_item_id.as_str())
                .ok()
                .map(|id| (id, o))
        })
        .collect();
    if ordered_items.is_empty()
        && !(context.effective_policy.verification_required()
            || context.effective_policy.review_required()
            || context.documentation_required()
            || context.reliability_required())
    {
        return Err(anyhow!("no work items matched execution selection"));
    }

    enforce_readiness(read_model, &context, &ordered_items)?;

    let mut mission = DecomposedMission::new(&context.mission_label);
    let mut index_by_item_id = HashMap::new();

    for (index, item) in ordered_items.iter().enumerate() {
        index_by_item_id.insert(item.id, index);
        let outline = outline_by_item.get(&item.id).copied();
        mission.tasks.push(Task {
            id: item.id.to_string(),
            description: render_task_description(item, outline),
            priority: map_priority(item.priority),
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: item.owner_persona_id.clone(),
            parent_task: parent_task_id(item, outline),
            task_type: map_task_type(effective_execution_profile(item, outline)),
        });
    }

    mission.dependency_graph = TaskDAG {
        nodes: (0..mission.tasks.len()).collect(),
        edges: Vec::new(),
    };
    mission.dependencies = compile_dependencies(&read_model.dependencies, &index_by_item_id);
    append_synthetic_phase_tasks(read_model, &context, &ordered_items, &mut mission);
    mission.dependency_graph.nodes = (0..mission.tasks.len()).collect();
    mission.dependency_graph.edges = mission
        .dependencies
        .iter()
        .map(|dependency| (dependency.from, dependency.to))
        .collect();

    let parallelizer = ParallelGroupIdentifier::new(10);
    let groups = parallelizer.identify_groups(&mission.dependency_graph)?;
    let durations = mission
        .dependency_graph
        .nodes
        .iter()
        .map(|node| (*node, Duration::from_secs(60)))
        .collect::<HashMap<_, _>>();
    mission.parallel_group_details = groups.clone();
    mission.parallel_groups = groups.iter().map(|group| group.task_ids.clone()).collect();
    mission.critical_path =
        parallelizer.calculate_critical_path(&mission.dependency_graph, &durations)?;
    mission.estimated_duration = Duration::from_secs(mission.critical_path.len() as u64 * 60);
    mission.execution_mode = context.effective_policy.execution_mode.clone();

    let mut queue = CoordinatorTaskQueue::new();
    queue.load_tasks(&mission)?;

    let mission_id = mission.mission_id.clone();
    let work_item_ids = ordered_items.iter().map(|item| item.id).collect();
    Ok(CompiledWorkPlan {
        mission_id,
        work_item_ids,
        mission,
        contract: MissionOperatingContract {
            story_id: context.story_id,
            prepared_story_id: context.prepared_story_id,
            profile_name: context.effective_policy.profile_display_name.clone(),
            claimed_requirement_ids: context.claimed_requirement_ids.clone(),
            review_required: context.effective_policy.review_required(),
            verification_required: context.effective_policy.verification_required(),
            reliability_required: context.reliability_required(),
            documentation_required: context.documentation_required(),
        },
    })
}

fn select_items<'a>(
    read_model: &'a ReadModelResponse,
    selected_work_item_ids: &[Uuid],
    selected_cycle_id: Option<Uuid>,
    selected_prepared_story_id: Option<Uuid>,
) -> Vec<&'a WorkItemView> {
    let mut items = read_model
        .work_items
        .iter()
        .filter(|item| item.tracker_state != "done")
        .filter(|item| match selected_cycle_id {
            Some(cycle_id) => item.cycle_id == Some(cycle_id),
            None => true,
        })
        .filter(|item| {
            selected_work_item_ids.is_empty() || selected_work_item_ids.contains(&item.id)
        })
        .filter(|item| match selected_prepared_story_id {
            Some(prepared_story_id) => item.prepared_story_id == Some(prepared_story_id),
            None => true,
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        left.wave_rank
            .unwrap_or_default()
            .cmp(&right.wave_rank.unwrap_or_default())
            .then(right.priority.cmp(&left.priority))
            .then(left.created_at.cmp(&right.created_at))
    });
    items
}

/// Order implementation tasks by `execution_card_json.tasks` when present; append unmatched items
/// in their original order (wave_rank sort from [`select_items`]).
fn order_items_by_execution_card<'a>(
    items: &[&'a WorkItemView],
    outlines: &[ExecutionTaskOutline],
) -> Vec<&'a WorkItemView> {
    if outlines.is_empty() {
        return items.to_vec();
    }
    let by_id: HashMap<Uuid, &WorkItemView> = items.iter().copied().map(|i| (i.id, i)).collect();
    let mut ordered = Vec::new();
    let mut seen = HashSet::new();
    for o in outlines {
        let Ok(id) = Uuid::parse_str(o.work_item_id.as_str()) else {
            continue;
        };
        if let Some(item) = by_id.get(&id) {
            ordered.push(*item);
            seen.insert(id);
        }
    }
    for item in items {
        if !seen.contains(&item.id) {
            ordered.push(*item);
        }
    }
    ordered
}

/// When a prepared story is in scope, **Fast Iterate** skips MOL gates; block that unless
/// `MOL_RAW_EXECUTION_ALLOWED` is enabled (see [`openakta_api_client::MolFeatureFlags`]).
fn enforce_mol_raw_execution_policy(
    context: &MissionCompilationContext,
    raw_execution_allowed: bool,
) -> Result<()> {
    if raw_execution_allowed {
        return Ok(());
    }
    if context.prepared_story_id.is_none() {
        return Ok(());
    }
    if context.effective_policy.profile != MissionProfile::FastIterate {
        return Ok(());
    }
    Err(anyhow!(
        "Fast Iterate (raw execution) is disabled for MOL prepared stories; set MOL_RAW_EXECUTION_ALLOWED=true, or use a profile other than Fast Iterate"
    ))
}

fn build_context(
    read_model: &ReadModelResponse,
    selected_items: &[&WorkItemView],
    selected_prepared_story_id: Option<Uuid>,
) -> Result<MissionCompilationContext> {
    let prepared_story_id = selected_prepared_story_id.or_else(|| {
        let unique = selected_items
            .iter()
            .filter_map(|item| item.prepared_story_id)
            .collect::<HashSet<_>>();
        if unique.len() == 1 {
            unique.into_iter().next()
        } else {
            None
        }
    });

    let preparation = prepared_story_id.and_then(|id| {
        read_model
            .story_preparations
            .iter()
            .find(|story| story.id == id)
    });
    let story_id = preparation
        .map(|story| story.story_id)
        .or_else(|| selected_items.iter().find_map(|item| item.story_id));

    let decision = prepared_story_id.and_then(|id| {
        read_model
            .execution_profile_decisions
            .iter()
            .find(|d| d.prepared_story_id == Some(id))
    });
    let effective_policy = match prepared_story_id {
        None => EffectiveMissionPolicy::from_profile_and_overrides(
            MissionProfile::FastIterate,
            PolicyOverrides::default(),
            MissionProfile::FastIterate.name().to_string(),
        ),
        Some(_) => resolve_effective_mission_policy(preparation, decision),
    };

    if effective_policy.requires_prepared_story() && prepared_story_id.is_none() {
        return Err(anyhow!(
            "{} execution requires a prepared story",
            effective_policy.profile.name()
        ));
    }

    let requirements = scoped_requirements(read_model, story_id, prepared_story_id);
    let claimed_requirement_ids =
        requirement_ids_for_items(read_model, selected_items, &requirements);
    let handoff_contract_count = read_model
        .handoff_contracts
        .iter()
        .filter(|contract| {
            prepared_story_id.is_some() && contract.prepared_story_id == prepared_story_id
        })
        .count();
    let documentation_gate_present = closure_gate_present(
        &read_model.closure_gates,
        story_id,
        prepared_story_id,
        "documentation",
    );
    let reliability_gate_present = closure_gate_present(
        &read_model.closure_gates,
        story_id,
        prepared_story_id,
        "reliability",
    );

    let execution_task_outlines = preparation
        .map(|p| execution_compile_task_outlines(p.execution_card_json.as_ref()))
        .unwrap_or_default();

    let execution_story_summary = preparation
        .and_then(|p| {
            p.execution_card_json
                .as_ref()
                .and_then(|j| j.get("story_summary").and_then(|x| x.as_str()))
        })
        .map(str::to_string);

    let mission_label = if let Some(preparation) = preparation {
        render_mission_label(
            preparation.mission_card_json.get("story_summary"),
            execution_story_summary.as_deref(),
            preparation.story_id,
        )
    } else if let Some(item) = selected_items.first() {
        format!("Mission execution for {}", item.title)
    } else {
        format!(
            "Mission execution for workspace {}",
            read_model.workspace.id
        )
    };

    let readiness_blockers = preparation
        .and_then(|story| story.readiness_blockers_json.as_ref())
        .map(readiness_blockers_from_json)
        .unwrap_or_default();

    Ok(MissionCompilationContext {
        story_id,
        prepared_story_id,
        effective_policy,
        mission_label,
        readiness_blockers,
        claimed_requirement_ids,
        handoff_contract_count,
        documentation_gate_present,
        reliability_gate_present,
        execution_task_outlines,
    })
}

fn scoped_requirements<'a>(
    read_model: &'a ReadModelResponse,
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
) -> Vec<&'a RequirementView> {
    read_model
        .requirements
        .iter()
        .filter(|requirement| {
            (story_id.is_none() || requirement.story_id == story_id)
                && (prepared_story_id.is_none()
                    || requirement.prepared_story_id == prepared_story_id)
        })
        .collect()
}

fn requirement_ids_for_items(
    read_model: &ReadModelResponse,
    selected_items: &[&WorkItemView],
    requirements: &[&RequirementView],
) -> Vec<Uuid> {
    let item_ids = selected_items
        .iter()
        .map(|item| item.id)
        .collect::<HashSet<_>>();
    let mut ids = read_model
        .requirement_coverage
        .iter()
        .filter(|coverage| item_ids.contains(&coverage.work_item_id))
        .map(|coverage| coverage.requirement_id)
        .collect::<HashSet<_>>();
    if ids.is_empty() && !requirements.is_empty() {
        ids.extend(requirements.iter().map(|requirement| requirement.id));
    }
    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.sort();
    ids
}

fn closure_gate_present(
    gates: &[ClosureGateView],
    story_id: Option<Uuid>,
    prepared_story_id: Option<Uuid>,
    gate_type: &str,
) -> bool {
    gates.iter().any(|gate| {
        gate.gate_type == gate_type
            && (story_id.is_none() || gate.story_id == story_id)
            && (prepared_story_id.is_none() || gate.prepared_story_id == prepared_story_id)
    })
}

fn enforce_readiness(
    read_model: &ReadModelResponse,
    context: &MissionCompilationContext,
    selected_items: &[&WorkItemView],
) -> Result<()> {
    if !context.effective_policy.requires_prepared_story() {
        return Ok(());
    }

    let prepared_story_id = context
        .prepared_story_id
        .ok_or_else(|| anyhow!("prepared story is required"))?;
    let preparation = read_model
        .story_preparations
        .iter()
        .find(|story| story.id == prepared_story_id)
        .ok_or_else(|| anyhow!("prepared story {} not found", prepared_story_id))?;

    if preparation.status != "ready" && preparation.status != "executing" {
        return Err(anyhow!(
            "prepared story {} is not ready for execution (status={})",
            prepared_story_id,
            preparation.status
        ));
    }

    if !context.readiness_blockers.is_empty() {
        return Err(anyhow!(
            "prepared story {} has readiness blockers: {}",
            prepared_story_id,
            context.readiness_blockers.join(", ")
        ));
    }

    let active_requirements =
        scoped_requirements(read_model, context.story_id, context.prepared_story_id)
            .into_iter()
            .filter(|requirement| {
                matches!(
                    requirement.status.as_str(),
                    "active" | "draft" | "implemented_claimed" | "verification_pending"
                )
            })
            .collect::<Vec<_>>();

    let coverage_by_requirement = read_model
        .requirement_coverage
        .iter()
        .filter(|coverage| {
            context.prepared_story_id.is_none()
                || selected_items.iter().any(|item| {
                    item.id == coverage.work_item_id
                        || item.prepared_story_id == context.prepared_story_id
                })
        })
        .map(|coverage| coverage.requirement_id)
        .collect::<HashSet<_>>();

    let uncovered = active_requirements
        .iter()
        .filter(|requirement| !coverage_by_requirement.contains(&requirement.id))
        .map(|requirement| requirement.title.clone())
        .collect::<Vec<_>>();
    if !uncovered.is_empty() {
        return Err(anyhow!(
            "prepared story {} has uncovered requirements: {}",
            prepared_story_id,
            uncovered.join(", ")
        ));
    }

    let ambiguous = active_requirements
        .iter()
        .filter(|requirement| {
            requirement.ambiguity_state == "needs_clarification"
                || requirement.status == "clarification_needed"
        })
        .map(|requirement| requirement.title.clone())
        .collect::<Vec<_>>();
    if !ambiguous.is_empty() {
        return Err(anyhow!(
            "prepared story {} still has ambiguous requirements: {}",
            prepared_story_id,
            ambiguous.join(", ")
        ));
    }

    let scope_requirement_ids: HashSet<Uuid> = active_requirements
        .iter()
        .map(|requirement| requirement.id)
        .collect();
    let selected_work_ids: HashSet<Uuid> = selected_items.iter().map(|item| item.id).collect();
    let unresolved_clarifications = read_model
        .clarifications
        .iter()
        .filter(|item| !clarification_status_resolved(&item.status))
        .filter(|item| {
            clarification_in_execution_scope(
                item,
                context.story_id,
                &scope_requirement_ids,
                &selected_work_ids,
            )
        })
        .map(|item| {
            let label = item.prompt_text.chars().take(120).collect::<String>();
            format!("{} ({})", label, item.id)
        })
        .collect::<Vec<_>>();
    if !unresolved_clarifications.is_empty() {
        return Err(anyhow!(
            "prepared story {} has unresolved clarification items (resolve via queue or mark answered): {}",
            prepared_story_id,
            unresolved_clarifications.join(", ")
        ));
    }

    Ok(())
}

/// Terminal statuses that do not block mission compilation (`answered` after local resolve; see
/// `work_mirror::WorkMirror::resolve_clarifications`).
fn clarification_status_resolved(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "answered" | "waived" | "cancelled" | "resolved"
    )
}

/// Clarifications scoped to this execution: same story, linked requirement in this prepared story,
/// or tied to a selected work item (AB9: mirror updates `status` on these rows in the read model).
fn clarification_in_execution_scope(
    item: &ClarificationItemView,
    story_id: Option<Uuid>,
    scope_requirement_ids: &HashSet<Uuid>,
    selected_work_ids: &HashSet<Uuid>,
) -> bool {
    if let (Some(story), Some(item_story)) = (story_id, item.story_id) {
        if story == item_story {
            return true;
        }
    }
    if let Some(rid) = item.requirement_id {
        if scope_requirement_ids.contains(&rid) {
            return true;
        }
    }
    if let Some(wid) = item.work_item_id {
        if selected_work_ids.contains(&wid) {
            return true;
        }
    }
    false
}

fn append_synthetic_phase_tasks(
    read_model: &ReadModelResponse,
    context: &MissionCompilationContext,
    selected_items: &[&WorkItemView],
    mission: &mut DecomposedMission,
) {
    let implementation_task_indexes = (0..selected_items.len()).collect::<Vec<_>>();

    if context.handoff_contract_count > 0 {
        let task_id = mission.tasks.len();
        mission.tasks.push(Task {
            id: format!(
                "handoff-{}",
                context
                    .prepared_story_id
                    .or(context.story_id)
                    .unwrap_or_else(Uuid::new_v4)
            ),
            description: format!(
                "Resolve and accept {} handoff contract(s) before verification.\n\nOwner persona: {}",
                context.handoff_contract_count,
                persona_id(read_model.workspace.id, ROLE_ARCHITECTURE_STEWARD)
            ),
            priority: Priority::High,
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: Some(persona_id(read_model.workspace.id, ROLE_ARCHITECTURE_STEWARD)),
            parent_task: None,
            task_type: TaskType::Review,
        });
        add_phase_dependencies(mission, task_id, &implementation_task_indexes);
    }

    let review_task_index = if context.effective_policy.review_required() {
        let task_id = mission.tasks.len();
        mission.tasks.push(Task {
            id: format!(
                "review-{}",
                context
                    .prepared_story_id
                    .or(context.story_id)
                    .unwrap_or_else(Uuid::new_v4)
            ),
            description: format!(
                "Review Steward gate.\n\nProfile: {}\nRequirements under closure: {}\nOwner persona: {}",
                context.effective_policy.profile.name(),
                context.claimed_requirement_ids.len(),
                persona_id(read_model.workspace.id, ROLE_REVIEW_STEWARD)
            ),
            priority: Priority::High,
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: Some(persona_id(read_model.workspace.id, ROLE_REVIEW_STEWARD)),
            parent_task: None,
            task_type: TaskType::Review,
        });
        add_phase_dependencies(mission, task_id, &implementation_task_indexes);
        Some(task_id)
    } else {
        None
    };

    let documentation_task_index = if context.documentation_required() {
        let task_id = mission.tasks.len();
        mission.tasks.push(Task {
            id: format!(
                "docs-{}",
                context
                    .prepared_story_id
                    .or(context.story_id)
                    .unwrap_or_else(Uuid::new_v4)
            ),
            description: format!(
                "Documentation alignment gate.\n\nDocumentation gate present: {}\nOwner persona: {}",
                context.documentation_gate_present,
                persona_id(read_model.workspace.id, ROLE_KNOWLEDGE_STEWARD)
            ),
            priority: Priority::High,
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: Some(persona_id(read_model.workspace.id, ROLE_KNOWLEDGE_STEWARD)),
            parent_task: None,
            task_type: TaskType::Review,
        });
        add_phase_dependencies(mission, task_id, &implementation_task_indexes);
        Some(task_id)
    } else {
        None
    };

    let reliability_task_index = if context.reliability_required() {
        let task_id = mission.tasks.len();
        mission.tasks.push(Task {
            id: format!(
                "reliability-{}",
                context
                    .prepared_story_id
                    .or(context.story_id)
                    .unwrap_or_else(Uuid::new_v4)
            ),
            description: format!(
                "Reliability gate.\n\nReliability gate present: {}\nOwner persona: {}",
                context.reliability_gate_present,
                persona_id(read_model.workspace.id, ROLE_RELIABILITY_STEWARD)
            ),
            priority: Priority::Critical,
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: Some(persona_id(
                read_model.workspace.id,
                ROLE_RELIABILITY_STEWARD,
            )),
            parent_task: None,
            task_type: TaskType::Review,
        });
        add_phase_dependencies(mission, task_id, &implementation_task_indexes);
        Some(task_id)
    } else {
        None
    };

    if context.effective_policy.verification_required() {
        let task_id = mission.tasks.len();
        mission.tasks.push(Task {
            id: format!(
                "verification-{}",
                context
                    .prepared_story_id
                    .or(context.story_id)
                    .unwrap_or_else(Uuid::new_v4)
            ),
            description: format!(
                "Verification Steward gate.\n\nProfile: {}\nRequirement claims under test: {}\nOwner persona: {}",
                context.effective_policy.profile.name(),
                context.claimed_requirement_ids.len(),
                persona_id(read_model.workspace.id, ROLE_VERIFICATION_STEWARD)
            ),
            priority: Priority::Critical,
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: Some(persona_id(read_model.workspace.id, ROLE_VERIFICATION_STEWARD)),
            parent_task: None,
            task_type: TaskType::Review,
        });
        add_phase_dependencies(mission, task_id, &implementation_task_indexes);
        if let Some(review_task_index) = review_task_index {
            mission.dependencies.push(Dependency::new(
                task_id,
                review_task_index,
                DependencyType::Hard,
            ));
        }
        if let Some(documentation_task_index) = documentation_task_index {
            mission.dependencies.push(Dependency::new(
                task_id,
                documentation_task_index,
                DependencyType::Hard,
            ));
        }
        if let Some(reliability_task_index) = reliability_task_index {
            mission.dependencies.push(Dependency::new(
                task_id,
                reliability_task_index,
                DependencyType::Hard,
            ));
        }
    }
}

fn add_phase_dependencies(mission: &mut DecomposedMission, task_id: usize, dependencies: &[usize]) {
    for dependency in dependencies {
        mission
            .dependencies
            .push(Dependency::new(task_id, *dependency, DependencyType::Hard));
    }
}

fn compile_dependencies(
    edges: &[DependencyEdgeView],
    index_by_item_id: &HashMap<Uuid, usize>,
) -> Vec<Dependency> {
    let mut compiled = Vec::new();
    for edge in edges {
        let Some(&from) = index_by_item_id.get(&edge.from_item_id) else {
            continue;
        };
        let Some(&to) = index_by_item_id.get(&edge.to_item_id) else {
            continue;
        };
        compiled.push(Dependency::new(
            from,
            to,
            match edge.strength.as_str() {
                "soft" => DependencyType::Soft,
                "data" => DependencyType::Data,
                _ => DependencyType::Hard,
            },
        ));
    }
    compiled
}

fn render_task_description(item: &WorkItemView, outline: Option<&ExecutionTaskOutline>) -> String {
    let title = outline
        .map(|o| o.title.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(item.title.as_str());
    let mut sections = vec![title.to_string()];
    if let Some(o) = outline {
        if o.wave_rank != 0 || !o.wave_label.is_empty() {
            let mut wave = format!("Wave rank: {}", o.wave_rank);
            if !o.wave_label.is_empty() {
                wave.push_str(&format!(" ({})", o.wave_label));
            }
            sections.push(wave);
        }
    }
    if let Some(description) = item.description_md.as_deref() {
        if !description.is_empty() {
            sections.push(description.to_string());
        }
    }
    if let Some(requirement_slice_json) = &item.requirement_slice_json {
        sections.push(format!("Requirement slice:\n{}", requirement_slice_json));
    }
    if let Some(owner_persona_id) = &item.owner_persona_id {
        sections.push(format!("Owner persona: {owner_persona_id}"));
    }
    sections.join("\n\n")
}

fn parent_task_id(item: &WorkItemView, outline: Option<&ExecutionTaskOutline>) -> Option<String> {
    outline
        .and_then(|o| {
            if o.parent_work_item_id.is_empty() {
                return None;
            }
            Uuid::parse_str(o.parent_work_item_id.as_str()).ok()
        })
        .map(|id| id.to_string())
        .or_else(|| item.parent_id.map(|id| id.to_string()))
}

fn effective_execution_profile<'a>(
    item: &'a WorkItemView,
    outline: Option<&'a ExecutionTaskOutline>,
) -> &'a str {
    outline
        .map(|o| o.execution_profile.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(item.execution_profile.as_str())
}

fn render_mission_label(
    mission_summary: Option<&Value>,
    execution_story_summary: Option<&str>,
    story_id: Uuid,
) -> String {
    execution_story_summary
        .filter(|s| !s.is_empty())
        .map(|value| format!("Mission for {value}"))
        .or_else(|| {
            mission_summary
                .and_then(Value::as_str)
                .map(|value| format!("Mission for {value}"))
        })
        .unwrap_or_else(|| format!("Mission for story {story_id}"))
}

fn readiness_blockers_from_json(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Value::Object(map) if map.is_empty() => Vec::new(),
        Value::Object(map) => map
            .iter()
            .filter_map(|(key, value)| {
                if value.is_null() || value == &Value::Bool(false) {
                    None
                } else if let Some(text) = value.as_str() {
                    Some(format!("{key}: {text}"))
                } else {
                    Some(key.clone())
                }
            })
            .collect(),
        Value::Null => Vec::new(),
        other => vec![other.to_string()],
    }
}

fn persona_id(workspace_id: Uuid, role: &str) -> String {
    format!("{workspace_id}:{role}")
}

fn map_priority(value: i32) -> Priority {
    match value {
        76..=100 => Priority::Critical,
        51..=75 => Priority::High,
        26..=50 => Priority::Normal,
        _ => Priority::Low,
    }
}

fn map_task_type(execution_profile: &str) -> TaskType {
    match execution_profile {
        "code_modification" => TaskType::CodeModification,
        "review" => TaskType::Review,
        "retrieval" => TaskType::Retrieval,
        _ => TaskType::General,
    }
}

impl MissionCompilationContext {
    fn reliability_required(&self) -> bool {
        self.effective_policy.reliability_required() || self.reliability_gate_present
    }

    fn documentation_required(&self) -> bool {
        self.effective_policy.documentation_required() || self.documentation_gate_present
    }
}

#[cfg(test)]
mod tests {
    use super::compile_work_plan;
    use chrono::Utc;
    use openakta_api_client::{
        ClarificationItemView, ClosureClaimView, ClosureGateView, DecisionRecordView,
        DependencyEdgeView, ExecutionProfileDecisionView, HandoffContractView,
        KnowledgeArtifactView, MemoryPromotionEventView, PersonaAssignmentView, PersonaView,
        PlanVersionView, ReadModelResponse, RequirementCoverageView, RequirementEdgeView,
        RequirementView, StoryIntakeView, StoryPreparationView, VerificationFindingView,
        VerificationRunView, WorkItemView, WorkspaceView,
    };
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn balanced_profile_requires_ready_prepared_story() {
        let mut read_model = sample_read_model();
        read_model.story_preparations[0].status = "prepared".to_string();

        let err = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap_err();

        assert!(err.to_string().contains("not ready"));
    }

    #[test]
    fn balanced_profile_adds_review_and_verification_tasks() {
        let read_model = sample_read_model();
        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert_eq!(compiled.contract.profile_name, "Balanced");
        assert!(compiled.contract.review_required);
        assert!(compiled.contract.verification_required);
        assert!(compiled
            .mission
            .tasks
            .iter()
            .any(|task| task.description.contains("Review Steward gate")));
        assert!(compiled
            .mission
            .tasks
            .iter()
            .any(|task| task.description.contains("Verification Steward gate")));
    }

    #[test]
    fn raw_work_items_remain_fast_iterate_compatible() {
        let mut read_model = sample_read_model();
        read_model.work_items[0].prepared_story_id = None;
        read_model.work_items[0].story_id = None;

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            None,
            false,
        )
        .unwrap();

        assert_eq!(compiled.contract.profile_name, "Fast Iterate");
    }

    #[test]
    fn balanced_profile_blocks_unresolved_clarifications_for_story() {
        let mut read_model = sample_read_model();
        let clarification_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let story_id = read_model.story_preparations[0].story_id;
        read_model.clarifications.push(ClarificationItemView {
            id: clarification_id,
            workspace_id: read_model.workspace.id,
            cycle_id: None,
            work_item_id: None,
            story_id: Some(story_id),
            requirement_id: None,
            mission_id: None,
            task_id: None,
            question_kind: "free_text".to_string(),
            prompt_text: "Which API version?".to_string(),
            schema_json: None,
            options_json: None,
            dedupe_fingerprint: "ab11-block-test".to_string(),
            status: "open".to_string(),
            raised_by_agent_id: None,
            created_at: Utc::now(),
            answered_at: None,
        });
        let err = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("unresolved clarification"),
            "got {}",
            err
        );
    }

    #[test]
    fn balanced_profile_allows_answered_clarifications() {
        let mut read_model = sample_read_model();
        let clarification_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let story_id = read_model.story_preparations[0].story_id;
        let now = Utc::now();
        read_model.clarifications.push(ClarificationItemView {
            id: clarification_id,
            workspace_id: read_model.workspace.id,
            cycle_id: None,
            work_item_id: None,
            story_id: Some(story_id),
            requirement_id: None,
            mission_id: None,
            task_id: None,
            question_kind: "free_text".to_string(),
            prompt_text: "Which API version?".to_string(),
            schema_json: None,
            options_json: None,
            dedupe_fingerprint: "ab11-allow-test".to_string(),
            status: "answered".to_string(),
            raised_by_agent_id: None,
            created_at: now,
            answered_at: Some(now),
        });
        compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .expect("answered clarification should not block readiness");
    }

    #[test]
    fn balanced_profile_ignores_unresolved_clarifications_for_other_story() {
        let mut read_model = sample_read_model();
        let other_story = Uuid::parse_str("99999999-9999-9999-9999-999999999999").unwrap();
        read_model.clarifications.push(ClarificationItemView {
            id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            workspace_id: read_model.workspace.id,
            cycle_id: None,
            work_item_id: None,
            story_id: Some(other_story),
            requirement_id: None,
            mission_id: None,
            task_id: None,
            question_kind: "free_text".to_string(),
            prompt_text: "Unrelated".to_string(),
            schema_json: None,
            options_json: None,
            dedupe_fingerprint: "ab11-scope-test".to_string(),
            status: "open".to_string(),
            raised_by_agent_id: None,
            created_at: Utc::now(),
            answered_at: None,
        });
        compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .expect("clarifications for another story must not block");
    }

    #[test]
    fn policy_json_overrides_review_without_changing_profile_name() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].policy_json = json!({"review_required": false});

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert_eq!(compiled.contract.profile_name, "Balanced");
        assert!(!compiled.contract.review_required);
        assert!(compiled.contract.verification_required);
        assert!(!compiled
            .mission
            .tasks
            .iter()
            .any(|task| task.description.contains("Review Steward gate")));
        assert!(compiled
            .mission
            .tasks
            .iter()
            .any(|task| task.description.contains("Verification Steward gate")));
    }

    #[test]
    fn policy_json_profile_name_overrides_decision_row() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].profile_name = "Balanced".to_string();
        read_model.execution_profile_decisions[0].policy_json =
            json!({"profile_name": "Fast Iterate"});

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert_eq!(compiled.contract.profile_name, "Fast Iterate");
        assert!(!compiled.contract.review_required);
        assert!(!compiled.contract.verification_required);
        assert!(!compiled
            .mission
            .tasks
            .iter()
            .any(|task| task.description.contains("Review Steward gate")));
    }

    #[test]
    fn mol_prepared_story_fast_iterate_rejected_when_raw_execution_disabled() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].policy_json =
            json!({"profile_name": "Fast Iterate"});

        let err = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            false,
        )
        .unwrap_err();

        assert!(
            err.to_string()
                .contains("Fast Iterate (raw execution) is disabled"),
            "got {}",
            err
        );
    }

    #[test]
    fn mol_prepared_story_fast_iterate_allowed_when_raw_execution_enabled() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].policy_json =
            json!({"profile_name": "Fast Iterate"});

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert_eq!(compiled.contract.profile_name, "Fast Iterate");
    }

    #[test]
    fn policy_json_execution_mode_sequential_on_balanced() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].policy_json =
            json!({"execution_mode": "sequential"});

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert_eq!(
            compiled.mission.execution_mode,
            openakta_agents::ExecutionMode::Sequential
        );
    }

    #[test]
    fn execution_card_policy_json_overrides_decision_policy_json() {
        let mut read_model = sample_read_model();
        read_model.execution_profile_decisions[0].policy_json =
            json!({"review_required": true, "verification_required": false});
        read_model.story_preparations[0].execution_card_json = Some(json!({
            "story_summary": "Balanced story",
            "policy_json": {"review_required": false, "verification_required": true}
        }));

        let compiled = compile_work_plan(
            &read_model,
            &[read_model.work_items[0].id],
            None,
            Some(read_model.story_preparations[0].id),
            true,
        )
        .unwrap();

        assert!(!compiled.contract.review_required);
        assert!(compiled.contract.verification_required);
    }

    #[test]
    fn execution_card_json_tasks_order_and_overrides() {
        let mut read_model = sample_read_model();
        let prepared_story_id = read_model.story_preparations[0].id;
        let story_id = read_model.story_preparations[0].story_id;
        let workspace_id = read_model.workspace.id;
        let requirement_id = read_model.requirements[0].id;
        let work_item_a = read_model.work_items[0].id;
        let work_item_b = Uuid::parse_str("ffffffff-ffff-ffff-ffff-ffffffffffff").unwrap();
        let now = read_model.work_items[0].created_at;

        read_model.work_items.push(WorkItemView {
            id: work_item_b,
            workspace_id,
            cycle_id: None,
            parent_id: None,
            item_type: "task".to_string(),
            execution_profile: "code_modification".to_string(),
            title: "Second item".to_string(),
            description_md: None,
            tracker_state: "in_progress".to_string(),
            run_state: "idle".to_string(),
            priority: 60,
            assignee_user_id: None,
            external_master: false,
            wave_rank: Some(2),
            wave_label: Some("B".to_string()),
            story_id: Some(story_id),
            prepared_story_id: Some(prepared_story_id),
            owner_persona_id: None,
            requirement_slice_json: Some(json!({"requirements": [requirement_id]})),
            handoff_contract_state: None,
            claim_state: None,
            updated_at: now,
            created_at: now,
        });
        read_model
            .requirement_coverage
            .push(RequirementCoverageView {
                id: Uuid::new_v4(),
                workspace_id,
                requirement_id,
                work_item_id: work_item_b,
                coverage_kind: "implementation".to_string(),
                status: "linked".to_string(),
                created_at: now,
                updated_at: now,
            });

        read_model.story_preparations[0].execution_card_json = Some(json!({
            "story_summary": "From execution card",
            "tasks": [
                {
                    "work_item_id": work_item_b.to_string(),
                    "title": "Card title B",
                    "execution_profile": "review",
                    "wave_rank": 2,
                    "wave_label": "B"
                },
                {
                    "work_item_id": work_item_a.to_string(),
                    "title": "Card title A",
                    "execution_profile": "code_modification",
                    "wave_rank": 1,
                    "wave_label": "A"
                }
            ]
        }));

        let compiled = compile_work_plan(
            &read_model,
            &[work_item_a, work_item_b],
            None,
            Some(prepared_story_id),
            true,
        )
        .unwrap();

        assert!(
            compiled
                .mission
                .original_mission
                .contains("From execution card"),
            "mission label should prefer execution_card_json.story_summary when set"
        );
        assert_eq!(compiled.mission.tasks[0].id, work_item_b.to_string());
        assert_eq!(compiled.mission.tasks[1].id, work_item_a.to_string());
        assert!(compiled.mission.tasks[0]
            .description
            .starts_with("Card title B"));
        assert_eq!(
            compiled.mission.tasks[0].task_type,
            openakta_agents::TaskType::Review
        );
    }

    fn sample_read_model() -> ReadModelResponse {
        let workspace_id = Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap();
        let story_id = Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap();
        let prepared_story_id = Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap();
        let work_item_id = Uuid::parse_str("dddddddd-dddd-dddd-dddd-dddddddddddd").unwrap();
        let requirement_id = Uuid::parse_str("eeeeeeee-eeee-eeee-eeee-eeeeeeeeeeee").unwrap();
        let now = Utc::now();

        ReadModelResponse {
            workspace: WorkspaceView {
                id: workspace_id,
                tenant_id: "tenant".to_string(),
                slug: "workspace".to_string(),
                name: "Workspace".to_string(),
                created_by: Uuid::nil(),
                created_at: now,
            },
            cycles: Vec::new(),
            phases: Vec::new(),
            work_items: vec![WorkItemView {
                id: work_item_id,
                workspace_id,
                cycle_id: None,
                parent_id: None,
                item_type: "task".to_string(),
                execution_profile: "code_modification".to_string(),
                title: "Implement feature".to_string(),
                description_md: Some("Ship the code path".to_string()),
                tracker_state: "in_progress".to_string(),
                run_state: "idle".to_string(),
                priority: 60,
                assignee_user_id: None,
                external_master: false,
                wave_rank: Some(1),
                wave_label: Some("A".to_string()),
                story_id: Some(story_id),
                prepared_story_id: Some(prepared_story_id),
                owner_persona_id: Some(format!(
                    "{workspace_id}:{}",
                    super::ROLE_IMPLEMENTATION_STEWARD
                )),
                requirement_slice_json: Some(json!({"requirements": [requirement_id]})),
                handoff_contract_state: Some("ready".to_string()),
                claim_state: Some("pending".to_string()),
                updated_at: now,
                created_at: now,
            }],
            dependencies: Vec::<DependencyEdgeView>::new(),
            clarifications: Vec::<ClarificationItemView>::new(),
            decisions: Vec::<DecisionRecordView>::new(),
            plan_versions: Vec::<PlanVersionView>::new(),
            story_intakes: vec![StoryIntakeView {
                id: story_id,
                workspace_id,
                external_ref: None,
                title: "Story".to_string(),
                raw_request_md: "Build it".to_string(),
                source_kind: "manual".to_string(),
                status: "ready".to_string(),
                urgency: "normal".to_string(),
                priority_band: "p2".to_string(),
                affected_surfaces_json: None,
                created_by: Uuid::nil(),
                created_at: now,
                updated_at: now,
            }],
            story_preparations: vec![StoryPreparationView {
                id: prepared_story_id,
                workspace_id,
                story_id,
                status: "ready".to_string(),
                mission_card_json: json!({"story_summary": "Balanced story"}),
                execution_card_json: None,
                dependency_summary_json: None,
                readiness_blockers_json: Some(json!([])),
                primary_execution_profile: "Balanced".to_string(),
                created_by: Uuid::nil(),
                created_at: now,
                updated_at: now,
                ready_at: Some(now),
            }],
            requirements: vec![RequirementView {
                id: requirement_id,
                workspace_id,
                story_id: Some(story_id),
                prepared_story_id: Some(prepared_story_id),
                plan_version_id: None,
                parent_requirement_id: None,
                title: "Requirement".to_string(),
                statement: "Must be true".to_string(),
                kind: "functional".to_string(),
                criticality: "normal".to_string(),
                source: "story".to_string(),
                ambiguity_state: "clear".to_string(),
                owner_persona_id: Some(format!("{workspace_id}:{}", super::ROLE_PLANNING_STEWARD)),
                status: "active".to_string(),
                created_at: now,
                updated_at: now,
            }],
            requirement_edges: Vec::<RequirementEdgeView>::new(),
            acceptance_checks: Vec::new(),
            requirement_coverage: vec![RequirementCoverageView {
                id: Uuid::new_v4(),
                workspace_id,
                requirement_id,
                work_item_id,
                coverage_kind: "implementation".to_string(),
                status: "linked".to_string(),
                created_at: now,
                updated_at: now,
            }],
            handoff_contracts: vec![HandoffContractView {
                id: Uuid::new_v4(),
                workspace_id,
                prepared_story_id: Some(prepared_story_id),
                from_work_item_id: Some(work_item_id),
                to_work_item_id: None,
                contract_kind: "api".to_string(),
                expected_artifact_json: None,
                acceptance_signal_json: None,
                status: "pending".to_string(),
                created_at: now,
                updated_at: now,
            }],
            execution_profile_decisions: vec![ExecutionProfileDecisionView {
                id: Uuid::new_v4(),
                workspace_id,
                story_id: Some(story_id),
                prepared_story_id: Some(prepared_story_id),
                profile_name: "Balanced".to_string(),
                policy_json: json!({}),
                inferred_from_json: None,
                override_reason_md: None,
                escalation_level: "advisory".to_string(),
                decided_by: "system".to_string(),
                created_at: now,
                updated_at: now,
            }],
            verification_runs: Vec::<VerificationRunView>::new(),
            verification_findings: Vec::<VerificationFindingView>::new(),
            closure_claims: Vec::<ClosureClaimView>::new(),
            closure_gates: vec![
                ClosureGateView {
                    id: Uuid::new_v4(),
                    workspace_id,
                    story_id: Some(story_id),
                    prepared_story_id: Some(prepared_story_id),
                    gate_type: "documentation".to_string(),
                    status: "pending".to_string(),
                    decided_by_persona_id: None,
                    rationale_md: None,
                    created_at: now,
                    updated_at: now,
                },
                ClosureGateView {
                    id: Uuid::new_v4(),
                    workspace_id,
                    story_id: Some(story_id),
                    prepared_story_id: Some(prepared_story_id),
                    gate_type: "reliability".to_string(),
                    status: "pending".to_string(),
                    decided_by_persona_id: None,
                    rationale_md: None,
                    created_at: now,
                    updated_at: now,
                },
            ],
            personas: Vec::<PersonaView>::new(),
            persona_assignments: Vec::<PersonaAssignmentView>::new(),
            knowledge_artifacts: Vec::<KnowledgeArtifactView>::new(),
            memory_promotion_events: Vec::<MemoryPromotionEventView>::new(),
            checkpoint_seq: 1,
        }
    }
}
