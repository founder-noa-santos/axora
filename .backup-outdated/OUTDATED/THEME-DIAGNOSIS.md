# 🎨 Theme System — Diagnosis & Implementation Plan

**Date:** 2026-03-17
**Status:** ⚠️ **Needs Implementation**

---

## 🔍 Diagnosis

### ✅ What Exists

1. **Settings Type** (`types/settings.ts`):
   ```typescript
   theme: {
     mode: 'light' | 'dark' | 'system';  // ✅ Correct type
     accentColor: string;
   }
   ```

2. **Settings Store** (`store/settings-store.ts`):
   - ✅ Has theme settings in state
   - ✅ Can update theme mode

3. **CSS Variables** (`styles/globals.css`):
   - ✅ Has `@theme` directive (Tailwind v4)
   - ✅ Has `:root` variables

---

## ❌ Problems Identified

### Problem 1: Only Dark Mode Exists

**Current CSS:**
```css
:root {
  --background: 222 47% 11%;  /* Only dark mode */
  --foreground: 210 40% 98%;
  /* ... all colors are dark mode only */
}
```

**Missing:**
- ❌ Light mode CSS variables
- ❌ `.dark` class switching
- ❌ Theme-aware color system

---

### Problem 2: No Theme Hook

**Missing:**
- ❌ `useTheme` hook doesn't exist
- ❌ No `hooks/use-theme.ts` file
- ❌ No system preference detection

---

### Problem 3: No Theme Provider

**Missing:**
- ❌ `ThemeProvider` component doesn't exist
- ❌ App not wrapped in theme context
- ❌ No macOS system preference listener

---

### Problem 4: Theme Not Applied

**Current `settings-store.ts`:**
```typescript
updateSetting: (section, key, value) => {
  set((state) => ({
    settings: {
      ...state.settings,
      [section]: { ...state.settings[section], [key]: value },
    },
    hasUnsavedChanges: true,
  }));
}
```

**Problem:** Updates state but **doesn't apply theme** to document!

---

### Problem 5: App.tsx Not Theme-Aware

**Current:**
```tsx
function App() {
  const [activePanel, setActivePanel] = useState<Panel>('chat');
  
  return (
    <div className="flex h-screen flex-col bg-background">
      {/* No theme provider */}
    </div>
  );
}
```

**Missing:**
- ❌ No theme provider wrapper
- ❌ No theme switching UI
- ❌ No system preference sync

---

## 🎯 Solution

### What Needs to Be Implemented

| Component | File | Status |
|-----------|------|--------|
| **CSS Variables** | `styles/globals.css` | ⚠️ Partial (only dark) |
| **Theme Hook** | `hooks/use-theme.ts` | ❌ Missing |
| **Theme Provider** | `components/theme-provider.tsx` | ❌ Missing |
| **Settings Sync** | `store/settings-store.ts` | ⚠️ Needs update |
| **App Wrapper** | `main.tsx` | ❌ Missing |

---

## 📋 Implementation Files Created

### 1. Meta-Prompt (Investigation)
**File:** `planning/shared/THEME-IMPLEMENTATION-META-PROMPT.md`

**Contains:**
- ✅ Full diagnosis
- ✅ Problem analysis
- ✅ shadcn/ui patterns
- ✅ Success criteria

---

### 2. Implementation Guide
**File:** `planning/shared/THEME-IMPLEMENTATION.md`

**Contains:**
- ✅ Step-by-step tasks
- ✅ Code examples
- ✅ Test cases
- ✅ Color reference

---

## 🎨 shadcn/ui Pattern

### The Correct Way

**CSS Variables:**
```css
/* Light mode (default) */
:root {
  --background: 0 0% 100%;
  --foreground: 222 47% 11%;
}

/* Dark mode */
.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
}

/* Usage */
.card {
  background-color: hsl(var(--background));  /* Auto-switches! */
}
```

**Theme Hook:**
```typescript
export function useTheme() {
  const [theme, setTheme] = useState('system');
  
  // Apply .dark class
  useEffect(() => {
    document.documentElement.classList.toggle('dark', theme === 'dark');
  }, [theme]);
  
  // Listen to macOS system preference
  useEffect(() => {
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      setSystemTheme(mediaQuery.matches ? 'dark' : 'light');
    };
    
    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, []);
  
  return { theme, setTheme };
}
```

---

## ✅ Success Criteria

After implementation:

- [ ] Light mode CSS variables defined
- [ ] Dark mode CSS variables defined
- [ ] `.dark` class applied/removed correctly
- [ ] `useTheme` hook working
- [ ] `ThemeProvider` wraps app
- [ ] System preference detection (macOS)
- [ ] Settings panel theme toggle functional
- [ ] No hard-coded colors
- [ ] No flash of unstyled content (FOUC)
- [ ] 10+ tests passing

---

## 🚀 Next Steps

**For Implementing LLM:**

1. **Read diagnosis:** `THEME-IMPLEMENTATION-META-PROMPT.md`
2. **Follow guide:** `THEME-IMPLEMENTATION.md`
3. **Implement in order:**
   - CSS variables (light + dark)
   - useTheme hook
   - ThemeProvider component
   - Settings sync
   - App wrapper
4. **Test:** Both themes, system preference
5. **Write tests:** 10+ passing

---

## 📚 Reference Files

| File | Purpose |
|------|---------|
| `planning/shared/THEME-IMPLEMENTATION-META-PROMPT.md` | Investigation & diagnosis |
| `planning/shared/THEME-IMPLEMENTATION.md` | Step-by-step guide |
| `apps/desktop/src/styles/globals.css` | Current CSS (needs update) |
| `apps/desktop/src/types/settings.ts` | Settings types |
| `apps/desktop/src/store/settings-store.ts` | Settings store |

---

**Ready for implementation!** 🚀

**Prompt files created:**
- ✅ Meta-prompt (investigation)
- ✅ Implementation guide (tasks)

**LLM can now implement following shadcn/ui patterns!**
