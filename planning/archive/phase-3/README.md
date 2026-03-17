# Phase 3 Agent Tasks

**Phase:** 3 — Coordinator Agent (Self-Orchestration)  
**Timeline:** 2-3 weeks (6-8 sprints)  
**Goal:** User talks to ONE agent, it manages everything

---

## 📁 Folder Structure

```
planning/phase-3/
├── agent-a/          # Documentation + Memory specialist
│   └── (tasks go here)
├── agent-b/          # Storage + Context specialist
│   └── (tasks go here)
└── agent-c/          # Implementation specialist (Coordinator core)
    └── (tasks go here)
```

---

## 🎯 Phase 3 Overview

**What:** Self-orchestration system

**Before (Phase 2):**
```
User → Opens 3 terminals
     → Copies prompts manually
     → Manages dependencies
     → Merges results manually
```

**After (Phase 3):**
```
User → "Implement authentication"
     →
Coordinator → Decomposes → Dispatches → Monitors → Merges
     →
User → "Done. Here's what I built."
```

---

## 📋 Sprint Breakdown

### Agent A (Documentation + Memory)
- **Sprint 2:** Context Compacting (rolling summary, hierarchical memory)
- **Sprint 4:** Blackboard v2 (shared state with versioning)
- **Sprint 6:** Progress Monitoring & Reporting

### Agent B (Storage + Context)
- **Sprint 3:** Worker Agent Pool (dynamic spawning, lifecycle)
- **Sprint 5:** Task Queue Management (priority, dependencies)

### Agent C (Implementation — Coordinator Core)
- **Sprint 1:** Coordinator Core Structure ← **START HERE**
- **Sprint 2:** Task Decomposition Engine (LLM-based)

---

## 🚀 Getting Started

**First Task:** Agent C — Sprint 1

**File:** `planning/phase-3/SPRINT-1-COORDINATOR-CORE.md`

**Command:**
```
Agent C: Start Phase 3 Sprint 1
File: planning/phase-3/SPRINT-1-COORDINATOR-CORE.md
Priority: CRITICAL
```

---

## 📊 Success Metrics

| Metric | Target |
|--------|--------|
| User Interventions | <1 per mission |
| Task Dispatch Time | <5 seconds |
| Context Token Reduction | 60-80% |
| Coordinator Overhead | <10% of total time |
| Worker Utilization | >80% |

---

**Ready to start Phase 3!**
