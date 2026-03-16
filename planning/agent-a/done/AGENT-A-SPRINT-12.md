# Agent A — Sprint 12: ACONIC Decomposition Documentation

**Phase:** 2  
**Sprint:** 12 (Documentation)  
**File:** `planning/shared/ACONIC-DECOMPOSITION-DESIGN.md`  
**Priority:** HIGH (blocks Agent C's implementation)  
**Estimated Tokens:** ~60K output  

---

## 🎯 Task

Create comprehensive design document for **ACONIC-based Task Decomposition** (from concurrent task decomposition research).

### Context

Research validates our Graph-Based pivot (R-10) and provides MATHEMATICAL FOUNDATION:
- **ACONIC Framework** — Constraint-induced complexity for decomposition
- **Treewidth Calculation** — Mathematical measure of task complexity
- **AOP Validation** — Solvability, Completeness, Non-redundancy
- **DAG Construction** — Dependency mapping with topological sort

**Your job:** Document this architecture so Agent C can implement.

---

## 📋 Deliverables

### 1. Create ACONIC-DECOMPOSITION-DESIGN.md

**File:** `planning/shared/ACONIC-DECOMPOSITION-DESIGN.md`

**Structure:**
```markdown
# ACONIC Task Decomposition

## Overview
- What: Constraint-induced complexity for task decomposition
- Why: Mathematical guarantee of valid decomposition (not heuristic)
- How: Constraint graph + treewidth calculation + AOP validation

## ACONIC Framework

### Constraint Graph
- Nodes: Task constraints (from mission parsing)
- Edges: Constraint relationships (dependencies, conflicts)
- Treewidth: Measure of graph complexity

### Treewidth Threshold
- LLM_OPTIMAL_LINEWIDTH = 5 (empirically determined)
- If treewidth > threshold → decompose recursively
- If treewidth <= threshold → safe to execute

## AOP Validation

### Solvability
- Every task must match agent capabilities
- Validation: Check against agent skill registry

### Completeness
- Union of all subtasks = original mission
- Validation: Coverage analysis (no missing requirements)

### Non-redundancy
- No overlapping responsibilities
- Validation: Intersection check (no duplicate work)

## DAG Construction

### Dependency Types
- Temporal (task A must complete before task B)
- Data (task B needs output from task A)
- Resource (tasks contend for same tool/resource)

### Topological Sorting
- Algorithm: Kahn's algorithm or DFS-based
- Output: Parallel groups (cohorts)
- Critical Path: Longest dependency chain

## Implementation Plan

### Phase 1: Constraint Parsing
- Parse mission → constraints
- Build constraint graph

### Phase 2: Treewidth Calculation
- Implement treewidth algorithm
- Set threshold (LLM_OPTIMAL_LINEWIDTH)

### Phase 3: AOP Validator
- Solvability check
- Completeness check
- Non-redundancy check

### Phase 4: DAG Builder
- Build DAG from validated constraints
- Topological sort → parallel groups
- Critical path identification

## API Design

```rust
pub struct MissionDecomposer {
    constraint_graph: ConstraintGraph,
    aop_validator: AOPValidator,
    dag_builder: DAGBuilder,
}

impl MissionDecomposer {
    pub fn decompose(&self, mission: &str) -> Result<DecomposedMission>;
}

pub struct DecomposedMission {
    tasks: Vec<Task>,
    dependencies: Vec<Dependency>,
    parallel_groups: Vec<Vec<TaskId>>,
    critical_path: Vec<TaskId>,
}
```

## Validation Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| AOP Compliance | 100% | Validator pass rate |
| Treewidth Accuracy | >95% | Comparison with manual analysis |
| Parallelization Quotient | >90% | Tasks in parallel groups / total tasks |
| Structural Agreement | >85% | Overlap with human-curated baseline |
```

**Length:** 2500-3500 words (comprehensive design spec)

---

### 2. Update CONCURRENT-IMPLEMENTATION.md

**File:** `planning/shared/CONCURRENT-IMPLEMENTATION.md`

**Add Section:**
```markdown
## ACONIC Decomposition (NEW)

Research validates mathematical approach to task decomposition:

### Before (Heuristic)
- LLM "think step-by-step" → unstructured list
- No validation of independence
- No complexity measurement

### After (ACONIC)
- Parse constraints → build constraint graph
- Calculate treewidth (complexity measure)
- AOP validation (Solvability, Completeness, Non-redundancy)
- DAG construction with topological sort

### Benefits
- Mathematical guarantee of valid decomposition
- Prevents LLM from hitting complexity ceiling
- Enables optimal parallelization
```

---

### 3. Create AOP-VALIDATOR-SPEC.md

**File:** `planning/shared/AOP-VALIDATOR-SPEC.md`

**Purpose:** Detailed AOP validation rules (for Agent C to implement)

**Structure:**
```markdown
# AOP Validator Specification

## Solvability Check

### Rule 1: Capability Match
```
FOR EACH task IN tasks:
    EXISTS agent IN agents:
        agent.capabilities MATCHES task.required_capabilities
```

### Rule 2: Tool Availability
```
FOR EACH task IN tasks:
    task.required_tools SUBSET OF available_tools
```

### Rule 3: Complexity Threshold
```
FOR EACH task IN tasks:
    task.treewidth <= LLM_OPTIMAL_LINEWIDTH
```

## Completeness Check

### Rule 1: Requirement Coverage
```
all_requirements = extract_requirements(original_mission)
covered_requirements = UNION(extract_requirements(task) FOR task IN tasks)
all_requirements == covered_requirements
```

### Rule 2: Implicit Requirements
```
FOR EACH implicit IN infer_implicit_requirements(original_mission):
    EXISTS task IN tasks:
        task.addresses(implicit)
```

## Non-redundancy Check

### Rule 1: Responsibility Overlap
```
FOR EACH pair (task_a, task_b) IN combinations(tasks, 2):
    task_a.responsibilities INTERSECTION task_b.responsibilities == EMPTY
```

### Rule 2: Duplicate Tool Calls
```
FOR EACH pair (task_a, task_b) IN combinations(tasks, 2):
    NOT (task_a.tool_calls == task_b.tool_calls AND task_a.tool_calls != EMPTY)
```

## Implementation

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
```

**Length:** 1500-2000 words (detailed spec)

---

### 4. Update RESEARCH SUMMARY

**File:** `planning/shared/RESEARCH-SUMMARY.md` (CREATE IF NOT EXISTS)

**Purpose:** One-page summary of all research findings

**Structure:**
```markdown
# Research Summary (Phase 2)

## R-10: DDD Agents Validation
- **Finding:** DDD REJECTED for individual devs
- **Adopted:** Graph-Based Workflow (LangGraph-style)
- **Status:** ✅ Implemented

## R-11: Concurrency + ReAct
- **Finding:** Concurrent execution mandatory
- **Adopted:** DAG-based parallel groups
- **Status:** ✅ In progress

## R-13: Influence Graph
- **Finding:** Static analysis > LLM for dependencies
- **Adopted:** Code influence graph (no LLM)
- **Status:** ✅ In progress

## Concurrent Task Decomposition Research
- **Finding:** ACONIC framework validates our approach
- **Adopted:** Constraint graph + treewidth + AOP validation
- **Status:** 🔄 Documentation (Agent A Sprint 12)

## Key Validations
1. ✅ Graph-Based > DDD (R-10 + Research)
2. ✅ O(N) coordination > O(N²) (R-10 + Research)
3. ✅ Blackboard > Chat (Research)
4. ✅ Pull-Based Context > Push (Research)
5. ✅ Dual-Thread ReAct > Sequential (Research)
```

---

## 📁 File Boundaries

**Create:**
- `planning/shared/ACONIC-DECOMPOSITION-DESIGN.md`
- `planning/shared/AOP-VALIDATOR-SPEC.md`
- `planning/shared/RESEARCH-SUMMARY.md`

**Update:**
- `planning/shared/CONCURRENT-IMPLEMENTATION.md`

**DO NOT Edit:**
- `crates/` (implementation — Agent C's domain)
- `research/` (research files are historical)

---

## ✅ Success Criteria

- [ ] `ACONIC-DECOMPOSITION-DESIGN.md` created (2500-3500 words)
- [ ] `AOP-VALIDATOR-SPEC.md` created (1500-2000 words)
- [ ] `RESEARCH-SUMMARY.md` created (1000-1500 words)
- [ ] `CONCURRENT-IMPLEMENTATION.md` updated
- [ ] All docs link to `PHASE-2-INTEGRATION-REACT-PATTERNS.md`
- [ ] Zero contradictions with existing docs

---

## 🔗 References

- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](./PHASE-2-INTEGRATION-REACT-PATTERNS.md) — Main integration doc
- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](./PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Graph pivot (validated)
- Research document provided by user — ACONIC framework source

---

**Start NOW. Agent C is blocked waiting for this design doc.**

**Priority: HIGH — this is on the critical path.**
