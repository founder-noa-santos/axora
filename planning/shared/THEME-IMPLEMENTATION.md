# Theme System Implementation

**Mission:** Implement proper light/dark theme system following shadcn/ui patterns with macOS system preference support.

---

## 🎯 Your Tasks

### Task 1: Update CSS Variables

**File:** `apps/desktop/src/styles/globals.css`

**What to do:**

1. Keep `@theme` directive for Tailwind v4 (both modes)
2. Update `:root` to have LIGHT mode colors
3. Add `.dark` selector with DARK mode colors

**Example:**
```css
@import "tailwindcss";

@theme {
  /* Define colors for Tailwind v4 */
  --color-background-light: hsl(0 0% 100%);
  --color-background-dark: hsl(222 47% 11%);
  /* ... etc */
}

/* LIGHT MODE (default) */
:root {
  --background: 0 0% 100%;
  --foreground: 222 47% 11%;
  --card: 0 0% 100%;
  --card-foreground: 222 47% 11%;
  --popover: 0 0% 100%;
  --popover-foreground: 222 47% 11%;
  --primary: 271 76% 53%;
  --primary-foreground: 210 40% 98%;
  --secondary: 160 84% 39%;
  --secondary-foreground: 210 40% 98%;
  --muted: 210 40% 96%;
  --muted-foreground: 215 16% 47%;
  --accent: 271 76% 63%;
  --accent-foreground: 210 40% 98%;
  --destructive: 0 84% 60%;
  --destructive-foreground: 210 40% 98%;
  --border: 214 32% 91%;
  --input: 214 32% 91%;
  --ring: 271 76% 53%;
}

/* DARK MODE */
.dark {
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  --card: 222 47% 15%;
  --card-foreground: 210 40% 98%;
  --popover: 222 47% 15%;
  --popover-foreground: 210 40% 98%;
  --primary: 271 76% 53%;
  --primary-foreground: 210 40% 98%;
  --secondary: 160 84% 39%;
  --secondary-foreground: 210 40% 98%;
  --muted: 217 33% 17%;
  --muted-foreground: 215 20% 65%;
  --accent: 271 76% 63%;
  --accent-foreground: 210 40% 98%;
  --destructive: 0 63% 31%;
  --destructive-foreground: 210 40% 98%;
  --border: 217 33% 25%;
  --input: 217 33% 25%;
  --ring: 271 76% 53%;
}
```

---

### Task 2: Create Theme Hook

**File:** `apps/desktop/src/hooks/use-theme.ts`

**Code:**
```typescript
import { useEffect, useState } from 'react';
import { useSettingsStore } from '@/store/settings-store';

type ThemeMode = 'light' | 'dark' | 'system';

export function useTheme() {
  const { settings, updateSetting } = useSettingsStore();
  const [systemTheme, setSystemTheme] = useState<'light' | 'dark'>(() => {
    if (typeof window === 'undefined') return 'dark';
    return window.matchMedia('(prefers-color-scheme: dark)').matches
      ? 'dark'
      : 'light';
  });

  // Resolve theme based on mode
  const theme: 'light' | 'dark' =
    settings.theme.mode === 'system'
      ? systemTheme
      : settings.theme.mode;

  // Apply theme class to document
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'dark') {
      root.classList.add('dark');
    } else {
      root.classList.remove('dark');
    }
  }, [theme]);

  // Listen for system theme changes
  useEffect(() => {
    if (settings.theme.mode !== 'system') return;

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      setSystemTheme(mediaQuery.matches ? 'dark' : 'light');
    };

    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [settings.theme.mode]);

  const setTheme = (mode: ThemeMode) => {
    updateSetting('theme', 'mode', mode);
  };

  return {
    theme,
    themeMode: settings.theme.mode,
    systemTheme,
    setTheme,
  };
}
```

---

### Task 3: Create Theme Provider

**File:** `apps/desktop/src/components/theme-provider.tsx`

**Code:**
```tsx
'use client';

import { useEffect } from 'react';
import { useSettingsStore } from '@/store/settings-store';

interface ThemeProviderProps {
  children: React.ReactNode;
  defaultTheme?: 'light' | 'dark' | 'system';
}

export function ThemeProvider({
  children,
  defaultTheme = 'system',
}: ThemeProviderProps) {
  const { settings, updateSetting } = useSettingsStore();

  // Initialize theme on mount (prevent FOUC)
  useEffect(() => {
    // If no theme saved, use default
    if (!settings.theme.mode) {
      updateSetting('theme', 'mode', defaultTheme);
    }
  }, []);

  return <>{children}</>;
}
```

---

### Task 4: Update Main.tsx

**File:** `apps/desktop/src/main.tsx`

**Add:**
```tsx
import { ThemeProvider } from '@/components/theme-provider';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <ThemeProvider defaultTheme="system">
      <App />
    </ThemeProvider>
  </React.StrictMode>
);
```

---

### Task 5: Add Theme Toggle to Settings

**File:** `apps/desktop/src/panels/SettingsPanel.tsx`

**Find the theme section and ensure it has:**
```tsx
<Select
  value={settings.theme.mode}
  onValueChange={(value: 'light' | 'dark' | 'system') =>
    updateSetting('theme', 'mode', value)
  }
>
  <SelectTrigger>
    <SelectValue />
  </SelectTrigger>
  <SelectContent>
    <SelectItem value="system">System (macOS)</SelectItem>
    <SelectItem value="light">Light</SelectItem>
    <SelectItem value="dark">Dark</SelectItem>
  </SelectContent>
</Select>
```

---

### Task 6: Create Tests

**File:** `apps/desktop/src/hooks/__tests__/use-theme.test.ts`

**Code:**
```typescript
import { renderHook, act } from '@testing-library/react';
import { useTheme } from '../use-theme';
import { useSettingsStore } from '@/store/settings-store';

describe('useTheme', () => {
  beforeEach(() => {
    // Reset settings
    useSettingsStore.getState().resetSettings();
  });

  it('should return current theme', () => {
    const { result } = renderHook(() => useTheme());
    expect(result.current.theme).toBeDefined();
  });

  it('should resolve system mode to actual theme', () => {
    const { result } = renderHook(() => useTheme());
    
    act(() => {
      result.current.setTheme('system');
    });

    expect(result.current.themeMode).toBe('system');
    expect(['light', 'dark']).toContain(result.current.theme);
  });

  it('should apply dark class when theme is dark', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('dark');
    });

    expect(document.documentElement.classList.contains('dark')).toBe(true);
  });

  it('should remove dark class when theme is light', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('light');
    });

    expect(document.documentElement.classList.contains('dark')).toBe(false);
  });

  it('should listen to system theme changes', () => {
    const { result } = renderHook(() => useTheme());

    act(() => {
      result.current.setTheme('system');
    });

    // Simulate system theme change
    const event = new MediaQueryListEvent('change', { matches: true });
    window.matchMedia('(prefers-color-scheme: dark)').dispatchEvent(event);

    expect(result.current.systemTheme).toBe('dark');
  });
});
```

---

## ✅ Checklist

After implementation, verify:

- [ ] CSS variables defined for BOTH light and dark
- [ ] `.dark` class applied/removed correctly
- [ ] `useTheme` hook returns correct values
- [ ] `ThemeProvider` wraps app
- [ ] System preference detection works
- [ ] Settings panel theme toggle works
- [ ] No hard-coded colors in CSS
- [ ] No flash of unstyled content
- [ ] 5+ tests passing

---

## 🎨 Color Reference

### Light Mode (Brand-Aligned)
```css
--background: 0 0% 100%      /* Pure white */
--foreground: 222 47% 11%    /* Dark blue-black */
--card: 0 0% 100%            /* White cards */
--muted: 210 40% 96%         /* Very light gray */
--border: 214 32% 91%        /* Light border */
--primary: 271 76% 53%       /* Electric Purple */
--secondary: 160 84% 39%     /* Emerald Green */
```

### Dark Mode (Current Brand)
```css
--background: 222 47% 11%    /* Deep blue */
--foreground: 210 40% 98%    /* Near white */
--card: 222 47% 15%          /* Slightly lighter blue */
--muted: 217 33% 17%         /* Muted blue-gray */
--border: 217 33% 25%        /* Blue border */
--primary: 271 76% 53%       /* Electric Purple */
--secondary: 160 84% 39%     /* Emerald Green */
```

---

**Start implementation now!** 🚀
