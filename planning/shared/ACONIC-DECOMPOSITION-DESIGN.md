# ACONIC Task Decomposition

**Date:** 2026-03-16
**Status:** ADOPTED
**Source:** Concurrent Task Decomposition Research
**Implements:** Graph-Based Workflow (Phase 2)

---

## 📋 Overview

### What

**ACONIC** (Constraint-Induced Complexity) is a **mathematical framework** for task decomposition that provides:
- **Constraint graph** representation of mission requirements
- **Treewidth calculation** for complexity measurement
- **AOP validation** (Solvability, Completeness, Non-redundancy)
- **DAG construction** with topological sorting

### Why

**Problem:** LLM-based decomposition is heuristic and unreliable:
- No guarantee of valid decomposition
- No complexity measurement
- No validation of independence
- Hits complexity ceiling with large missions

**Solution:** ACONIC provides **mathematical guarantees**:
- Valid decomposition (AOP validated)
- Complexity bounded (treewidth threshold)
- Optimal parallelization (DAG-based)
- No redundant work (non-redundancy check)

### How

```
Mission → Constraint Parsing → Constraint Graph → Treewidth Check
                                                    ↓ (if > threshold)
                                            Decompose Recursively
                                                    ↓ (if <= threshold)
                                            AOP Validation
                                                    ↓
                                            DAG Construction
                                                    ↓
                                            Topological Sort → Parallel Groups
```

---

## 🧩 ACONIC Framework

### Constraint Graph

A **constraint graph** represents mission requirements as a graph:

```rust
pub struct ConstraintGraph {
    nodes: Vec<ConstraintNode>,
    edges: Vec<ConstraintEdge>,
}

pub struct ConstraintNode {
    id: ConstraintId,
    constraint_type: ConstraintType,
    description: String,
    metadata: ConstraintMetadata,
}

pub enum ConstraintType {
    Temporal,      // "Must complete before X"
    Data,          // "Needs output from Y"
    Resource,      // "Requires tool Z"
    Capability,    // "Needs skill W"
    Quality,       // "Must meet standard S"
}

pub struct ConstraintEdge {
    from: ConstraintId,
    to: ConstraintId,
    edge_type: EdgeType,
    strength: f32,
}

pub enum EdgeType {
    Dependency,    // A must complete before B
    Conflict,      // A and B cannot both be true
    Correlation,   // A and B are related
}
```

**Example: "Add OAuth login with rate limiting"**

```
Constraint Graph:
┌─────────────────┐
│  OAuth Login    │
│  (Capability)   │
└────────┬────────┘
         │ Dependency
         ▼
┌─────────────────┐
│  Rate Limiting  │
│  (Quality)      │
└─────────────────┘
```

### Treewidth Calculation

**Treewidth** measures graph complexity — higher treewidth = harder to decompose.

**Definition:**
> Treewidth is the minimum width of a tree decomposition of the graph.

**Intuition:**
- Treewidth 1: Tree (easy to decompose)
- Treewidth 2: Series-parallel graph (moderate)
- Treewidth 3+: Complex graph (needs decomposition)

**Algorithm:**
```rust
pub fn calculate_treewidth(graph: &ConstraintGraph) -> usize {
    // Use minimum degree heuristic for approximation
    let mut graph_copy = graph.clone();
    let mut max_degree = 0;
    
    while !graph_copy.nodes.is_empty() {
        // Find node with minimum degree
        let min_node = graph_copy.nodes
            .iter()
            .min_by_key(|n| graph_copy.degree(n.id))
            .unwrap();
        
        // Track degree before removal
        let degree = graph_copy.degree(min_node.id);
        max_degree = max(max_degree, degree);
        
        // Remove node and add fill-in edges
        graph_copy.remove_node(min_node.id);
    }
    
    max_degree
}
```

**LLM Optimal Treewidth:**
```rust
// Empirically determined threshold
const LLM_OPTIMAL_LINEWIDTH: usize = 5;

// If treewidth > 5, LLM struggles with decomposition
// If treewidth <= 5, LLM can handle the task
```

---

## ✅ AOP Validation

AOP (And-Or-Parallel) validation ensures decomposition is **mathematically sound**.

### Solvability

**Every task must be solvable by available agents.**

**Validation Rules:**

1. **Capability Match**
```rust
fn check_solvability(tasks: &[Task], agents: &[Agent]) -> ValidationResult {
    for task in tasks {
        let has_capable_agent = agents.iter().any(|agent| {
            agent.capabilities.contains_all(&task.required_capabilities)
        });
        
        if !has_capable_agent {
            return ValidationResult::Failed {
                reason: format!("No agent capable of {:?}", task.required_capabilities),
            };
        }
    }
    
    ValidationResult::Passed
}
```

2. **Tool Availability**
```rust
fn check_tools(tasks: &[Task], available_tools: &[Tool]) -> ValidationResult {
    for task in tasks {
        for required_tool in &task.required_tools {
            if !available_tools.contains(required_tool) {
                return ValidationResult::Failed {
                    reason: format!("Tool {:?} not available", required_tool),
                };
            }
        }
    }
    
    ValidationResult::Passed
}
```

3. **Complexity Threshold**
```rust
fn check_complexity(tasks: &[Task], threshold: usize) -> ValidationResult {
    for task in tasks {
        let task_treewidth = calculate_treewidth(&task.constraint_graph);
        if task_treewidth > threshold {
            return ValidationResult::Failed {
                reason: format!(
                    "Task {:?} has treewidth {} (threshold: {})",
                    task.id, task_treewidth, threshold
                ),
            };
        }
    }
    
    ValidationResult::Passed
}
```

### Completeness

**Union of all subtasks must equal original mission.**

**Validation Rules:**

1. **Requirement Coverage**
```rust
fn check_completeness(mission: &Mission, tasks: &[Task]) -> ValidationResult {
    let all_requirements = extract_requirements(mission);
    let covered_requirements: HashSet<_> = tasks
        .iter()
        .flat_map(|task| extract_requirements(task))
        .collect();
    
    let missing: Vec<_> = all_requirements
        .difference(&covered_requirements)
        .collect();
    
    if !missing.is_empty() {
        return ValidationResult::Failed {
            reason: format!("Missing requirements: {:?}", missing),
        };
    }
    
    ValidationResult::Passed
}
```

2. **Implicit Requirements**
```rust
fn check_implicit_requirements(mission: &Mission, tasks: &[Task]) -> ValidationResult {
    let implicit = infer_implicit_requirements(mission);
    
    for implicit_req in &implicit {
        let addressed = tasks.iter().any(|task| {
            task.addresses(implicit_req)
        });
        
        if !addressed {
            return ValidationResult::Failed {
                reason: format!("Implicit requirement not addressed: {:?}", implicit_req),
            };
        }
    }
    
    ValidationResult::Passed
}
```

### Non-redundancy

**No overlapping responsibilities between tasks.**

**Validation Rules:**

1. **Responsibility Overlap**
```rust
fn check_non_redundancy(tasks: &[Task]) -> ValidationResult {
    for (i, task_a) in tasks.iter().enumerate() {
        for task_b in tasks.iter().skip(i + 1) {
            let overlap = task_a.responsibilities
                .intersection(&task_b.responsibilities);
            
            if !overlap.is_empty() {
                return ValidationResult::Failed {
                    reason: format!(
                        "Tasks {:?} and {:?} have overlapping responsibilities: {:?}",
                        task_a.id, task_b.id, overlap
                    ),
                };
            }
        }
    }
    
    ValidationResult::Passed
}
```

2. **Duplicate Tool Calls**
```rust
fn check_duplicate_tools(tasks: &[Task]) -> ValidationResult {
    for (i, task_a) in tasks.iter().enumerate() {
        for task_b in tasks.iter().skip(i + 1) {
            // Skip if both tasks legitimately need the same tool
            if task_a.tool_calls.is_empty() {
                continue;
            }
            
            if task_a.tool_calls == task_b.tool_calls {
                return ValidationResult::Warning {
                    reason: format!(
                        "Tasks {:?} and {:?} make identical tool calls",
                        task_a.id, task_b.id
                    ),
                };
            }
        }
    }
    
    ValidationResult::Passed
}
```

---

## 🔗 DAG Construction

### Dependency Types

**1. Temporal Dependencies**
```
Task A must complete before Task B starts

Example: "Write tests" → "Run tests"
```

**2. Data Dependencies**
```
Task B needs output from Task A

Example: "Generate API spec" → "Implement API"
```

**3. Resource Dependencies**
```
Tasks contend for same tool/resource

Example: "Deploy to staging" and "Deploy to production" (same deploy tool)
```

### DAG Representation

```rust
pub struct TaskDAG {
    tasks: HashMap<TaskId, Task>,
    edges: Vec<DependencyEdge>,
}

pub struct DependencyEdge {
    from: TaskId,  // Prerequisite
    to: TaskId,    // Dependent
    dependency_type: DependencyType,
}

pub enum DependencyType {
    Temporal,
    Data,
    Resource,
}
```

### Topological Sorting

**Algorithm: Kahn's Algorithm**

```rust
pub fn topological_sort(dag: &TaskDAG) -> Vec<Vec<TaskId>> {
    // Calculate in-degree for each task
    let mut in_degree = HashMap::new();
    for task_id in dag.tasks.keys() {
        in_degree.insert(task_id.clone(), 0);
    }
    
    for edge in &dag.edges {
        *in_degree.get_mut(&edge.to).unwrap() += 1;
    }
    
    // Find all tasks with no prerequisites
    let mut queue: Vec<TaskId> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(id, _)| id.clone())
        .collect();
    
    let mut parallel_groups: Vec<Vec<TaskId>> = Vec::new();
    
    while !queue.is_empty() {
        // All tasks in queue can run in parallel
        let current_group = queue.clone();
        parallel_groups.push(current_group.clone());
        
        queue.clear();
        
        // Process current group
        for task_id in &current_group {
            // Find all tasks that depend on this one
            for edge in &dag.edges {
                if edge.from == *task_id {
                    let dependent_degree = in_degree.get_mut(&edge.to).unwrap();
                    *dependent_degree -= 1;
                    
                    if *dependent_degree == 0 {
                        queue.push(edge.to.clone());
                    }
                }
            }
        }
    }
    
    parallel_groups
}
```

### Critical Path Identification

```rust
pub fn find_critical_path(dag: &TaskDAG) -> Vec<TaskId> {
    // Calculate earliest start time for each task
    let mut earliest_start = HashMap::new();
    let mut earliest_finish = HashMap::new();
    
    // Forward pass
    for group in topological_sort(dag) {
        for task_id in group {
            let task = &dag.tasks[&task_id];
            
            // Find max finish time of prerequisites
            let prereq_finish = dag.edges
                .iter()
                .filter(|e| e.to == task_id)
                .map(|e| earliest_finish[&e.from])
                .max()
                .unwrap_or(0);
            
            earliest_start.insert(task_id.clone(), prereq_finish);
            earliest_finish.insert(task_id.clone(), prereq_finish + task.estimated_duration);
        }
    }
    
    // Backward pass to find critical path
    let project_duration = earliest_finish.values().max().copied().unwrap_or(0);
    
    let mut latest_finish = HashMap::new();
    let mut latest_start = HashMap::new();
    
    for group in topological_sort(dag).iter().rev() {
        for task_id in group {
            let task = &dag.tasks[task_id];
            
            // Find min start time of dependents
            let dependent_start = dag.edges
                .iter()
                .filter(|e| e.from == task_id)
                .map(|e| latest_start[&e.to])
                .min()
                .unwrap_or(project_duration);
            
            latest_finish.insert(task_id.clone(), dependent_start);
            latest_start.insert(task_id.clone(), dependent_start - task.estimated_duration);
        }
    }
    
    // Critical path: tasks with zero slack
    let mut critical_path = Vec::new();
    for task_id in dag.tasks.keys() {
        let slack = latest_start[task_id] - earliest_start[task_id];
        if slack == 0 {
            critical_path.push(task_id.clone());
        }
    }
    
    critical_path
}
```

---

## 🚀 Implementation Plan

### Phase 1: Constraint Parsing (8 hours)

**Goal:** Parse mission → constraint graph

**Tasks:**
- [ ] Define constraint types (temporal, data, resource, capability, quality)
- [ ] Implement mission parser (extract constraints from natural language)
- [ ] Build constraint graph data structure
- [ ] Add edge inference (detect dependencies between constraints)

**API:**
```rust
pub struct ConstraintParser {
    llm: LlmClient,
}

impl ConstraintParser {
    pub fn parse(&self, mission: &str) -> Result<ConstraintGraph> {
        // Use LLM to extract constraints
        let constraints = self.llm.extract_constraints(mission).await?;
        
        // Build graph
        let mut graph = ConstraintGraph::new();
        for constraint in constraints {
            graph.add_node(constraint);
        }
        
        // Infer edges
        self.infer_edges(&mut graph)?;
        
        Ok(graph)
    }
}
```

---

### Phase 2: Treewidth Calculation (8 hours)

**Goal:** Implement treewidth algorithm with threshold

**Tasks:**
- [ ] Implement minimum degree heuristic
- [ ] Add exact treewidth calculation (for small graphs)
- [ ] Set LLM_OPTIMAL_LINEWIDTH threshold
- [ ] Add recursive decomposition (if treewidth > threshold)

**API:**
```rust
pub struct TreewidthCalculator {
    threshold: usize,
}

impl TreewidthCalculator {
    pub fn new() -> Self {
        Self {
            threshold: LLM_OPTIMAL_LINEWIDTH,
        }
    }
    
    pub fn calculate(&self, graph: &ConstraintGraph) -> TreewidthResult {
        let treewidth = self.minimum_degree_heuristic(graph);
        
        TreewidthResult {
            value: treewidth,
            within_threshold: treewidth <= self.threshold,
            needs_decomposition: treewidth > self.threshold,
        }
    }
}
```

---

### Phase 3: AOP Validator (8 hours)

**Goal:** Implement AOP validation (Solvability, Completeness, Non-redundancy)

**Tasks:**
- [ ] Implement solvability check (capability + tools + complexity)
- [ ] Implement completeness check (requirement coverage)
- [ ] Implement non-redundancy check (overlap detection)
- [ ] Create validation report

**API:**
```rust
pub struct AOPValidator {
    agent_registry: AgentRegistry,
    tool_registry: ToolRegistry,
}

impl AOPValidator {
    pub fn validate(&self, mission: &str, tasks: &[Task]) -> Result<AOPReport> {
        let solvability = self.check_solvability(tasks)?;
        let completeness = self.check_completeness(mission, tasks)?;
        let non_redundancy = self.check_non_redundancy(tasks)?;
        
        Ok(AOPReport {
            solvability,
            completeness,
            non_redundancy,
            passed: solvability.passed && completeness.passed && non_redundancy.passed,
        })
    }
}
```

---

### Phase 4: DAG Builder (8 hours)

**Goal:** Build DAG from validated constraints, topological sort

**Tasks:**
- [ ] Build DAG from constraint graph
- [ ] Implement topological sort (Kahn's algorithm)
- [ ] Identify parallel groups (cohorts)
- [ ] Calculate critical path

**API:**
```rust
pub struct DAGBuilder {
    dependency_detector: DependencyDetector,
}

impl DAGBuilder {
    pub fn build(&self, constraints: &ConstraintGraph) -> Result<TaskDAG> {
        let mut dag = TaskDAG::new();
        
        // Add tasks
        for constraint in &constraints.nodes {
            dag.add_task(self.constraint_to_task(constraint)?);
        }
        
        // Add edges
        for edge in &constraints.edges {
            dag.add_edge(self.edge_to_dependency(edge)?);
        }
        
        Ok(dag)
    }
    
    pub fn get_parallel_groups(&self, dag: &TaskDAG) -> Vec<Vec<TaskId>> {
        topological_sort(dag)
    }
    
    pub fn get_critical_path(&self, dag: &TaskDAG) -> Vec<TaskId> {
        find_critical_path(dag)
    }
}
```

---

## 📐 API Design

### MissionDecomposer

```rust
pub struct MissionDecomposer {
    constraint_parser: ConstraintParser,
    treewidth_calculator: TreewidthCalculator,
    aop_validator: AOPValidator,
    dag_builder: DAGBuilder,
}

impl MissionDecomposer {
    pub fn new(
        llm: LlmClient,
        agent_registry: AgentRegistry,
        tool_registry: ToolRegistry,
    ) -> Self {
        Self {
            constraint_parser: ConstraintParser::new(llm),
            treewidth_calculator: TreewidthCalculator::new(),
            aop_validator: AOPValidator::new(agent_registry, tool_registry),
            dag_builder: DAGBuilder::new(),
        }
    }
    
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        // Step 1: Parse constraints
        let constraint_graph = self.constraint_parser.parse(mission)?;
        
        // Step 2: Check treewidth
        let treewidth_result = self.treewidth_calculator.calculate(&constraint_graph);
        
        // Step 3: If too complex, decompose recursively
        if treewidth_result.needs_decomposition {
            return self.decompose_recursive(mission, &constraint_graph);
        }
        
        // Step 4: Build DAG
        let dag = self.dag_builder.build(&constraint_graph)?;
        
        // Step 5: Extract tasks
        let tasks = dag.tasks.values().cloned().collect();
        
        // Step 6: AOP validation
        let aop_report = self.aop_validator.validate(mission, &tasks)?;
        
        if !aop_report.passed {
            return Err(Error::AOPValidationFailed(aop_report));
        }
        
        // Step 7: Get parallel groups and critical path
        let parallel_groups = self.dag_builder.get_parallel_groups(&dag);
        let critical_path = self.dag_builder.get_critical_path(&dag);
        
        Ok(DecomposedMission {
            tasks,
            dependencies: dag.edges,
            parallel_groups,
            critical_path,
        })
    }
    
    fn decompose_recursive(
        &self,
        mission: &str,
        graph: &ConstraintGraph,
    ) -> Result<DecomposedMission> {
        // Split graph into components
        let components = graph.find_connected_components();
        
        // Decompose each component
        let mut all_tasks = Vec::new();
        let mut all_dependencies = Vec::new();
        
        for component in components {
            let sub_mission = self.extract_sub_mission(mission, &component)?;
            let decomposed = self.decompose(&sub_mission)?;
            
            all_tasks.extend(decomposed.tasks);
            all_dependencies.extend(decomposed.dependencies);
        }
        
        // Merge results
        Ok(DecomposedMission {
            tasks: all_tasks,
            dependencies: all_dependencies,
            parallel_groups: self.merge_parallel_groups(all_tasks),
            critical_path: Vec::new(), // Recalculated after merge
        })
    }
}
```

### DecomposedMission

```rust
pub struct DecomposedMission {
    /// All tasks (flattened)
    pub tasks: Vec<Task>,
    
    /// Dependencies between tasks
    pub dependencies: Vec<DependencyEdge>,
    
    /// Groups of tasks that can run in parallel
    pub parallel_groups: Vec<Vec<TaskId>>,
    
    /// Longest dependency chain (critical for timeline)
    pub critical_path: Vec<TaskId>,
}

impl DecomposedMission {
    /// Get total number of tasks
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }
    
    /// Get parallelization quotient (tasks in parallel / total tasks)
    pub fn parallelization_quotient(&self) -> f32 {
        let tasks_in_parallel: usize = self.parallel_groups
            .iter()
            .map(|group| group.len())
            .sum();
        
        tasks_in_parallel as f32 / self.task_count() as f32
    }
    
    /// Get estimated total duration (sum of critical path)
    pub fn estimated_duration(&self, tasks: &HashMap<TaskId, Task>) -> Duration {
        self.critical_path
            .iter()
            .map(|id| tasks[id].estimated_duration)
            .sum()
    }
}
```

---

## 📊 Validation Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **AOP Compliance** | 100% | Validator pass rate |
| **Treewidth Accuracy** | >95% | Comparison with manual analysis |
| **Parallelization Quotient** | >90% | Tasks in parallel groups / total tasks |
| **Structural Agreement** | >85% | Overlap with human-curated baseline |
| **Decomposition Time** | <5s | Time to decompose mission |
| **False Positive Rate** | <5% | Invalid decompositions accepted |
| **False Negative Rate** | <10% | Valid decompositions rejected |

---

## 🔗 Related Documents

- [`GRAPH-WORKFLOW-DESIGN.md`](./GRAPH-WORKFLOW-DESIGN.md) — Graph-based workflow
- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Pivot decision
- [`AOP-VALIDATOR-SPEC.md`](./AOP-VALIDATOR-SPEC.md) — AOP validation specification
- [`CONCURRENT-IMPLEMENTATION.md`](./CONCURRENT-IMPLEMENTATION.md) — Implementation plan

---

## 📝 Design Decisions

### Why Treewidth Threshold = 5?

**Empirical basis:**
- LLMs handle tree-structured tasks well (treewidth 1)
- Performance degrades at treewidth 3-4
- Significant failures at treewidth 6+

**Conservative choice:**
- Threshold 5 provides safety margin
- Can be tuned based on real-world performance

### Why Kahn's Algorithm?

**Alternatives considered:**
- DFS-based topological sort
- Tarjan's algorithm

**Kahn's advantages:**
- Naturally produces parallel groups (cohorts)
- Easy to understand and debug
- O(V + E) complexity

### Why Recursive Decomposition?

**Problem:** Some missions have treewidth > threshold

**Solution:** Recursively decompose until each sub-mission has treewidth <= threshold

**Benefit:** Mathematical guarantee of manageable complexity

---

**This design provides MATHEMATICAL FOUNDATION for task decomposition, not heuristic guessing.**
