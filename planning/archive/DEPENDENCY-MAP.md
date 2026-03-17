# Phase 3 + Phase 4 — Complete Dependency Map

**Created:** 2026-03-17  
**Purpose:** Visual guide for parallel execution and dependencies

---

## 📊 Complete Sprint Overview

**Total Sprints:** 14 (8 Phase 3 + 6 Phase 4)

| Phase | Agent | Sprint | Title | Status |
|-------|-------|--------|-------|--------|
| **P3** | C | C1 | Coordinator Core | ✅ COMPLETE |
| **P3** | A | A1 | Context Compacting | ✅ COMPLETE |
| **P3** | B | B1 | Worker Pool | ✅ COMPLETE |
| **P3** | C | C2 | Task Decomposition | ✅ COMPLETE |
| **P3** | A | A2 | Blackboard v2 | 🔄 IN PROGRESS |
| **P3** | B | B2 | Task Queue | 🔄 IN PROGRESS |
| **P3** | C | C3 | Result Merging | 🔄 IN PROGRESS |
| **P3** | A | A3 | Progress Monitoring | ⏳ PENDING |
| **P4** | C | C4 | Tauri Setup | ⏳ READY |
| **P4** | A | A4 | UI Components | ⏳ READY |
| **P4** | B | B4 | Settings | ⏳ READY |
| **P4** | C | C5 | Chat Interface | ⏳ READY |
| **P4** | A | A5 | Progress Dashboard | ⏳ READY |
| **P4** | B | B5 | API Integration | ⏳ READY |
| **P4** | C | C6 | Integration + Polish | ⏳ READY |

---

## 🎯 Dependency Graph (Visual)

```
═══════════════════════════════════════════════════════════════════════════════
PHASE 3 (Backend — Coordinator Agent)
═══════════════════════════════════════════════════════════════════════════════

✅ C1 (Coordinator Core)
   └─ ✅ COMPLETE — Foundation for all Phase 3

✅ A1 (Context Compacting)
   └─ ✅ COMPLETE — 60-80% token reduction

✅ B1 (Worker Pool)
   └─ ✅ COMPLETE — Dynamic worker management

✅ C2 (Task Decomposition)
   └─ ✅ COMPLETE — LLM + Graph decomposition

🔄 A2 (Blackboard v2) ← START NOW
   ├─ Requires: A1 ✅
   └─ Blocks: A3, C3, B5

🔄 B2 (Task Queue) ← START NOW
   ├─ Requires: B1 ✅
   └─ Blocks: C2 (already done), C6

🔄 C3 (Result Merging) ← START NOW
   ├─ Requires: C2 ✅, A2 🔄
   └─ Blocks: None (final Phase 3 workflow)

⏳ A3 (Progress Monitoring) ← AFTER A2
   ├─ Requires: A2 🔄
   └─ Blocks: None (final Phase 3 sprint)


═══════════════════════════════════════════════════════════════════════════════
PHASE 4 (Frontend — Desktop App)
═══════════════════════════════════════════════════════════════════════════════

⏳ C4 (Tauri Setup) ← START NOW (parallel with Phase 3)
   ├─ Requires: None
   └─ Blocks: A4, B4, C5

⏳ A4 (UI Components) ← AFTER C4
   ├─ Requires: C4 ⏳
   └─ Blocks: B4, C5, A5

⏳ B4 (Settings) ← AFTER A4
   ├─ Requires: A4 ⏳
   └─ Blocks: None

⏳ C5 (Chat Interface) ← AFTER C4, A4
   ├─ Requires: C4 ⏳, A4 ⏳
   └─ Blocks: C6

⏳ A5 (Progress Dashboard) ← AFTER A4
   ├─ Requires: A4 ⏳
   └─ Blocks: C6

⏳ B5 (API Integration) ← AFTER Phase 3 API contract
   ├─ Requires: Phase 3 API contract
   └─ Blocks: C6

⏳ C6 (Integration + Polish) ← ALL COMPLETE
   ├─ Requires: C4, A4, B4, C5, A5, B5 ✅ + Phase 3 ✅
   └─ Blocks: None (FINAL sprint — leads to release)
```

---

## 🚀 Parallel Execution Groups

### GROUP 1: START NOW (No Dependencies)

**Phase 3:**
```
┌─────────────────────────────────────────────────────────────┐
│  START IMMEDIATELY (parallel — no dependencies)            │
├─────────────────────────────────────────────────────────────┤
│  🔄 A2: Blackboard v2 (Agent A) — 8h                       │
│  🔄 B2: Task Queue (Agent B) — 8h                          │
│  🔄 C3: Result Merging (Agent C) — 8h                      │
└─────────────────────────────────────────────────────────────┘
```

**Phase 4:**
```
┌─────────────────────────────────────────────────────────────┐
│  START IMMEDIATELY (parallel with Phase 3)                 │
├─────────────────────────────────────────────────────────────┤
│  ⏳ C4: Tauri Setup (Agent C) — 8h ← START FIRST          │
│  ⏳ A4: UI Components (Agent A) — 8h (after C4)           │
└─────────────────────────────────────────────────────────────┘
```

**Why Parallel:**
- Phase 3 (backend) and Phase 4 (frontend) are independent
- Frontend uses mock API until Phase 3 complete
- Maximizes throughput (all agents busy)

---

### GROUP 2: After Group 1 Complete

**Phase 3:**
```
┌─────────────────────────────────────────────────────────────┐
│  AFTER A2, B2, C3 COMPLETE                                 │
├─────────────────────────────────────────────────────────────┤
│  ⏳ A3: Progress Monitoring (Agent A) — 8h                 │
│     (Final Phase 3 sprint)                                 │
└─────────────────────────────────────────────────────────────┘
```

**Phase 4:**
```
┌─────────────────────────────────────────────────────────────┐
│  AFTER C4, A4 COMPLETE                                     │
├─────────────────────────────────────────────────────────────┤
│  ⏳ B4: Settings (Agent B) — 8h                            │
│  ⏳ C5: Chat Interface (Agent C) — 8h                      │
│  ⏳ A5: Progress Dashboard (Agent A) — 8h                  │
└─────────────────────────────────────────────────────────────┘
```

**Why Parallel:**
- B4, C5, A5 have no dependencies on each other
- All depend only on C4, A4 complete

---

### GROUP 3: After Phase 3 Complete

**Phase 4:**
```
┌─────────────────────────────────────────────────────────────┐
│  AFTER PHASE 3 COMPLETE (API contract defined)             │
├─────────────────────────────────────────────────────────────┤
│  ⏳ B5: API Integration (Agent B) — 8h                     │
│     (Connect to real Phase 3 API)                          │
└─────────────────────────────────────────────────────────────┘
```

**Why Wait:**
- B5 needs real API contract from Phase 3
- Can use mock API until then (development continues)

---

### GROUP 4: Final Integration

**Phase 4:**
```
┌─────────────────────────────────────────────────────────────┐
│  AFTER ALL PHASE 3 + PHASE 4 SPRINTS COMPLETE              │
├─────────────────────────────────────────────────────────────┤
│  ⏳ C6: Integration + Polish (Agent C) — 8h                │
│     (FINAL SPRINT — leads to production release)           │
└─────────────────────────────────────────────────────────────┘
```

**Why Last:**
- Needs ALL components working
- Full integration testing
- Performance optimization
- Release builds

---

## 📅 Timeline Visualization

```
Week 1 (Now):
├─ Phase 3: A2, B2, C3 (parallel) ← START NOW
└─ Phase 4: C4, A4 (parallel) ← START NOW

Week 2:
├─ Phase 3: A3 (final sprint)
└─ Phase 4: B4, C5, A5 (parallel)

Week 3:
├─ Phase 3: DONE ✅
└─ Phase 4: B5 (API Integration)

Week 4:
└─ Phase 4: C6 (Integration + Polish) ← FINAL

Week 5:
└─ RELEASE READY 🎉
```

---

## 🔍 Detailed Dependencies

### Phase 3 Dependencies

```
C1 (Coordinator Core) ✅
   └─ No dependencies (foundation)

A1 (Context Compacting) ✅
   └─ No dependencies

B1 (Worker Pool) ✅
   └─ No dependencies

C2 (Task Decomposition) ✅
   └─ Requires: C1 ✅

A2 (Blackboard v2) 🔄
   ├─ Requires: A1 ✅
   └─ Status: IN PROGRESS

B2 (Task Queue) 🔄
   ├─ Requires: B1 ✅
   └─ Status: IN PROGRESS

C3 (Result Merging) 🔄
   ├─ Requires: C2 ✅, A2 🔄
   └─ Status: IN PROGRESS (waiting for A2)

A3 (Progress Monitoring) ⏳
   ├─ Requires: A2 🔄
   └─ Status: PENDING (final Phase 3)
```

### Phase 4 Dependencies

```
C4 (Tauri Setup) ⏳
   └─ No dependencies (foundation)
   └─ START NOW (parallel with Phase 3)

A4 (UI Components) ⏳
   ├─ Requires: C4 ⏳
   └─ Status: READY (after C4)

B4 (Settings) ⏳
   ├─ Requires: A4 ⏳
   └─ Status: READY (after A4)

C5 (Chat Interface) ⏳
   ├─ Requires: C4 ⏳, A4 ⏳
   └─ Status: READY (after C4, A4)

A5 (Progress Dashboard) ⏳
   ├─ Requires: A4 ⏳
   └─ Status: READY (after A4)

B5 (API Integration) ⏳
   ├─ Requires: Phase 3 API contract
   └─ Status: READY (after Phase 3)

C6 (Integration + Polish) ⏳
   ├─ Requires: C4, A4, B4, C5, A5, B5 ✅ + Phase 3 ✅
   └─ Status: READY (ALL complete)
```

---

## 🎯 Critical Path

**Longest path through all sprints:**

```
Phase 3 Critical Path:
C1 → C2 → C3 → A3
(0h) (8h) (8h) (8h) = 24h minimum

Phase 4 Critical Path:
C4 → A4 → C5 → C6
(8h) (8h) (8h) (8h) = 32h minimum

Combined (with parallel execution):
Week 1-2: Phase 3 (A2, B2, C3, A3) + Phase 4 (C4, A4)
Week 3:   Phase 4 (B4, C5, A5, B5)
Week 4:   Phase 4 (C6)

Total: 4-5 weeks to production release
```

---

## 📊 Parallel Execution Summary

### Can Start NOW (No Dependencies)

**Phase 3 (3 sprints):**
- A2: Blackboard v2 (Agent A)
- B2: Task Queue (Agent B)
- C3: Result Merging (Agent C)

**Phase 4 (1 sprint):**
- C4: Tauri Setup (Agent C) ← START FIRST

**Total:** 4 sprints can start immediately

---

### Can Start After Week 1

**Phase 3 (1 sprint):**
- A3: Progress Monitoring (Agent A)

**Phase 4 (3 sprints):**
- A4: UI Components (Agent A) — after C4
- B4: Settings (Agent B) — after A4
- C5: Chat Interface (Agent C) — after C4, A4

**Total:** 4 sprints can start after Week 1

---

### Can Start After Phase 3 Complete

**Phase 4 (2 sprints):**
- B5: API Integration (Agent B)
- A5: Progress Dashboard (Agent A) — after A4

**Total:** 2 sprints can start after Phase 3

---

### Final Sprint (After ALL Complete)

**Phase 4 (1 sprint):**
- C6: Integration + Polish (Agent C)

**Total:** 1 final sprint

---

## ✅ Action Items

### RIGHT NOW (Start Immediately)

**Agent A:**
```
Start: A2 (Blackboard v2)
File: planning/phase-3/agent-a/SPRINT-A2-BLACKBOARD-V2.md
```

**Agent B:**
```
Start: B2 (Task Queue)
File: planning/phase-3/agent-b/SPRINT-B2-TASK-QUEUE.md
```

**Agent C:**
```
Start: C3 (Result Merging) OR C4 (Tauri Setup)
Priority: C4 FIRST (Phase 4 foundation)
File: planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md
```

---

### AFTER Week 1 Complete

**Agent A:**
```
Start: A3 (Progress Monitoring) + A4 (UI Components)
```

**Agent B:**
```
Start: B4 (Settings)
```

**Agent C:**
```
Start: C5 (Chat Interface)
```

---

**START 4 SPRINTS NOW (A2, B2, C3, C4). Maximum parallelism!**
