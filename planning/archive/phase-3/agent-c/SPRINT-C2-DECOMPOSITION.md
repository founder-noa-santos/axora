# Phase 3 Sprint C2: Task Decomposition Engine

**Agent:** C (Implementation Specialist — Coordinator Core)  
**Sprint:** C2  
**Priority:** CRITICAL  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement LLM + Graph hybrid task decomposition that converts missions into parallelizable task DAGs.

**Context:** Phase 2 has rule-based decomposition (limited). Phase 3 needs LLM-based decomposition with graph validation for correctness.

**Difficulty:** ⚠️ **HIGH** — LLM integration, DAG construction, parallel group identification

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: LLM Decomposer
**Task:** Implement LLM-based mission decomposition
**File:** `crates/openakta-agents/src/decomposer/llm_decomposer.rs`
**Deliverables:**
- `LLMDecomposer` struct
- `decompose(mission)` → `Vec<RawTask>` using LLM
- Prompt template for decomposition
- Parse LLM output into structured tasks
- 5+ tests

### Subagent 2: Graph Builder
**Task:** Build dependency graph from raw tasks
**File:** `crates/openakta-agents/src/decomposer/graph_builder.rs`
**Deliverables:**
- `GraphBuilder` struct
- `build_dag(raw_tasks)` → `TaskDAG`
- `infer_dependencies()` using influence graph
- `validate_dag()` ensures no cycles
- 5+ tests

### Subagent 3: Parallel Group Identifier
**Task:** Identify parallel groups from DAG
**File:** `crates/openakta-agents/src/decomposer/parallel_groups.rs`
**Deliverables:**
- `ParallelGroupIdentifier` struct
- `identify_groups(dag)` → `Vec<ParallelGroup>`
- `calculate_critical_path(dag)` → longest path
- `optimize_for_parallelism()` reorders tasks
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review LLM + graph builder + parallel groups
   - Resolve conflicts (LLM output vs graph validation)

2. **Integrate Components:**
   - Create `crates/openakta-agents/src/decomposer/v2.rs` (main module)
   - Combine LLM + graph + parallel groups
   - Export unified `MissionDecomposer` struct

3. **Implement Hybrid Validation:**
   - LLM proposes decomposition
   - Graph validates (no cycles, all dependencies satisfied)
   - Reject and retry if invalid

4. **Write Integration Tests:**
   - Test decomposition accuracy (matches human decomposition)
   - Test parallel group identification (maximizes parallelism)
   - Test critical path calculation (accurate)
   - Test with complex missions (10+ tasks)

5. **Update Documentation:**
   - Add module to `crates/openakta-agents/src/lib.rs`
   - Add decomposition examples

---

## 📐 Technical Spec

### Mission Decomposer Interface

```rust
pub struct MissionDecomposer {
    llm_decomposer: LLMDecomposer,
    graph_builder: GraphBuilder,
    parallel_identifier: ParallelGroupIdentifier,
    influence_graph: Arc<InfluenceGraph>,
    config: DecomposerConfig,
}

pub struct DecomposerConfig {
    pub max_tasks: usize,           // Default: 50
    pub max_parallelism: usize,     // Default: 10
    pub llm_model: String,          // Default: "gpt-4"
    pub retry_on_invalid: bool,     // Default: true
    pub max_retries: usize,         // Default: 3
}

pub struct DecomposedMission {
    pub mission_id: String,
    pub tasks: Vec<Task>,
    pub dependency_graph: TaskDAG,
    pub parallel_groups: Vec<ParallelGroup>,
    pub critical_path: Vec<TaskId>,
    pub estimated_duration: Duration,
}

pub struct ParallelGroup {
    pub group_id: usize,
    pub task_ids: Vec<TaskId>,
    pub can_run_in_parallel: bool,
    pub dependencies_satisfied: bool,
}

pub struct TaskDAG {
    pub nodes: Vec<TaskId>,
    pub edges: Vec<(TaskId, TaskId)>, // (from, to)
}

impl MissionDecomposer {
    pub fn new(influence_graph: Arc<InfluenceGraph>, config: DecomposerConfig) -> Self;
    
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission>;
    
    pub fn validate_decomposition(&self, mission: &DecomposedMission) -> Result<()>;
}
```

### LLM Decomposition Prompt

```
You are an expert task planner. Decompose the following mission into discrete, actionable tasks.

Mission: {mission}

For each task, provide:
1. Task ID (unique identifier)
2. Task Description (clear, actionable)
3. Dependencies (list of task IDs this task depends on)
4. Estimated Duration (in minutes)
5. Required Capabilities (coding, testing, documentation, etc.)

Output format (JSON):
{
  "tasks": [
    {
      "id": "task-1",
      "description": "...",
      "dependencies": [],
      "estimated_duration": 10,
      "capabilities": ["coding"]
    }
  ]
}

Rules:
- Each task must be independently executable
- Dependencies must form a DAG (no cycles)
- Max {max_tasks} tasks
- Identify tasks that can run in parallel
```

### Hybrid Validation Algorithm

```
1. LLM proposes decomposition (raw tasks)
2. Graph builder constructs DAG
3. Validate DAG:
   - No cycles (topological sort succeeds)
   - All dependencies reference existing tasks
   - No orphan tasks (all connected)
4. If invalid:
   - Retry LLM with error feedback
   - Max 3 retries
5. If valid:
   - Identify parallel groups
   - Calculate critical path
   - Return DecomposedMission
```

### Parallel Group Identification

```
1. Topological sort of DAG
2. For each task in sorted order:
   - Calculate level (max level of dependencies + 1)
3. Group tasks by level
4. Tasks in same level can run in parallel
5. Optimize: split large groups, merge small groups
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `MissionDecomposer` compiles and works
- [ ] 15+ tests passing (5 per subagent + 10 integration)
- [ ] LLM decomposition accurate (matches human)
- [ ] Graph validation catches cycles
- [ ] Parallel groups maximize parallelism
- [ ] Critical path calculation accurate
- [ ] Documentation updated

---

## 🔗 Dependencies

**Requires:**
- Sprint C1 complete (Coordinator Core for context)
- Sprint B1 complete (Worker Pool for parallelism info)

**Blocks:**
- Sprint C3 (Merging needs decomposed tasks)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: LLM Decomposer (parallel)
  ├─ Subagent 2: Graph Builder (parallel)
  └─ Subagent 3: Parallel Groups (parallel)
  ↓
Lead Agent: Integration + Validation + Tests
```

**LLM Integration:**
- Use async LLM calls (non-blocking)
- Cache decomposition results (avoid re-decomposing same mission)
- Retry on invalid output (max 3 retries)
- Parse JSON output strictly (reject malformed)

**Difficulty: HIGH**
- 3 subagents to coordinate
- LLM integration (async, error handling)
- Graph algorithms (DAG, topological sort)
- Hybrid validation (LLM + graph)

**Review Checklist:**
- [ ] LLM prompt produces valid JSON
- [ ] Graph validation catches all cycles
- [ ] Parallel groups are correct (no dependency violations)
- [ ] Critical path is accurate
- [ ] Retry logic works (invalid → retry)

---

**Start AFTER Sprint C1 complete.**
