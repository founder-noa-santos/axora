# Concurrent Implementation Plan — Phase 2

**Date:** 2026-03-16
**Goal:** Enable parallel agent work without conflicts

---

## 🎯 Current Status

**Completed:**
- ✅ Sprint 1: Prefix Caching (axora-cache crate)
- ✅ Sprint 2: Diff-Based Communication (axora-cache crate)
- ✅ Sprint 3: Code Minification (axora-cache crate)
- ✅ Sprint 6: Documentation Management (axora-docs crate)
- ✅ Sprint 9: Integration & Benchmarking
- ✅ Sprint 11: Graph Workflow Design

**In Progress:**
- 🔄 Sprint 12: ACONIC Decomposition Documentation

**Pending:**
- 📋 Sprint 5: TOON Serialization
- 📋 Sprint 7: Task Decomposition Implementation (ACONIC-based)
- 📋 Sprint 8: Context Distribution

---

## 🔀 ACONIC Decomposition (NEW)

Research validates mathematical approach to task decomposition:

### Before (Heuristic)
- LLM "think step-by-step" → unstructured list
- No validation of independence
- No complexity measurement
- High failure rate for complex missions

### After (ACONIC)
- Parse constraints → build constraint graph
- Calculate treewidth (complexity measure)
- AOP validation (Solvability, Completeness, Non-redundancy)
- DAG construction with topological sort

### Benefits
- **Mathematical guarantee** of valid decomposition
- **Prevents LLM** from hitting complexity ceiling
- **Enables optimal parallelization** (90%+ parallelization quotient)
- **Validated by research** (not just heuristic)

### Implementation Plan

| Phase | Task | Owner | Files | Estimated Time |
|-------|------|-------|-------|----------------|
| **Phase 1** | Constraint Parsing | Agent C | `crates/axora-agents/src/constraint.rs` | 8 hours |
| **Phase 2** | Treewidth Calculation | Agent C | `crates/axora-agents/src/treewidth.rs` | 8 hours |
| **Phase 3** | AOP Validator | Agent C | `crates/axora-agents/src/aop_validator.rs` | 8 hours |
| **Phase 4** | DAG Builder | Agent C | `crates/axora-agents/src/dag_builder.rs` | 8 hours |

### Documents
- [`ACONIC-DECOMPOSITION-DESIGN.md`](./ACONIC-DECOMPOSITION-DESIGN.md) — Main design spec
- [`AOP-VALIDATOR-SPEC.md`](./AOP-VALIDATOR-SPEC.md) — Validator specification
- [`RESEARCH-SUMMARY.md`](./RESEARCH-SUMMARY.md) — Research context

---

## 🔀 Concurrent Sprints Analysis

### 3 Sprints That Can Run Concurrently NOW

#### Track A: Token Optimization (axora-cache crate)

**Sprint 3: Code Minification**
- **Owner:** Agent A
- **Files:** `crates/axora-cache/src/minifier.rs`
- **Dependencies:** None (standalone utility)
- **Risk:** Low (pure function, no state)

**Sprint 5: TOON Serialization**
- **Owner:** Agent B
- **Files:** `crates/axora-cache/src/toon.rs`
- **Dependencies:** None (standalone serializer)
- **Risk:** Low (pure function, no state)

**Why concurrent:**
- ✅ Different files (no overlap)
- ✅ No shared state
- ✅ Both are pure functions
- ✅ Can be tested independently
- ✅ Merge conflicts unlikely

---

#### Track B: Agent Infrastructure (axora-agents crate)

**Sprint 3b: Heartbeat System**
- **Owner:** Agent C
- **Files:** `crates/axora-agents/src/heartbeat.rs`
- **Dependencies:** State machine (already exists)
- **Risk:** Medium (integrates with state machine)

**Why concurrent with Track A:**
- ✅ Different crate (axora-agents vs axora-cache)
- ✅ No shared code
- ✅ Independent testing
- ✅ Different concerns (orchestration vs optimization)

---

### 3 Additional Sprints for Future Concurrency

#### Track C: Documentation System (NEW)

**Sprint 6a: Doc Format & Schema**
- **Owner:** Agent D
- **Files:** `crates/axora-docs/src/format.rs`, `crates/axora-docs/src/schema.rs`
- **Dependencies:** None (new crate)
- **Risk:** Low (new crate, isolated)

**Sprint 6b: Doc RAG & Retrieval**
- **Owner:** Agent E
- **Files:** `crates/axora-docs/src/retriever.rs`
- **Dependencies:** Doc format (Sprint 6a must complete first)
- **Risk:** Medium (depends on 6a)

**Sprint 6c: Living Docs & Auto-Update**
- **Owner:** Agent F
- **Files:** `crates/axora-docs/src/living.rs`
- **Dependencies:** Doc format + RAG (6a + 6b must complete)
- **Risk:** High (complex integration)

**Why concurrent (with dependencies):**
- ✅ Clear dependency chain (6a → 6b → 6c)
- ✅ Agents can start 6a immediately
- ✅ Agent E can prepare while 6a completes
- ✅ Agent F can design while 6a/6b progress

---

## 📊 Concurrency Matrix

| Sprint | Crate | Files | Dependencies | Can Run With |
|--------|-------|-------|--------------|--------------|
| **3: Code Minification** | axora-cache | `minifier.rs` | None | 5, 3b |
| **5: TOON Serialization** | axora-cache | `toon.rs` | None | 3, 3b |
| **3b: Heartbeat** | axora-agents | `heartbeat.rs` | State machine | 3, 5 |
| **6a: Doc Format** | axora-docs (new) | `format.rs`, `schema.rs` | None | 6b (after), 6c (after) |
| **6b: Doc RAG** | axora-docs | `retriever.rs` | 6a | 6c (after) |
| **6c: Living Docs** | axora-docs | `living.rs` | 6a, 6b | None (last) |

---

## 🚀 Recommended Parallel Execution

### Wave 1: Start NOW (3 concurrent agents)

**Agent A:** Sprint 3 (Code Minification)  
**Agent B:** Sprint 5 (TOON Serialization)  
**Agent C:** Sprint 3b (Heartbeat)

**Duration:** ~8 hours each  
**Risk:** Low (different crates, no overlap)  
**Coordination:** Minimal (weekly sync is enough)

---

### Wave 2: After Wave 1 Completes (3 concurrent agents)

**Agent D:** Sprint 6a (Doc Format)  
**Agent E:** Sprint 6b prep (design while 6a completes)  
**Agent F:** Sprint 6c design (while 6a/6b progress)

**Duration:** ~16 hours total (staggered start)  
**Risk:** Medium (dependencies require coordination)  
**Coordination:** Daily sync between D/E/F

---

### Wave 3: ACONIC Implementation (NEW)

**Agent C:** Sprint 7 (Task Decomposition Implementation)

**Status:** R-10 validation complete — DDD REJECTED, ACONIC ADOPTED
**Duration:** ~32 hours (4 phases × 8 hours)
**Risk:** Medium (mathematical foundation, well-specified)
**Coordination:** Requires Graph Workflow Design complete

**Files:**
- `crates/axora-agents/src/constraint.rs` — Constraint parsing
- `crates/axora-agents/src/treewidth.rs` — Treewidth calculation
- `crates/axora-agents/src/aop_validator.rs` — AOP validation
- `crates/axora-agents/src/dag_builder.rs` — DAG construction

---

## ⚠️ Conflict Prevention

### File Boundaries

**axora-cache crate:**
```
src/
├── l1_cache.rs      # Agent A (Sprint 3)
├── l2_cache.rs      # Existing
├── l3_cache.rs      # Existing
├── prefix_cache.rs  # Existing (Sprint 1)
├── diff.rs          # Existing (Sprint 2)
├── minifier.rs      # Agent A (Sprint 3) ← NEW
├── toon.rs          # Agent B (Sprint 5) ← NEW
└── lib.rs           # Coordinate merges
```

**axora-agents crate:**
```
src/
├── agent.rs              # Existing
├── capabilities.rs       # Existing
├── conflict.rs           # Existing
├── state_machine.rs      # Existing
├── memory.rs             # Existing
├── communication.rs      # Existing
├── heartbeat.rs          # Agent C (Sprint 3b) ← NEW
└── lib.rs                # Coordinate merges
```

**axora-docs crate (NEW):**
```
src/
├── format.rs      # Agent D (Sprint 6a) ← NEW
├── schema.rs      # Agent D (Sprint 6a) ← NEW
├── retriever.rs   # Agent E (Sprint 6b) ← NEW
├── living.rs      # Agent F (Sprint 6c) ← NEW
└── lib.rs         # Coordinate merges
```

---

## 📋 Merge Strategy

### For Wave 1 (Low Risk)

1. **Each agent works on separate files**
2. **Daily git pull** to stay in sync
3. **PR review** by another agent before merge
4. **Run tests** before merging (`cargo test --workspace`)

### For Wave 2 (Medium Risk)

1. **Agent D (6a) merges first** (foundation)
2. **Agent E (6b) waits for 6a** before merging
3. **Agent F (6c) waits for 6a + 6b** before merging
4. **Daily sync** to coordinate dependencies

### For Wave 3 (Medium Risk)

1. **Requires design docs complete** (ACONIC-DECOMPOSITION-DESIGN.md)
2. **Small, incremental PRs** (one phase at a time)
3. **Test after each merge** (unit tests for each component)
4. **Validation with real missions** (ensure AOP passes)

---

## 🎯 Agent Assignment Recommendations

### Wave 1 Agents (Start Immediately)

| Agent | Sprint | Crate | Files | Estimated Time |
|-------|--------|-------|-------|----------------|
| **Agent A** | 3 | axora-cache | `minifier.rs` | 8 hours |
| **Agent B** | 5 | axora-cache | `toon.rs` | 8 hours |
| **Agent C** | 3b | axora-agents | `heartbeat.rs` | 8 hours |

### Wave 2 Agents (After Wave 1)

| Agent | Sprint | Crate | Files | Estimated Time |
|-------|--------|-------|-------|----------------|
| **Agent D** | 6a | axora-docs | `format.rs`, `schema.rs` | 8 hours |
| **Agent E** | 6b | axora-docs | `retriever.rs` | 8 hours |
| **Agent F** | 6c | axora-docs | `living.rs` | 8 hours |

### Wave 3 Agent (ACONIC Implementation)

| Agent | Sprint | Crate | Files | Estimated Time |
|-------|--------|-------|-------|----------------|
| **Agent C** | 7 | axora-agents | `constraint.rs`, `treewidth.rs`, `aop_validator.rs`, `dag_builder.rs` | 32 hours |

---

## 📊 Timeline

```
Week 1-2:
├─ Wave 1: Sprints 3, 5, 3b (concurrent)
│  ├─ Agent A: Code Minification ✅
│  ├─ Agent B: TOON Serialization ✅
│  └─ Agent C: Heartbeat ✅
│
└─ R-10: DDD Validation (separate track) → REJECTED

Week 3-4:
├─ Wave 2: Sprints 6a, 6b, 6c (staggered)
│  ├─ Agent D: Doc Format ✅
│  ├─ Agent E: Doc RAG ✅
│  └─ Agent F: Living Docs ✅
│
└─ R-11: Concurrency + ReAct → ADOPTED

Week 5-6:
├─ Wave 3: Sprint 7 (ACONIC Implementation)
│  └─ Agent C: Constraint Parsing + Treewidth + AOP + DAG
│
├─ Sprint 9: Integration & Benchmarking ✅
├─ Sprint 11: Graph Workflow Design ✅
└─ Sprint 12: ACONIC Documentation ✅

Week 7+:
├─ Sprint 8: Context Distribution
├─ Sprint 5: TOON Serialization (if still needed)
└─ Phase 3: Desktop App Planning
```

---

## ✅ Next Steps

1. **Agent C:** Start ACONIC implementation (Sprint 7)
   - Follow ACONIC-DECOMPOSITION-DESIGN.md
   - Follow AOP-VALIDATOR-SPEC.md
   - Implement in 4 phases (constraint → treewidth → AOP → DAG)

2. **All agents:** Review RESEARCH-SUMMARY.md for context

3. **No conflicts expected** if agents stay in assigned files.

---

**Ready to dispatch Agent C for ACONIC implementation.**
