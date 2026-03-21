# Phase 3 — Coordinator Agent (Self-Orchestration)

**Priority:** 🔴 CRITICAL  
**Estimated Duration:** 2-3 weeks (6-8 sprints)  
**Goal:** Eliminate manual coordination — user talks to ONE agent, it manages everything

---

## 🎯 Problem We're Solving

**Current State (Phase 2):**
```
User → Opens 3 terminals
     → Copies prompts manually
     → Manages dependencies between agents
     → Merges results manually
     → Is "babysitter" + "pigeon courier"
```

**Desired State (Phase 3):**
```
User → "Implement authentication system"
     →
Coordinator → Decomposes task
            → Dispatches to workers
            → Monitors progress
            → Merges results
            → Reports to user
     →
User → "Done. Here's what I built."
```

---

## 📋 Sprint Breakdown

### Sprint 1: Coordinator Core Structure
**File:** `crates/openakta-agents/src/coordinator/v2.rs`
- Coordinator agent struct
- Worker registry
- Task queue management
- Basic dispatch mechanism

**Deliverables:**
- `Coordinator` struct with worker management
- `dispatch_task()` method
- `monitor_progress()` method
- 10+ tests

**Estimated:** 8 hours

---

### Sprint 2: Task Decomposition Engine
**File:** `crates/openakta-agents/src/decomposer/v2.rs`
- LLM-based mission decomposition
- Dependency graph construction
- Parallel group identification
- Critical path calculation

**Deliverables:**
- `MissionDecomposer` with LLM integration
- `decompose(mission)` → `DecomposedMission`
- Dependency graph (DAG)
- 10+ tests

**Estimated:** 8 hours

---

### Sprint 3: Worker Agent Pool
**File:** `crates/openakta-agents/src/worker_pool.rs`
- Dynamic worker spawning
- Worker lifecycle management
- Health monitoring
- Auto-restart on failure

**Deliverables:**
- `WorkerPool` struct
- `spawn_worker()`, `terminate_worker()`
- Health check system
- 10+ tests

**Estimated:** 8 hours

---

### Sprint 4: Blackboard (Shared State)
**File:** `crates/openakta-cache/src/blackboard/v2.rs`
- Shared state for all agents
- Versioned context (prevents conflicts)
- Subscribe/notify pattern
- Atomic updates

**Deliverables:**
- `Blackboard` with versioning
- `subscribe()`, `publish()`, `notify()`
- Conflict resolution
- 10+ tests

**Estimated:** 8 hours

---

### Sprint 5: Context Compacting
**File:** `crates/openakta-cache/src/compactor.rs`
- Rolling summary (last N turns)
- Hierarchical memory (recent full, old summarized)
- Importance scoring (prune low-importance)
- Token budget enforcement

**Deliverables:**
- `ContextCompactor` struct
- `compact(context)` → `CompactContext`
- 60-80% token reduction
- 10+ tests

**Estimated:** 8 hours

---

### Sprint 6: Progress Monitoring & Reporting
**File:** `crates/openakta-agents/src/monitor.rs`
- Real-time progress tracking
- ETA calculation
- Blocker detection
- User-facing status reports

**Deliverables:**
- `ProgressMonitor` struct
- `get_status()` → `StatusReport`
- Blocker alerts
- 10+ tests

**Estimated:** 8 hours

---

## 🎯 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| User Interventions | <1 per mission | Manual count |
| Task Dispatch Time | <5 seconds | Automated timing |
| Context Token Reduction | 60-80% | Before/after comparison |
| Coordinator Overhead | <10% of total time | Profiling |
| Worker Utilization | >80% | Monitoring dashboard |

---

## 🔗 Dependencies

**From Phase 2 (Complete ✅):**
- Heartbeat System (agent lifecycle)
- Graph Workflow (deterministic execution)
- Dual-Thread ReAct (interruptible agents)
- Memory Architecture (semantic, episodic, procedural)

**New for Phase 3:**
- Blackboard v2 (shared state with versioning)
- Context Compacting (token reduction)
- Progress Monitoring (user visibility)

---

## 📅 Timeline

| Week | Sprints | Milestone |
|------|---------|-----------|
| 1 | 1, 2 | Coordinator + Decomposition |
| 2 | 3, 4 | Worker Pool + Blackboard |
| 3 | 5, 6 | Compacting + Monitoring |

**Total:** 3 weeks, 6 sprints, ~48 hours

---

## 🚀 After Phase 3

**Phase 4: Desktop App (Tauri + React)**
- Chat UI (talk to Coordinator)
- Progress visualization
- Configuration (BYOK vs subscription)
- 8-10 sprints (~3-4 weeks)

**Phase 5: Beta Testing**
- 5-10 beta users
- E2E testing
- Feedback iteration
- 4-6 sprints (~2 weeks)

**Phase 6: Production Launch**
- Installers (.dmg, .exe, .deb)
- Auto-update
- Documentation
- Marketing

---

## 💡 Recommended Next Action

**Start Phase 3, Sprint 1: Coordinator Core Structure**

**Why:**
1. Solves YOUR pain point (manual coordination)
2. Foundation for everything else
3. Can test immediately (dogfooding)
4. Clear differentiator vs competitors

**Command:**
```
Agent C: Start Phase 3 Sprint 1
File: planning/phase-3/COORDINATOR-CORE.md
Priority: CRITICAL
```

---

**Ready to start Phase 3?**
