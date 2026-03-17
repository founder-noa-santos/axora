# ADR-050: Use shadcn/ui for Desktop App UI Components

**Date:** 2026-03-17  
**Status:** ✅ ACCEPTED  
**Phase:** Phase 4 (Desktop App)  
**Impact:** Sprint A4 (UI Components)

---

## Context

**Problem:** Building UI components from scratch (Button, Input, Card, etc.) is time-consuming and reinvents the wheel.

**Opportunity:** shadcn/ui provides high-quality, accessible, customizable React components built on Radix UI primitives.

**Question:** Should we use shadcn/ui for Phase 4 desktop app instead of building components from scratch?

---

## Decision

**USE shadcn/ui for Phase 4 Desktop App.**

**Why:**
- ✅ **Saves 8+ hours** (no need to build Button, Input, Card from scratch)
- ✅ **Accessibility built-in** (WCAG compliant, keyboard navigation)
- ✅ **Customizable** (Tailwind CSS, design tokens)
- ✅ **Well-maintained** (active community, frequent updates)
- ✅ **Tauri-compatible** (works perfectly with Tauri v2)
- ✅ **Dark mode support** (built-in theme switching)

**Trade-offs:**
- ⚠️ **Tailwind CSS required** (we need to use Tailwind)
- ⚠️ **Learning curve** (team needs to learn shadcn patterns)
- ⚠️ **Bundle size** (+50KB for components we use)

---

## Impact on Sprint A4

### Before (Building from Scratch)

```
Sprint A4: React UI Components (8 hours)
├─ Subagent 1: Design Tokens (colors, typography, spacing)
├─ Subagent 2: Core Components (Button, Input, Card, Badge)
└─ Subagent 3: Layout Components (Sidebar, Panel, Header)

Total: Build 20+ components from scratch
```

### After (Using shadcn/ui)

```
Sprint A4: shadcn/ui Setup + Customization (4 hours)
├─ Subagent 1: shadcn/ui Setup + Tailwind Config
├─ Subagent 2: Theme Customization (colors, fonts to match BRAND-INSPIRATION.md)
└─ Subagent 3: Custom Components (only what shadcn doesn't have)

Total: Setup shadcn + customize theme + build 2-3 custom components
Time Saved: 4 hours (50% reduction)
```

---

## Implementation Plan

### Step 1: Install shadcn/ui

```bash
cd apps/desktop
npx shadcn-ui@latest init
```

**Config:**
```json
{
  "style": "default",
  "tailwind": {
    "config": "tailwind.config.js",
    "css": "src/styles/globals.css",
    "baseColor": "slate"
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils"
  }
}
```

---

### Step 2: Install Required Components

```bash
# Core components (used in all panels)
npx shadcn-ui@latest add button
npx shadcn-ui@latest add input
npx shadcn-ui@latest add card
npx shadcn-ui@latest add badge
npx shadcn-ui@latest add progress
npx shadcn-ui@latest add tabs
npx shadcn-ui@latest add scroll-area
npx shadcn-ui@latest add separator

# Chat components
npx shadcn-ui@latest add textarea
npx shadcn-ui@latest add avatar

# Settings components
npx shadcn-ui@latest add select
npx shadcn-ui@latest add switch
npx shadcn-ui@latest add label
npx shadcn-ui@latest add slider

# Dashboard components
npx shadcn-ui@latest add alert
npx shadcn-ui@latest add tooltip
```

---

### Step 3: Customize Theme (BRAND-INSPIRATION.md)

```javascript
// apps/desktop/src/styles/globals.css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer base {
  :root {
    /* Brand colors from BRAND-INSPIRATION.md */
    --background: 222 47% 11%;      /* Deep Blue dark */
    --foreground: 210 40% 98%;
    
    --card: 222 47% 15%;
    --card-foreground: 210 40% 98%;
    
    --popover: 222 47% 15%;
    --popover-foreground: 210 40% 98%;
    
    /* Primary: Electric Purple */
    --primary: 271 76% 53%;
    --primary-foreground: 210 40% 98%;
    
    /* Secondary: Emerald Green */
    --secondary: 160 84% 39%;
    --secondary-foreground: 210 40% 98%;
    
    /* Muted: Slate Gray */
    --muted: 217 33% 17%;
    --muted-foreground: 215 20% 65%;
    
    /* Accent: Electric Purple light */
    --accent: 271 76% 63%;
    --accent-foreground: 210 40% 98%;
    
    --destructive: 0 63% 31%;
    --destructive-foreground: 210 40% 98%;
    
    --border: 217 33% 25%;
    --input: 217 33% 25%;
    --ring: 271 76% 53%;
    
    --radius: 0.5rem;
  }
}
```

---

### Step 4: Update Sprint A4

**File:** `planning/phase-4/agent-a/SPRINT-A4-UI-COMPONENTS.md`

**Changes:**
- Remove: "Build Core Components from scratch"
- Add: "Setup shadcn/ui + customize theme"
- Reduce: Sprint time from 8h to 4h
- Reduce: Subagents from 3 to 2

---

## Updated Sprint A4

```markdown
# Phase 4 Sprint A4: shadcn/ui Setup + Customization

**Agent:** A (UI Components + Progress Display)  
**Sprint:** A4  
**Priority:** HIGH  
**Estimated:** 4 hours (was 8 hours)  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Setup shadcn/ui component library and customize theme to match BRAND-INSPIRATION.md.

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: shadcn/ui Setup + Tailwind Config
**Task:** Install and configure shadcn/ui
**File:** `apps/desktop/src/components/ui/`
**Deliverables:**
- shadcn/ui installed and configured
- Tailwind CSS configured
- Core components installed (Button, Input, Card, Badge, etc.)
- 5+ tests

### Subagent 2: Theme Customization + Custom Components
**Task:** Customize theme to match brand + build custom components
**File:** `apps/desktop/src/styles/globals.css`
**Deliverables:**
- Theme colors (Deep Blue, Electric Purple, Emerald Green, Slate Gray)
- Typography (Inter font)
- Dark mode support
- Custom components (only what shadcn doesn't have)
- 5+ tests
```

---

## Benefits

**Time Saved:**
- Sprint A4: 8h → 4h (50% reduction)
- Future sprints: Components ready to use (no building from scratch)
- **Total Phase 4:** Save 4-8 hours

**Quality Gained:**
- Accessibility: WCAG compliant out of box
- Consistency: All components follow same patterns
- Maintenance: shadcn maintains components (not us)

**Developer Experience:**
- Easy to use: `import { Button } from "@/components/ui/button"`
- Well-documented: shadcn has excellent docs
- Community: Large community, many examples

---

## Migration Guide (If Already Started Building Components)

**If you already built some components:**

```bash
# Keep custom components in:
apps/desktop/src/components/custom/

# Use shadcn for standard components:
apps/desktop/src/components/ui/

# Update imports:
- Old: import { Button } from '@/components/custom/Button'
- New: import { Button } from '@/components/ui/button'
```

---

## Testing Strategy

```typescript
// apps/desktop/src/components/ui/__tests__/components.spec.tsx
import { render, screen } from '@testing-library/react';
import { Button } from '../button';
import { Input } from '../input';
import { Card } from '../card';

describe('shadcn/ui Components', () => {
  it('Button renders correctly', () => {
    render(<Button>Click me</Button>);
    expect(screen.getByRole('button')).toHaveTextContent('Click me');
  });
  
  it('Button supports variants', () => {
    const { container: primary } = render(<Button variant="default">Primary</Button>);
    const { container: secondary } = render(<Button variant="secondary">Secondary</Button>);
    
    expect(primary.firstChild).toHaveClass('bg-primary');
    expect(secondary.firstChild).toHaveClass('bg-secondary');
  });
  
  it('Input is accessible', () => {
    render(<Input aria-label="Test input" />);
    expect(screen.getByLabelText('Test input')).toBeInTheDocument();
  });
});
```

---

## Alternatives Considered

### Option 1: Build from Scratch (REJECTED)
- **Pros:** Full control, no dependencies
- **Cons:** Time-consuming, accessibility concerns, maintenance burden
- **Why Rejected:** Reinvents wheel, takes 8+ hours

### Option 2: Material UI (REJECTED)
- **Pros:** Comprehensive, well-maintained
- **Cons:** Heavy bundle (~200KB), Material Design doesn't match brand
- **Why Rejected:** Too heavy, wrong design language

### Option 3: Chakra UI (REJECTED)
- **Pros:** Easy to use, good DX
- **Cons:** Heavy bundle (~150KB), less customizable
- **Why Rejected:** Heavier than shadcn, less flexible

### Option 4: shadcn/ui (ACCEPTED)
- **Pros:** Lightweight (~50KB), customizable, accessible, modern
- **Cons:** Requires Tailwind CSS
- **Why Accepted:** Best balance of features, size, and customization

---

## Compliance

**Accessibility:**
- ✅ WCAG 2.1 AA compliant (shadcn components)
- ✅ Keyboard navigation (built-in)
- ✅ Screen reader support (ARIA labels)

**Performance:**
- ✅ Tree-shakeable (only import what you use)
- ✅ Bundle size: ~50KB (gzipped)
- ✅ No runtime CSS-in-JS overhead

**Brand Alignment:**
- ✅ Customizable colors (Electric Purple, Emerald Green, etc.)
- ✅ Dark mode support (built-in)
- ✅ Modern, clean design (matches BRAND-INSPIRATION.md)

---

## Conclusion

**USE shadcn/ui for Phase 4 Desktop App.**

**Impact:**
- Sprint A4: 8h → 4h (50% faster)
- Better accessibility (WCAG compliant)
- Better maintainability (shadcn maintains components)
- Better developer experience (easy to use, well-documented)

**Next Step:** Update Sprint A4 and start implementation.

---

**Approved by:** Architecture Decision Review  
**Date:** 2026-03-17
