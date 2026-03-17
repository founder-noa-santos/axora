# Phase 4 Sprint B4: Settings & Configuration Panel

**Agent:** B (API Integration + Configuration)  
**Sprint:** B4  
**Priority:** MEDIUM  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement Settings panel for model selection, token limits, worker pool settings, and theme preferences.

**Context:** Users need to configure AXORA behavior (models, limits, preferences).

**Difficulty:** ⚠️ **MEDIUM** — Settings UI, local storage, sync with backend

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: Settings UI + Forms
**Task:** Implement Settings panel UI with forms
**File:** `apps/desktop/src/panels/SettingsPanel.tsx`
**Deliverables:**
- Settings panel layout (sections, groups)
- Form components (select, input, toggle)
- Validation (required fields, ranges)
- Save/cancel buttons
- 5+ tests

### Subagent 2: Settings Storage + Sync
**Task:** Implement settings storage and backend sync
**File:** `apps/desktop/src/store/settings-store.ts`
**Deliverables:**
- Settings state management (Zustand/Context)
- Local storage persistence
- Backend sync (API integration)
- Settings import/export
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 2 Subagents:**
   - Assign tasks to both subagents
   - Review UI + storage implementations
   - Ensure consistency

2. **Integrate Components:**
   - Connect Settings UI to settings store
   - Implement real-time validation
   - Add save confirmation dialogs

3. **Implement Settings Sections:**
   - Model selection (Ollama, OpenAI, etc.)
   - Token limits (max tokens per request)
   - Worker pool (min/max workers)
   - Theme preferences (light/dark)

4. **Write Integration Tests:**
   - Test settings save/load
   - Test validation works
   - Test persistence across restarts
   - Test backend sync

5. **Update Documentation:**
   - Add settings documentation
   - Add configuration examples

---

## 📐 Technical Spec

### Settings Interface

```typescript
// apps/desktop/src/types/settings.ts
export interface AppSettings {
  // Model configuration
  model: {
    provider: 'ollama' | 'openai' | 'anthropic';
    model: string;
    baseUrl?: string; // For Ollama
    apiKey?: string;  // For OpenAI/Anthropic
  };
  
  // Token limits
  tokens: {
    maxTokensPerRequest: number;  // Default: 4096
    maxContextTokens: number;     // Default: 8192
    tokenBudget: number;          // Default: 100000
  };
  
  // Worker pool
  workers: {
    minWorkers: number;   // Default: 2
    maxWorkers: number;   // Default: 10
    healthCheckInterval: number; // Default: 30 (seconds)
  };
  
  // Theme
  theme: {
    mode: 'light' | 'dark' | 'system';
    accentColor: string;  // Default: 'electric-purple'
  };
  
  // Advanced
  advanced: {
    enableLogging: boolean;
    logLevel: 'debug' | 'info' | 'warn' | 'error';
    autoUpdate: boolean;
  };
}

export const defaultSettings: AppSettings = {
  model: {
    provider: 'ollama',
    model: 'qwen2.5-coder:7b',
    baseUrl: 'http://localhost:11434',
  },
  tokens: {
    maxTokensPerRequest: 4096,
    maxContextTokens: 8192,
    tokenBudget: 100000,
  },
  workers: {
    minWorkers: 2,
    maxWorkers: 10,
    healthCheckInterval: 30,
  },
  theme: {
    mode: 'dark',
    accentColor: 'electric-purple',
  },
  advanced: {
    enableLogging: true,
    logLevel: 'info',
    autoUpdate: true,
  },
};
```

### Settings Store

```typescript
// apps/desktop/src/store/settings-store.ts
import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { AppSettings, defaultSettings } from '../types/settings';

interface SettingsStore {
  settings: AppSettings;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  loadSettings: () => Promise<void>;
  saveSettings: (settings: Partial<AppSettings>) => Promise<void>;
  resetSettings: () => void;
  exportSettings: () => string;
  importSettings: (json: string) => Promise<void>;
}

export const useSettingsStore = create<SettingsStore>()(
  persist(
    (set, get) => ({
      settings: defaultSettings,
      isLoading: false,
      error: null,
      
      loadSettings: async () => {
        set({ isLoading: true });
        try {
          // Load from backend API
          const response = await fetch('/api/settings');
          const settings = await response.json();
          set({ settings, isLoading: false });
        } catch (error) {
          set({ error: error.message, isLoading: false });
        }
      },
      
      saveSettings: async (newSettings) => {
        set({ isLoading: true });
        try {
          const settings = { ...get().settings, ...newSettings };
          // Save to backend API
          await fetch('/api/settings', {
            method: 'PUT',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(settings),
          });
          set({ settings, isLoading: false });
        } catch (error) {
          set({ error: error.message, isLoading: false });
        }
      },
      
      resetSettings: () => {
        set({ settings: defaultSettings });
      },
      
      exportSettings: () => {
        return JSON.stringify(get().settings, null, 2);
      },
      
      importSettings: async (json: string) => {
        try {
          const settings = JSON.parse(json);
          await get().saveSettings(settings);
        } catch (error) {
          set({ error: 'Invalid settings JSON' });
        }
      },
    }),
    {
      name: 'axora-settings', // Local storage key
      partialize: (state) => ({ settings: state.settings }),
    }
  )
);
```

### Settings Panel UI

```tsx
// apps/desktop/src/panels/SettingsPanel.tsx
import { useState } from 'react';
import { useSettingsStore } from '../store/settings-store';
import { Button } from '../components/ui/Button';
import { Input } from '../components/ui/Input';
import { Select } from '../components/ui/Select';
import { Toggle } from '../components/ui/Toggle';
import { Card } from '../components/ui/Card';

export function SettingsPanel() {
  const { settings, saveSettings, resetSettings } = useSettingsStore();
  const [hasChanges, setHasChanges] = useState(false);
  
  const handleChange = (section: string, key: string, value: any) => {
    // Update local state
    setHasChanges(true);
  };
  
  const handleSave = async () => {
    await saveSettings(settings);
    setHasChanges(false);
  };
  
  return (
    <div className="settings-panel">
      <header className="panel-header">
        <h2>Settings</h2>
        <div className="actions">
          <Button variant="ghost" onClick={resetSettings}>
            Reset to Defaults
          </Button>
          <Button 
            variant="primary" 
            onClick={handleSave}
            disabled={!hasChanges}
          >
            Save Changes
          </Button>
        </div>
      </header>
      
      <div className="settings-content">
        {/* Model Configuration Section */}
        <Card title="Model Configuration">
          <Select
            label="Provider"
            value={settings.model.provider}
            onChange={(v) => handleChange('model', 'provider', v)}
            options={[
              { value: 'ollama', label: 'Ollama (Local)' },
              { value: 'openai', label: 'OpenAI' },
              { value: 'anthropic', label: 'Anthropic' },
            ]}
          />
          <Input
            label="Model"
            value={settings.model.model}
            onChange={(v) => handleChange('model', 'model', v)}
          />
          {settings.model.provider === 'ollama' && (
            <Input
              label="Base URL"
              value={settings.model.baseUrl}
              onChange={(v) => handleChange('model', 'baseUrl', v)}
            />
          )}
        </Card>
        
        {/* Token Limits Section */}
        <Card title="Token Limits">
          <Input
            type="number"
            label="Max Tokens per Request"
            value={settings.tokens.maxTokensPerRequest}
            onChange={(v) => handleChange('tokens', 'maxTokensPerRequest', parseInt(v))}
          />
          {/* ... other token settings */}
        </Card>
        
        {/* Worker Pool Section */}
        <Card title="Worker Pool">
          <Input
            type="number"
            label="Min Workers"
            value={settings.workers.minWorkers}
            onChange={(v) => handleChange('workers', 'minWorkers', parseInt(v))}
          />
          <Input
            type="number"
            label="Max Workers"
            value={settings.workers.maxWorkers}
            onChange={(v) => handleChange('workers', 'maxWorkers', parseInt(v))}
          />
        </Card>
        
        {/* Theme Section */}
        <Card title="Theme">
          <Select
            label="Mode"
            value={settings.theme.mode}
            onChange={(v) => handleChange('theme', 'mode', v)}
            options={[
              { value: 'light', label: 'Light' },
              { value: 'dark', label: 'Dark' },
              { value: 'system', label: 'System' },
            ]}
          />
        </Card>
      </div>
    </div>
  );
}
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] Settings panel compiles and works
- [ ] Settings save/load works
- [ ] Local storage persistence works
- [ ] 10+ tests passing (5 per subagent + 5 integration)
- [ ] All settings sections implemented
- [ ] Validation works (invalid values rejected)

---

## 🔗 Dependencies

**Requires:**
- Sprint A4 complete (UI Components for forms)

**Blocks:**
- Sprint C6 (Integration needs settings)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Settings UI (parallel)
  └─ Subagent 2: Settings Storage (parallel)
  ↓
Lead Agent: Integration + Validation + Tests
```

**Settings Design:**
- Group related settings (Model, Tokens, Workers, Theme)
- Sensible defaults (work out of box)
- Validation (prevent invalid values)
- Import/export (portability)

**Difficulty: MEDIUM**
- 2 subagents to coordinate
- Form validation complexity
- Local storage + backend sync
- Settings migration (versioning)

**Review Checklist:**
- [ ] All settings sections implemented
- [ ] Validation works (invalid rejected)
- [ ] Persistence works (survives restart)
- [ ] Backend sync works (when available)
- [ ] Import/export works

---

**Start AFTER Sprint A4 complete.**
