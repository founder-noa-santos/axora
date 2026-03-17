//! Task queue integration for Coordinator v2.
//!
//! This module provides a lightweight, dependency-aware queue facade over
//! [`DecomposedMission`] so the top-level coordinator can load decomposed work,
//! reserve the next dispatchable task, and mark work complete.
//!
//! `axora-agents` does not currently depend on `axora-indexing`, so this file
//! mirrors the relevant queue semantics locally while keeping the API shaped for
//! future replacement with the shared atomic queue.

use crate::decomposer::{DecomposedMission, Dependency, DependencyType};
use crate::task::{Priority, Task, TaskStatus};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Result type for queue integration operations.
pub type Result<T> = std::result::Result<T, QueueIntegrationError>;

/// Errors produced by [`TaskQueueIntegration`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum QueueIntegrationError {
    /// A mission attempted to load duplicate task identifiers.
    #[error("duplicate task id: {0}")]
    DuplicateTaskId(String),

    /// A dependency referenced an out-of-bounds task index.
    #[error("invalid dependency index: {0}")]
    InvalidDependency(usize),

    /// A task could not be found.
    #[error("task not found: {0}")]
    TaskNotFound(String),

    /// A task has already been completed.
    #[error("task already completed: {0}")]
    TaskAlreadyCompleted(String),
}

#[derive(Debug, Clone)]
struct QueueTaskRecord {
    index: usize,
    task: Task,
}

/// Coordinator-facing task queue integration.
///
/// `get_next_dispatchable_task()` reserves the returned task by marking it
/// `InProgress`, which prevents duplicate dispatches until the caller marks it
/// complete or resets it externally.
#[derive(Debug, Clone, Default)]
pub struct TaskQueueIntegration {
    mission: Option<String>,
    task_order: Vec<String>,
    tasks: HashMap<String, QueueTaskRecord>,
    dependencies: Vec<Dependency>,
    completed: HashSet<String>,
    in_progress: HashSet<String>,
}

impl TaskQueueIntegration {
    /// Creates an empty queue integration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Loads all tasks from a decomposed mission into the coordinator queue.
    pub fn load_tasks(&mut self, mission: &DecomposedMission) -> Result<usize> {
        self.clear();
        self.mission = Some(mission.original_mission.clone());

        for dependency in &mission.dependencies {
            if dependency.from >= mission.tasks.len() || dependency.to >= mission.tasks.len() {
                return Err(QueueIntegrationError::InvalidDependency(
                    dependency.from.max(dependency.to),
                ));
            }
        }

        for (index, task) in mission.tasks.iter().cloned().enumerate() {
            if self.tasks.contains_key(&task.id) {
                return Err(QueueIntegrationError::DuplicateTaskId(task.id));
            }

            self.task_order.push(task.id.clone());
            self.tasks
                .insert(task.id.clone(), QueueTaskRecord { index, task });
        }

        self.dependencies = mission.dependencies.clone();
        Ok(self.tasks.len())
    }

    /// Returns the next dependency-ready task and reserves it for dispatch.
    pub fn get_next_dispatchable_task(&mut self) -> Option<Task> {
        let mut candidates = self
            .task_order
            .iter()
            .filter_map(|task_id| self.tasks.get(task_id))
            .filter(|record| self.is_dispatchable(record))
            .map(|record| {
                (
                    priority_rank(&record.task.priority),
                    record.index,
                    record.task.id.clone(),
                )
            })
            .collect::<Vec<_>>();

        candidates.sort_by(|left, right| right.0.cmp(&left.0).then(left.1.cmp(&right.1)));

        let next_id = candidates.into_iter().next()?.2;
        let record = self.tasks.get_mut(&next_id)?;
        record.task.status = TaskStatus::InProgress;
        self.in_progress.insert(next_id.clone());
        Some(record.task.clone())
    }

    /// Marks a reserved task complete so dependent tasks can be dispatched.
    pub fn mark_task_complete(&mut self, task_id: &str) -> Result<()> {
        let record = self
            .tasks
            .get_mut(task_id)
            .ok_or_else(|| QueueIntegrationError::TaskNotFound(task_id.to_string()))?;

        if self.completed.contains(task_id) {
            return Err(QueueIntegrationError::TaskAlreadyCompleted(
                task_id.to_string(),
            ));
        }

        record.task.status = TaskStatus::Completed;
        self.in_progress.remove(task_id);
        self.completed.insert(task_id.to_string());
        Ok(())
    }

    /// Returns true when all loaded tasks are complete.
    pub fn is_complete(&self) -> bool {
        !self.tasks.is_empty() && self.completed.len() == self.tasks.len()
    }

    /// Returns the total number of loaded tasks.
    pub fn total_tasks(&self) -> usize {
        self.tasks.len()
    }

    /// Returns the number of completed tasks.
    pub fn completed_tasks(&self) -> usize {
        self.completed.len()
    }

    /// Returns the original mission description, if loaded.
    pub fn mission(&self) -> Option<&str> {
        self.mission.as_deref()
    }

    /// Retrieves a snapshot of a tracked task.
    pub fn get_task(&self, task_id: &str) -> Option<&Task> {
        self.tasks.get(task_id).map(|record| &record.task)
    }

    fn clear(&mut self) {
        self.mission = None;
        self.task_order.clear();
        self.tasks.clear();
        self.dependencies.clear();
        self.completed.clear();
        self.in_progress.clear();
    }

    fn is_dispatchable(&self, record: &QueueTaskRecord) -> bool {
        if self.completed.contains(&record.task.id) || self.in_progress.contains(&record.task.id) {
            return false;
        }

        matches!(
            record.task.status,
            TaskStatus::Pending | TaskStatus::Assigned
        ) && self.dependencies_satisfied(record.index)
    }

    fn dependencies_satisfied(&self, task_index: usize) -> bool {
        self.dependencies
            .iter()
            .filter(|dependency| dependency.from == task_index)
            .filter(|dependency| {
                matches!(
                    dependency.dep_type,
                    DependencyType::Hard | DependencyType::Data
                )
            })
            .all(|dependency| {
                self.task_id_for_index(dependency.to)
                    .map(|task_id| self.completed.contains(task_id))
                    .unwrap_or(false)
            })
    }

    fn task_id_for_index(&self, index: usize) -> Option<&str> {
        self.tasks
            .values()
            .find(|record| record.index == index)
            .map(|record| record.task.id.as_str())
    }
}

fn priority_rank(priority: &Priority) -> u8 {
    match priority {
        Priority::Critical => 4,
        Priority::High => 3,
        Priority::Normal => 2,
        Priority::Low => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::{QueueIntegrationError, TaskQueueIntegration};
    use crate::decomposer::{DecomposedMission, Dependency, DependencyType, TaskDAG};
    use crate::task::{Priority, Task, TaskStatus};

    fn task_with_id(id: &str, description: &str, priority: Priority) -> Task {
        Task {
            id: id.to_string(),
            description: description.to_string(),
            priority,
            status: TaskStatus::Pending,
            assigned_to: None,
            parent_task: None,
        }
    }

    fn mission_with_tasks(tasks: Vec<Task>, dependencies: Vec<Dependency>) -> DecomposedMission {
        let edges = dependencies
            .iter()
            .map(|dependency| (dependency.from, dependency.to))
            .collect::<Vec<_>>();

        let mut mission = DecomposedMission::new("test mission");
        mission.tasks = tasks;
        mission.dependency_graph = TaskDAG {
            nodes: (0..mission.tasks.len()).collect(),
            edges,
        };
        mission.dependencies = dependencies;
        mission.execution_mode = crate::graph::ExecutionMode::Parallel;
        mission
    }

    #[test]
    fn load_tasks_imports_all_tasks() {
        let mission = mission_with_tasks(
            vec![
                task_with_id("task-1", "one", Priority::Normal),
                task_with_id("task-2", "two", Priority::High),
            ],
            Vec::new(),
        );
        let mut integration = TaskQueueIntegration::new();

        let loaded = integration.load_tasks(&mission).unwrap();

        assert_eq!(loaded, 2);
        assert_eq!(integration.total_tasks(), 2);
        assert_eq!(integration.mission(), Some("test mission"));
    }

    #[test]
    fn get_next_dispatchable_task_prefers_highest_priority_ready_task() {
        let mission = mission_with_tasks(
            vec![
                task_with_id("task-1", "normal", Priority::Normal),
                task_with_id("task-2", "critical", Priority::Critical),
                task_with_id("task-3", "low", Priority::Low),
            ],
            Vec::new(),
        );
        let mut integration = TaskQueueIntegration::new();
        integration.load_tasks(&mission).unwrap();

        let task = integration.get_next_dispatchable_task().unwrap();

        assert_eq!(task.id, "task-2");
        assert_eq!(
            integration
                .get_task("task-2")
                .map(|task| task.status.clone()),
            Some(TaskStatus::InProgress)
        );
    }

    #[test]
    fn hard_dependency_blocks_dispatch_until_completion() {
        let mission = mission_with_tasks(
            vec![
                task_with_id("task-1", "first", Priority::Normal),
                task_with_id("task-2", "second", Priority::Critical),
            ],
            vec![Dependency::new(1, 0, DependencyType::Hard)],
        );
        let mut integration = TaskQueueIntegration::new();
        integration.load_tasks(&mission).unwrap();

        let first = integration.get_next_dispatchable_task().unwrap();
        assert_eq!(first.id, "task-1");
        assert!(integration.get_next_dispatchable_task().is_none());

        integration.mark_task_complete("task-1").unwrap();

        let second = integration.get_next_dispatchable_task().unwrap();
        assert_eq!(second.id, "task-2");
    }

    #[test]
    fn soft_dependency_does_not_block_dispatch() {
        let mission = mission_with_tasks(
            vec![
                task_with_id("task-1", "first", Priority::Normal),
                task_with_id("task-2", "second", Priority::High),
            ],
            vec![Dependency::new(1, 0, DependencyType::Soft)],
        );
        let mut integration = TaskQueueIntegration::new();
        integration.load_tasks(&mission).unwrap();

        let first = integration.get_next_dispatchable_task().unwrap();
        let second = integration.get_next_dispatchable_task().unwrap();

        assert_eq!(first.id, "task-2");
        assert_eq!(second.id, "task-1");
    }

    #[test]
    fn mark_task_complete_advances_progress_and_unlocks_completion_state() {
        let mission = mission_with_tasks(
            vec![task_with_id("task-1", "only", Priority::Normal)],
            Vec::new(),
        );
        let mut integration = TaskQueueIntegration::new();
        integration.load_tasks(&mission).unwrap();
        integration.get_next_dispatchable_task().unwrap();

        integration.mark_task_complete("task-1").unwrap();

        assert_eq!(integration.completed_tasks(), 1);
        assert!(integration.is_complete());
        assert_eq!(
            integration
                .get_task("task-1")
                .map(|task| task.status.clone()),
            Some(TaskStatus::Completed)
        );
    }

    #[test]
    fn load_tasks_rejects_invalid_dependency_indices() {
        let mission = mission_with_tasks(
            vec![task_with_id("task-1", "only", Priority::Normal)],
            vec![Dependency::new(2, 0, DependencyType::Hard)],
        );
        let mut integration = TaskQueueIntegration::new();

        let error = integration.load_tasks(&mission).unwrap_err();

        assert_eq!(error, QueueIntegrationError::InvalidDependency(2));
    }

    #[test]
    fn mark_task_complete_rejects_duplicate_completion() {
        let mission = mission_with_tasks(
            vec![task_with_id("task-1", "only", Priority::Normal)],
            Vec::new(),
        );
        let mut integration = TaskQueueIntegration::new();
        integration.load_tasks(&mission).unwrap();
        integration.get_next_dispatchable_task().unwrap();
        integration.mark_task_complete("task-1").unwrap();

        let error = integration.mark_task_complete("task-1").unwrap_err();

        assert_eq!(
            error,
            QueueIntegrationError::TaskAlreadyCompleted("task-1".to_string())
        );
    }
}
