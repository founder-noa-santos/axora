# Planning Directory

**Last Updated:** 2026-03-17

---

## 📁 Structure

```
planning/
├── README.md               ← This file (structure + conventions)
├── STATUS-DASHBOARD.md     ← Quick visual status (READ FIRST)
├── CURRENT-STATUS.md       ← Detailed status
├── SPRINT-COMPLETION-TEMPLATE.md ← Template for completion reports
│
├── agent-a/
│   ├── current_task.md     ← Agent A: READ THIS FIRST
│   ├── AGENT-A-STATUS.md   ← Agent A status (updated after each sprint)
│   └── done/               ← Completed sprint reports
│
├── agent-b/
│   ├── current_task.md     ← Agent B: READ THIS FIRST
│   ├── AGENT-B-STATUS.md   ← Agent B status (updated after each sprint)
│   └── done/               ← Completed sprint reports
│
├── agent-c/
│   ├── current_task.md     ← Agent C: READ THIS FIRST
│   ├── AGENT-C-STATUS.md   ← Agent C status (updated after each sprint)
│   └── done/               ← Completed sprint reports
│
└── archive/                ← Historical documents (old phases, sprints, etc.)
```

---

## 🎯 Getting Started

### For Each Agent

1. **Read your current task FIRST:**
   ```bash
   # Agent A
   cat planning/agent-a/current_task.md
   
   # Agent B
   cat planning/agent-b/current_task.md
   
   # Agent C
   cat planning/agent-c/current_task.md
   ```

2. **Check your status:**
   ```bash
   cat planning/agent-a/AGENT-A-STATUS.md
   cat planning/agent-b/AGENT-B-STATUS.md
   cat planning/agent-c/AGENT-C-STATUS.md
   ```

3. **View dashboard:**
   ```bash
   cat planning/STATUS-DASHBOARD.md
   ```

---

## 📝 AFTER COMPLETING A SPRINT

**IMPORTANT:** After finishing a sprint, you MUST update these files:

### Step 1: Update your `current_task.md`
```markdown
# Change status to:
**Status:** ✅ **SPRINT [X] COMPLETE**

# Update next task section
```

### Step 2: Update your `AGENT-*-STATUS.md`
Add sprint to completed table:
```markdown
| [Sprint ID] | [Title] | `[file/path]` | ✅ |
```

Update workload summary:
```markdown
| Metric | Value |
|--------|-------|
| Total Sprints | [X]+1 |
| Completed | [X]+1 ✅ |
| In Progress | 0 |
```

### Step 3: Create Completion Report
Create file: `planning/agent-[a/b/c]/done/SPRINT-[X]-COMPLETION.md`

Use template: `planning/SPRINT-COMPLETION-TEMPLATE.md`

Include:
- Summary of what was built
- Test count
- Success criteria checklist
- Technical details (code snippets)
- Metrics

### Step 4: Update `planning/CURRENT-STATUS.md`
Update the status table and phase progress section.

### Step 5: Update `planning/STATUS-DASHBOARD.md`
Move your sprint from "In Progress" to "Completed".
Update phase progress percentage.

---

## 📊 Current Assignments

| Agent | Current Sprint | Task | Priority |
|-------|---------------|------|----------|
| **A** | A3 | Progress Monitoring | HIGH |
| **B** | B2 | Task Queue Management | HIGH |
| **C** | C6 | Phase 4 Integration | HIGH |

**All agents:** Focus on current tasks!

---

## 📁 File Conventions

| File | Purpose | Update Frequency |
|------|---------|------------------|
| `current_task.md` | **Active task** — Read this first! | After each sprint |
| `AGENT-*-STATUS.md` | Agent status (completed sprints, metrics) | After each sprint |
| `done/` | Completed sprint reports (historical) | Append only |
| `STATUS-DASHBOARD.md` | Quick visual status | After each sprint |
| `CURRENT-STATUS.md` | Detailed status with phase progress | After each sprint |

---

## 🗄️ Archive

The `archive/` folder contains:
- Old phase plans (Phase 1, 2, 3, 4)
- Historical documents
- Old coordination boards
- Research notes

**Do not modify archive/** — It's read-only historical data.

---

## ✅ Definition of Organized

- ✅ Only active files in agent folders
- ✅ Completed sprints in `done/` subfolders
- ✅ Historical docs in `archive/`
- ✅ Clear current task for each agent
- ✅ Status files updated after each sprint

---

## 🚀 Quick Commands

```bash
# View your current task
cat planning/agent-a/current_task.md  # Agent A
cat planning/agent-b/current_task.md  # Agent B
cat planning/agent-c/current_task.md  # Agent C

# View dashboard
cat planning/STATUS-DASHBOARD.md

# View detailed status
cat planning/CURRENT-STATUS.md
```

---

**Start by reading your `current_task.md`!** 🚀
