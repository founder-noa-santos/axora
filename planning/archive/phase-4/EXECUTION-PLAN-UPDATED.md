# Phase 4 — Execution Plan (UPDATED: shadcn/ui)

**Updated:** 2026-03-17  
**Status:** Phase 3 DONE — Focus 100% on Phase 4  
**Update:** A4 uses shadcn/ui (50% faster — 4h instead of 8h) — ADR-050

---

## ✅ Phase 3 Status: COMPLETE

All Phase 3 sprints are done. Backend is ready!

---

## 🚀 Phase 4 — What To Do NOW

### START IMMEDIATELY (3 sprints in parallel)

```
═══════════════════════════════════════════════════════════════
WEEK 1: FOUNDATION (Start These 3 Sprints NOW)
═══════════════════════════════════════════════════════════════

🟢 C4: Tauri v2 Setup (Agent C) — 8h
   File: planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md
   Why First: Creates app shell (Tauri + React)

🟢 A4: shadcn/ui Setup + Customization (Agent A) — 4h ⚡ NEW!
   File: planning/phase-4/agent-a/SPRINT-A4-UI-COMPONENTS.md
   Update: Uses shadcn/ui (ADR-050) — 50% faster!
   Subagents: 2 (shadcn setup, Theme customization)

🟢 B4: Settings Panel (Agent B) — 8h
   File: planning/phase-4/agent-b/SPRINT-B4-SETTINGS.md
   Note: Needs A4 complete (UI components)
```

---

## 📊 Week-by-Week Plan

### Week 1 (START TODAY)

**Agent C:** C4 (Tauri Setup) — 8h  
**Agent A:** A4 (shadcn/ui Setup) — 4h ⚡  
**Agent B:** B4 (Settings) — 8h (after A4)

### Week 2

**Agent C:** C5 (Chat Interface) — 8h  
**Agent A:** A5 (Progress Dashboard) — 8h  
**Agent B:** B5 (API Integration) — 8h

### Week 3

**Agent C:** C6 (Integration + Polish) — 8h ← FINAL

**Outcome:** Production-ready desktop app in 3 weeks!

---

## 🎯 Immediate Actions (TODAY)

### For Agent C (Priority #1)

**START WITH C4:**
```bash
1. Read: planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md
2. Create: apps/desktop/ directory
3. End goal: Tauri app running with React shell
```

### For Agent A (UPDATED — shadcn/ui!)

**START WITH A4:**
```bash
1. Read: planning/phase-4/agent-a/SPRINT-A4-UI-COMPONENTS.md
2. Install shadcn/ui:
   cd apps/desktop
   npx shadcn-ui@latest init
   npx shadcn-ui@latest add button input card badge progress
3. Customize theme (brand colors from BRAND-INSPIRATION.md)
4. End goal: UI component library with 15+ components
```

**Time:** 4 hours (50% faster than building from scratch!)

### For Agent B

**WAIT FOR A4, THEN START B4:**
```bash
1. Wait for Agent A to complete UI components
2. Read: planning/phase-4/agent-b/SPRINT-B4-SETTINGS.md
3. End goal: Settings panel working
```

---

## 📁 All Phase 4 Files

```
planning/phase-4/
├── EXECUTION-PLAN.md            # This file (UPDATED)
├── PHASE-4-MASTER-PLAN.md       # Complete spec
├── ALL-SPRINTS-INDEX.md         # All sprints index
├── GETTING-STARTED.md           # Quick start
├── agent-a/
│   ├── SPRINT-A4-UI-COMPONENTS.md    ← START (Agent A) ⚡ UPDATED
│   └── SPRINT-A5-PROGRESS-DASHBOARD.md
├── agent-b/
│   ├── SPRINT-B4-SETTINGS.md         ← START after A4 (Agent B)
│   └── SPRINT-B5-API-INTEGRATION.md
└── agent-c/
    ├── SPRINT-C4-TAURI-SETUP.md      ← START FIRST (Agent C)
    ├── SPRINT-C5-CHAT-INTERFACE.md
    └── SPRINT-C6-INTEGRATION.md
```

---

## 📚 References

- [ADR-050: Use shadcn/ui](../../docs/adr/ADR-050-use-shadcn-ui.md)
- [BRAND-INSPIRATION.md](../../BRAND-INSPIRATION.md)

---

**START WITH C4 NOW. All Phase 4 depends on Tauri setup!**

**UPDATE:** A4 now uses shadcn/ui — 50% faster (4h instead of 8h)!
