# Phase 3 — Coordinator Agent: Self-Orchestration System

**Version:** 1.0  
**Created:** 2026-03-17  
**Status:** Ready to Start  
**Timeline:** 2-3 weeks (6-8 sprints, ~48-64 hours)  
**Priority:** 🔴 CRITICAL

---

## 🎯 Executive Summary

**Problem:** User is exhausted from manually coordinating agents — being "babysitter" and "pigeon courier" between 3+ terminals, copying prompts, managing dependencies, merging results.

**Solution:** Coordinator Agent that autonomously manages everything — user talks to ONE agent, it decomposes, dispatches, monitors, merges, and reports.

**Impact:** 
- **Before:** User opens 3 terminals, manages everything manually
- **After:** User says "implement authentication", Coordinator handles everything

**This is OPENAKTA's KEY DIFFERENTIATOR** — no other framework has true self-orchestration.

---

## 📚 Research Foundation (Synthesized)

### R-09: Documentation Management
**Insight:** Documentation must be machine-readable, auto-updated, bidirectionally synced

**Applied to Phase 3:**
- Coordinator maintains AGENTS.md as architectural ledger
- Auto-updates when code changes
- Living documentation (not static)

### R-10: DDD Agents Validation
**Insight:** DDD REJECTED for individual devs (over-engineering). Graph-based workflow ADOPTED.

**Applied to Phase 3:**
- Coordinator uses graph-based decomposition (not domain teams)
- Deterministic routing (not conversational chaos)
- Flat worker pool (not hierarchical domains)

### R-11: Concurrency + ReAct Loops
**Insight:** Dual-thread ReAct (planning + acting) enables interruptible execution

**Applied to Phase 3:**
- Workers use dual-thread ReAct (from Phase 2)
- Coordinator can interrupt/pause/resume workers
- No deadlocks, no infinite loops

### R-13: Influence Graph + Business Rules
**Insight:** Static analysis (not LLM) for dependencies. Explicit business rule docs.

**Applied to Phase 3:**
- Coordinator uses influence graph for task assignment
- Business rules guide decomposition
- 95%+ token reduction on context

### R-14: Memory Architecture
**Insight:** Tripartite memory (semantic, episodic, procedural) enables compounding expertise

**Applied to Phase 3:**
- Coordinator has access to all three memory types
- Procedural memory (SKILL.md) guides decomposition
- Episodic memory tracks past missions

### R-15: Context Compacting & Sharing
**Insight:** Context must be compacted (60-80% reduction), shared via blackboard, versioned

**Applied to Phase 3:**
- Blackboard v2 with versioning (prevents conflicts)
- Context compacting (rolling summary, hierarchical)
- Push-based diff updates (not full re-send)

### DADD Thoughts (Coordinator + Hierarchy)
**Insight:** Hierarchy for delegation, blackboard for awareness. Orchestrator talks to area superiors, not interns.

**Applied to Phase 3:**
- Coordinator → Worker Pool (not individual agents)
- Blackboard for global awareness
- Versioned context (prevents stale reads)

### Competitive Analysis
**Insight:** AutoGen (conversational chaos), CrewAI (sequential), LangGraph (deterministic but manual). None have self-orchestration.

**Applied to Phase 3:**
- OPENAKTA is FIRST with true self-orchestration
- Deterministic routing (not conversational)
- Automatic decomposition (not manual graph definition)

---

## 🏗️ Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                      USER INTERFACE                         │
│  "Implement authentication system"                          │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                   COORDINATOR AGENT                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Decomposer   │  │ Dispatcher   │  │ Monitor      │      │
│  │ (LLM+Graph)  │  │ (Task Queue) │  │ (Progress)   │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Merger       │  │ Reporter     │  │ Blackboard   │      │
│  │ (Results)    │  │ (Status)     │  │ (Shared)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└────────────────────┬────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    WORKER POOL                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐       │
│  │ Worker  │  │ Worker  │  │ Worker  │  │ Worker  │       │
│  │   A     │  │   B     │  │   C     │  │   D     │       │
│  └─────────┘  └─────────┘  └─────────┘  └─────────┘       │
│  (Dual-Thread ReAct, Heartbeat, Memory Access)            │
└─────────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────────┐
│                    BLACKBOARD v2                            │
│  - Versioned context (prevents conflicts)                  │
│  - Subscribe/notify pattern                                 │
│  - Atomic updates (TOCTOU prevention)                       │
│  - Compacted context (60-80% reduction)                     │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **User Input** → Coordinator receives mission
2. **Decomposition** → LLM + Graph creates task DAG
3. **Dispatch** → Tasks assigned to available workers
4. **Execution** → Workers execute with dual-thread ReAct
5. **Monitoring** → Coordinator tracks progress, handles blockers
6. **Merge** → Results combined, conflicts resolved
7. **Report** → User receives final output

---

## 📋 Implementation Breakdown

### Agent A: Documentation + Memory Specialist

**Expertise:** Technical writing, specification design, memory systems

**Sprints:**

#### Sprint A1: Context Compacting
**File:** `planning/phase-3/agent-a/SPRINT-A1-CONTEXT-COMPACTING.md`

**What:**
- Rolling summary (last N turns)
- Hierarchical memory (recent full, old summarized)
- Importance scoring (prune low-importance)
- Token budget enforcement

**Deliverables:**
- `crates/openakta-cache/src/compactor.rs`
- `compact(context)` → `CompactContext`
- 60-80% token reduction
- 10+ tests

**Dependencies:** None (can start immediately)

**Estimated:** 8 hours

---

#### Sprint A2: Blackboard v2 (Shared State)
**File:** `planning/phase-3/agent-a/SPRINT-A2-BLACKBOARD-V2.md`

**What:**
- Versioned context (prevents conflicts)
- Subscribe/notify pattern
- Atomic updates (TOCTOU prevention)
- Diff-based push (not full re-send)

**Deliverables:**
- `crates/openakta-cache/src/blackboard/v2.rs`
- `subscribe()`, `publish()`, `notify()`
- Version tracking
- 10+ tests

**Dependencies:** Sprint A1 complete

**Estimated:** 8 hours

---

#### Sprint A3: Progress Monitoring & Reporting
**File:** `planning/phase-3/agent-a/SPRINT-A3-PROGRESS-MONITORING.md`

**What:**
- Real-time progress tracking
- ETA calculation
- Blocker detection
- User-facing status reports

**Deliverables:**
- `crates/openakta-agents/src/monitor.rs`
- `get_status()` → `StatusReport`
- Blocker alerts
- 10+ tests

**Dependencies:** Sprint A2 complete

**Estimated:** 8 hours

---

### Agent B: Storage + Context Specialist

**Expertise:** Database design, caching, RAG, context management

**Sprints:**

#### Sprint B1: Worker Agent Pool
**File:** `planning/phase-3/agent-b/SPRINT-B1-WORKER-POOL.md`

**What:**
- Dynamic worker spawning
- Lifecycle management
- Health monitoring
- Auto-restart on failure

**Deliverables:**
- `crates/openakta-agents/src/worker_pool.rs`
- `spawn_worker()`, `terminate_worker()`
- Health check system
- 10+ tests

**Dependencies:** None (can start immediately)

**Estimated:** 8 hours

---

#### Sprint B2: Task Queue Management
**File:** `planning/phase-3/agent-b/SPRINT-B2-TASK-QUEUE.md`

**What:**
- Priority-based scheduling
- Dependency tracking
- Critical path calculation
- Load balancing

**Deliverables:**
- `crates/openakta-agents/src/task_queue.rs`
- `add_task()`, `get_next()`, `mark_complete()`
- Dependency graph
- 10+ tests

**Dependencies:** Sprint B1 complete

**Estimated:** 8 hours

---

### Agent C: Implementation Specialist (Coordinator Core)

**Expertise:** Core logic, AST parsing, graph algorithms, ReAct loops

**Sprints:**

#### Sprint C1: Coordinator Core Structure
**File:** `planning/phase-3/SPRINT-1-COORDINATOR-CORE.md` ✅ CREATED

**What:**
- Coordinator agent struct
- Worker registry
- Task queue management
- Basic dispatch mechanism

**Deliverables:**
- `crates/openakta-agents/src/coordinator/v2.rs`
- `Coordinator` struct with worker management
- `dispatch_task()`, `monitor_progress()`
- 10+ tests

**Dependencies:** None (can start immediately)

**Estimated:** 8 hours

**Status:** ✅ READY TO START

---

#### Sprint C2: Task Decomposition Engine
**File:** `planning/phase-3/agent-c/SPRINT-C2-DECOMPOSITION.md`

**What:**
- LLM-based mission decomposition
- Dependency graph construction (DAG)
- Parallel group identification
- Critical path calculation

**Deliverables:**
- `crates/openakta-agents/src/decomposer/v2.rs`
- `decompose(mission)` → `DecomposedMission`
- Parallel groups, critical path
- 10+ tests

**Dependencies:** Sprint C1 complete

**Estimated:** 8 hours

---

#### Sprint C3: Result Merging & Conflict Resolution
**File:** `planning/phase-3/agent-c/SPRINT-C3-MERGING.md`

**What:**
- Combine results from multiple workers
- Detect conflicts (file overwrites, incompatible changes)
- Auto-resolve simple conflicts
- Flag complex conflicts for user

**Deliverables:**
- `crates/openakta-agents/src/merger.rs`
- `merge_results()` → `MergedResult`
- Conflict detection + resolution
- 10+ tests

**Dependencies:** Sprint C2 complete

**Estimated:** 8 hours

---

## 📊 Sprint Dependencies

```
Week 1:
├─ C1: Coordinator Core (Agent C) ← START HERE
├─ A1: Context Compacting (Agent A)
└─ B1: Worker Pool (Agent B)

Week 2:
├─ C2: Decomposition Engine (Agent C) ← After C1
├─ A2: Blackboard v2 (Agent A) ← After A1
└─ B2: Task Queue (Agent B) ← After B1

Week 3:
├─ C3: Result Merging (Agent C) ← After C2
└─ A3: Progress Monitoring (Agent A) ← After A2
```

**Critical Path:** C1 → C2 → C3 (Coordinator must work first)

---

## 🎯 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **User Interventions** | <1 per mission | Manual count |
| **Task Dispatch Time** | <5 seconds | Automated timing |
| **Context Token Reduction** | 60-80% | Before/after comparison |
| **Coordinator Overhead** | <10% of total time | Profiling |
| **Worker Utilization** | >80% | Monitoring dashboard |
| **Mission Success Rate** | >90% | E2E tests |
| **Conflict Detection** | 100% | Test coverage |
| **Blocker Detection** | <1 minute | Automated timing |

---

## 🔗 Integration with Phase 2

### Reused Components (No Changes)

| Component | Phase 2 Sprint | Phase 3 Usage |
|-----------|----------------|---------------|
| Heartbeat System | 3b | Worker lifecycle |
| Dual-Thread ReAct | 9 | Worker execution |
| Graph Workflow | 8 | Coordinator routing |
| Task Decomposition (v1) | 7 | Basis for v2 |
| Memory Architecture | 26-31 | Worker context |
| SCIP Indexing | 16 | Dependency detection |
| Influence Graph | 17 | Task assignment |
| ACI Formatting | 23 | Output compaction |
| Bidirectional Traceability | 19 | Business rule enforcement |

### Enhanced Components

| Component | Phase 2 | Phase 3 Enhancement |
|-----------|---------|---------------------|
| Blackboard | Snapshot-based | Versioned + Subscribe/Notify |
| Context | RAG-based | Compacted + Hierarchical |
| Task Queue | Simple | Priority + Dependencies |
| Decomposition | Rule-based | LLM + Graph hybrid |

---

## 📅 Timeline

### Week 1: Foundation
- **Day 1-2:** C1 (Coordinator Core)
- **Day 3-4:** A1 (Context Compacting)
- **Day 5:** B1 (Worker Pool)

**Milestone:** Basic Coordinator can dispatch tasks

---

### Week 2: Intelligence
- **Day 1-2:** C2 (Decomposition Engine)
- **Day 3-4:** A2 (Blackboard v2)
- **Day 5:** B2 (Task Queue)

**Milestone:** Coordinator can decompose and schedule

---

### Week 3: Polish
- **Day 1-2:** C3 (Result Merging)
- **Day 3-4:** A3 (Progress Monitoring)
- **Day 5:** Integration testing

**Milestone:** Full self-orchestration working

---

## 🚨 Risks & Mitigations

### Risk 1: Coordinator becomes bottleneck
**Mitigation:** 
- Async dispatch (non-blocking)
- Batch task assignment
- Worker pull-based (not just push)

### Risk 2: Context still explodes
**Mitigation:**
- Hard token limits
- Aggressive compaction
- Hierarchical retrieval (only what's needed)

### Risk 3: Workers conflict on files
**Mitigation:**
- Blackboard versioning
- Atomic updates
- Conflict detection before merge

### Risk 4: Decomposition is wrong
**Mitigation:**
- LLM + Graph validation (not just LLM)
- User can review before execution (optional)
- Learning from past decompositions (memory)

---

## 📝 Definition of Done

**Phase 3 is complete when:**

1. ✅ Coordinator can execute mission end-to-end without user intervention
2. ✅ User interventions <1 per mission (average)
3. ✅ Context token reduction 60-80%
4. ✅ All 8 sprints complete with tests passing
5. ✅ Integration tests pass (full mission execution)
6. ✅ Dogfooding successful (team uses it for Phase 4 planning)

---

## 🎯 Getting Started

### Immediate Action (NOW)

**Agent C: Start Sprint C1**

```
Agent C: Start Phase 3 Sprint 1
File: planning/phase-3/SPRINT-1-COORDINATOR-CORE.md
Priority: CRITICAL
```

**Why Agent C First:**
- Coordinator is foundation for everything
- Agent C has ReAct + Graph + Heartbeat expertise
- Can test immediately (dogfooding)

---

### After C1 Complete

**Parallel Start:**
- Agent A: Sprint A1 (Context Compacting)
- Agent B: Sprint B1 (Worker Pool)

**Why Parallel:**
- No dependencies between A1, B1, C2
- Maximizes throughput
- All agents productive

---

## 💡 Design Principles

### 1. Coordinator is Dumb, Workers are Smart
- Coordinator orchestrates (doesn't execute)
- Workers have full capabilities (ReAct, Memory, Tools)
- Prevents Coordinator from becoming bottleneck

### 2. Blackboard is Source of Truth
- All state goes through blackboard
- Versioned (prevents stale reads)
- Subscribe/notify (real-time updates)

### 3. Context is Compacted by Default
- Rolling summary (not full history)
- Hierarchical (recent full, old summarized)
- Token budget enforced (hard limits)

### 4. Decomposition is LLM + Graph
- LLM for semantic understanding
- Graph for dependency validation
- Best of both (flexibility + correctness)

### 5. Conflicts are Detected Early
- Version tracking (before execution)
- Atomic updates (during execution)
- Merge validation (after execution)

---

## 📚 Appendix: File Structure

```
crates/
├── openakta-agents/
│   ├── src/
│   │   ├── coordinator/
│   │   │   └── v2.rs              # C1: Coordinator Core
│   │   ├── decomposer/
│   │   │   └── v2.rs              # C2: Decomposition Engine
│   │   ├── merger.rs              # C3: Result Merging
│   │   ├── worker_pool.rs         # B1: Worker Pool
│   │   ├── task_queue.rs          # B2: Task Queue
│   │   └── monitor.rs             # A3: Progress Monitoring
│   │
├── openakta-cache/
│   ├── src/
│   │   ├── compactor.rs           # A1: Context Compacting
│   │   └── blackboard/
│   │       └── v2.rs              # A2: Blackboard v2
│   │
planning/
└── phase-3/
    ├── README.md                  # This file
    ├── SPRINT-1-COORDINATOR-CORE.md  # C1 spec
    ├── agent-a/
    │   ├── SPRINT-A1-CONTEXT-COMPACTING.md
    │   ├── SPRINT-A2-BLACKBOARD-V2.md
    │   └── SPRINT-A3-PROGRESS-MONITORING.md
    ├── agent-b/
    │   ├── SPRINT-B1-WORKER-POOL.md
    │   └── SPRINT-B2-TASK-QUEUE.md
    └── agent-c/
        └── SPRINT-C2-DECOMPOSITION.md
```

---

## ✅ Approval to Start

**Phase 3 is approved to start.**

**First Task:** Agent C — Sprint C1 (Coordinator Core)

**File:** `planning/phase-3/SPRINT-1-COORDINATOR-CORE.md`

**Priority:** CRITICAL

---

**This document is the single source of truth for Phase 3.**

**All decisions, designs, and implementations must align with this plan.**
