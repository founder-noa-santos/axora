# Phase 3 Sprint C3: Result Merging & Conflict Resolution

**Agent:** C (Implementation Specialist — Coordinator Core)  
**Sprint:** C3  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement result merging with automatic conflict detection and resolution for multi-worker outputs.

**Context:** Phase 2 has no merging (manual). Phase 3 needs automatic merging with conflict detection (file overwrites, incompatible changes).

**Difficulty:** ⚠️ **MEDIUM-HIGH** — Conflict detection, auto-resolution, user escalation

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: Result Combiner
**Task:** Implement combining results from multiple workers
**File:** `crates/axora-agents/src/merger/result_combiner.rs`
**Deliverables:**
- `ResultCombiner` struct
- `combine_results(results)` → `MergedResult`
- `merge_code_changes()` uses three-way merge
- `merge_documentation()` concatenates with structure
- 5+ tests

### Subagent 2: Conflict Detector + Resolver
**Task:** Implement conflict detection and auto-resolution
**File:** `crates/axora-agents/src/merger/conflict_resolver.rs`
**Deliverables:**
- `ConflictDetector` struct
- `detect_conflicts(results)` → `Vec<Conflict>`
- `ConflictResolver` struct
- `auto_resolve(conflict)` for simple conflicts
- `escalate_to_user(conflict)` for complex conflicts
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 2 Subagents:**
   - Assign tasks to both subagents
   - Review combiner + conflict resolver
   - Resolve integration issues

2. **Integrate Components:**
   - Create `crates/axora-agents/src/merger.rs` (main module)
   - Combine result combiner + conflict detector + resolver
   - Export unified `ResultMerger` struct

3. **Implement Three-Way Merge:**
   - Base version (before mission)
   - Worker A changes
   - Worker B changes
   - Auto-merge non-conflicting changes

4. **Write Integration Tests:**
   - Test merging compatible changes (success)
   - Test conflict detection (overlapping changes)
   - Test auto-resolution (simple conflicts)
   - Test user escalation (complex conflicts)

5. **Update Documentation:**
   - Add module to `crates/axora-agents/src/lib.rs`
   - Add merging examples

---

## 📐 Technical Spec

### Result Merger Interface

```rust
pub struct ResultMerger {
    combiner: ResultCombiner,
    detector: ConflictDetector,
    resolver: ConflictResolver,
    config: MergerConfig,
}

pub struct MergerConfig {
    pub auto_resolve_simple: bool,    // Default: true
    pub max_conflicts_before_escalate: usize, // Default: 5
    pub use_three_way_merge: bool,    // Default: true
}

pub struct MergedResult {
    pub mission_id: String,
    pub success: bool,
    pub combined_output: String,
    pub conflicts: Vec<Conflict>,
    pub resolved_conflicts: usize,
    pub escalated_conflicts: usize,
    pub merged_files: Vec<MergedFile>,
}

pub struct MergedFile {
    pub file_path: PathBuf,
    pub base_version: String,
    pub merged_content: String,
    pub has_conflicts: bool,
    pub contributors: Vec<WorkerId>,
}

pub struct Conflict {
    pub conflict_id: String,
    pub conflict_type: ConflictType,
    pub file_path: PathBuf,
    pub worker_a: WorkerId,
    pub worker_b: WorkerId,
    pub description: String,
    pub resolution: Option<ConflictResolution>,
}

pub enum ConflictType {
    FileOverwrite,      // Both workers modified same file
    IncompatibleChanges, // Changes conflict semantically
    DependencyMismatch, // Workers used different versions
    ResourceConflict,   // Both modified same resource
}

pub enum ConflictResolution {
    AutoMerged,         // Automatically resolved
    WorkerAWins,        // Used worker A's version
    WorkerBWins,        // Used worker B's version
    ManualResolution,   // Requires user intervention
}

impl ResultMerger {
    pub fn new(config: MergerConfig) -> Self;
    
    pub fn merge(&self, results: Vec<TaskResult>, base_state: &State) -> Result<MergedResult>;
    
    pub fn detect_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict>;
    
    pub fn auto_resolve_conflicts(&self, conflicts: Vec<Conflict>) -> Vec<Conflict>;
    
    pub fn get_escalated_conflicts(&self, merged_result: &MergedResult) -> Vec<Conflict>;
}
```

### Three-Way Merge Algorithm

```
1. Get base version (before mission started)
2. Get worker A changes (diff from base)
3. Get worker B changes (diff from base)
4. For each file:
   - If only one worker changed: use that change
   - If both changed, non-overlapping: merge both
   - If both changed, overlapping: mark as conflict
5. Combine all merged files
6. Return MergedResult with conflicts
```

### Conflict Detection Algorithm

```
1. Group results by file/resource
2. For each file with multiple contributors:
   - Check if changes overlap (same lines/sections)
   - Check semantic compatibility (API changes)
   - Check dependency versions
3. Classify conflict type:
   - FileOverwrite: both wrote same file
   - IncompatibleChanges: semantic conflict
   - DependencyMismatch: different versions
   - ResourceConflict: shared resource modified
4. Return list of conflicts
```

### Auto-Resolution Strategies

```
1. Non-overlapping changes: merge both
2. One change is additive only: keep both
3. One change is documentation: keep both
4. Both change same logic:
   - Use worker with higher success rate
   - Or escalate to user
5. Multiple conflicts (>5): escalate all
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `ResultMerger` compiles and works
- [ ] 10+ tests passing (5 per subagent + 5 integration)
- [ ] Three-way merge works (non-conflicting merged)
- [ ] Conflict detection accurate (no false negatives)
- [ ] Auto-resolution works for simple conflicts
- [ ] Complex conflicts escalated to user
- [ ] Documentation updated

---

## 🔗 Dependencies

**Requires:**
- Sprint C2 complete (Decomposition for task structure)
- Sprint A2 complete (Blackboard v2 for base state)

**Blocks:**
- None (final step in Coordinator workflow)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Result Combiner (parallel)
  └─ Subagent 2: Conflict Detector + Resolver (parallel)
  ↓
Lead Agent: Integration + Three-Way Merge + Tests
```

**Merge Complexity:**
- Simple case: no conflicts (90% of missions)
- Medium case: auto-resolvable conflicts (8%)
- Hard case: requires user escalation (2%)

**Difficulty: MEDIUM-HIGH**
- 2 subagents to coordinate
- Three-way merge implementation
- Conflict detection heuristics
- Auto-resolution logic

**Review Checklist:**
- [ ] Three-way merge correct (no data loss)
- [ ] Conflicts detected accurately
- [ ] Auto-resolution safe (doesn't break code)
- [ ] Escalation includes all context
- [ ] Performance acceptable (<1s for merge)

---

**Start AFTER Sprint C2 complete.**
