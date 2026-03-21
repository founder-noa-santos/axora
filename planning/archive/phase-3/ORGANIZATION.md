# Phase 3 — Agent Task Organization

**Created:** 2026-03-17  
**Purpose:** Define Phase 3 agent structure and tasks

---

## 📁 Folder Structure Created

```
planning/
├── archive/
│   └── phase-2/
│       ├── README.md              # Phase 2 archive index
│       ├── agent-a/               # MOVE: Phase 2 Agent A tasks (10 sprints)
│       ├── agent-b/               # MOVE: Phase 2 Agent B tasks (11 sprints)
│       └── agent-c/               # MOVE: Phase 2 Agent C tasks (9 sprints)
│
└── phase-3/
    ├── README.md                  # Phase 3 overview
    ├── SPRINT-1-COORDINATOR-CORE.md  # First sprint (ready to start)
    ├── agent-a/                   # NEW: Phase 3 Agent A tasks
    ├── agent-b/                   # NEW: Phase 3 Agent B tasks
    └── agent-c/                   # NEW: Phase 3 Agent C tasks
```

---

## ✅ What Was Created

### 1. Archive Folder
- **Path:** `planning/archive/phase-2/`
- **Purpose:** Store Phase 2 agent tasks
- **Status:** ✅ Created with README.md

### 2. Phase 3 Agent Folders
- **Path:** `planning/phase-3/agent-a/`, `agent-b/`, `agent-c/`
- **Purpose:** Phase 3 agent tasks
- **Status:** ✅ Created (empty, ready for tasks)

### 3. Phase 3 README
- **Path:** `planning/phase-3/README.md`
- **Purpose:** Phase 3 overview and getting started
- **Status:** ✅ Created

---

## 📋 Manual Step Required

**Move Phase 2 agent folders to archive:**

```bash
cd /Users/noasantos/Downloads/openakta/planning

# Move old agent folders to archive
mv agent-a archive/phase-2/
mv agent-b archive/phase-2/
mv agent-c archive/phase-2/

# Verify structure
ls -la
ls -la phase-3/
ls -la archive/phase-2/
```

**Why Manual:** System permissions prevent automated move

---

## 🎯 Phase 3 Agent Assignments

### Agent A (Documentation + Memory Specialist)
**Sprints:**
- Sprint 2: Context Compacting
- Sprint 4: Blackboard v2
- Sprint 6: Progress Monitoring

**Folder:** `planning/phase-3/agent-a/`

---

### Agent B (Storage + Context Specialist)
**Sprints:**
- Sprint 3: Worker Agent Pool
- Sprint 5: Task Queue Management

**Folder:** `planning/phase-3/agent-b/`

---

### Agent C (Implementation — Coordinator Core)
**Sprints:**
- **Sprint 1: Coordinator Core Structure** ← START HERE
- Sprint 2: Task Decomposition Engine

**Folder:** `planning/phase-3/agent-c/`

---

## 🚀 Next Action

**1. Move Phase 2 folders to archive (manual):**
```bash
cd /Users/noasantos/Downloads/openakta/planning
mv agent-a agent-b agent-c archive/phase-2/
```

**2. Start Phase 3 Sprint 1:**
```
Agent C: Start Phase 3 Sprint 1
File: planning/phase-3/SPRINT-1-COORDINATOR-CORE.md
Priority: CRITICAL
```

---

## 📊 Summary

| Item | Status |
|------|--------|
| Archive folder created | ✅ |
| Phase 3 agent folders created | ✅ |
| Phase 3 README created | ✅ |
| Phase 2 Sprint 1 spec created | ✅ |
| Move Phase 2 folders | ⏳ Manual step |

---

**Structure is ready. Move folders and start Phase 3!**
