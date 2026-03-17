# Agent B — Phase 4 Review & Completion Tasks

**Date:** 2026-03-17
**Status:** Ready for Review
**Priority:** HIGH

---

## 📋 Context

Agent B completed **11 sprints** in Phase 2/3 ✅ — documented in `planning/agent-b/AGENT-B-STATUS.md`.

**Key deliverables:**
- Token Optimization (TOON, Context Pruning, Repository Map)
- Context Management (Blackboard, RAG, Distribution)
- Infrastructure (SCIP Indexing, Influence Graph, Semaphores, Atomic Checkout)

**What changed:**
- **Tailwind CSS v4 migration** completed (frontend only, no Rust changes)
- **assistant-ui integration** completed (chat interface)
- **Settings Store** already exists (`apps/desktop/src/store/settings-store.ts`)

---

## ✅ Tasks to Review/Complete

### Task 1: Verify Settings Store Compatibility

**File:** `apps/desktop/src/store/settings-store.ts`

**Current Status:** ✅ Already implemented and working

**Check:**
- [ ] Settings store loads/saves correctly
- [ ] Local storage persistence works (`axora-settings` key)
- [ ] Backend API sync works (when `/api/settings` available)
- [ ] Settings validation works (import/export)
- [ ] Zustand persist middleware works with new build

**Test:**
```bash
cd apps/desktop
pnpm vitest run src/test/settings-store.test.ts
```

**Note:** Settings store already exists — just verify it works after Tailwind v4 migration.

---

### Task 2: Sprint B4 — Settings Panel (PARTIALLY COMPLETE)

**File:** `planning/phase-4/agent-b/SPRINT-B4-SETTINGS.md`

**Status:** ✅ **UI Complete** — SettingsPanel.tsx already exists

**What's already done:**
- Settings Panel UI implemented (`apps/desktop/src/panels/SettingsPanel.tsx`)
- Model configuration (Ollama, OpenAI, Anthropic)
- Token limits configuration
- Worker pool settings
- Theme preferences
- Save/Reset buttons

**Check:**
- [ ] All settings sections render correctly
- [ ] Form validation works (provider, model, base URL)
- [ ] API key field is password type
- [ ] Save button disabled when no changes
- [ ] Reset button confirms before resetting

**Action if needed:**
The Settings Panel is already implemented — just verify it works with Tailwind v4.

---

### Task 3: Sprint B5 — API Integration Layer (NOT STARTED)

**File:** `planning/phase-4/agent-b/SPRINT-B5-API-INTEGRATION.md`

**Status:** ⚠️ **PENDING** — Not started yet

**What to implement:**

#### 3.1: REST API Client
**File:** `apps/desktop/src/api/rest-client.ts`

```typescript
// API client for missions, workers, settings
export const api = {
  missions: {
    submit: (mission: string) => POST('/api/missions', { mission }),
    getStatus: (id: string) => GET(`/api/missions/${id}`),
    list: () => GET('/api/missions'),
    cancel: (id: string) => DELETE(`/api/missions/${id}`),
  },
  workers: {
    list: () => GET('/api/workers'),
    getStatus: (id: string) => GET(`/api/workers/${id}`),
  },
  settings: {
    get: () => GET('/api/settings'),
    update: (settings: AppSettings) => PUT('/api/settings', settings),
  },
};
```

#### 3.2: WebSocket Client
**File:** `apps/desktop/src/api/websocket.ts`

```typescript
// WebSocket client for real-time progress
export class WebSocketClient {
  connect(url: string) { ... }
  onEvent(handler: (event: WebSocketEvent) => void) { ... }
  disconnect() { ... }
}

// Event types
export type WebSocketEvent =
  | { type: 'mission:started'; payload: { missionId: string } }
  | { type: 'mission:progress'; payload: MissionProgressEvent }
  | { type: 'mission:completed'; payload: { missionId: string; result: string } }
  | { type: 'mission:failed'; payload: { missionId: string; error: string } }
  | { type: 'worker:status'; payload: WorkerStatusEvent };
```

#### 3.3: Mock API for Development
**File:** `apps/desktop/src/api/mock-api.ts`

```typescript
// Mock API for development (before Phase 3 backend ready)
export const mockApi = {
  submitMission: async (mission: string) => {
    // Simulate mission execution
    emit('mission:started', { id: 'mock-1' });
    setTimeout(() => {
      emit('mission:progress', { id: 'mock-1', progress: 50 });
    }, 1000);
    setTimeout(() => {
      emit('mission:completed', { id: 'mock-1', result: 'Success' });
    }, 2000);
  },
  // ... other mock endpoints
};
```

**Subagents Pattern:**
```
Lead Agent:
  ├─ Subagent 1: REST API Client
  ├─ Subagent 2: WebSocket Client
  └─ Subagent 3: Mock API
  ↓
Lead Agent: Integration + Tests
```

**Estimated:** 8 hours

**Dependencies:**
- ✅ Sprint A4 complete (UI Components) — DONE
- ⚠️ API contract defined (from Phase 3 Coordinator)

---

### Task 4: Update Documentation

**File:** `planning/agent-b/AGENT-B-STATUS.md`

**Add:**
```markdown
| 32 | API Integration Layer | `apps/desktop/src/api/` | 🔄 In Progress |
```

---

## 🧪 Testing Checklist

Run these tests:

```bash
cd apps/desktop

# Typecheck
pnpm typecheck

# Build
pnpm build

# Test Settings Store
pnpm vitest run src/test/settings-store.test.ts

# Test Settings Panel
pnpm vitest run src/test/settings-panel.test.tsx
```

**Expected:**
- ✅ All tests passing
- ✅ Build succeeds
- ✅ No TypeScript errors

---

## 📦 Deliverables

### Already Complete ✅
1. Settings Store (Zustand)
2. Settings Panel UI
3. Model configuration (Ollama, OpenAI, Anthropic)
4. Token limits configuration
5. Worker pool settings

### Pending ⚠️
1. **Sprint B5** — API Integration Layer (REST + WebSocket + Mock)
2. **Documentation update** — Add Sprint 32 to status

---

## 🚀 How to Start

### For Review Tasks:
```bash
# Start dev server
cd apps/desktop
pnpm dev

# Open Settings panel
# Navigate to Settings → Verify all sections work
```

### For Sprint B5 (API Integration):
```bash
# Read the sprint plan
cat planning/phase-4/agent-b/SPRINT-B5-API-INTEGRATION.md

# Start implementation
# 1. Create rest-client.ts
# 2. Create websocket.ts
# 3. Create mock-api.ts
# 4. Write tests
```

---

## ⚠️ Important Notes

1. **Settings Panel** — Already implemented, just verify compatibility
2. **Settings Store** — Already implemented, just verify persistence
3. **No Rust changes** — Tailwind v4 is frontend only
4. **Sprint B5** — This is the main remaining task for Agent B

---

## 📚 Reference Files

- **Agent B Status:** `planning/agent-b/AGENT-B-STATUS.md`
- **Sprint B4 Plan:** `planning/phase-4/agent-b/SPRINT-B4-SETTINGS.md`
- **Sprint B5 Plan:** `planning/phase-4/agent-b/SPRINT-B5-API-INTEGRATION.md`
- **Phase 4 Master Plan:** `planning/phase-4/PHASE-4-MASTER-PLAN.md`
- **Settings Store:** `apps/desktop/src/store/settings-store.ts`
- **Settings Panel:** `apps/desktop/src/panels/SettingsPanel.tsx`

---

## ✅ Definition of Done

Agent B is done when:

- [ ] Settings Store verified working
- [ ] Settings Panel verified working
- [ ] **Sprint B5 complete** (API Integration Layer)
- [ ] Documentation updated (AGENT-B-STATUS.md)
- [ ] All tests passing (20+ total)

---

**Start with:** Review Settings Store/Panel → Then Sprint B5

**Priority:** Sprint B5 (API Integration) is the only remaining major task.
