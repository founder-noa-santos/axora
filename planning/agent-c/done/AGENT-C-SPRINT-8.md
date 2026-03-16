# Agent C — Sprint 8: Task Decomposition Pivot (Graph-Based, Not DDD)

**Sprint:** 8 of Phase 2 (REPLACES Sprint 7)  
**File:** `crates/axora-agents/src/decomposer.rs` + `crates/axora-agents/src/graph.rs`  
**Estimated Tokens:** ~100K output tokens  

---

## 🎯 Task

Pivot Task Decomposition from **DDD-based** to **Graph-Based Deterministic Workflow**.

### Context

R-10 research proved:
- DDD decomposition creates cross-domain routing bottlenecks
- Sequential tasks degrade 39-70% with DDD (context fragmentation)
- Graph-based (LangGraph-style) is deterministic + O(N) coordination

**Your job:** Implement Graph-Based Decomposition (not Domain-Based).

---

## 📋 Deliverables

### 1. Refactor decomposer.rs

**Remove:**
- Any DDD-specific code (domain teams, bounded contexts)
- Parallel group logic based on domains

**Keep:**
- `MissionDecomposer` struct
- `DecomposedMission` struct
- `decompose()` method

**Add:**
```rust
pub struct MissionDecomposer {
    // Graph-based decomposition (not domain-based)
    workflow_templates: HashMap<TaskType, WorkflowTemplate>,
    
    // Sequential vs parallel detection
    parallelism_detector: ParallelismDetector,
}

pub struct WorkflowTemplate {
    // Deterministic graph structure
    nodes: Vec<NodeDefinition>,
    edges: Vec<EdgeDefinition>,
    
    // Guard conditions for transitions
    guards: Vec<GuardCondition>,
}

impl MissionDecomposer {
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission> {
        // 1. Detect task type (sequential vs parallelizable)
        let task_type = self.detect_task_type(mission);
        
        // 2. Get workflow template
        let template = self.workflow_templates.get(&task_type)?;
        
        // 3. Instantiate graph (deterministic, not semantic)
        let graph = template.instantiate(mission)?;
        
        // 4. Return decomposed mission
        Ok(DecomposedMission {
            tasks: graph.nodes,
            dependencies: graph.edges,
            execution_mode: task_type.execution_mode(), // Sequential or Parallel
        })
    }
}
```

---

### 2. Create graph.rs (NEW FILE)

**File:** `crates/axora-agents/src/graph.rs`

**Purpose:** Graph-Based Deterministic Workflow Engine

**Structure:**
```rust
//! Graph-Based Deterministic Workflow
//!
//! This module implements LangGraph-style state machine:
//! - Nodes: Agent roles (Planner, Executor, Reviewer)
//! - Edges: Explicit state transitions
//! - Guards: Validation before transition

use serde::{Deserialize, Serialize};

/// Workflow graph (state machine)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    /// Nodes (agent roles)
    pub nodes: Vec<Node>,
    
    /// Edges (state transitions)
    pub edges: Vec<Edge>,
    
    /// Current state
    pub current_state: Option<NodeId>,
}

/// Node (agent role in workflow)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub role: NodeRole,
    pub description: String,
}

/// Agent role types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRole {
    /// Decompose task, create execution plan
    Planner,
    
    /// Execute task (write code, run commands)
    Executor,
    
    /// Validate output (review code, run tests)
    Reviewer,
    
    /// Resolve conflicts (integration issues)
    Resolver,
}

/// Edge (state transition)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub condition: TransitionCondition,
}

/// Transition condition (guard)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionCondition {
    /// Always transition (unconditional)
    Always,
    
    /// Transition on success
    OnSuccess,
    
    /// Transition on failure
    OnFailure,
    
    /// Custom guard condition
    Guard(String), // Guard expression (parsed at runtime)
}

impl WorkflowGraph {
    /// Create new workflow graph
    pub fn new() -> Self;
    
    /// Add node to graph
    pub fn add_node(&mut self, node: Node);
    
    /// Add edge to graph
    pub fn add_edge(&mut self, edge: Edge);
    
    /// Execute graph (deterministic traversal)
    pub async fn execute(&mut self, task: &Task) -> Result<TaskResult>;
    
    /// Validate transition (guard conditions)
    pub fn validate_transition(&self, from: NodeId, to: NodeId, state: &ExecutionState) -> bool;
}
```

---

### 3. Implement Deterministic Routing

**File:** `crates/axora-agents/src/graph.rs` (add to existing)

```rust
impl WorkflowGraph {
    /// Execute graph with deterministic routing
    pub async fn execute(&mut self, task: &Task) -> Result<TaskResult> {
        let mut state = ExecutionState::new(task);
        
        // Start at initial node
        self.current_state = Some(self.get_initial_node());
        
        loop {
            let current_node_id = self.current_state.unwrap();
            let current_node = self.get_node(current_node_id).unwrap();
            
            // Execute current node
            let result = current_node.execute(&mut state).await?;
            
            // Find next node (deterministic, based on guards)
            let next_node = self.find_next_node(current_node_id, &result, &state);
            
            match next_node {
                Some(next_id) => {
                    // Validate transition (guard conditions)
                    if self.validate_transition(current_node_id, next_id, &state) {
                        self.current_state = Some(next_id);
                    } else {
                        // Guard failed, escalate to resolver
                        self.current_state = Some(self.get_resolver_node());
                    }
                }
                None => {
                    // No next node, execution complete
                    return Ok(state.finalize());
                }
            }
        }
    }
}
```

---

### 4. Add Sequential vs Parallel Detection

**File:** `crates/axora-agents/src/decomposer.rs` (add to existing)

```rust
pub struct ParallelismDetector {
    // Heuristics for detecting parallelizable tasks
}

impl ParallelismDetector {
    /// Detect if task is sequential or parallelizable
    pub fn detect(&self, mission: &str, dependencies: &[Dependency]) -> ExecutionMode {
        // Analyze dependency graph
        let has_circular_deps = self.detect_circular_dependencies(dependencies);
        let max_depth = self.calculate_critical_path(dependencies);
        
        // Heuristics:
        // - Circular deps → Sequential (can't parallelize)
        // - Deep dependency chain → Sequential (coordination overhead)
        // - Independent subtasks → Parallelizable
        
        if has_circular_deps || max_depth > 5 {
            ExecutionMode::Sequential
        } else {
            ExecutionMode::Parallel
        }
    }
}

pub enum ExecutionMode {
    /// Execute sequentially (O(N) total time)
    Sequential,
    
    /// Execute in parallel (O(1) time, O(N²) coordination)
    Parallel,
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/axora-agents/src/graph.rs` (NEW)

**Update:**
- `crates/axora-agents/src/decomposer.rs` (refactor to graph-based)

**DO NOT Edit:**
- `crates/axora-cache/` (Agent B's domain)
- `crates/axora-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_graph_creation() { }

#[test]
fn test_deterministic_execution() { }

#[test]
fn test_transition_guards() { }

#[test]
fn test_sequential_vs_parallel_detection() { }

#[test]
fn test_coordination_overhead_linear() { }

#[test]
fn test_workflow_template_instantiation() { }

#[test]
fn test_error_handling_with_resolver() { }

#[test]
fn test_full_workflow_execution() { }
```

---

## ✅ Success Criteria

- [ ] `decomposer.rs` refactored (graph-based, not DDD)
- [ ] `graph.rs` created (Workflow Graph implementation)
- [ ] Deterministic routing implemented
- [ ] Sequential vs parallel detection works
- [ ] 8+ tests passing
- [ ] Coordination overhead O(N) (not O(N²))
- [ ] Execution is deterministic (no semantic routing)

---

## 🔗 References

- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](../shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Main pivot doc
- [`research/prompts/10-ddd-agents-validation.md`](../research/prompts/10-ddd-agents-validation.md) — R-10 research
- [`research/prompts/11-concurrency-react-loops.md`](../research/prompts/11-concurrency-react-loops.md) — Concurrency patterns

---

**Start NOW. Focus on deterministic graph execution, not domain-based decomposition.**
