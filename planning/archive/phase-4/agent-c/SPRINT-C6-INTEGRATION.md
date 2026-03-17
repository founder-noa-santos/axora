# Phase 4 Sprint C6: Integration + Polish

**Agent:** C (Implementation Specialist — Tauri + Backend Integration)  
**Sprint:** C6  
**Priority:** CRITICAL  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Integrate all Phase 4 components with real Phase 3 backend, perform E2E testing, optimize performance, and create release builds.

**Context:** All Phase 4 components work with mock API. Now integrate with real Phase 3 backend and polish for release.

**Difficulty:** ⚠️ **HIGH** — Full integration, E2E testing, performance optimization, cross-platform builds

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: Backend Integration
**Task:** Connect all panels to real Phase 3 API
**File:** `apps/desktop/src/integration/backend-integration.ts`
**Deliverables:**
- Switch from mock API to real API
- Test all endpoints (missions, workers, settings)
- Test WebSocket events (real-time progress)
- Fix integration issues
- 5+ tests

### Subagent 2: E2E Testing + Bug Fixes
**Task:** Implement E2E tests and fix bugs
**File:** `apps/desktop/tests/e2e/`
**Deliverables:**
- E2E test suite (Playwright)
- Test full mission flow (submit → progress → complete)
- Test settings (save/load)
- Test real-time progress (WebSocket)
- Bug fixes from testing
- 10+ tests

### Subagent 3: Performance + Release Builds
**Task:** Optimize performance and create release builds
**File:** `apps/desktop/scripts/build.ts`
**Deliverables:**
- Performance optimization (code splitting, lazy loading)
- Build optimization (tree shaking, minification)
- Release builds (.dmg, .exe, .deb)
- Code signing (if certificates available)
- Performance benchmarks
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review integration + E2E + performance
   - Resolve integration issues

2. **Integrate All Components:**
   - Chat Panel → Real API
   - Progress Dashboard → Real WebSocket
   - Settings → Real backend
   - Ensure all panels work together

3. **Implement Error Boundaries:**
   - Graceful error handling
   - User-friendly error messages
   - Recovery from errors

4. **Write E2E Tests:**
   - Full mission flow
   - Settings persistence
   - Real-time progress
   - Cross-platform tests

5. **Create Release Builds:**
   - macOS (.dmg)
   - Windows (.exe, .msi)
   - Linux (.deb, .AppImage)
   - Performance benchmarks

---

## 📐 Technical Spec

### E2E Test Suite (Playwright)

```typescript
// apps/desktop/tests/e2e/mission-flow.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Mission Flow', () => {
  test('should submit mission and see progress', async ({ page }) => {
    await page.goto('http://localhost:5173');
    
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Build authentication system');
    await page.click('[data-testid="submit-mission"]');
    
    // Wait for mission to start
    await expect(page.locator('[data-testid="active-missions"]'))
      .toContainText('running');
    
    // Wait for progress
    await page.waitForSelector('[data-testid="progress-bar"]');
    const progress = await page.locator('[data-testid="progress-bar"]').getAttribute('value');
    expect(parseInt(progress)).toBeGreaterThan(0);
    
    // Wait for completion
    await page.waitForSelector('[data-testid="mission-complete"]', { timeout: 60000 });
    await expect(page.locator('[data-testid="mission-status"]'))
      .toHaveText('completed');
  });
  
  test('should save settings and persist across restarts', async ({ page }) => {
    await page.goto('http://localhost:5173');
    
    // Open settings
    await page.click('[data-testid="settings-button"]');
    
    // Change settings
    await page.selectOption('[data-testid="model-provider"]', 'openai');
    await page.fill('[data-testid="api-key"]', 'sk-test123');
    await page.click('[data-testid="save-settings"]');
    
    // Verify saved
    await expect(page.locator('[data-testid="settings-saved"]'))
      .toBeVisible();
    
    // Reload and verify settings persist
    await page.reload();
    await page.click('[data-testid="settings-button"]');
    await expect(page.locator('[data-testid="model-provider"]'))
      .toHaveValue('openai');
  });
  
  test('should show real-time progress updates', async ({ page }) => {
    await page.goto('http://localhost:5173');
    
    // Submit mission
    await page.fill('[data-testid="mission-input"]', 'Test real-time progress');
    await page.click('[data-testid="submit-mission"]');
    
    // Wait for progress updates
    let lastProgress = 0;
    for (let i = 0; i < 5; i++) {
      const progress = await page.locator('[data-testid="progress-value"]').textContent();
      expect(parseInt(progress)).toBeGreaterThanOrEqual(lastProgress);
      lastProgress = parseInt(progress);
      await page.waitForTimeout(2000);
    }
  });
});
```

### Performance Optimization

```typescript
// apps/desktop/vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { compression } from 'vite-plugin-compression';

export default defineConfig({
  plugins: [
    react(),
    compression({
      algorithm: 'gzip',
      ext: '.gz',
    }),
  ],
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-react': ['react', 'react-dom'],
          'vendor-tauri': ['@tauri-apps/api/core'],
          'vendor-zustand': ['zustand'],
        },
      },
    },
    chunkSizeWarningLimit: 500,
    minify: 'terser',
    terserOptions: {
      compress: {
        drop_console: true,
        drop_debugger: true,
      },
    },
  },
  optimizeDeps: {
    include: ['react', 'react-dom', 'zustand'],
  },
});
```

### Release Build Script

```typescript
// apps/desktop/scripts/build.ts
import { execSync } from 'child_process';
import { platform } from 'os';

const targets = {
  darwin: ['dmg'],
  windows: ['nsis'],
  linux: ['deb', 'appimage'],
};

function build(target: string[]) {
  console.log(`Building for ${target.join(', ')}...`);
  
  execSync(`tauri build --target ${target.join(' --target ')}`, {
    stdio: 'inherit',
    env: {
      ...process.env,
      TAURI_SIGNING_PRIVATE_KEY: process.env.TAURI_SIGNING_PRIVATE_KEY,
      APPLE_CERTIFICATE: process.env.APPLE_CERTIFICATE,
      WINDOWS_CERTIFICATE: process.env.WINDOWS_CERTIFICATE,
    },
  });
}

const currentPlatform = platform();
const target = targets[currentPlatform as keyof typeof targets] || ['appimage'];

build(target);

console.log('Build complete!');
console.log('Output: apps/desktop/src-tauri/target/release/bundle/');
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] All panels work with real API
- [ ] E2E tests pass (10+ tests)
- [ ] Performance benchmarks met (<2s startup, <100ms API)
- [ ] Release builds created (.dmg, .exe, .deb)
- [ ] 20+ tests passing (5+10+5 from subagents)
- [ ] No critical bugs
- [ ] Documentation complete

---

## 🔗 Dependencies

**Requires:**
- Phase 3 complete (real backend API)
- All Phase 4 sprints complete (C4, A4, B4, C5, A5, B5)

**Blocks:**
- None (final Phase 4 sprint)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Backend Integration (parallel)
  ├─ Subagent 2: E2E Testing (parallel)
  └─ Subagent 3: Performance + Builds (parallel)
  ↓
Lead Agent: Full Integration + Polish + Release
```

**Integration Checklist:**
- [ ] Chat Panel → Real API
- [ ] Progress Dashboard → Real WebSocket
- [ ] Settings → Real backend
- [ ] All panels work together
- [ ] Error handling works
- [ ] E2E tests pass
- [ ] Performance benchmarks met
- [ ] Release builds created

**Difficulty: HIGH**
- 3 subagents to coordinate
- Full backend integration (Phase 3)
- E2E testing complexity
- Cross-platform builds (macOS, Windows, Linux)
- Performance optimization

**Review Checklist:**
- [ ] All panels work with real API
- [ ] WebSocket real-time progress works
- [ ] E2E tests pass (full mission flow)
- [ ] Performance benchmarks met
- [ ] Release builds created for all platforms
- [ ] No critical bugs

---

**Start AFTER Phase 3 complete AND all Phase 4 sprints complete.**

**This is FINAL Phase 4 sprint — leads to production release.**
