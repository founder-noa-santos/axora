# Phase 4 Sprint C4: Tauri v2 Setup

**Agent:** C (Implementation Specialist — Tauri + Backend Integration)  
**Sprint:** C4  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement Tauri v2 setup with React frontend, creating the foundation for Phase 4 desktop app.

**Context:** Phase 3 is backend-only (CLI/API). Phase 4 needs cross-platform desktop app.

**Difficulty:** ⚠️ **MEDIUM** — Tauri setup, build configuration, IPC setup

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Tauri Backend Setup
**Task:** Setup Tauri v2 Rust backend
**File:** `apps/desktop/src-tauri/src/main.rs`
**Deliverables:**
- Tauri v2 project structure
- Window configuration (size, title, icon)
- IPC setup (Rust ↔ React)
- System tray integration (basic)
- 5+ tests

### Subagent 2: React Frontend Setup
**Task:** Setup React + Vite frontend
**File:** `apps/desktop/src/App.tsx`
**Deliverables:**
- React 18 + Vite project structure
- TypeScript configuration
- ESLint + Prettier setup
- Basic app shell (header, sidebar, content)
- 5+ tests

### Subagent 3: Build Configuration
**Task:** Configure builds for all platforms
**File:** `apps/desktop/src-tauri/tauri.conf.json`
**Deliverables:**
- macOS build (.dmg)
- Windows build (.exe, .msi)
- Linux build (.deb, .AppImage)
- Code signing setup (placeholder)
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review Tauri + React + build config
   - Resolve integration issues

2. **Integrate Components:**
   - Ensure Tauri ↔ React communication works
   - Test IPC (invoke commands from React)
   - Verify builds on current platform

3. **Implement Basic IPC:**
   - `ping()` command (Rust → React)
   - `get_version()` command (app version)
   - Event emission (Rust → React)

4. **Write Integration Tests:**
   - Test Tauri app starts
   - Test React renders
   - Test IPC commands work
   - Test build completes

5. **Update Documentation:**
   - Add README for desktop app
   - Add development setup guide

---

## 📐 Technical Spec

### Tauri Configuration

```rust
// apps/desktop/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::Manager;

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ping, get_version])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Tauri Config (tauri.conf.json)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OPENAKTA",
  "version": "0.1.0",
  "identifier": "dev.openakta.app",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "pnpm build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "OPENAKTA — Autonomous AI Orchestration",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["dmg", "exe", "deb"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

### React App Shell

```tsx
// apps/desktop/src/App.tsx
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

function App() {
  const [pingResult, setPingResult] = useState('');

  useEffect(() => {
    // Test IPC on mount
    invoke('ping').then(setPingResult);
  }, []);

  return (
    <div className="app">
      <header className="header">
        <h1>OPENAKTA</h1>
      </header>
      <div className="sidebar">
        {/* Navigation */}
      </div>
      <main className="content">
        <p>IPC Test: {pingResult}</p>
      </main>
    </div>
  );
}

export default App;
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] Tauri app compiles and runs
- [ ] React frontend renders
- [ ] IPC commands work (ping, get_version)
- [ ] 15+ tests passing (5 per subagent + 5 integration)
- [ ] Build completes for current platform
- [ ] Documentation added (README, setup guide)

---

## 🔗 Dependencies

**None** — Can start immediately (FIRST Phase 4 sprint)

**Blocks:**
- Sprint C5 (Chat Interface needs Tauri setup)
- Sprint A4 (UI Components need React setup)
- Sprint B4 (Settings need app shell)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Tauri Backend (parallel)
  ├─ Subagent 2: React Frontend (parallel)
  └─ Subagent 3: Build Config (parallel)
  ↓
Lead Agent: Integration + IPC + Tests
```

**Tauri v2 Notes:**
- Tauri v2 has breaking changes from v1
- Use `@tauri-apps/api/core` for IPC
- Window configuration in `tauri.conf.json`
- System tray is optional (can add later)

**Difficulty: MEDIUM**
- 3 subagents to coordinate
- Tauri v2 (newer, less documentation)
- Cross-platform build complexity
- IPC setup (Rust ↔ React)

**Review Checklist:**
- [ ] Tauri app starts without errors
- [ ] React renders in Tauri window
- [ ] IPC commands work (invoke from React)
- [ ] Build completes (at least for current platform)
- [ ] App icon displays correctly

---

**START NOW. This is FIRST Phase 4 sprint.**
