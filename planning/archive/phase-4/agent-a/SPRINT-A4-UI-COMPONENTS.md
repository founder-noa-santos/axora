# Phase 4 Sprint A4: shadcn/ui Setup + Customization

**Agent:** A (UI Components + Progress Display)  
**Sprint:** A4  
**Priority:** HIGH  
**Estimated:** 4 hours (was 8 hours — 50% faster with shadcn!)  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Setup shadcn/ui component library and customize theme to match BRAND-INSPIRATION.md.

**Context:** Building UI components from scratch is time-consuming. shadcn/ui provides high-quality, accessible, customizable components.

**Decision:** Use shadcn/ui (ADR-050) instead of building from scratch.

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: shadcn/ui Setup + Tailwind Config
**Task:** Install and configure shadcn/ui
**File:** `apps/desktop/src/components/ui/`
**Deliverables:**
- shadcn/ui installed and configured
- Tailwind CSS configured
- Core components installed (Button, Input, Card, Badge, Progress, etc.)
- Aliases configured (`@/components/ui/...`)
- 5+ tests

### Subagent 2: Theme Customization + Custom Components
**Task:** Customize theme to match brand + build custom components
**File:** `apps/desktop/src/styles/globals.css`
**Deliverables:**
- Theme colors (Deep Blue, Electric Purple, Emerald Green, Slate Gray)
- Typography (Inter font)
- Dark mode support (default)
- Custom components (only what shadcn doesn't have)
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 2 Subagents:**
   - Assign tasks to both subagents
   - Review shadcn setup + theme customization
   - Ensure components work in Tauri app

2. **Install shadcn/ui:**
   ```bash
   cd apps/desktop
   npx shadcn-ui@latest init
   npx shadcn-ui@latest add button input card badge progress tabs scroll-area separator
   ```

3. **Customize Theme:**
   - Update `globals.css` with brand colors (from BRAND-INSPIRATION.md)
   - Configure Tailwind config
   - Test dark mode (default)

4. **Write Integration Tests:**
   - Test components render correctly
   - Test accessibility (keyboard navigation)
   - Test theme colors applied

5. **Update Documentation:**
   - Add component usage examples
   - Add theme customization guide

---

## 📐 Technical Spec

### shadcn/ui Installation

```bash
# Initialize shadcn/ui
cd apps/desktop
npx shadcn-ui@latest init

# Configuration (accept defaults)
? Would you like to use TypeScript (recommended)? Yes
? Which style would you like to use? Default
? Which color would you like to use as base color? Slate
? Would you like to use a CSS variable for colors? Yes
? Would you like to use Tailwind CSS for styling? Yes

# Install core components
npx shadcn-ui@latest add button
npx shadcn-ui@latest add input
npx shadcn-ui@latest add card
npx shadcn-ui@latest add badge
npx shadcn-ui@latest add progress
npx shadcn-ui@latest add tabs
npx shadcn-ui@latest add scroll-area
npx shadcn-ui@latest add separator
npx shadcn-ui@latest add textarea
npx shadcn-ui@latest add avatar
npx shadcn-ui@latest add select
npx shadcn-ui@latest add switch
npx shadcn-ui@latest add label
npx shadcn-ui@latest add alert
npx shadcn-ui@latest add tooltip
```

---

### Theme Customization (BRAND-INSPIRATION.md)

```css
/* apps/desktop/src/styles/globals.css */
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    /* Brand colors from BRAND-INSPIRATION.md */
    /* Deep Blue background */
    --background: 222 47% 11%;
    --foreground: 210 40% 98%;
    
    /* Card backgrounds (slightly lighter) */
    --card: 222 47% 15%;
    --card-foreground: 210 40% 98%;
    
    /* Popovers (menus, dropdowns) */
    --popover: 222 47% 15%;
    --popover-foreground: 210 40% 98%;
    
    /* Primary: Electric Purple (main brand color) */
    --primary: 271 76% 53%;
    --primary-foreground: 210 40% 98%;
    
    /* Secondary: Emerald Green (success, growth) */
    --secondary: 160 84% 39%;
    --secondary-foreground: 210 40% 98%;
    
    /* Muted: Slate Gray (subtle elements) */
    --muted: 217 33% 17%;
    --muted-foreground: 215 20% 65%;
    
    /* Accent: Electric Purple light */
    --accent: 271 76% 63%;
    --accent-foreground: 210 40% 98%;
    
    /* Destructive: Red (errors, warnings) */
    --destructive: 0 63% 31%;
    --destructive-foreground: 210 40% 98%;
    
    /* Borders and inputs */
    --border: 217 33% 25%;
    --input: 217 33% 25%;
    --ring: 271 76% 53%; /* Electric Purple for focus rings */
    
    /* Border radius */
    --radius: 0.5rem;
  }
}

@layer base {
  * {
    @apply border-border;
  }
  body {
    @apply bg-background text-foreground;
    font-family: 'Inter', system-ui, sans-serif;
  }
}
```

---

### Tailwind Config

```javascript
// apps/desktop/tailwind.config.js
/** @type {import('tailwindcss').Config} */
module.exports = {
  darkMode: ["class"],
  content: [
    './pages/**/*.{ts,tsx}',
    './components/**/*.{ts,tsx}',
    './app/**/*.{ts,tsx}',
    './src/**/*.{ts,tsx}',
  ],
  theme: {
    container: {
      center: true,
      padding: "2rem",
      screens: {
        "2xl": "1400px",
      },
    },
    extend: {
      colors: {
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        destructive: {
          DEFAULT: "hsl(var(--destructive))",
          foreground: "hsl(var(--destructive-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        accent: {
          DEFAULT: "hsl(var(--accent))",
          foreground: "hsl(var(--accent-foreground))",
        },
        popover: {
          DEFAULT: "hsl(var(--popover))",
          foreground: "hsl(var(--popover-foreground))",
        },
        card: {
          DEFAULT: "hsl(var(--card))",
          foreground: "hsl(var(--card-foreground))",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      keyframes: {
        "accordion-down": {
          from: { height: 0 },
          to: { height: "var(--radix-accordion-content-height)" },
        },
        "accordion-up": {
          from: { height: "var(--radix-accordion-content-height)" },
          to: { height: 0 },
        },
      },
      animation: {
        "accordion-down": "accordion-down 0.2s ease-out",
        "accordion-up": "accordion-up 0.2s ease-out",
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
}
```

---

### Component Usage Examples

```tsx
// apps/desktop/src/App.tsx
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';

function App() {
  return (
    <div className="app">
      <header className="header">
        <h1 className="text-2xl font-bold text-primary">AXORA</h1>
        <Badge variant="secondary">v0.1.0</Badge>
      </header>
      
      <div className="sidebar">
        {/* Navigation */}
      </div>
      
      <main className="content">
        <Card>
          <CardHeader>
            <CardTitle>Welcome to AXORA</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-muted-foreground">
              Autonomous AI Orchestration System
            </p>
            
            <div className="mt-4">
              <Progress value={75} className="w-full" />
              <p className="text-sm text-muted-foreground mt-2">
                System ready
              </p>
            </div>
            
            <div className="mt-4 flex gap-2">
              <Button variant="primary">New Mission</Button>
              <Button variant="secondary">Settings</Button>
            </div>
          </CardContent>
        </Card>
      </main>
    </div>
  );
}
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] shadcn/ui installed and configured
- [ ] Tailwind CSS configured with brand colors
- [ ] All core components installed (15+ components)
- [ ] Dark mode works (default theme)
- [ ] 10+ tests passing (5 per subagent + integration)
- [ ] Components work in Tauri app
- [ ] Documentation added (usage examples)

---

## 🔗 Dependencies

**Requires:**
- Sprint C4 complete (Tauri setup for React app)

**Blocks:**
- Sprint B4 (Settings needs UI components)
- Sprint C5 (Chat Interface needs UI components)
- Sprint A5 (Progress Dashboard needs UI components)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: shadcn/ui Setup (parallel)
  └─ Subagent 2: Theme Customization (parallel)
  ↓
Lead Agent: Integration + Tests
```

**shadcn/ui Benefits:**
- 50% faster (8h → 4h)
- Accessibility built-in (WCAG compliant)
- Customizable (Tailwind CSS)
- Well-maintained (active community)

**Difficulty: MEDIUM**
- 2 subagents to coordinate
- shadcn/ui setup (straightforward)
- Theme customization (requires brand alignment)
- Testing (accessibility tests)

**Review Checklist:**
- [ ] shadcn/ui installed correctly
- [ ] All components import correctly
- [ ] Theme colors match BRAND-INSPIRATION.md
- [ ] Dark mode works
- [ ] Components work in Tauri app
- [ ] Accessibility tests pass

---

## 📚 References

- [shadcn/ui Documentation](https://ui.shadcn.com)
- [ADR-050: Use shadcn/ui](../../docs/adr/ADR-050-use-shadcn-ui.md)
- [BRAND-INSPIRATION.md](../../BRAND-INSPIRATION.md)
- [Tailwind CSS Documentation](https://tailwindcss.com)

---

**Start AFTER Sprint C4 complete.**

**Time: 4 hours (50% faster than building from scratch!)**
