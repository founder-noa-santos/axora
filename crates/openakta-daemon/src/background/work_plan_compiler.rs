use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use openakta_agents::{
    CoordinatorTaskQueue, DecomposedMission, Dependency, DependencyType, ExecutionMode,
    ParallelGroupIdentifier, Priority, Task, TaskDAG, TaskType,
};
use openakta_api_client::{DependencyEdgeView, ReadModelResponse, WorkItemView};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CompiledWorkPlan {
    pub mission_id: String,
    pub work_item_ids: Vec<Uuid>,
    pub mission: DecomposedMission,
}

pub fn compile_work_plan(
    read_model: &ReadModelResponse,
    selected_work_item_ids: &[Uuid],
    selected_cycle_id: Option<Uuid>,
) -> Result<CompiledWorkPlan> {
    let selected_items = select_items(read_model, selected_work_item_ids, selected_cycle_id);
    if selected_items.is_empty() {
        return Err(anyhow!("no work items matched execution selection"));
    }

    let mut mission = DecomposedMission::new(&format!(
        "Work plan for workspace {}",
        read_model.workspace.id
    ));
    let mut index_by_item_id = HashMap::new();

    for (index, item) in selected_items.iter().enumerate() {
        index_by_item_id.insert(item.id, index);
        mission.tasks.push(Task {
            id: item.id.to_string(),
            description: render_task_description(item),
            priority: map_priority(item.priority),
            status: openakta_agents::TaskStatus::Pending,
            assigned_to: None,
            parent_task: item.parent_id.map(|value| value.to_string()),
            task_type: map_task_type(&item.execution_profile),
        });
    }

    mission.dependency_graph = TaskDAG {
        nodes: (0..mission.tasks.len()).collect(),
        edges: Vec::new(),
    };
    mission.dependencies = compile_dependencies(&read_model.dependencies, &index_by_item_id);
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
    mission.critical_path = parallelizer.calculate_critical_path(&mission.dependency_graph, &durations)?;
    mission.estimated_duration = Duration::from_secs(mission.critical_path.len() as u64 * 60);
    mission.execution_mode = ExecutionMode::Parallel;

    let mut queue = CoordinatorTaskQueue::new();
    queue.load_tasks(&mission)?;

    let mission_id = mission.mission_id.clone();
    let work_item_ids = selected_items.iter().map(|item| item.id).collect();
    Ok(CompiledWorkPlan {
        mission_id,
        work_item_ids,
        mission,
    })
}

fn select_items<'a>(
    read_model: &'a ReadModelResponse,
    selected_work_item_ids: &[Uuid],
    selected_cycle_id: Option<Uuid>,
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

fn render_task_description(item: &WorkItemView) -> String {
    match item.description_md.as_deref() {
        Some(description) if !description.is_empty() => format!("{}\n\n{}", item.title, description),
        _ => item.title.clone(),
    }
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
