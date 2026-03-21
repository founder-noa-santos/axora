//! DAG-based dependency tracking for queued tasks.

use std::collections::{HashMap, HashSet, VecDeque};

use thiserror::Error;

/// Dependency-tracker result type.
pub type Result<T> = std::result::Result<T, DependencyTrackerError>;

/// Errors produced by [`DependencyTracker`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum DependencyTrackerError {
    /// The dependency would introduce a cycle.
    #[error("circular dependency: {task_id} depends on {depends_on}")]
    CircularDependency {
        /// Dependent task.
        task_id: String,
        /// Dependency that would close the cycle.
        depends_on: String,
    },
}

/// Maintains dependency edges and completion state for queue scheduling.
#[derive(Debug, Clone, Default)]
pub struct DependencyTracker {
    tasks: HashSet<String>,
    dependencies: HashMap<String, HashSet<String>>,
    dependents: HashMap<String, HashSet<String>>,
    completed: HashSet<String>,
}

impl DependencyTracker {
    /// Create an empty dependency tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a task without dependencies.
    pub fn register_task(&mut self, task_id: impl Into<String>) {
        let task_id = task_id.into();
        self.tasks.insert(task_id.clone());
        self.dependencies.entry(task_id.clone()).or_default();
        self.dependents.entry(task_id).or_default();
    }

    /// Add a dependency edge from `task_id` to `depends_on`.
    pub fn add_dependency(
        &mut self,
        task_id: impl Into<String>,
        depends_on: impl Into<String>,
    ) -> Result<()> {
        let task_id = task_id.into();
        let depends_on = depends_on.into();
        self.register_task(task_id.clone());
        self.register_task(depends_on.clone());

        if task_id == depends_on || self.reachable(&depends_on, &task_id) {
            return Err(DependencyTrackerError::CircularDependency {
                task_id,
                depends_on,
            });
        }

        self.dependencies
            .entry(task_id.clone())
            .or_default()
            .insert(depends_on.clone());
        self.dependents
            .entry(depends_on)
            .or_default()
            .insert(task_id);
        Ok(())
    }

    /// Return all tasks whose dependencies are satisfied and are not completed.
    pub fn get_ready_tasks(&self) -> Vec<String> {
        let mut ready = self
            .tasks
            .iter()
            .filter(|task_id| !self.completed.contains(*task_id))
            .filter(|task_id| self.is_ready(task_id))
            .cloned()
            .collect::<Vec<_>>();
        ready.sort();
        ready
    }

    /// Returns true if the dependency graph currently contains a cycle.
    pub fn detect_cycles(&self) -> bool {
        let mut indegree = self
            .tasks
            .iter()
            .map(|task_id| {
                (
                    task_id.clone(),
                    self.dependencies.get(task_id).map_or(0, HashSet::len),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut queue = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(task_id, _)| task_id.clone())
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;

        while let Some(task_id) = queue.pop_front() {
            visited += 1;
            if let Some(dependents) = self.dependents.get(&task_id) {
                for dependent in dependents {
                    if let Some(entry) = indegree.get_mut(dependent) {
                        *entry -= 1;
                        if *entry == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        visited != self.tasks.len()
    }

    /// Mark a task complete.
    pub fn mark_completed(&mut self, task_id: &str) -> bool {
        if !self.tasks.contains(task_id) {
            return false;
        }
        self.completed.insert(task_id.to_string())
    }

    /// Returns true when the task exists and all of its dependencies are complete.
    pub fn is_ready(&self, task_id: &str) -> bool {
        self.tasks.contains(task_id)
            && self
                .dependencies
                .get(task_id)
                .map(|dependencies| dependencies.iter().all(|dep| self.completed.contains(dep)))
                .unwrap_or(true)
    }

    /// Returns the dependencies for the given task.
    pub fn dependencies_for(&self, task_id: &str) -> Vec<String> {
        let mut dependencies = self
            .dependencies
            .get(task_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        dependencies.sort();
        dependencies
    }

    /// Returns the dependents for the given task.
    pub fn dependents_of(&self, task_id: &str) -> Vec<String> {
        let mut dependents = self
            .dependents
            .get(task_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        dependents.sort();
        dependents
    }

    fn reachable(&self, start: &str, target: &str) -> bool {
        let mut stack = vec![start.to_string()];
        let mut visited = HashSet::new();

        while let Some(task_id) = stack.pop() {
            if task_id == target {
                return true;
            }
            if !visited.insert(task_id.clone()) {
                continue;
            }

            if let Some(neighbors) = self.dependencies.get(&task_id) {
                stack.extend(neighbors.iter().cloned());
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dependency_registers_both_nodes() {
        let mut tracker = DependencyTracker::new();
        tracker.add_dependency("task-b", "task-a").unwrap();

        assert_eq!(
            tracker.dependencies_for("task-b"),
            vec!["task-a".to_string()]
        );
        assert_eq!(tracker.dependents_of("task-a"), vec!["task-b".to_string()]);
    }

    #[test]
    fn get_ready_tasks_returns_roots_first() {
        let mut tracker = DependencyTracker::new();
        tracker.register_task("task-a");
        tracker.add_dependency("task-b", "task-a").unwrap();

        assert_eq!(tracker.get_ready_tasks(), vec!["task-a".to_string()]);
    }

    #[test]
    fn completed_dependency_unblocks_dependents() {
        let mut tracker = DependencyTracker::new();
        tracker.add_dependency("task-b", "task-a").unwrap();

        tracker.mark_completed("task-a");

        assert_eq!(tracker.get_ready_tasks(), vec!["task-b".to_string()]);
    }

    #[test]
    fn add_dependency_rejects_direct_cycle() {
        let mut tracker = DependencyTracker::new();
        tracker.add_dependency("task-b", "task-a").unwrap();

        let error = tracker.add_dependency("task-a", "task-b").unwrap_err();
        assert_eq!(
            error,
            DependencyTrackerError::CircularDependency {
                task_id: "task-a".to_string(),
                depends_on: "task-b".to_string(),
            }
        );
    }

    #[test]
    fn add_dependency_rejects_indirect_cycle() {
        let mut tracker = DependencyTracker::new();
        tracker.add_dependency("task-b", "task-a").unwrap();
        tracker.add_dependency("task-c", "task-b").unwrap();

        let error = tracker.add_dependency("task-a", "task-c").unwrap_err();
        assert_eq!(
            error,
            DependencyTrackerError::CircularDependency {
                task_id: "task-a".to_string(),
                depends_on: "task-c".to_string(),
            }
        );
    }

    #[test]
    fn detect_cycles_is_false_for_valid_dag() {
        let mut tracker = DependencyTracker::new();
        tracker.add_dependency("task-b", "task-a").unwrap();
        tracker.add_dependency("task-c", "task-b").unwrap();

        assert!(!tracker.detect_cycles());
    }
}
