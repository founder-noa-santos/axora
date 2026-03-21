# OPENAKTA — Next Steps & Roadmap

**Date:** 2026-03-16  
**Status:** Phase 2 Complete ✅  
**Next:** Phase 3 (Coordinator Agent)

---

## 📊 What We Have (Phase 2 Complete)

### ✅ Token Optimization (90%+ Reduction)
- Prefix Caching (50-90%)
- Diff-Based Communication (89-98%)
- Code Minification (24-42%)
- TOON Serialization (50-60%)
- Context Pruning (95-99%)
- Repository Map (AST + PageRank, 90%+)

### ✅ Memory Architecture (Tripartite)
- Semantic Memory (Vector DB — Qdrant)
- Episodic Memory (SQLite time-series)
- Procedural Memory (SKILL.md files)
- Consolidation Pipeline (episodic → procedural)
- MemGAS Retrieval (GMM clustering + entropy routing)
- Memory Lifecycle (Ebbinghaus decay + utility pruning)

### ✅ Agent Framework
- Heartbeat System (timer + event-driven)
- Graph Workflow (deterministic execution)
- Task Decomposition (ACONIC-based)
- Dual-Thread ReAct (planning + acting threads)
- ACI Formatting (output truncation/pagination)
- Bidirectional Traceability (code ↔ business rules)

### ✅ Infrastructure
- SCIP Indexing (language-agnostic code indexing)
- Influence Graph (AST + PageRank)
- Sliding-Window Semaphores (concurrency throttling)
- Atomic Checkout (task locking — prevents duplicates)
- Snapshot Blackboard (TOCTOU prevention)

### 📈 Stats
- **30 Sprints** complete
- **296 Tests** passing
- **~21,000 Lines** of code
- **34 Files** created/modified
- **100% Tests** passing

---

## 🎯 What's Next (Options)

### Option 1: Phase 3 — Coordinator Agent 🔴 RECOMMENDED

**What:** Self-orchestration system — user talks to ONE agent, it manages everything

**Why First:**
- ✅ Solves YOUR problem (tired of being "babysitter")
- ✅ Key differentiator (no other framework has this)
- ✅ Enables Phase 4 (Desktop App needs Coordinator)
- ✅ Can test immediately (dogfooding)

**Sprints:** 6 sprints
- Sprint 1: Coordinator Core Structure
- Sprint 2: Task Decomposition Engine
- Sprint 3: Worker Agent Pool
- Sprint 4: Blackboard v2 (Shared State)
- Sprint 5: Context Compacting
- Sprint 6: Progress Monitoring & Reporting

**Timeline:** 2-3 weeks (~48 hours)

**Outcome:**
```
User: "Implement authentication system"
  ↓
Coordinator: Decomposes → Dispatches → Monitors → Merges
  ↓
User: "Done. Here's what I built."
```

---

### Option 2: Phase 4 — Desktop App (Tauri + React)

**What:** User-facing desktop application

**Why Second (not first):**
- ⚠️ Without Coordinator, just shows manual chaos
- ✅ With Coordinator, shows autonomous operation
- ❌ Don't build UI before backend is ready

**Sprints:** 8-10 sprints
- Tauri v2 setup
- Chat UI (talk to Coordinator)
- Progress visualization
- Configuration (BYOK vs subscription)
- Settings (model selection, token limits)

**Timeline:** 3-4 weeks

**Outcome:**
- Desktop app (.dmg, .exe, .deb)
- Chat interface
- Progress dashboard
- Settings panel

---

### Option 3: Phase 5 — Beta Testing

**What:** Real users testing the system

**Why Third:**
- ⚠️ Need working product first (Coordinator + Desktop)
- ✅ Validate with users before launch
- ❌ Don't beta test incomplete product

**Sprints:** 4-6 sprints
- E2E testing
- Beta program (5-10 users)
- Feedback collection
- Iteration

**Timeline:** 2 weeks

**Outcome:**
- User feedback report
- Bug fixes
- UX improvements
- Ready for launch

---

### Option 4: Phase 6 — Production Launch

**What:** Public release

**Why Last:**
- ⚠️ Need validated product (beta feedback)
- ✅ Installers, auto-update, docs
- ❌ Don't launch without polish

**Sprints:** 6-8 sprints
- Installers (.dmg, .exe, .deb)
- Auto-update mechanism
- User documentation
- Marketing materials

**Timeline:** 3-4 weeks

**Outcome:**
- Public launch
- Website + docs
- Installers
- Support system

---

## 📅 Recommended Roadmap

```
NOW (Week 1-3):
├─ Phase 3: Coordinator Agent
│  ├─ Sprint 1: Coordinator Core
│  ├─ Sprint 2: Task Decomposition
│  ├─ Sprint 3: Worker Pool
│  ├─ Sprint 4: Blackboard v2
│  ├─ Sprint 5: Context Compacting
│  └─ Sprint 6: Progress Monitoring
│
└─ Milestone: Self-orchestration working

Week 4-7:
├─ Phase 4: Desktop App
│  ├─ Tauri v2 + React
│  ├─ Chat UI
│  ├─ Progress dashboard
│  └─ Configuration
│
└─ Milestone: Desktop app usable

Week 8-9:
├─ Phase 5: Beta Testing
│  ├─ 5-10 beta users
│  ├─ E2E testing
│  └─ Feedback iteration
│
└─ Milestone: Product validated

Week 10-13:
├─ Phase 6: Production Launch
│  ├─ Installers
│  ├─ Auto-update
│  ├─ Documentation
│  └─ Launch
│
└─ Milestone: Public release
```

**Total:** 13 weeks (~3 months) to launch

---

## 🚀 Immediate Next Action

**Start Phase 3, Sprint 1: Coordinator Core Structure**

**Agent Assignment:** Agent C (implementation specialist)

**File:** `planning/phase-3/COORDINATOR-CORE.md` (created)

**Command:**
```
Agent C: Start Phase 3 Sprint 1
File: planning/phase-3/COORDINATOR-CORE.md
Priority: CRITICAL
```

**Why Agent C:**
- Has ReAct loop expertise (Sprint 9)
- Has Graph Workflow expertise (Sprint 7)
- Has Heartbeat expertise (Sprint 3b)
- All needed for Coordinator

---

## 💡 Decision Framework

**Choose based on your goal:**

| Goal | Choose | Timeline |
|------|--------|----------|
| Solve coordination pain NOW | Phase 3 (Coordinator) | 2-3 weeks |
| Have UI to show investors | Phase 4 (Desktop) | 3-4 weeks |
| Validate with users | Phase 5 (Beta) | 2 weeks |
| Launch product | Phase 6 (Launch) | 3-4 weeks |

**Recommended:** Phase 3 first (foundational), then 4, 5, 6

---

## ❓ Questions to Decide

1. **What's your runway?**
   - <3 months → Aggressive (skip Beta)
   - 3-6 months → Balanced (full roadmap)
   - 6+ months → Conservative (extensive testing)

2. **What's your priority?**
   - Solve your pain → Phase 3
   - Show to investors → Phase 4
   - Validate idea → Phase 5
   - Launch business → Phase 6

3. **Who's your competition?**
   - If close → Aggressive (launch fast)
   - If far → Balanced (quality focus)

---

## ✅ My Recommendation

**Phase 3 → Phase 4 → Phase 5 → Phase 6**

**Why:**
1. **Phase 3** solves YOUR problem (coordination)
2. **Phase 4** gives you UI to show
3. **Phase 5** validates with users
4. **Phase 6** launches product

**Timeline:** 3 months to launch

**Start:** Phase 3, Sprint 1 (Coordinator Core) — NOW

---

**Ready to start Phase 3?**
