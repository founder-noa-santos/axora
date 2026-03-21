//! Parallel group identification and critical path analysis.

use super::{ParallelGroup, TaskDAG, TaskId};
use crate::error::AgentError;
use crate::Result;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::Duration;

/// Computes parallel groups and critical paths.
pub struct ParallelGroupIdentifier {
    max_parallelism: usize,
}

impl ParallelGroupIdentifier {
    /// Creates a new identifier.
    pub fn new(max_parallelism: usize) -> Self {
        Self {
            max_parallelism: max_parallelism.max(1),
        }
    }

    /// Identifies groups using dependency levels.
    pub fn identify_groups(&self, dag: &TaskDAG) -> Result<Vec<ParallelGroup>> {
        let levels = self.calculate_levels(dag)?;
        let mut by_level = BTreeMap::<usize, Vec<TaskId>>::new();
        for (task_id, level) in levels {
            by_level.entry(level).or_default().push(task_id);
        }

        Ok(by_level
            .into_iter()
            .enumerate()
            .map(|(group_id, (_level, mut task_ids))| {
                task_ids.sort_unstable();
                ParallelGroup {
                    group_id,
                    can_run_in_parallel: task_ids.len() > 1,
                    dependencies_satisfied: true,
                    task_ids,
                }
            })
            .collect())
    }

    /// Calculates the critical path as the longest weighted path in the DAG.
    pub fn calculate_critical_path(
        &self,
        dag: &TaskDAG,
        durations: &HashMap<TaskId, Duration>,
    ) -> Result<Vec<TaskId>> {
        let topo = self.topological_order(dag)?;
        let predecessors = predecessors(dag);
        let mut dist = HashMap::<TaskId, Duration>::new();
        let mut parent = HashMap::<TaskId, TaskId>::new();

        for &node in &topo {
            let base = durations
                .get(&node)
                .copied()
                .unwrap_or(Duration::from_secs(60));
            let mut best = base;
            let mut best_parent = None;

            for &pred in predecessors.get(&node).map(Vec::as_slice).unwrap_or(&[]) {
                let pred_dist = dist.get(&pred).copied().unwrap_or(Duration::ZERO) + base;
                if pred_dist > best {
                    best = pred_dist;
                    best_parent = Some(pred);
                }
            }

            dist.insert(node, best);
            if let Some(pred) = best_parent {
                parent.insert(node, pred);
            }
        }

        let end = topo
            .iter()
            .max_by_key(|task_id| dist.get(task_id).copied().unwrap_or(Duration::ZERO))
            .copied()
            .ok_or_else(|| {
                AgentError::GraphValidation(
                    "cannot compute critical path for empty DAG".to_string(),
                )
            })?;

        let mut path = vec![end];
        let mut cursor = end;
        while let Some(&pred) = parent.get(&cursor) {
            path.push(pred);
            cursor = pred;
        }
        path.reverse();
        Ok(path)
    }

    /// Splits overly large groups while preserving group order.
    pub fn optimize_for_parallelism(
        &self,
        groups: Vec<ParallelGroup>,
        _dag: &TaskDAG,
    ) -> Vec<ParallelGroup> {
        let mut optimized = Vec::new();
        for group in groups {
            if group.task_ids.len() <= self.max_parallelism {
                optimized.push(ParallelGroup {
                    group_id: optimized.len(),
                    ..group
                });
                continue;
            }

            for chunk in group.task_ids.chunks(self.max_parallelism) {
                optimized.push(ParallelGroup {
                    group_id: optimized.len(),
                    task_ids: chunk.to_vec(),
                    can_run_in_parallel: chunk.len() > 1,
                    dependencies_satisfied: true,
                });
            }
        }
        optimized
    }

    fn calculate_levels(&self, dag: &TaskDAG) -> Result<HashMap<TaskId, usize>> {
        let topo = self.topological_order(dag)?;
        let predecessors = predecessors(dag);
        let mut levels = HashMap::new();

        for node in topo {
            let level = predecessors
                .get(&node)
                .map(|preds| {
                    preds
                        .iter()
                        .filter_map(|pred| levels.get(pred))
                        .max()
                        .copied()
                        .unwrap_or(0)
                        + usize::from(!preds.is_empty())
                })
                .unwrap_or(0);
            levels.insert(node, level);
        }

        Ok(levels)
    }

    fn topological_order(&self, dag: &TaskDAG) -> Result<Vec<TaskId>> {
        let mut in_degree = vec![0usize; dag.nodes.len()];
        let mut outgoing = vec![Vec::new(); dag.nodes.len()];

        for &(from, to) in &dag.edges {
            in_degree[from] += 1;
            outgoing[to].push(from);
        }

        let mut queue = in_degree
            .iter()
            .enumerate()
            .filter_map(|(idx, degree)| (*degree == 0).then_some(idx))
            .collect::<VecDeque<_>>();
        let mut order = Vec::with_capacity(dag.nodes.len());

        while let Some(node) = queue.pop_front() {
            order.push(node);
            for &next in &outgoing[node] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push_back(next);
                }
            }
        }

        if order.len() != dag.nodes.len() {
            return Err(AgentError::GraphValidation(
                "cycle detected while sorting DAG".to_string(),
            )
            .into());
        }

        Ok(order)
    }
}

fn predecessors(dag: &TaskDAG) -> HashMap<TaskId, Vec<TaskId>> {
    let mut map = HashMap::<TaskId, Vec<TaskId>>::new();
    for &(from, to) in &dag.edges {
        map.entry(from).or_default().push(to);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dag() -> TaskDAG {
        TaskDAG {
            nodes: vec![0, 1, 2, 3, 4],
            edges: vec![(2, 0), (2, 1), (3, 2), (4, 2)],
        }
    }

    #[test]
    fn test_identify_groups_by_level() {
        let identifier = ParallelGroupIdentifier::new(10);
        let groups = identifier.identify_groups(&sample_dag()).unwrap();

        assert_eq!(groups[0].task_ids, vec![0, 1]);
        assert_eq!(groups[1].task_ids, vec![2]);
    }

    #[test]
    fn test_calculate_critical_path() {
        let identifier = ParallelGroupIdentifier::new(10);
        let durations = HashMap::from([
            (0, Duration::from_secs(60)),
            (1, Duration::from_secs(30)),
            (2, Duration::from_secs(90)),
            (3, Duration::from_secs(120)),
            (4, Duration::from_secs(45)),
        ]);

        let path = identifier
            .calculate_critical_path(&sample_dag(), &durations)
            .unwrap();

        assert_eq!(path, vec![0, 2, 3]);
    }

    #[test]
    fn test_optimize_for_parallelism_splits_large_group() {
        let identifier = ParallelGroupIdentifier::new(2);
        let groups = vec![ParallelGroup {
            group_id: 0,
            task_ids: vec![0, 1, 2, 3],
            can_run_in_parallel: true,
            dependencies_satisfied: true,
        }];

        let optimized = identifier.optimize_for_parallelism(groups, &sample_dag());

        assert_eq!(optimized.len(), 2);
        assert_eq!(optimized[0].task_ids, vec![0, 1]);
        assert_eq!(optimized[1].task_ids, vec![2, 3]);
    }

    #[test]
    fn test_identify_groups_rejects_cycle() {
        let identifier = ParallelGroupIdentifier::new(10);
        let dag = TaskDAG {
            nodes: vec![0, 1],
            edges: vec![(0, 1), (1, 0)],
        };

        assert!(identifier.identify_groups(&dag).is_err());
    }

    #[test]
    fn test_calculate_critical_path_singleton() {
        let identifier = ParallelGroupIdentifier::new(10);
        let dag = TaskDAG {
            nodes: vec![0],
            edges: vec![],
        };
        let durations = HashMap::from([(0, Duration::from_secs(15))]);

        let path = identifier
            .calculate_critical_path(&dag, &durations)
            .unwrap();

        assert_eq!(path, vec![0]);
    }

    #[test]
    fn test_optimize_preserves_small_groups() {
        let identifier = ParallelGroupIdentifier::new(4);
        let groups = vec![ParallelGroup {
            group_id: 0,
            task_ids: vec![1, 2],
            can_run_in_parallel: true,
            dependencies_satisfied: true,
        }];

        let optimized = identifier.optimize_for_parallelism(groups.clone(), &sample_dag());

        assert_eq!(optimized.len(), 1);
        assert_eq!(optimized[0].task_ids, groups[0].task_ids);
    }
}
