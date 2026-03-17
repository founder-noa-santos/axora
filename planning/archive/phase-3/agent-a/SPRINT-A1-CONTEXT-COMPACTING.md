# Phase 3 Sprint A1: Context Compacting

**Agent:** A (Documentation + Memory Specialist)  
**Sprint:** A1  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement context compacting system that reduces token usage by 60-80% while preserving critical information.

**Context:** Phase 2 agents send full context every time (50K+ tokens). Phase 3 needs compacted context (10-20K tokens) for Coordinator efficiency.

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Rolling Summary Implementation
**Task:** Implement rolling summary (last N turns)
**File:** `crates/axora-cache/src/compactor/rolling_summary.rs`
**Deliverables:**
- `RollingSummary` struct
- `add_turn()` method
- `summarize()` method (last 10 turns full, older summarized)
- 5+ tests

### Subagent 2: Hierarchical Memory Implementation
**Task:** Implement hierarchical memory (recent full, old summarized)
**File:** `crates/axora-cache/src/compactor/hierarchical_memory.rs`
**Deliverables:**
- `HierarchicalMemory` struct
- 3 levels: Recent (0-10), Mid (11-50), Old (50+)
- `get_context()` returns appropriate level
- 5+ tests

### Subagent 3: Importance Scoring + Pruning
**Task:** Implement importance scoring (prune low-importance)
**File:** `crates/axora-cache/src/compactor/importance_scorer.rs`
**Deliverables:**
- `ImportanceScorer` struct
- Score 0-1 (high = keep, low = prune)
- `prune_below(threshold)` method
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate Subagents:**
   - Assign tasks above to 3 subagents
   - Review their code before merging
   - Resolve conflicts between implementations

2. **Integrate Components:**
   - Create `crates/axora-cache/src/compactor.rs` (main module)
   - Combine rolling summary + hierarchical + importance
   - Export unified `ContextCompactor` struct

3. **Write Integration Tests:**
   - Test full compaction pipeline
   - Verify 60-80% token reduction
   - Test edge cases (empty context, huge context)

4. **Update Documentation:**
   - Add module to `crates/axora-cache/src/lib.rs`
   - Update README with compaction examples

---

## 📐 Technical Spec

### Main Compactor Interface

```rust
pub struct ContextCompactor {
    rolling_summary: RollingSummary,
    hierarchical: HierarchicalMemory,
    scorer: ImportanceScorer,
    config: CompactorConfig,
}

pub struct CompactorConfig {
    recent_turns_full: usize,      // Default: 10
    mid_turns_summarized: usize,   // Default: 40
    old_turns_archived: usize,     // Default: 50
    importance_threshold: f32,      // Default: 0.3
    max_tokens: usize,              // Default: 20000
}

impl ContextCompactor {
    pub fn new(config: CompactorConfig) -> Self;
    
    pub fn compact(&self, context: &Context) -> Result<CompactContext>;
    
    pub fn compression_ratio(&self, original: usize, compacted: usize) -> f32;
}

pub struct CompactContext {
    pub content: String,
    pub original_tokens: usize,
    pub compacted_tokens: usize,
    pub compression_ratio: f32,
}
```

### Compression Algorithm

```
1. Add all turns to rolling summary
2. Apply hierarchical memory (recent full, old summarized)
3. Score each element (0-1)
4. Prune elements below threshold
5. If still over max_tokens, increase threshold and repeat
6. Return CompactContext with metrics
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] `ContextCompactor` compiles and works
- [ ] 15+ tests passing (5 per subagent + 5 integration)
- [ ] 60-80% token reduction achieved on test data
- [ ] Documentation updated
- [ ] Code reviewed and merged

---

## 🔗 Dependencies

**None** — Can start immediately

**Blocks:**
- Sprint A2 (Blackboard v2 needs compaction)
- Sprint C2 (Decomposition needs compacted context)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Rolling Summary (parallel)
  ├─ Subagent 2: Hierarchical Memory (parallel)
  └─ Subagent 3: Importance Scoring (parallel)
  ↓
Lead Agent: Integration + Tests
```

**Parallel Execution:** All 3 subagents can work simultaneously (no dependencies between them)

**Review Checklist:**
- [ ] Each subagent has 5+ tests
- [ ] No conflicts in file structure
- [ ] All imports resolve correctly
- [ ] Integration tests cover full pipeline

---

**Start NOW. Delegate to subagents immediately.**
