//! Load balancing and critical-path calculations for queued tasks.

use crate::worker_pool::WorkerId;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::time::Duration;

/// Snapshot of task metadata used by the load balancer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadBalancingTask {
    /// Task identifier.
    pub task_id: String,
    /// Dependency identifiers that must complete first.
    pub dependencies: Vec<String>,
    /// Task priority from `0..=100`.
    pub priority: u8,
    /// Precomputed critical path length, if available.
    pub critical_path_length: usize,
}

/// Assignment of tasks to a worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerAssignment {
    /// Assigned worker identifier.
    pub worker_id: WorkerId,
    /// Tasks reserved for the worker.
    pub task_ids: Vec<String>,
}

/// Computes critical path data and spreads ready work across workers.
#[derive(Debug, Clone, Default)]
pub struct LoadBalancer;

impl LoadBalancer {
    /// Create a new load balancer.
    pub fn new() -> Self {
        Self
    }

    /// Return the longest dependency chain in the DAG.
    pub fn calculate_critical_path(&self, tasks: &[LoadBalancingTask]) -> Vec<String> {
        let task_map = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task.clone()))
            .collect::<HashMap<_, _>>();
        let mut memo = HashMap::<String, Vec<String>>::new();
        let mut best = Vec::new();

        for task in tasks {
            let path = longest_path_to(&task.task_id, &task_map, &mut memo);
            if path.len() > best.len() {
                best = path;
            }
        }

        best
    }

    /// Spread tasks across workers using critical-path length first, then priority.
    pub fn balance_load(
        &self,
        workers: &[WorkerId],
        tasks: &[LoadBalancingTask],
    ) -> Vec<WorkerAssignment> {
        let mut assignments = workers
            .iter()
            .cloned()
            .map(|worker_id| WorkerAssignment {
                worker_id,
                task_ids: Vec::new(),
            })
            .collect::<Vec<_>>();

        if assignments.is_empty() || tasks.is_empty() {
            return assignments;
        }

        let mut ranked = tasks.to_vec();
        let computed_lengths = self.path_lengths(tasks);
        ranked.sort_by_key(|task| {
            (
                Reverse(task.critical_path_length.max(
                    *computed_lengths.get(&task.task_id).unwrap_or(&1),
                )),
                Reverse(task.priority),
                task.task_id.clone(),
            )
        });

        let mut loads = vec![0usize; assignments.len()];
        for task in ranked {
            let target = loads
                .iter()
                .enumerate()
                .min_by_key(|(index, load)| (**load, assignments[*index].task_ids.len()))
                .map(|(index, _)| index)
                .unwrap_or(0);
            assignments[target].task_ids.push(task.task_id.clone());
            loads[target] += task
                .critical_path_length
                .max(*computed_lengths.get(&task.task_id).unwrap_or(&1))
                .max(1);
        }

        assignments
    }

    /// Estimate completion time from the current critical path.
    pub fn estimate_completion_time(&self, tasks: &[LoadBalancingTask]) -> Duration {
        Duration::from_secs(self.calculate_critical_path(tasks).len() as u64)
    }

    fn path_lengths(&self, tasks: &[LoadBalancingTask]) -> HashMap<String, usize> {
        let task_map = tasks
            .iter()
            .map(|task| (task.task_id.clone(), task.clone()))
            .collect::<HashMap<_, _>>();
        let mut memo = HashMap::new();
        for task in tasks {
            longest_path_to(&task.task_id, &task_map, &mut memo);
        }
        memo.into_iter()
            .map(|(task_id, path)| (task_id, path.len()))
            .collect()
    }
}

fn longest_path_to(
    task_id: &str,
    tasks: &HashMap<String, LoadBalancingTask>,
    memo: &mut HashMap<String, Vec<String>>,
) -> Vec<String> {
    if let Some(path) = memo.get(task_id) {
        return path.clone();
    }

    let mut best_prefix = Vec::new();
    if let Some(task) = tasks.get(task_id) {
        for dependency in &task.dependencies {
            let candidate = longest_path_to(dependency, tasks, memo);
            if candidate.len() > best_prefix.len() {
                best_prefix = candidate;
            }
        }
    }

    best_prefix.push(task_id.to_string());
    memo.insert(task_id.to_string(), best_prefix.clone());
    best_prefix
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(task_id: &str, dependencies: &[&str], priority: u8, critical_path_length: usize) -> LoadBalancingTask {
        LoadBalancingTask {
            task_id: task_id.to_string(),
            dependencies: dependencies.iter().map(|dependency| dependency.to_string()).collect(),
            priority,
            critical_path_length,
        }
    }

    #[test]
    fn calculate_critical_path_returns_longest_chain() {
        let balancer = LoadBalancer::new();
        let tasks = vec![
            task("a", &[], 50, 1),
            task("b", &["a"], 50, 2),
            task("c", &["b"], 50, 3),
            task("d", &["a"], 50, 2),
        ];

        assert_eq!(
            balancer.calculate_critical_path(&tasks),
            vec!["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn balance_load_distributes_tasks_across_workers() {
        let balancer = LoadBalancer::new();
        let assignments = balancer.balance_load(
            &["w1".to_string(), "w2".to_string()],
            &[
                task("a", &[], 90, 3),
                task("b", &[], 80, 2),
                task("c", &[], 70, 1),
                task("d", &[], 60, 1),
            ],
        );

        assert_eq!(assignments.len(), 2);
        assert_eq!(
            assignments.iter().map(|assignment| assignment.task_ids.len()).sum::<usize>(),
            4
        );
        assert!(assignments.iter().all(|assignment| !assignment.task_ids.is_empty()));
    }

    #[test]
    fn balance_load_prioritizes_critical_tasks() {
        let balancer = LoadBalancer::new();
        let assignments = balancer.balance_load(
            &["w1".to_string()],
            &[
                task("a", &[], 10, 1),
                task("b", &[], 90, 4),
                task("c", &[], 80, 2),
            ],
        );

        assert_eq!(
            assignments[0].task_ids,
            vec!["b".to_string(), "c".to_string(), "a".to_string()]
        );
    }

    #[test]
    fn estimate_completion_time_uses_critical_path_length() {
        let balancer = LoadBalancer::new();
        let tasks = vec![
            task("a", &[], 50, 1),
            task("b", &["a"], 50, 2),
            task("c", &["b"], 50, 3),
        ];

        assert_eq!(balancer.estimate_completion_time(&tasks), Duration::from_secs(3));
    }

    #[test]
    fn empty_inputs_return_empty_assignments() {
        let balancer = LoadBalancer::new();
        assert!(balancer.balance_load(&[], &[task("a", &[], 50, 1)]).is_empty());
        assert!(
            balancer
                .balance_load(&["w1".to_string()], &[])
                .first()
                .unwrap()
                .task_ids
                .is_empty()
        );
    }

    #[test]
    fn calculate_critical_path_handles_single_task() {
        let balancer = LoadBalancer::new();
        assert_eq!(
            balancer.calculate_critical_path(&[task("solo", &[], 50, 1)]),
            vec!["solo".to_string()]
        );
    }
}
