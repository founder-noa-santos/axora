# Meta-Prompt: OPENAKTA Chat UI Integration with assistant-ui

## Context

You are an AI researcher tasked with analyzing the OPENAKTA codebase and the **assistant-ui** library to provide integration guidance for building a production-ready chat interface.

---

## 📋 Your Mission

Read and understand the OPENAKTA codebase structure, then analyze how **assistant-ui** (https://www.assistant-ui.com/examples) can be integrated to replace/enhance the current chat UI implementation.

Your output will be a **detailed integration prompt** that another LLM (implementer) will use to:
1. Install and configure assistant-ui components
2. Set up necessary hooks and state management
3. Prepare the environment for OpenAI-compatible API integration
4. Update existing plans that haven't been implemented yet

---

## 🏗️ OPENAKTA Architecture Overview

### Project Structure
```
openakta/
├── proto/              # Protocol Buffer schemas
├── crates/             # Rust workspace
│   ├── openakta-proto/    # Generated protobuf code
│   ├── openakta-storage/  # SQLite storage layer
│   ├── openakta-core/     # Core business logic
│   └── openakta-daemon/   # Main daemon executable
├── apps/
│   └── desktop/        # Tauri v2 desktop app (React + TypeScript)
└── docs/               # Documentation
```

### Tech Stack (Frontend)
- **Framework:** React 18.2 + TypeScript
- **Desktop:** Tauri v2
- **Styling:** Tailwind CSS 4.x + shadcn/ui components (migrating from v3.4)
- **State Management:** Zustand
- **UI Components:** Radix UI primitives (via shadcn)
- **Build:** Vite 5

### Current UI Implementation
- Location: `apps/desktop/src/`
- Components: `apps/desktop/src/components/ui/`
- Main App: `apps/desktop/src/App.tsx`
- Settings Panel: `apps/desktop/src/panels/SettingsPanel.tsx`

### Key Features Already Implemented
1. **Settings Management** (Zustand store)
   - Model configuration (Ollama, OpenAI, Anthropic)
   - Token limits
   - Worker pool settings
   - Theme preferences

2. **UI Component Library** (shadcn/ui)
   - Button, Input, Card, Badge, Progress
   - Select, Switch, Label, Separator
   - Alert, Tabs, ScrollArea, Tooltip
   - Avatar, Textarea

3. **OpenAI Compatibility Focus**
   - Settings already support OpenAI API key configuration
   - Target: Any OpenAI-compatible endpoint (Chinese AI providers, etc.)

---

## 🔍 assistant-ui Library Analysis

### What is assistant-ui?
A React component library specifically designed for building chat/conversation UIs with AI assistants.

### Key Features to Investigate
1. **Chat Components**
   - Message list rendering
   - User/AI message bubbles
   - Streaming text display
   - Code block rendering
   - Markdown support

2. **Input Components**
   - Chat input with auto-resize
   - Submit handling
   - Attachment support (if available)

3. **State Management**
   - Conversation state
   - Message queue
   - Loading states
   - Error handling

4. **Hooks**
   - `useChat` or similar
   - `useSendMessage`
   - `useStreaming`
   - Custom hooks for API integration

5. **Theming**
   - Compatibility with Tailwind
   - Dark mode support
   - Customization options

---

## 📊 Analysis Requirements

### 1. Codebase Understanding
Read these files to understand current implementation:
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/panels/SettingsPanel.tsx`
- `apps/desktop/src/store/settings-store.ts`
- `apps/desktop/src/types/settings.ts`
- `apps/desktop/src/components/ui/*` (existing components)
- `apps/desktop/package.json` (dependencies)
- `apps/desktop/tailwind.config.js`

### 2. assistant-ui Integration Points
Identify:
- Which assistant-ui components replace current UI
- How to integrate with existing Zustand stores
- Compatibility with shadcn/ui (both use Radix)
- Required npm packages to install
- Configuration changes needed

### 3. OpenAI Compatibility
Focus on:
- Components that support streaming responses
- Easy integration with OpenAI-compatible APIs
- Support for custom base URLs (for Ollama, Chinese providers)
- API key management integration

### 4. Tailwind CSS v4 Migration
The project must be migrated from Tailwind CSS 3.4 to v4.x. Identify:
- Changes needed in `tailwind.config.js` (v4 uses CSS-first configuration)
- Migration from `tailwind.config.js` to `@theme` directive in CSS
- Update `@tailwindcss/postcss` to v4 compatible version
- Remove `tailwind.config.js` in favor of CSS variables
- Compatibility with shadcn/ui components in v4
- Any breaking changes affecting existing components

---

## 🎯 Output Format

Your output should be a **comprehensive integration prompt** in English with the following structure:

```markdown
# OPENAKTA Chat UI Integration Plan

## Overview
[Brief description of what will be implemented]

## Phase 0: Tailwind CSS v4 Migration
### Migration Steps
[Steps to migrate from Tailwind 3.4 to v4]

### Configuration Changes
- Remove `tailwind.config.js`
- Add `@theme` directive in CSS
- Update PostCSS config for v4
- Update any v3-specific syntax

### Compatibility Checks
[Verify shadcn/ui components work with v4]

## Phase 1: Environment Setup
### Dependencies to Install
```bash
pnpm add [package-name]
```

### Configuration Changes
[Changes to tailwind.config.js, tsconfig.json, etc.]

## Phase 2: Component Integration
### Components to Import from assistant-ui
[List specific components and their use cases]

### Components to Keep from shadcn/ui
[List existing components that remain]

## Phase 3: Chat Interface Implementation
### New Components to Create
- ChatPanel.tsx
- MessageList.tsx
- ChatInput.tsx
- [etc.]

### Hooks to Implement
- useChatSession
- useOpenAICompatibleAPI
- [etc.]

## Phase 4: State Management Integration
### Zustand Store Updates
[Changes to settings-store.ts]

### New Stores
- chat-store.ts
- message-store.ts

## Phase 5: API Integration
### OpenAI-Compatible Client
[Implementation for universal OpenAI-compatible client]

### Streaming Support
[How to handle streaming responses]

## Phase 6: Plan Updates
### Plans to Modify
[List which planned but not-yet-implemented features need updates]

### Plans to Keep Unchanged
[List which existing implementations should NOT be touched]

## Testing Checklist
- [ ] Chat messages render correctly
- [ ] Streaming works with OpenAI
- [ ] Streaming works with Ollama
- [ ] Settings integration works
- [ ] Dark mode works
- [ ] No regressions in existing features
- [ ] Tailwind v4 migration complete (no v3 config remaining)
- [ ] All shadcn/ui components work with v4
- [ ] CSS variables properly defined in @theme directive
```

---

## ⚠️ Critical Constraints

### DO NOT MODIFY
1. **Already Implemented Features**
   - Settings Panel (`SettingsPanel.tsx`)
   - Settings Store (`settings-store.ts`)
   - Existing shadcn/ui components (only update for Tailwind v4 compatibility)
   - Tauri configuration
   - Rust backend code

2. **Existing Architecture**
   - Zustand state management pattern
   - TypeScript configuration
   - Build pipeline

3. **Tailwind v4 Migration**
   - Preserve all existing design tokens (colors, spacing, etc.)
   - Maintain dark/light mode functionality
   - Keep CSS variable-based theming

### FOCUS ON
1. **Chat UI Only**
   - New chat interface components
   - Message rendering
   - Input handling
   - API integration

2. **OpenAI Compatibility**
   - Universal client for OpenAI-compatible APIs
   - Support for Chinese AI providers
   - Custom base URL support
   - API key management

3. **assistant-ui Integration**
   - Use assistant-ui components where they add value
   - Keep shadcn/ui for general UI (buttons, cards, etc.)
   - Ensure both libraries work together

---

## 🎨 Design Principles

1. **Consistency**
   - Match existing shadcn/ui design language
   - Use same color scheme (CSS variables)
   - Maintain dark/light mode support
   - Preserve Tailwind v4 theme variables

2. **Performance**
   - Virtual scrolling for long conversations
   - Efficient re-renders
   - Streaming optimization
   - Tailwind v4's improved CSS bundling

3. **Accessibility**
   - Keyboard navigation
   - Screen reader support
   - ARIA labels

4. **Extensibility**
   - Easy to add new AI providers
   - Modular component design
   - Clear separation of concerns
   - Tailwind v4's CSS-first configuration for easier customization

---

## 📚 Reference Links

- **assistant-ui Docs:** https://www.assistant-ui.com
- **Examples:** https://www.assistant-ui.com/examples
- **OPENAKTA Architecture:** See `AGENTS.md` and `docs/`
- **shadcn/ui:** https://ui.shadcn.com
- **Tauri v2:** https://v2.tauri.app

---

## ✅ Success Criteria

Your integration prompt is successful if:
1. Another LLM can implement the chat UI without ambiguity
2. OpenAI-compatible APIs work out of the box
3. Existing features remain untouched
4. The chat UI is production-ready
5. Streaming responses work correctly
6. Settings integration is seamless

---

## 🚀 Next Steps After Your Analysis

1. Read the OPENAKTA codebase (focus on `apps/desktop/src/`)
2. Analyze assistant-ui components and examples
3. Identify integration points
4. Create the comprehensive integration prompt (format above)
5. Highlight which plans need updating vs. which are immutable

**Good luck! Your analysis will directly guide the chat UI implementation.**
