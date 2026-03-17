# Phase 3 — All Sprint Prompts Index

**Status:** ✅ ALL SPRINTS CREATED  
**Total Sprints:** 8 (A1-A3, B1-B2, C1-C3)  
**Subagents:** ENABLED (GPT-5.4 optimized)  
**Estimated Total:** 64 hours (8 sprints × 8 hours)

---

## 📋 Sprint Overview

| Agent | Sprint | Title | Priority | Subagents | Status |
|-------|--------|-------|----------|-----------|--------|
| **A** | A1 | Context Compacting | HIGH | 3 | ✅ Ready |
| **A** | A2 | Blackboard v2 | HIGH | 2 | ✅ Ready |
| **A** | A3 | Progress Monitoring | MEDIUM | 2 | ✅ Ready |
| **B** | B1 | Worker Agent Pool | CRITICAL | 4 | ✅ Ready |
| **B** | B2 | Task Queue Management | HIGH | 3 | ✅ Ready |
| **C** | C1 | Coordinator Core | CRITICAL | 3 | ✅ Ready |
| **C** | C2 | Task Decomposition | CRITICAL | 3 | ✅ Ready |
| **C** | C3 | Result Merging | HIGH | 2 | ✅ Ready |

---

## 🚀 Execution Order

### Week 1: Foundation
```
START HERE:
├─ C1: Coordinator Core (Agent C) ← CRITICAL PATH
├─ A1: Context Compacting (Agent A) ← Uses subagents
└─ B1: Worker Agent Pool (Agent B) ← HARDEST
```

### Week 2: Intelligence
```
After Week 1 complete:
├─ C2: Task Decomposition (Agent C) ← Needs C1
├─ A2: Blackboard v2 (Agent A) ← Needs A1
└─ B2: Task Queue (Agent B) ← Needs B1
```

### Week 3: Polish
```
After Week 2 complete:
├─ C3: Result Merging (Agent C) ← Needs C2
└─ A3: Progress Monitoring (Agent A) ← Needs A2
```

---

## 📁 Sprint Files

### Agent A (Documentation + Memory Specialist)
- [`agent-a/SPRINT-A1-CONTEXT-COMPACTING.md`](./agent-a/SPRINT-A1-CONTEXT-COMPACTING.md) — 3 subagents
- [`agent-a/SPRINT-A2-BLACKBOARD-V2.md`](./agent-a/SPRINT-A2-BLACKBOARD-V2.md) — 2 subagents
- [`agent-a/SPRINT-A3-PROGRESS-MONITORING.md`](./agent-a/SPRINT-A3-PROGRESS-MONITORING.md) — 2 subagents

### Agent B (Storage + Context Specialist — HARDEST)
- [`agent-b/SPRINT-B1-WORKER-POOL.md`](./agent-b/SPRINT-B1-WORKER-POOL.md) — 4 subagents ⚠️
- [`agent-b/SPRINT-B2-TASK-QUEUE.md`](./agent-b/SPRINT-B2-TASK-QUEUE.md) — 3 subagents ⚠️

### Agent C (Implementation Specialist — Coordinator Core)
- [`SPRINT-1-COORDINATOR-CORE.md`](./SPRINT-1-COORDINATOR-CORE.md) — 3 subagents (C1)
- [`agent-c/SPRINT-C2-DECOMPOSITION.md`](./agent-c/SPRINT-C2-DECOMPOSITION.md) — 3 subagents
- [`agent-c/SPRINT-C3-MERGING.md`](./agent-c/SPRINT-C3-MERGING.md) — 2 subagents

---

## 🎯 Difficulty Distribution

| Agent | Sprints | Total Subagents | Difficulty | Why |
|-------|---------|-----------------|------------|-----|
| **A** | 3 | 7 | Medium | Memory/compaction (well-defined) |
| **B** | 2 | 7 | **HARDEST** | Concurrent worker management, DAG scheduling |
| **C** | 3 | 8 | High | Coordinator core (critical path) |

**Agent B gets hardest tasks** (concurrent state management, complex algorithms)

---

## 📊 Subagent Summary

**Total Subagents:** 22 across 8 sprints

**Breakdown:**
- Agent A: 7 subagents (3+2+2)
- Agent B: 7 subagents (4+3) ← Most complex
- Agent C: 8 subagents (3+3+2) ← Most critical

**Pattern:**
```
Lead Agent:
  ├─ Subagent 1: [Component] (parallel)
  ├─ Subagent 2: [Component] (parallel)
  └─ Subagent 3: [Component] (parallel)
  ↓
Lead Agent: Integration + Tests
```

---

## ✅ Success Criteria (Phase 3)

**All sprints complete when:**
- [ ] 8 sprints complete
- [ ] 80+ tests passing (10+ per sprint average)
- [ ] Coordinator can execute missions autonomously
- [ ] 60-80% token reduction achieved
- [ ] User interventions <1 per mission
- [ ] All subagents integrated and working

---

## 🔗 Dependencies Graph

```
C1 (Coordinator Core)
├─ Blocks: C2, A1, B1
└─ Required by: ALL

A1 (Context Compacting)
├─ Requires: None
├─ Blocks: A2, C2
└─ Parallel with: C1, B1

B1 (Worker Pool)
├─ Requires: None
├─ Blocks: B2, C1 (integration)
└─ Parallel with: C1, A1

A2 (Blackboard v2)
├─ Requires: A1
├─ Blocks: A3, C2, C3
└─ Parallel with: B2, C2

B2 (Task Queue)
├─ Requires: B1
├─ Blocks: C2
└─ Parallel with: A2, C2

C2 (Decomposition)
├─ Requires: C1, A1, B2
├─ Blocks: C3
└─ Parallel with: A3

A3 (Progress Monitoring)
├─ Requires: A2
├─ Blocks: None
└─ Final sprint

C3 (Result Merging)
├─ Requires: C2, A2
├─ Blocks: None
└─ Final sprint
```

---

## 📝 Notes for Execution

**All prompts written in English** (LLM-to-LLM, not for humans)

**GPT-5.4 Subagents:**
- Each sprint specifies exact subagent tasks
- Lead agent coordinates integration
- Parallel execution where possible

**Critical Path:**
```
C1 → C2 → C3 (Coordinator must work first)
```

**Start with C1** (Coordinator Core) — foundation for everything else.

---

**ALL SPRINTS READY. START WITH C1.**
