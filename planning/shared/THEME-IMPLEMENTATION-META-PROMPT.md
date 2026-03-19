# Meta-Prompt: AXORA Theme System Implementation

**Context:** You are an AI researcher tasked with analyzing the AXORA codebase and implementing a proper theme system following shadcn/ui patterns for light/dark mode that automatically follows macOS system preferences.

---

## 🔍 Investigation & Diagnosis

### Current State Analysis

**What I Found:**

1. **CSS Variables Exist** (`apps/desktop/src/styles/globals.css`):
   - ✅ Has `@theme` directive with Tailwind v4
   - ✅ Has `:root` variables for shadcn compatibility
   - ❌ **PROBLEM:** Only dark mode colors defined (hardcoded)
   - ❌ **PROBLEM:** No light mode theme defined
   - ❌ **PROBLEM:** No `.dark` class support

2. **Settings Type Exists** (`apps/desktop/src/types/settings.ts`):
   ```typescript
   theme: {
     mode: 'light' | 'dark' | 'system';  // ✅ Correct type
     accentColor: string;
   }
   ```

3. **Settings Store Exists** (`apps/desktop/src/store/settings-store.ts`):
   - ✅ Has theme settings in state
   - ❌ **PROBLEM:** No actual theme application logic
   - ❌ **PROBLEM:** Doesn't sync with CSS variables

4. **App.tsx**:
   - ❌ **PROBLEM:** No theme provider/hook
   - ❌ **PROBLEM:** No system preference detection
   - ❌ **PROBLEM:** No dynamic class application

5. **Missing Files**:
   - ❌ No `useTheme` hook
   - ❌ No `ThemeProvider` component
   - ❌ No theme CSS variable switching logic

---

## 🎯 What Needs to Be Implemented

### Problem Statement

**Current Issue:**
- Theme is hardcoded to dark mode only
- No support for light mode
- No support for system preference (macOS)
- CSS variables don't switch between themes
- Settings store doesn't apply theme changes

**Required Solution:**
Implement a theme system that:
1. Follows shadcn/ui patterns (CSS variables + `.dark` class)
2. Supports light, dark, and system modes
3. Automatically follows macOS system appearance
4. Uses CSS variables (no hard-coded colors)
5. Persists user preference
6. Applies theme changes dynamically

---

## 📋 Implementation Plan

### Phase 1: Theme Hook & Provider

**Create:** `apps/desktop/src/hooks/use-theme.ts`

**Requirements:**
```typescript
// Must support these modes
type ThemeMode = 'light' | 'dark' | 'system';

// Hook API
export function useTheme() {
  return {
    theme: 'dark' | 'light',  // Resolved theme
    setTheme: (mode: ThemeMode) => void,
    systemTheme: 'dark' | 'light',  // Current system preference
  };
}
```

**Features:**
- ✅ Listen to macOS system preference changes (`matchMedia`)
- ✅ Resolve 'system' mode to actual light/dark
- ✅ Apply `.dark` class to `<html>` element
- ✅ Persist preference to localStorage
- ✅ Sync with settings store

---

### Phase 2: Theme Provider Component

**Create:** `apps/desktop/src/components/theme-provider.tsx`

**Requirements:**
```tsx
export function ThemeProvider({ children, defaultTheme = 'system' }) {
  // Initialize theme from settings store
  // Apply theme class on mount
  // Listen for system changes
  return children;
}
```

**Integration:**
- Wrap `<App />` in `main.tsx`
- Read from `useSettingsStore`
- Apply theme before first paint (prevent flash)

---

### Phase 3: CSS Variable Themes

**Update:** `apps/desktop/src/styles/globals.css`

**Current (WRONG):**
```css
:root {
  --background: 222 47% 11%;  /* Only dark mode */
  --foreground: 210 40% 98%;
}
```

**Required (CORRECT):**
```css
/* Light mode (default) */
:root {
  --background: 0 0% 100%;
  --foreground: 222 47% 11%;
  --card: 0 0% 100%;
  --primary: 271 76% 53%;
  /* ... all other variables */
}

/* Dark mode */
.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  --card: 222 47% 15%;
  --primary: 271 76% 53%;
  /* ... all other variables */
}
```

**Brand Colors to Preserve:**
- Primary: Electric Purple `hsl(271 76% 53%)`
- Secondary: Emerald Green `hsl(160 84% 39%)`
- Accent: Electric Purple Light `hsl(271 76% 63%)`

**Light Mode Palette:**
- Background: White `hsl(0 0% 100%)`
- Foreground: Dark `hsl(222 47% 11%)`
- Card: White `hsl(0 0% 100%)`
- Muted: Light Gray `hsl(210 40% 96%)`
- Border: Light Gray `hsl(214 32% 91%)`

---

### Phase 4: Settings Integration

**Update:** `apps/desktop/src/store/settings-store.ts`

**Add:**
```typescript
// Apply theme when settings change
useEffect(() => {
  const unsubscribe = useSettingsStore.subscribe(
    (state) => state.settings.theme.mode,
    (mode) => {
      // Apply theme change
      applyTheme(mode);
    }
  );
  return unsubscribe;
}, []);
```

---

### Phase 5: App Integration

**Update:** `apps/desktop/src/main.tsx`

```tsx
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ThemeProvider>
      <App />
    </ThemeProvider>
  </React.StrictMode>
);
```

---

## 🎨 shadcn/ui Patterns to Follow

### 1. CSS Variables (No Hard-coded Colors)

**❌ WRONG:**
```css
.card {
  background-color: #1a1a2e;  /* Hard-coded */
}
```

**✅ CORRECT:**
```css
.card {
  background-color: hsl(var(--card));  /* CSS variable */
}
```

### 2. Dark Mode via `.dark` Class

**❌ WRONG:**
```css
@media (prefers-color-scheme: dark) {
  .card { background: dark; }
}
```

**✅ CORRECT:**
```css
:root {
  --card: 0 0% 100%;  /* Light */
}

.dark {
  --card: 222 47% 15%;  /* Dark */
}

.card {
  background-color: hsl(var(--card));  /* Auto-switches */
}
```

### 3. System Preference Detection

**✅ CORRECT:**
```typescript
// Hook into macOS system preference
const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');

mediaQuery.addEventListener('change', (e) => {
  setSystemTheme(e.matches ? 'dark' : 'light');
});
```

---

## ✅ Success Criteria

Your implementation is complete when:

- [ ] `useTheme` hook created and working
- [ ] `ThemeProvider` component wraps app
- [ ] CSS variables defined for BOTH light and dark modes
- [ ] `.dark` class applied to `<html>` element
- [ ] System preference detection works (macOS)
- [ ] Settings store theme changes apply immediately
- [ ] No hard-coded colors in CSS
- [ ] No flash of unstyled content (FOUC)
- [ ] All existing components work in both themes
- [ ] 10+ tests passing

---

## 📚 Reference Files

**Read These First:**
1. `apps/desktop/src/styles/globals.css` — Current CSS
2. `apps/desktop/src/types/settings.ts` — Settings types
3. `apps/desktop/src/store/settings-store.ts` — Settings state
4. `apps/desktop/src/App.tsx` — Main app component
5. `apps/desktop/src/main.tsx` — Entry point

**shadcn/ui References:**
- https://ui.shadcn.com/docs/theming
- https://ui.shadcn.com/docs/dark-mode

---

## 🚀 Implementation Order

**Execute in this exact order:**

1. **Read all reference files** (understand current state)
2. **Update `globals.css`** — Add light/dark CSS variables
3. **Create `use-theme.ts` hook** — Theme logic
4. **Create `theme-provider.tsx`** — Provider component
5. **Update `settings-store.ts`** — Add theme application
6. **Update `main.tsx`** — Wrap with provider
7. **Test both themes** — Verify switching works
8. **Write tests** — 10+ passing tests

---

## 🎯 Example Usage

After implementation, this should work:

```typescript
// In any component
import { useTheme } from '@/hooks/use-theme';

function MyComponent() {
  const { theme, setTheme, systemTheme } = useTheme();
  
  return (
    <div>
      <p>Current theme: {theme}</p>
      <p>System theme: {systemTheme}</p>
      
      <Button onClick={() => setTheme('light')}>Light</Button>
      <Button onClick={() => setTheme('dark')}>Dark</Button>
      <Button onClick={() => setTheme('system')}>System</Button>
    </div>
  );
}
```

---

## 📝 Notes

- **Priority:** macOS system preference support
- **Pattern:** Follow shadcn/ui exactly (CSS variables + `.dark` class)
- **No:** Hard-coded colors, inline styles, or `@media (prefers-color-scheme)`
- **Yes:** CSS variables, `.dark` class, `matchMedia` API

**Start implementation now!** 🚀
