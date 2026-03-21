# LLM → LLM handoff: Desktop chat prompt bar layout (Electron + Next.js)

Copy everything below the line into your context for the fixing model.

---

## Role and expectations

You are taking over a **visual/layout bug** in the OPENAKTA desktop app (`apps/desktop`): the chat **composer** (`ChatPromptBar`) does not match the intended **Codex-style** design. The user reports it still looks **too short (“minimum height”)**, **cramped**, and previously showed **clipped placeholder text** at the left edge of the rounded pill (glyphs appearing stacked/cut off, as if only “A” / “X” were visible).

**Do not** settle for the first idea that “sounds right.” Verify in the **actual DOM and computed styles** (DevTools in Electron or browser), and trace **flex/grid + overflow + Tailwind merge** interactions. The failure mode is subtle; easy fixes (only tweaking `min-height` or slapping `w-full`) may not survive **shrink rules**, **`field-sizing-content`**, **`overflow-hidden`**, or **wrong flex alignment on column layouts**.

**Constraints:**

- **Do not edit** `apps/desktop/components/ai-elements/*` (vendor-style AI Elements; project rule). Fix via **`ChatPromptBar.tsx`**, **`components/ui/*`**, or wrappers—**unless** the team explicitly approves changing upstream shadcn/input-group behavior.
- Prefer **one coherent layout** over stacking conflicting utilities that fight each other after `tailwind-merge`.

## Product intent (what “good” looks like)

- Large **rounded** composer (“pill”) with comfortable **vertical padding**; textarea should feel like a **multi-line** input, not a single-line search bar.
- **Inside** the pill (conceptually): model + effort controls on the **left**, attachment + **round send** on the **right**; **below** the pill (outside): row for runtime (“Local”), access (“Full access” in accent color), branch.
- Reference (UX inspiration): Vercel [AI Elements Prompt Input](https://elements.ai-sdk.dev/components/prompt-input)—mirror **spacing and hierarchy**, not necessarily every prop.

## Relevant files (read in this order)

1. `apps/desktop/components/chat/ChatPromptBar.tsx` — our composition and overrides.
2. `apps/desktop/components/ai-elements/prompt-input.tsx` — how `PromptInput` wraps children:
   - Renders `<form className={cn("w-full", className)}>` then **`<InputGroup className="overflow-hidden">`**.
   - `PromptInputBody` is **`display: contents`** (wrapper disappears from layout; children sit directly under `InputGroup`).
   - `PromptInputTextarea` → `InputGroupTextarea` with default classes including **`field-sizing-content max-h-48 min-h-16`** (merged with consumer `className`).
3. `apps/desktop/components/ui/input-group.tsx` — `InputGroup`:
   - Base: **`flex w-full items-center`**, plus **`has-[>textarea]:h-auto`**, and when a **`data-align=block-end`** addon exists: **`flex-col`** (footer path).
   - **Hypothesis:** with **`flex-col`**, **`items-center`** can still cause **cross-axis sizing** issues (children not stretching to full width → placeholder/text clipped at rounded edges). A previous attempt added **`[&_[data-slot=input-group]]:!items-stretch`** on the **form** via `PromptInput`’s `className`; **confirm** this rule actually hits the `InputGroup` node and **wins** over `items-center` in the final stylesheet (specificity/order).
4. `apps/desktop/components/ui/textarea.tsx` — base **`min-h-[80px]`** on `<textarea>` before merges.

## What was already tried (and may still be insufficient)

In `ChatPromptBar.tsx`:

- **Shell classes** on `PromptInput` targeting `[data-slot=input-group]` for pill styling (radius, border, blur, padding) plus **`!items-stretch`** to override default **`items-center`**.
- **Textarea overrides:** `min-h-[100px]`, `w-full`, `min-w-0`, `self-stretch`, padding/typography.

**User feedback:** issue **not** resolved to their satisfaction—treat the above as **non-definitive**.

## Suspicions to investigate (not all may apply)

1. **`overflow-hidden` on `InputGroup`** inside `prompt-input.tsx` may interact with **focus rings**, **scrollbar**, or **text measurement** in a way that exaggerates clipping. You cannot remove it from ai-elements without breaking the rule—consider **padding** on the inner textarea, **`min-w-0`** on flex ancestors, or an **approved** small fork/override path.
2. **`field-sizing-content`** (from `PromptInputTextarea`) may make **empty** textarea height **content-driven** in ways that **ignore** or **fight** `min-h-*` depending on browser and cascade. Inspect **computed height** when empty vs with one line of text.
3. **`tailwind-merge` / `cn()` order**: consumer `className` might not override defaults as assumed; confirm **which `min-height` wins** on the real element.
4. **`InputGroupAddon` `block-end`** footer uses **`order-last`** and horizontal padding from variants—check for **negative margins** on triggers (`has-[>button]:…`) clipping siblings.
5. **Electron / Chromium version**: subpixel + `border-radius` + tight flex height sometimes shows **edge clipping**; verify against plain Chromium.
6. **`PromptInputBody` + `contents`**: any bug in mental model of **direct children** vs wrapper can mislead; list **actual flex children** of `InputGroup` in DevTools.

## Acceptance checks

- Empty state: textarea area has **clearly readable** placeholder, **no horizontal clipping** at the left/right of the pill; vertical **breathing room** matches a **composer**, not a compact input.
- Multiline: grows up to **max height** behavior defined by AI Elements (`max-h-48` in defaults) unless product asks otherwise.
- Footer row **inside** pill: model/effort + attach/send **aligned** and not vertically squashed.
- **No regressions** to submit, attachments, or keyboard (Enter submit, Shift+Enter newline).

## Commands

From `apps/desktop`: `pnpm typecheck` after changes.

---

End of handoff.
