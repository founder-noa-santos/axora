# UI Components Guide

**Purpose:** Documentation for shadcn/ui components and theme customization  
**Last Updated:** 2026-03-17  
**Status:** Active

---

## 📦 Overview

AXORA uses **shadcn/ui** for high-quality, accessible, customizable UI components.

**Benefits:**
- Accessibility built-in (WCAG compliant)
- Customizable via Tailwind CSS
- Well-maintained (active community)
- 50% faster development vs building from scratch

---

## 🎨 Theme

### Brand Colors (from BRAND-INSPIRATION.md)

| Color | HSL Value | Usage |
|-------|-----------|-------|
| **Deep Blue** | `222 47% 11%` | Background |
| **Electric Purple** | `271 76% 53%` | Primary (main brand color) |
| **Emerald Green** | `160 84% 39%` | Secondary (success, growth) |
| **Slate Gray** | `217 33% 17%` | Muted (subtle elements) |

### CSS Variables

Located in `apps/desktop/src/styles/globals.css`:

```css
:root {
  /* Background */
  --background: 222 47% 11%;
  --foreground: 210 40% 98%;
  
  /* Cards */
  --card: 222 47% 15%;
  --card-foreground: 210 40% 98%;
  
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
  
  /* Borders */
  --border: 217 33% 25%;
  --input: 217 33% 25%;
  --ring: 271 76% 53%;
}
```

### Dark Mode

**Default theme is dark mode** (aligned with brand identity).

To add light mode support, extend `globals.css`:

```css
@media (prefers-color-scheme: light) {
  :root {
    --background: 0 0% 100%;
    --foreground: 222 47% 11%;
    /* ... other light mode colors */
  }
}
```

---

## 🧩 Available Components

### Core Components (15+)

| Component | Import | Description |
|-----------|--------|-------------|
| **Button** | `@/components/ui/button` | Clickable button with variants |
| **Input** | `@/components/ui/input` | Text input field |
| **Card** | `@/components/ui/card` | Container with header, content, footer |
| **Badge** | `@/components/ui/badge` | Small status/label badge |
| **Progress** | `@/components/ui/progress` | Progress bar indicator |
| **Tabs** | `@/components/ui/tabs` | Tabbed navigation |
| **ScrollArea** | `@/components/ui/scroll-area` | Custom scrollable area |
| **Separator** | `@/components/ui/separator` | Visual divider |
| **Textarea** | `@/components/ui/textarea` | Multi-line text input |
| **Avatar** | `@/components/ui/avatar` | User avatar with fallback |
| **Select** | `@/components/ui/select` | Dropdown select |
| **Switch** | `@/components/ui/switch` | Toggle switch |
| **Label** | `@/components/ui/label` | Form label |
| **Alert** | `@/components/ui/alert` | Alert banner |
| **Tooltip** | `@/components/ui/tooltip` | Hover tooltip |

---

## 📖 Usage Examples

### Button

```tsx
import { Button } from '@/components/ui/button';

function Example() {
  return (
    <div className="space-x-2">
      <Button>Default</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="outline">Outline</Button>
      <Button variant="destructive">Destructive</Button>
      <Button size="sm">Small</Button>
      <Button size="lg">Large</Button>
      <Button disabled>Disabled</Button>
    </div>
  );
}
```

### Card

```tsx
import { Card, CardHeader, CardTitle, CardDescription, CardContent } from '@/components/ui/card';

function Example() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Welcome to AXORA</CardTitle>
        <CardDescription>Autonomous AI Orchestration System</CardDescription>
      </CardHeader>
      <CardContent>
        <p>Your content here...</p>
      </CardContent>
    </Card>
  );
}
```

### Input + Button Form

```tsx
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';

function Example() {
  return (
    <form className="flex gap-2">
      <Input placeholder="Enter text..." className="flex-1" />
      <Button>Submit</Button>
    </form>
  );
}
```

### Badge

```tsx
import { Badge } from '@/components/ui/badge';

function Example() {
  return (
    <div className="space-x-2">
      <Badge>Default</Badge>
      <Badge variant="secondary">Secondary</Badge>
      <Badge variant="outline">Outline</Badge>
      <Badge variant="destructive">Destructive</Badge>
    </div>
  );
}
```

### Progress

```tsx
import { Progress } from '@/components/ui/progress';

function Example() {
  return (
    <div className="space-y-2">
      <div className="flex justify-between text-sm">
        <span>Loading</span>
        <span>75%</span>
      </div>
      <Progress value={75} className="w-full" />
    </div>
  );
}
```

### Tabs

```tsx
import { Tabs, TabsList, TabsTrigger, TabsContent } from '@/components/ui/tabs';

function Example() {
  return (
    <Tabs defaultValue="account">
      <TabsList>
        <TabsTrigger value="account">Account</TabsTrigger>
        <TabsTrigger value="settings">Settings</TabsTrigger>
      </TabsList>
      <TabsContent value="account">
        Account content...
      </TabsContent>
      <TabsContent value="settings">
        Settings content...
      </TabsContent>
    </Tabs>
  );
}
```

---

## 🧪 Testing

### Running Tests

```bash
cd apps/desktop
pnpm vitest run src/components/ui/__tests__
```

### Test Coverage

- **Button:** 6 tests (variants, sizes, asChild, disabled, click handling)
- **Card:** 6 tests (all parts, styles, className)
- **Input:** 6 tests (types, ref, disabled, className)
- **Badge:** 5 tests (variants, styles, className)
- **Progress:** 7 tests (values, styles, className)

**Total: 30 tests passing**

### Writing New Tests

```tsx
import { render, screen } from '@testing-library/react'
import { Button } from '../button'
import { describe, it, expect } from 'vitest'

describe('Button', () => {
  it('renders correctly', () => {
    render(<Button>Click me</Button>)
    expect(screen.getByRole('button')).toBeInTheDocument()
  })
})
```

---

## 🔧 Configuration

### Tailwind Config

Located in `apps/desktop/tailwind.config.js`:

```javascript
export default {
  darkMode: ["class"],
  content: ['./src/**/*.{ts,tsx}'],
  theme: {
    extend: {
      colors: {
        border: "hsl(var(--border))",
        background: "hsl(var(--background))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        // ... other colors
      },
    },
  },
  plugins: [require("tailwindcss-animate")],
}
```

### Components.json

shadcn/ui configuration:

```json
{
  "$schema": "https://ui.shadcn.com/schema.json",
  "style": "default",
  "rsc": false,
  "tsx": true,
  "tailwind": {
    "config": "tailwind.config.js",
    "css": "src/styles/globals.css",
    "baseColor": "slate",
    "cssVariables": true
  },
  "aliases": {
    "components": "@/components",
    "utils": "@/lib/utils"
  }
}
```

---

## 📁 File Structure

```
apps/desktop/src/
├── components/
│   └── ui/
│       ├── __tests__/
│       │   ├── button.test.tsx
│       │   ├── card.test.tsx
│       │   ├── input.test.tsx
│       │   ├── badge.test.tsx
│       │   └── progress.test.tsx
│       ├── button.tsx
│       ├── input.tsx
│       ├── card.tsx
│       ├── badge.tsx
│       ├── progress.tsx
│       ├── tabs.tsx
│       ├── scroll-area.tsx
│       ├── separator.tsx
│       ├── textarea.tsx
│       ├── avatar.tsx
│       ├── select.tsx
│       ├── switch.tsx
│       ├── label.tsx
│       ├── alert.tsx
│       └── tooltip.tsx
├── lib/
│   └── utils.ts          # cn() utility function
├── styles/
│   └── globals.css       # Theme CSS variables
└── App.tsx               # Example usage
```

---

## 🚀 Adding New Components

```bash
cd apps/desktop
npx shadcn-ui@latest add <component-name>
```

Example:
```bash
npx shadcn-ui@latest add dialog
npx shadcn-ui@latest add dropdown-menu
npx shadcn-ui@latest add toast
```

---

## ♿ Accessibility

All shadcn/ui components are **WCAG compliant**:

- Keyboard navigation support
- ARIA attributes
- Focus management
- Screen reader friendly

### Keyboard Navigation

| Component | Keys | Action |
|-----------|------|--------|
| Button | `Enter`, `Space` | Activate |
| Input | `Tab` | Focus/blur |
| Select | `Arrow keys`, `Enter` | Navigate options |
| Tabs | `Arrow keys` | Switch tabs |
| Switch | `Space` | Toggle |

---

## 🎯 Best Practices

1. **Use semantic variants:** Choose button/variant based on meaning, not just color
2. **Consistent spacing:** Use Tailwind spacing utilities (`gap-2`, `p-4`, etc.)
3. **Responsive design:** Use responsive prefixes (`md:`, `lg:`)
4. **Dark mode first:** Design for dark mode (default theme)
5. **Test accessibility:** Use keyboard navigation and screen readers

---

## 📚 References

- [shadcn/ui Documentation](https://ui.shadcn.com)
- [Tailwind CSS Documentation](https://tailwindcss.com)
- [BRAND-INSPIRATION.md](../../BRAND-INSPIRATION.md)
- [ADR-050: Use shadcn/ui](../../docs/adr/ADR-050-use-shadcn-ui.md)

---

**Last Updated:** 2026-03-17  
**Maintained By:** Agent A (UI Components + Progress Display)
