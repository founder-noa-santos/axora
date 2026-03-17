//! Task DAG construction and validation.

use super::{Dependency, RawTask, TaskDAG, TaskId};
use crate::error::AgentError;
use crate::Result;
use axora_indexing::InfluenceGraph;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

/// Output from building a task graph.
#[derive(Debug, Clone)]
pub struct BuiltGraph {
    /// Materialized DAG.
    pub dag: TaskDAG,
    /// Compatibility dependency list.
    pub dependencies: Vec<Dependency>,
    /// Raw task ID to runtime task ID mapping.
    pub id_map: HashMap<String, TaskId>,
}

/// Builds and validates task DAGs.
pub struct GraphBuilder {
    influence_graph: Option<Arc<InfluenceGraph>>,
}

impl GraphBuilder {
    /// Creates a graph builder.
    pub fn new(influence_graph: Option<Arc<InfluenceGraph>>) -> Self {
        Self { influence_graph }
    }

    /// Builds a DAG from raw tasks.
    pub fn build_dag(&self, raw_tasks: &[RawTask]) -> Result<BuiltGraph> {
        if raw_tasks.is_empty() {
            return Err(AgentError::InvalidDecomposition(
                "cannot build DAG from zero tasks".to_string(),
            )
            .into());
        }

        let id_map = raw_tasks
            .iter()
            .enumerate()
            .map(|(idx, task)| (task.id.clone(), idx))
            .collect::<HashMap<_, _>>();

        let mut dependencies = Vec::new();
        for task in raw_tasks {
            let from = *id_map.get(&task.id).ok_or_else(|| {
                AgentError::GraphValidation(format!("missing task id {}", task.id))
            })?;

            for dep_id in &task.dependencies {
                let to = *id_map.get(dep_id).ok_or_else(|| {
                    AgentError::GraphValidation(format!(
                        "task {} depends on missing task {}",
                        task.id, dep_id
                    ))
                })?;
                dependencies.push(Dependency::hard(from, to));
            }
        }

        dependencies.extend(self.infer_dependencies(raw_tasks, &id_map));
        dedupe_dependencies(&mut dependencies);

        let dag = TaskDAG {
            nodes: (0..raw_tasks.len()).collect(),
            edges: dependencies.iter().map(|dep| (dep.from, dep.to)).collect(),
        };

        self.validate_runtime_dag(&dag, raw_tasks.len())?;

        Ok(BuiltGraph {
            dag,
            dependencies,
            id_map,
        })
    }

    /// Infers extra dependencies using target files and the influence graph.
    pub fn infer_dependencies(
        &self,
        raw_tasks: &[RawTask],
        id_map: &HashMap<String, TaskId>,
    ) -> Vec<Dependency> {
        let Some(graph) = &self.influence_graph else {
            return Vec::new();
        };

        let mut inferred = Vec::new();

        for dependent in raw_tasks {
            for predecessor in raw_tasks {
                if dependent.id == predecessor.id {
                    continue;
                }

                let depends = dependent.target_files.iter().any(|dependent_file| {
                    predecessor.target_files.iter().any(|predecessor_file| {
                        graph
                            .get_dependencies(dependent_file)
                            .map(|deps| deps.contains(predecessor_file))
                            .unwrap_or(false)
                    })
                });

                if depends {
                    if let (Some(&from), Some(&to)) =
                        (id_map.get(&dependent.id), id_map.get(&predecessor.id))
                    {
                        inferred.push(Dependency::data(from, to));
                    }
                }
            }
        }

        inferred
    }

    /// Validates a runtime DAG.
    pub fn validate_runtime_dag(&self, dag: &TaskDAG, task_count: usize) -> Result<()> {
        for &(from, to) in &dag.edges {
            if from >= task_count || to >= task_count {
                return Err(AgentError::GraphValidation(format!(
                    "edge ({from}, {to}) references task outside 0..{task_count}"
                ))
                .into());
            }
        }

        self.ensure_acyclic(dag)?;
        self.ensure_connected(dag, task_count)?;
        Ok(())
    }

    fn ensure_acyclic(&self, dag: &TaskDAG) -> Result<()> {
        let mut in_degree = vec![0usize; dag.nodes.len()];
        let mut outgoing = vec![Vec::new(); dag.nodes.len()];

        for &(from, to) in &dag.edges {
            outgoing[to].push(from);
            in_degree[from] += 1;
        }

        let mut queue = in_degree
            .iter()
            .enumerate()
            .filter_map(|(idx, degree)| (*degree == 0).then_some(idx))
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;

        while let Some(node) = queue.pop_front() {
            visited += 1;
            for &next in &outgoing[node] {
                in_degree[next] -= 1;
                if in_degree[next] == 0 {
                    queue.push_back(next);
                }
            }
        }

        if visited != dag.nodes.len() {
            return Err(
                AgentError::GraphValidation("cycle detected in task DAG".to_string()).into(),
            );
        }

        Ok(())
    }

    fn ensure_connected(&self, dag: &TaskDAG, task_count: usize) -> Result<()> {
        if task_count <= 1 {
            return Ok(());
        }

        let mut adjacency = vec![Vec::new(); task_count];
        for &(from, to) in &dag.edges {
            adjacency[from].push(to);
            adjacency[to].push(from);
        }

        let mut visited = HashSet::new();
        let mut queue = VecDeque::from([0usize]);
        visited.insert(0usize);

        while let Some(node) = queue.pop_front() {
            for &neighbor in &adjacency[node] {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        if visited.len() != task_count {
            return Err(AgentError::GraphValidation(format!(
                "task graph is disconnected: visited {} of {} tasks",
                visited.len(),
                task_count
            ))
            .into());
        }

        Ok(())
    }
}

fn dedupe_dependencies(dependencies: &mut Vec<Dependency>) {
    let mut seen = HashSet::new();
    dependencies.retain(|dep| seen.insert((dep.from, dep.to)));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tasks() -> Vec<RawTask> {
        vec![
            RawTask {
                id: "task-0".to_string(),
                description: "Analyze".to_string(),
                dependencies: vec![],
                estimated_duration: 10,
                capabilities: vec!["coding".to_string()],
                target_files: vec!["db/schema.sql".to_string()],
            },
            RawTask {
                id: "task-1".to_string(),
                description: "Implement".to_string(),
                dependencies: vec!["task-0".to_string()],
                estimated_duration: 15,
                capabilities: vec!["coding".to_string()],
                target_files: vec!["api/server.rs".to_string()],
            },
            RawTask {
                id: "task-2".to_string(),
                description: "Write tests".to_string(),
                dependencies: vec!["task-1".to_string()],
                estimated_duration: 10,
                capabilities: vec!["testing".to_string()],
                target_files: vec![],
            },
        ]
    }

    #[test]
    fn test_build_dag_maps_tasks() {
        let builder = GraphBuilder::new(None);
        let graph = builder.build_dag(&sample_tasks()).unwrap();

        assert_eq!(graph.dag.nodes, vec![0, 1, 2]);
        assert_eq!(graph.dependencies.len(), 2);
    }

    #[test]
    fn test_build_dag_rejects_missing_dependency() {
        let builder = GraphBuilder::new(None);
        let mut tasks = sample_tasks();
        tasks[1].dependencies = vec!["missing".to_string()];

        assert!(builder.build_dag(&tasks).is_err());
    }

    #[test]
    fn test_validate_runtime_dag_rejects_cycles() {
        let builder = GraphBuilder::new(None);
        let dag = TaskDAG {
            nodes: vec![0, 1],
            edges: vec![(0, 1), (1, 0)],
        };

        assert!(builder.validate_runtime_dag(&dag, 2).is_err());
    }

    #[test]
    fn test_validate_runtime_dag_rejects_disconnected_graph() {
        let builder = GraphBuilder::new(None);
        let dag = TaskDAG {
            nodes: vec![0, 1, 2],
            edges: vec![(1, 0)],
        };

        assert!(builder.validate_runtime_dag(&dag, 3).is_err());
    }

    #[test]
    fn test_validate_runtime_dag_accepts_singleton() {
        let builder = GraphBuilder::new(None);
        let dag = TaskDAG {
            nodes: vec![0],
            edges: vec![],
        };

        builder.validate_runtime_dag(&dag, 1).unwrap();
    }

    #[test]
    fn test_dedupe_dependencies() {
        let mut deps = vec![Dependency::hard(1, 0), Dependency::hard(1, 0)];
        dedupe_dependencies(&mut deps);

        assert_eq!(deps.len(), 1);
    }
}
