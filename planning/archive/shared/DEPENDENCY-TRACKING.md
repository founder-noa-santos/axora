# Dependency Tracking — Phase 2 Sprints

**Date:** 2026-03-16  
**Purpose:** Track dependencies between sprints (minimize cross-agent dependencies)

---

## 📊 Sprint Dependency Matrix

| Sprint | Agent | Dependencies | Blocked By | Blocks |
|--------|-------|--------------|------------|--------|
| **11** | A | None | — | 12 |
| **11** | B | None | — | 12 |
| **8** | C | None | — | 9 |
| **12** | A | None | — | 9 (C) |
| **12** | B | None | — | 9 (C) |
| **9** | C | A-12, B-12 | A-12, B-12 | — |
| **16** | B | None | — | 17 |
| **17** | B | 16 | 16 | 20 |
| **18** | A | None | — | 19 (C) |
| **19** | C | 18 | 18 (A) | — |
| **20** | B | 17 | 17 | — |

---

## 🔄 Dependency Groups (Same Agent When Possible)

### Group 1: Graph-Based Workflow (Agent C)
- **Sprint 8:** Graph Workflow Implementation
- **Sprint 9:** Dual-Thread ReAct
- **Dependency:** Needs A-12 (ACONIC docs) + B-12 (Blackboard)
- **Why same agent:** Sequential implementation (graph → ReAct)

### Group 2: Blackboard + Context (Agent B)
- **Sprint 11:** Context + RAG Pivot
- **Sprint 12:** Snapshot Blackboard
- **Sprint 16:** SCIP Indexing
- **Sprint 17:** Influence Vector
- **Sprint 20:** Context Pruning
- **Why same agent:** All related to context/storage layer

### Group 3: Documentation + Rules (Agent A)
- **Sprint 11:** Documentation Pivot
- **Sprint 12:** ACONIC Decomposition Docs
- **Sprint 18:** Business Rule Documentation
- **Why same agent:** All documentation-focused

### Group 4: Traceability (Agent C)
- **Sprint 19:** Bidirectional Traceability
- **Dependency:** Needs A-18 (Business Rule format)
- **Why Agent C:** Code annotation parsing (AST work, aligns with C's skills)

---

## ⚠️ Exceptional Cross-Agent Dependencies

### Dependency 1: C-9 Blocked by A-12 + B-12

**Sprint C-9 (Dual-Thread ReAct) needs:**
- **A-12:** ACONIC Decomposition Docs (DAG structure)
- **B-12:** Snapshot Blackboard (shared state)

**Why Exceptional:**
- C-9 is the CORE execution engine
- Cannot implement without ACONIC docs (no DAG structure)
- Cannot implement without Blackboard (no shared state)

**Mitigation:**
- Agent A: Prioritize Sprint 12 (ACONIC docs)
- Agent B: Prioritize Sprint 12 (Blackboard)
- Agent C: Work on Sprint 8 (Graph Workflow) while waiting

**Status:** 🟡 **ACTIVE** — Agent A and B should complete Sprint 12 ASAP

---

### Dependency 2: C-19 Blocked by A-18

**Sprint C-19 (Bidirectional Traceability) needs:**
- **A-18:** Business Rule Documentation (Markdown + YAML format)

**Why Exceptional:**
- C-19 parses @req annotations in code
- Needs to know business rule format (from A-18)
- Links code → rules in graph

**Mitigation:**
- Agent A: Complete Sprint 18 before C starts 19
- Agent C: Work on other sprints (8, 9) while waiting

**Status:** 🟢 **FUTURE** — Not critical path yet

---

## 📋 Sprint Assignment Rationale

### Agent A (Documentation Specialist)
**Strengths:** Technical writing, specification design, research synthesis

**Sprints:**
- 11: Documentation Pivot ✅
- 12: ACONIC Decomposition Docs ✅
- 18: Business Rule Documentation (future)

**Why:** All documentation-focused, no implementation dependencies.

---

### Agent B (Storage + Context Specialist)
**Strengths:** Database design, caching, RAG, context management

**Sprints:**
- 11: Context + RAG Pivot ✅
- 12: Snapshot Blackboard ✅
- 16: SCIP Indexing (future)
- 17: Influence Vector (future)
- 20: Context Pruning (future)

**Why:** All related to storage/context layer, sequential implementation.

---

### Agent C (Implementation Specialist)
**Strengths:** Core logic, AST parsing, graph algorithms, ReAct loops

**Sprints:**
- 8: Graph Workflow ✅
- 9: Dual-Thread ReAct (blocked by A-12, B-12)
- 19: Bidirectional Traceability (future, blocked by A-18)

**Why:** All implementation-heavy, requires AST/graph skills.

---

## 🚨 Exceptional Dependencies Summary

| Dependency | Type | Severity | Mitigation |
|------------|------|----------|------------|
| **C-9 ← A-12 + B-12** | Cross-Agent | 🔴 HIGH | A and B prioritize Sprint 12 |
| **C-19 ← A-18** | Cross-Agent | 🟡 LOW | Future dependency, not critical path |

**All other dependencies are INTRA-AGENT (same agent, sequential implementation).**

---

## ✅ Dependency Resolution Protocol

### For Cross-Agent Dependencies (Exceptional Cases)

1. **Blocked agent notifies coordinator** (Agent C → Coordinator)
2. **Coordinator prioritizes blocking sprints** (A-12, B-12)
3. **Blocked agent works on alternative sprints** (C-8 while waiting for C-9)
4. **Coordinator notifies when unblocked** (A-12 + B-12 complete → C-9 unblocked)

### For Intra-Agent Dependencies (Same Agent)

1. **Agent manages own sequence** (no coordinator needed)
2. **Agent reports progress** (Sprint 11 complete → starting Sprint 12)
3. **No external blocking** (agent controls own timeline)

---

## 📊 Current Status

**Active Sprints:**
- ✅ Agent A: 11, 12 (no dependencies)
- ✅ Agent B: 11, 12 (no dependencies)
- ✅ Agent C: 8 (no dependencies)

**Blocked Sprints:**
- 🟡 Agent C: 9 (blocked by A-12, B-12)

**Future Sprints:**
- 📋 Agent A: 18 (no dependencies)
- 📋 Agent B: 16, 17, 20 (16 → 17 → 20)
- 📋 Agent C: 19 (blocked by A-18)

---

**Only 2 exceptional cross-agent dependencies. All others are intra-agent (sequential).**

**Coordinator should prioritize A-12 and B-12 to unblock C-9.**
