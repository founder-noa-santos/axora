# Phase 4 — Desktop App: Master Plan

**Version:** 1.0  
**Created:** 2026-03-17  
**Status:** Ready to Start  
**Timeline:** 2 weeks (6 sprints, ~48 hours)  
**Priority:** HIGH (parallel with Phase 3 completion)

---

## 🎯 Executive Summary

**What:** Desktop application (Tauri v2 + React) for users to interact with Coordinator Agent.

**Problem:** Phase 3 works autonomously but only via CLI/API. Users need visual interface.

**Solution:** Cross-platform desktop app with chat interface, progress dashboard, and configuration.

**Key Feature:** **Parallel Development** — Frontend works with mock API while Phase 3 completes, then integrate.

---

## 📚 Research Foundation (Synthesized)

### From Phase 3
**Coordinator API:** Defined contract for frontend ↔ backend communication

**Applied to Phase 4:**
- REST API for mission submission
- WebSocket for real-time progress
- Local storage for settings/history

### From BRAND-INSPIRATION.md
**Visual Identity:** Orchestration, Intelligence, Flow, Precision

**Applied to Phase 4:**
- Color palette: Deep Blue, Electric Purple, Emerald Green, Slate Gray
- Design: Clean geometric, minimalist, sharp lines
- Avoid: AI clichés (robots, brains, chat bubbles)

### From Competitive Analysis
**Industry Standard:** VS Code-like experience (familiar to developers)

**Applied to Phase 4:**
- Sidebar navigation
- Chat panel (like terminal)
- Progress panel (like build output)
- Settings panel (like preferences)

---

## 🏗️ Architecture Overview

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                    DESKTOP APP (Tauri + React)              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Chat Panel   │  │ Progress     │  │ Settings     │      │
│  │ (Mission     │  │ Dashboard    │  │ Panel        │      │
│  │  Input)      │  │ (Real-time)  │  │ (Config)     │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
│                                                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │              Tauri Backend (Rust)                    │  │
│  │  - Native window management                          │  │
│  │  - File system access                                │  │
│  │  - System tray integration                           │  │
│  │  - IPC to React frontend                             │  │
│  └──────────────────────────────────────────────────────┘  │
└────────────────────┬────────────────────────────────────────┘
                     │
                     │ REST API + WebSocket
                     ▼
┌─────────────────────────────────────────────────────────────┐
│              COORDINATOR (Phase 3 Backend)                  │
│  - Mission execution                                        │
│  - Worker management                                        │
│  - Progress reporting                                       │
└─────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **User Input** → Chat Panel → POST /api/missions
2. **Mission Start** → Coordinator → WebSocket progress events
3. **Progress Update** → WebSocket → Progress Dashboard
4. **Mission Complete** → Results → Chat Panel + File System

---

## 📋 Implementation Breakdown

### Agent A: UI Components + Progress Display

**Expertise:** Documentation, Memory, Visual State

**Sprints:**

#### Sprint A4: React UI Components
**File:** `planning/phase-4/agent-a/SPRINT-A4-UI-COMPONENTS.md`

**What:**
- Design system (colors, typography, spacing)
- Core components (Button, Input, Card, Badge)
- Layout components (Sidebar, Panel, Header)
- Theme support (light/dark mode)

**Deliverables:**
- `apps/desktop/src/components/ui/` (component library)
- Storybook for component documentation
- 20+ components with tests

**Dependencies:** None (can start immediately)

**Estimated:** 8 hours

---

#### Sprint A5: Progress Dashboard
**File:** `planning/phase-4/agent-a/SPRINT-A5-PROGRESS-DASHBOARD.md`

**What:**
- Real-time progress visualization
- ETA display
- Blocker alerts
- Worker status panel

**Deliverables:**
- `apps/desktop/src/panels/ProgressPanel.tsx`
- WebSocket integration for real-time updates
- Progress bars, charts, status indicators
- 10+ tests

**Dependencies:** Sprint A4 complete (UI components)

**Estimated:** 8 hours

---

### Agent B: API Integration + Configuration

**Expertise:** Storage, Context, Data Flow

**Sprints:**

#### Sprint B4: Settings & Configuration Panel
**File:** `planning/phase-4/agent-b/SPRINT-B4-SETTINGS.md`

**What:**
- Model selection (Ollama, OpenAI, etc.)
- Token limits configuration
- Worker pool settings
- Theme preferences

**Deliverables:**
- `apps/desktop/src/panels/SettingsPanel.tsx`
- Local storage for settings
- Settings sync with backend
- 10+ tests

**Dependencies:** Sprint A4 complete (UI components)

**Estimated:** 8 hours

---

#### Sprint B5: API Integration Layer
**File:** `planning/phase-4/agent-b/SPRINT-B5-API-INTEGRATION.md`

**What:**
- REST API client (missions, workers, settings)
- WebSocket client (real-time progress)
- Error handling + retry logic
- Mock API for development

**Deliverables:**
- `apps/desktop/src/api/` (API client library)
- Mock API server for development
- API documentation
- 15+ tests

**Dependencies:** API contract defined (Phase 3)

**Estimated:** 8 hours

---

### Agent C: Tauri Setup + Chat + Integration

**Expertise:** Coordinator Core, Backend Integration

**Sprints:**

#### Sprint C4: Tauri v2 Setup
**File:** `planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md`

**What:**
- Tauri v2 project setup
- Window configuration (size, title, icon)
- System tray integration
- IPC setup (Rust ↔ React)

**Deliverables:**
- `apps/desktop/src-tauri/` (Tauri backend)
- `apps/desktop/` (React frontend)
- Build configuration for all platforms
- 5+ tests

**Dependencies:** None (can start immediately)

**Estimated:** 8 hours

---

#### Sprint C5: Chat Interface
**File:** `planning/phase-4/agent-c/SPRINT-C5-CHAT-INTERFACE.md`

**What:**
- Chat panel (mission input)
- Message history display
- File attachment (for context)
- Mission submission

**Deliverables:**
- `apps/desktop/src/panels/ChatPanel.tsx`
- Message store (state management)
- File upload integration
- 10+ tests

**Dependencies:** Sprint A4 complete (UI components), Sprint C4 complete (Tauri setup)

**Estimated:** 8 hours

---

#### Sprint C6: Integration + Polish
**File:** `planning/phase-4/agent-c/SPRINT-C6-INTEGRATION.md`

**What:**
- Connect all panels to real API
- End-to-end testing
- Performance optimization
- Bug fixes + polish

**Deliverables:**
- Working desktop app
- E2E tests passing
- Performance benchmarks
- Release build (.dmg, .exe, .deb)

**Dependencies:** Phase 3 complete, all Phase 4 sprints complete

**Estimated:** 8 hours

---

## 📊 Sprint Dependencies

```
Week 1 (Start Immediately):
├─ C4: Tauri Setup (Agent C) ← START HERE
├─ A4: UI Components (Agent A) ← Parallel
└─ B4: Settings (Agent B) ← After A4

Week 2:
├─ C5: Chat Interface (Agent C) ← After C4, A4
├─ A5: Progress Dashboard (Agent A) ← After A4
└─ B5: API Integration (Agent B) ← After API contract

Week 3 (Integration):
└─ C6: Integration + Polish (Agent C) ← All complete
```

**Critical Path:** C4 → C5 → C6

---

## 🎯 Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Build Time** | <5 minutes | Automated timing |
| **App Size** | <50 MB | Binary size |
| **Startup Time** | <2 seconds | Cold start |
| **API Latency** | <100ms | REST calls |
| **WebSocket Lag** | <500ms | Real-time updates |
| **Test Coverage** | >80% | Coverage report |
| **E2E Tests** | 100% passing | CI/CD pipeline |
| **Platform Support** | macOS, Windows, Linux | Build artifacts |

---

## 🔗 Integration with Phase 3

### API Contract (Defined Now)

**REST Endpoints:**
```
POST   /api/missions          # Submit new mission
GET    /api/missions/:id      # Get mission status
GET    /api/missions          # List all missions
DELETE /api/missions/:id      # Cancel mission

GET    /api/workers           # List workers
GET    /api/workers/:id       # Get worker status

GET    /api/settings          # Get settings
PUT    /api/settings          # Update settings
```

**WebSocket Events:**
```
mission:started      # Mission execution started
mission:progress     # Progress update (0-100%)
mission:completed    # Mission completed successfully
mission:failed       # Mission failed with error
worker:status        # Worker status change
```

### Mock API Strategy

**While Phase 3 completes:**
```typescript
// apps/desktop/src/api/mock-server.ts
export const mockApi = {
  submitMission: async (mission: string) => {
    // Simulate mission execution
    setTimeout(() => {
      emit('mission:started', { id: 'mock-1' });
      // Simulate progress
      emit('mission:progress', { id: 'mock-1', progress: 50 });
      // Simulate completion
      emit('mission:completed', { id: 'mock-1', result: 'Success' });
    }, 1000);
  },
  // ... other mock endpoints
};
```

**When Phase 3 complete:**
```typescript
// Switch from mock to real API
import { realApi } from './real-api';
// Replace mockApi with realApi in app initialization
```

---

## 📅 Timeline

### Week 1: Foundation
- **Day 1-2:** C4 (Tauri Setup)
- **Day 3-4:** A4 (UI Components)
- **Day 5:** B4 (Settings)

**Milestone:** App shell works, UI components ready

---

### Week 2: Panels
- **Day 1-2:** C5 (Chat Interface)
- **Day 3-4:** A5 (Progress Dashboard)
- **Day 5:** B5 (API Integration)

**Milestone:** All panels working with mock API

---

### Week 3: Integration
- **Day 1-2:** C6 (Integration with real API)
- **Day 3-4:** E2E testing + bug fixes
- **Day 5:** Release builds

**Milestone:** Production-ready desktop app

---

## 🚨 Risks & Mitigations

### Risk 1: Phase 3 delayed
**Mitigation:** 
- Frontend uses mock API (independent development)
- Integration sprint (C6) scheduled after Phase 3 complete
- Buffer time built into timeline

### Risk 2: Tauri v2 compatibility issues
**Mitigation:**
- Use stable Tauri v2 beta
- Test on all platforms early
- Fallback to Tauri v1 if critical issues

### Risk 3: API contract changes
**Mitigation:**
- Define contract now (before frontend starts)
- Version API (v1, v2) for backward compatibility
- Mock API can be updated independently

### Risk 4: Performance issues (slow startup, large bundle)
**Mitigation:**
- Code splitting (lazy load panels)
- Tree shaking (remove unused code)
- Performance budgets (enforce limits)

---

## 📝 Definition of Done

**Phase 4 is complete when:**

1. ✅ All 6 sprints complete
2. ✅ 60+ tests passing
3. ✅ Desktop app builds for all platforms (.dmg, .exe, .deb)
4. ✅ App startup <2 seconds
5. ✅ Real-time progress works (WebSocket)
6. ✅ Chat interface submits missions successfully
7. ✅ Settings persist across restarts
8. ✅ E2E tests pass (full mission flow)
9. ✅ Performance benchmarks met
10. ✅ Brand identity applied (colors, logo, typography)

---

## 🎯 Getting Started

### Immediate Action (NOW)

**Agent C: Start Sprint C4**

```
Agent C: Start Phase 4 Sprint 4
File: planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md
Priority: HIGH
```

**Why Agent C First:**
- Tauri setup is foundation for all frontend work
- Agent C has backend expertise (Coordinator integration)
- Can test immediately (app shell)

---

### After C4 Complete

**Parallel Start:**
- Agent A: Sprint A4 (UI Components)
- Agent B: Sprint B4 (Settings) — after A4

**Why Parallel:**
- No dependencies between C4, A4
- Maximizes throughput
- All agents productive

---

## 💡 Design Principles

### 1. Developer-First UX
- Familiar layout (VS Code-like)
- Keyboard shortcuts (power user friendly)
- Dark mode by default (developers prefer dark)

### 2. Real-Time Feedback
- Progress updates (<500ms lag)
- Blocker alerts (immediate notification)
- Status indicators (always visible)

### 3. Minimal Configuration
- Sensible defaults (work out of box)
- Advanced settings (hidden by default)
- Import/export config (portability)

### 4. Cross-Platform Consistency
- Same UX on macOS, Windows, Linux
- Native feel on each platform
- Shared codebase (90%+ code reuse)

---

## 📚 Appendix: File Structure

```
apps/desktop/
├── src/                      # React frontend
│   ├── components/
│   │   └── ui/               # A4: UI Components
│   ├── panels/
│   │   ├── ChatPanel.tsx     # C5: Chat Interface
│   │   ├── ProgressPanel.tsx # A5: Progress Dashboard
│   │   └── SettingsPanel.tsx # B4: Settings
│   ├── api/
│   │   ├── mock-api.ts       # B5: Mock API
│   │   ├── real-api.ts       # B5: Real API
│   │   └── websocket.ts      # B5: WebSocket client
│   ├── store/                # State management
│   ├── theme/                # Design system
│   └── App.tsx               # Main app component
│
├── src-tauri/                # C4: Tauri backend
│   ├── src/
│   │   └── main.rs           # Tauri app entry
│   ├── Cargo.toml            # Rust dependencies
│   └── tauri.conf.json       # Tauri config
│
├── package.json              # Frontend dependencies
└── vite.config.ts            # Build configuration
```

---

## ✅ Approval to Start

**Phase 4 is approved to start.**

**First Task:** Agent C — Sprint C4 (Tauri Setup)

**File:** `planning/phase-4/agent-c/SPRINT-C4-TAURI-SETUP.md`

**Priority:** HIGH

---

**This document is the single source of truth for Phase 4.**

**All decisions, designs, and implementations must align with this plan.**
