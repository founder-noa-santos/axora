# AXORA Chat UI Integration Plan

## Overview

This plan details the integration of **assistant-ui** into the AXORA desktop application to create a production-ready chat interface with OpenAI-compatible API support. The implementation will:

1. Migrate from Tailwind CSS 3.4 to v4.x (CSS-first configuration)
2. Install and configure assistant-ui components
3. Build a chat interface using assistant-ui primitives
4. Integrate with existing Zustand stores
5. Support streaming responses from OpenAI-compatible APIs (OpenAI, Ollama, Chinese providers)
6. Maintain visual consistency with existing shadcn/ui components

---

## Phase 0: Tailwind CSS v4 Migration

### Migration Steps

The project currently uses Tailwind CSS 3.4 with `@tailwindcss/postcss` 4.2.1, which is incompatible. We need to:

1. **Remove legacy Tailwind v3 packages**
2. **Install Tailwind v4 with Vite plugin**
3. **Migrate from `tailwind.config.js` to CSS-first configuration**
4. **Update PostCSS configuration**
5. **Preserve all existing design tokens**

### Configuration Changes

#### Step 1: Update Dependencies

```bash
cd apps/desktop

# Remove v3 tailwind
pnpm remove tailwindcss

# Install v4 tailwind with Vite plugin
pnpm add -D tailwindcss@4 @tailwindcss/vite

# Keep postcss and autoprefixer (they work with v4)
```

#### Step 2: Update `vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

export default defineConfig({
  plugins: [
    react(),
    tailwindcss(), // Add Tailwind v4 Vite plugin
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    target: 'esnext',
    outDir: 'dist',
    emptyOutDir: true,
  },
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
    css: false,
  },
});
```

#### Step 3: Update `postcss.config.js`

Tailwind v4 works differently - remove tailwindcss from PostCSS:

```javascript
export default {
  plugins: {
    autoprefixer: {},
  },
};
```

#### Step 4: Migrate `src/styles/globals.css`

Replace the v3 directives with v4 `@import` and add `@theme`:

```css
/* apps/desktop/src/styles/globals.css */

/* Import Tailwind v4 */
@import "tailwindcss";

/* Define theme variables using @theme directive */
@theme {
  /* Brand colors from existing globals.css */
  --color-background: hsl(222 47% 11%);
  --color-foreground: hsl(210 40% 98%);
  
  --color-card: hsl(222 47% 15%);
  --color-card-foreground: hsl(210 40% 98%);
  
  --color-popover: hsl(222 47% 15%);
  --color-popover-foreground: hsl(210 40% 98%);
  
  /* Primary: Electric Purple */
  --color-primary: hsl(271 76% 53%);
  --color-primary-foreground: hsl(210 40% 98%);
  
  /* Secondary: Emerald Green */
  --color-secondary: hsl(160 84% 39%);
  --color-secondary-foreground: hsl(210 40% 98%);
  
  /* Muted: Slate Gray */
  --color-muted: hsl(217 33% 17%);
  --color-muted-foreground: hsl(215 20% 65%);
  
  /* Accent: Electric Purple light */
  --color-accent: hsl(271 76% 63%);
  --color-accent-foreground: hsl(210 40% 98%);
  
  /* Destructive: Red */
  --color-destructive: hsl(0 63% 31%);
  --color-destructive-foreground: hsl(210 40% 98%);
  
  /* Borders and inputs */
  --color-border: hsl(217 33% 25%);
  --color-input: hsl(217 33% 25%);
  --color-ring: hsl(271 76% 53%);
  
  /* Border radius */
  --radius: 0.5rem;
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  
  /* shadcn compatibility mappings */
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
}

/* Legacy CSS variable definitions for shadcn/ui components */
:root {
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
  --radius: 0.5rem;
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

#### Step 5: Remove `tailwind.config.js`

Delete `apps/desktop/tailwind.config.js` - v4 doesn't need it.

### Compatibility Checks

After migration, verify:

- [ ] All shadcn/ui components render correctly
- [ ] Dark mode still works (`dark` class on HTML element)
- [ ] CSS variables are properly resolved
- [ ] No console errors about Tailwind directives

---

## Phase 1: Environment Setup

### Dependencies to Install

```bash
cd apps/desktop

# Core assistant-ui packages
pnpm add @assistant-ui/react @assistant-ui/react-ai-sdk

# AI SDK for OpenAI-compatible APIs
pnpm add ai @ai-sdk/react @ai-sdk/openai

# For Ollama support (OpenAI-compatible)
pnpm add ollama-ai-provider

# Existing dependencies (already installed, verify versions)
# - zustand ✓
# - @radix-ui/react-* ✓
# - lucide-react ✓
```

### TypeScript Configuration

Update `tsconfig.json` to ensure proper module resolution:

```json
{
  "compilerOptions": {
    "target": "ES2020",
    "useDefineForClassFields": true,
    "lib": ["ES2020", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "baseUrl": ".",
    "paths": {
      "@/*": ["./src/*"]
    }
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

---

## Phase 2: Component Integration

### Components to Import from assistant-ui

| Component | Package | Purpose |
|-----------|---------|---------|
| `Thread` | `@assistant-ui/react` | Main chat container with messages, composer, auto-scroll |
| `Composer` | `@assistant-ui/react` | Chat input with auto-resize |
| `MessagePrimitive` | `@assistant-ui/react` | Message rendering primitives |
| `ThreadPrimitive` | `@assistant-ui/react` | Low-level thread building blocks |
| `AssistantRuntimeProvider` | `@assistant-ui/react` | Context provider for chat state |
| `useChatRuntime` | `@assistant-ui/react-ai-sdk` | Hook for AI SDK integration |
| `AssistantChatTransport` | `@assistant-ui/react-ai-sdk` | Transport for OpenAI-compatible APIs |

### Components to Keep from shadcn/ui

These existing components remain unchanged:

- `Button` - General UI buttons
- `Card`, `CardHeader`, `CardTitle`, `CardContent` - Card containers
- `Input` - Form inputs (for settings)
- `Textarea` - Multi-line text (for settings)
- `Select`, `SelectItem`, `SelectTrigger`, `SelectContent` - Dropdowns
- `Switch` - Toggle switches
- `Label` - Form labels
- `Badge` - Status badges
- `Progress` - Progress bars
- `Separator` - Visual dividers
- `Tooltip` - Hover tooltips
- `Avatar` - User/agent avatars
- `Alert`, `AlertDescription` - Error/success alerts
- `ScrollArea` - Custom scrollable areas

### Why This Split?

- **assistant-ui**: Handles chat-specific complexities (streaming, message state, auto-scroll, composer behavior)
- **shadcn/ui**: Provides consistent general-purpose UI primitives
- Both use Radix UI under the hood, ensuring compatibility

---

## Phase 3: Chat Interface Implementation

### New Components to Create

#### 1. `apps/desktop/src/components/chat/Thread.tsx`

Main chat thread component using assistant-ui primitives:

```tsx
import { ThreadPrimitive } from '@assistant-ui/react';
import { Composer } from './Composer';
import { UserMessage } from './UserMessage';
import { AssistantMessage } from './AssistantMessage';
import { WelcomeScreen } from './WelcomeScreen';

export function Thread() {
  return (
    <ThreadPrimitive.Root
      className="bg-background flex h-full flex-col overflow-hidden"
      style={{ ['--thread-max-width' as string]: '48rem' }}
    >
      <ThreadPrimitive.Viewport className="flex h-full flex-col items-center overflow-y-scroll scroll-smooth bg-inherit px-4 pt-8">
        {/* Empty state */}
        <ThreadPrimitive.If empty>
          <WelcomeScreen />
        </ThreadPrimitive.If>

        {/* Message list */}
        <ThreadPrimitive.Messages
          components={{
            UserMessage: UserMessage,
            AssistantMessage: AssistantMessage,
          }}
        />

        {/* Spacer for scroll */}
        <ThreadPrimitive.If empty={false}>
          <div className="min-h-8 flex-grow" />
        </ThreadPrimitive.If>

        {/* Composer at bottom */}
        <div className="sticky bottom-0 mt-3 flex w-full max-w-[var(--thread-max-width)] flex-col items-center justify-end rounded-t-lg bg-inherit pb-4">
          <ThreadPrimitive.ScrollToBottom />
          <Composer />
        </div>
      </ThreadPrimitive.Viewport>
    </ThreadPrimitive.Root>
  );
}
```

#### 2. `apps/desktop/src/components/chat/Composer.tsx`

Chat input component:

```tsx
import { ComposerPrimitive, ThreadPrimitive } from '@assistant-ui/react';
import { SendHorizontal, Square } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function Composer() {
  return (
    <ComposerPrimitive.Root className="focus-within:border-ring/20 flex w-full flex-wrap items-end rounded-xl border bg-card px-3 py-3 shadow-sm transition-colors ease-in">
      <ComposerPrimitive.Input
        rows={1}
        autoFocus
        placeholder="Ask AXORA anything..."
        className="placeholder:text-muted-foreground max-h-40 flex-grow resize-none border-none bg-transparent px-2 py-2 text-sm outline-none focus:ring-0 disabled:cursor-not-allowed"
      />
      <ComposerActions />
    </ComposerPrimitive.Root>
  );
}

function ComposerActions() {
  return (
    <div className="flex items-center gap-2">
      <ThreadPrimitive.If running={false}>
        <ComposerPrimitive.Send asChild>
          <Button
            size="icon"
            className="h-8 w-8 shrink-0"
            aria-label="Send message"
          >
            <SendHorizontal className="h-4 w-4" />
          </Button>
        </ComposerPrimitive.Send>
      </ThreadPrimitive.If>
      
      <ThreadPrimitive.If running>
        <ComposerPrimitive.Cancel asChild>
          <Button
            size="icon"
            variant="destructive"
            className="h-8 w-8 shrink-0"
            aria-label="Cancel generation"
          >
            <Square className="h-4 w-4 fill-current" />
          </Button>
        </ComposerPrimitive.Cancel>
      </ThreadPrimitive.If>
    </div>
  );
}
```

#### 3. `apps/desktop/src/components/chat/UserMessage.tsx`

User message bubble:

```tsx
import { MessagePrimitive } from '@assistant-ui/react';
import { User } from 'lucide-react';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';

export function UserMessage() {
  return (
    <MessagePrimitive.Root className="grid w-full max-w-[var(--thread-max-width)] grid-cols-[auto_1fr] gap-3 py-4">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-primary text-primary-foreground">
          <User className="h-4 w-4" />
        </AvatarFallback>
      </Avatar>
      
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold">You</span>
        </div>
        <div className="text-foreground break-words leading-7">
          <MessagePrimitive.Content />
        </div>
      </div>
    </MessagePrimitive.Root>
  );
}
```

#### 4. `apps/desktop/src/components/chat/AssistantMessage.tsx`

Assistant (AI) message bubble with markdown support:

```tsx
import { MessagePrimitive } from '@assistant-ui/react';
import { Bot } from 'lucide-react';
import { Avatar, AvatarFallback } from '@/components/ui/avatar';
import { MarkdownText } from './MarkdownText';
import { ActionBar } from './ActionBar';

export function AssistantMessage() {
  return (
    <MessagePrimitive.Root className="grid w-full max-w-[var(--thread-max-width)] grid-cols-[auto_1fr] gap-3 py-4">
      <Avatar className="h-8 w-8">
        <AvatarFallback className="bg-secondary text-secondary-foreground">
          <Bot className="h-4 w-4" />
        </AvatarFallback>
      </Avatar>
      
      <div className="flex flex-col gap-1">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold">AXORA</span>
          <MessagePrimitive.StatusIndicator />
        </div>
        <div className="text-foreground max-w-[calc(var(--thread-max-width)*0.9)] break-words leading-7">
          <MessagePrimitive.Content
            components={{ Text: MarkdownText }}
          />
        </div>
        <ActionBar />
      </div>
    </MessagePrimitive.Root>
  );
}
```

#### 5. `apps/desktop/src/components/chat/MarkdownText.tsx`

Markdown rendering for messages:

```tsx
import { useMemo } from 'react';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { vscDarkPlus } from 'react-syntax-highlighter/dist/esm/styles/prism';
import remarkGfm from 'remark-gfm';

interface MarkdownTextProps {
  text: string;
}

export function MarkdownText({ text }: MarkdownTextProps) {
  const components = useMemo(
    () => ({
      code({ node, inline, className, children, ...props }: any) {
        const match = /language-(\w+)/.exec(className || '');
        return !inline && match ? (
          <SyntaxHighlighter
            style={vscDarkPlus}
            language={match[1]}
            PreTag="div"
            {...props}
          >
            {String(children).replace(/\n$/, '')}
          </SyntaxHighlighter>
        ) : (
          <code className="rounded bg-muted px-1 py-0.5 text-sm" {...props}>
            {children}
          </code>
        );
      },
    }),
    []
  );

  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={components}
      className="prose prose-invert max-w-none prose-pre:bg-transparent prose-pre:p-0"
    >
      {text}
    </ReactMarkdown>
  );
}
```

**Note:** Install required markdown dependencies:
```bash
pnpm add react-markdown react-syntax-highlighter remark-gfm
pnpm add -D @types/react-syntax-highlighter
```

#### 6. `apps/desktop/src/components/chat/ActionBar.tsx`

Message action buttons (copy, regenerate):

```tsx
import { ActionBarPrimitive, MessagePrimitive } from '@assistant-ui/react';
import { Copy, RefreshCw, Check } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useState } from 'react';

export function ActionBar() {
  const [copied, setCopied] = useState(false);

  const handleCopy = async () => {
    // Get message content from context
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="mt-2 flex items-center gap-1">
      <ActionBarPrimitive.Copy asChild>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 gap-1 text-xs"
          onClick={handleCopy}
        >
          {copied ? (
            <Check className="h-3 w-3" />
          ) : (
            <Copy className="h-3 w-3" />
          )}
          {copied ? 'Copied' : 'Copy'}
        </Button>
      </ActionBarPrimitive.Copy>
      
      <MessagePrimitive.If assistant>
        <ActionBarPrimitive.Reload asChild>
          <Button variant="ghost" size="sm" className="h-7 gap-1 text-xs">
            <RefreshCw className="h-3 w-3" />
            Regenerate
          </Button>
        </ActionBarPrimitive.Reload>
      </MessagePrimitive.If>
    </div>
  );
}
```

#### 7. `apps/desktop/src/components/chat/WelcomeScreen.tsx`

Empty state / welcome screen:

```tsx
import { Sparkles, Code2, Zap, Shield } from 'lucide-react';

export function WelcomeScreen() {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      <div className="mb-6 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-primary to-accent">
        <Sparkles className="h-8 w-8 text-white" />
      </div>
      
      <h1 className="mb-2 text-2xl font-bold">Welcome to AXORA</h1>
      <p className="mb-8 max-w-md text-muted-foreground">
        Your AI-powered coding assistant. Ask me to write code, debug issues, 
        or help with any programming task.
      </p>
      
      <div className="grid max-w-lg grid-cols-3 gap-4">
        <FeatureCard
          icon={<Code2 className="h-5 w-5" />}
          title="Write Code"
          description="Generate code in any language"
        />
        <FeatureCard
          icon={<Zap className="h-5 w-5" />}
          title="Debug"
          description="Find and fix bugs quickly"
        />
        <FeatureCard
          icon={<Shield className="h-5 w-5" />}
          title="Review"
          description="Get code review suggestions"
        />
      </div>
    </div>
  );
}

function FeatureCard({ icon, title, description }: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex flex-col items-center rounded-lg border border-border bg-card p-4">
      <div className="mb-2 text-primary">{icon}</div>
      <h3 className="mb-1 text-sm font-medium">{title}</h3>
      <p className="text-xs text-muted-foreground">{description}</p>
    </div>
  );
}
```

#### 8. `apps/desktop/src/panels/ChatPanel.tsx`

Main chat panel (container):

```tsx
import { Thread } from '@/components/chat/Thread';
import { RuntimeProvider } from '@/components/chat/RuntimeProvider';

export function ChatPanel() {
  return (
    <div className="flex h-full flex-col">
      <RuntimeProvider>
        <Thread />
      </RuntimeProvider>
    </div>
  );
}
```

### Hooks to Implement

#### 1. `apps/desktop/src/hooks/useOpenAICompatibleRuntime.ts`

Custom hook for OpenAI-compatible API integration:

```tsx
import { useMemo } from 'react';
import { useChatRuntime, AssistantChatTransport } from '@assistant-ui/react-ai-sdk';
import { useSettingsStore } from '@/store/settings-store';

export function useOpenAICompatibleRuntime() {
  const { settings } = useSettingsStore();
  
  const runtime = useChatRuntime({
    transport: new AssistantChatTransport({
      api: getApiEndpoint(settings),
      headers: getApiHeaders(settings),
    }),
  });

  return runtime;
}

function getApiEndpoint(settings: AppSettings): string {
  switch (settings.model.provider) {
    case 'ollama':
      return `${settings.model.baseUrl}/api/chat`;
    case 'openai':
      return 'https://api.openai.com/v1/chat/completions';
    case 'anthropic':
      return 'https://api.anthropic.com/v1/messages';
    default:
      return settings.model.baseUrl || '/api/chat';
  }
}

function getApiHeaders(settings: AppSettings): Record<string, string> {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  if (settings.model.apiKey) {
    headers['Authorization'] = `Bearer ${settings.model.apiKey}`;
  }

  // Provider-specific headers
  if (settings.model.provider === 'anthropic') {
    headers['anthropic-version'] = '2023-06-01';
  }

  return headers;
}
```

---

## Phase 4: State Management Integration

### Zustand Store Updates

No changes needed to `settings-store.ts` - it already supports:
- Model provider selection (ollama, openai, anthropic)
- Base URL configuration (for Ollama/Chinese providers)
- API key management

### New Stores

#### 1. `apps/desktop/src/store/chat-store.ts`

Additional chat-specific state (assistant-ui handles most chat state):

```typescript
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface ChatState {
  // UI state only - messages handled by assistant-ui runtime
  isSidebarOpen: boolean;
  activeConversationId: string | null;
  conversationIds: string[];
}

interface ChatActions {
  toggleSidebar: () => void;
  setActiveConversation: (id: string | null) => void;
  createConversation: () => string;
  deleteConversation: (id: string) => void;
}

export const useChatStore = create<ChatState & ChatActions>()(
  persist(
    (set, get) => ({
      isSidebarOpen: true,
      activeConversationId: null,
      conversationIds: [],

      toggleSidebar: () => {
        set((state) => ({ isSidebarOpen: !state.isSidebarOpen }));
      },

      setActiveConversation: (id) => {
        set({ activeConversationId: id });
      },

      createConversation: () => {
        const id = crypto.randomUUID();
        set((state) => ({
          conversationIds: [id, ...state.conversationIds],
          activeConversationId: id,
        }));
        return id;
      },

      deleteConversation: (id) => {
        set((state) => ({
          conversationIds: state.conversationIds.filter((cid) => cid !== id),
          activeConversationId:
            state.activeConversationId === id
              ? state.conversationIds[0] || null
              : state.activeConversationId,
        }));
      },
    }),
    {
      name: 'axora-chat',
    }
  )
);
```

### Runtime Provider Component

#### `apps/desktop/src/components/chat/RuntimeProvider.tsx`

```tsx
import { AssistantRuntimeProvider } from '@assistant-ui/react';
import { useOpenAICompatibleRuntime } from '@/hooks/useOpenAICompatibleRuntime';
import { ReactNode } from 'react';

interface RuntimeProviderProps {
  children: ReactNode;
}

export function RuntimeProvider({ children }: RuntimeProviderProps) {
  const runtime = useOpenAICompatibleRuntime();

  return (
    <AssistantRuntimeProvider runtime={runtime}>
      {children}
    </AssistantRuntimeProvider>
  );
}
```

---

## Phase 5: API Integration

### OpenAI-Compatible Client

The `useOpenAICompatibleRuntime` hook (shown above) handles the API integration. Key features:

1. **Provider Detection**: Automatically configures endpoint based on selected provider
2. **Custom Base URL**: Supports any OpenAI-compatible endpoint (Ollama, Chinese providers like DeepSeek, Qwen, etc.)
3. **Authentication**: Injects API keys from settings store
4. **Streaming**: Uses AI SDK's streaming capabilities

### Streaming Support

assistant-ui's `useChatRuntime` automatically handles:
- SSE (Server-Sent Events) streaming
- Token-by-token rendering
- Cancellation mid-stream
- Error recovery

No additional configuration needed - it works out of the box with OpenAI-compatible endpoints.

### Example: Chinese Provider Support

Users can configure Chinese AI providers in settings:

```typescript
// Settings configuration for DeepSeek
{
  model: {
    provider: 'openai', // Use OpenAI-compatible mode
    model: 'deepseek-chat',
    baseUrl: 'https://api.deepseek.com',
    apiKey: 'sk-...'
  }
}

// Settings configuration for Alibaba Qwen
{
  model: {
    provider: 'openai',
    model: 'qwen-turbo',
    baseUrl: 'https://dashscope.aliyuncs.com/compatible-mode/v1',
    apiKey: 'sk-...'
  }
}

// Settings configuration for Moonshot (Kimi)
{
  model: {
    provider: 'openai',
    model: 'moonshot-v1-8k',
    baseUrl: 'https://api.moonshot.cn/v1',
    apiKey: 'sk-...'
  }
}
```

---

## Phase 6: Plan Updates

### Plans to Modify

| Plan | Location | Updates Needed |
|------|----------|----------------|
| SPRINT-C5 | `planning/phase-4/agent-c/SPRINT-C5-CHAT-INTERFACE.md` | Replace custom implementation with assistant-ui integration. Update file paths from custom components to assistant-ui components. |
| SPRINT-C6 | `planning/phase-4/agent-c/SPRINT-C6-INTEGRATION.md` | Update integration notes to mention assistant-ui as the chat foundation. |
| PHASE-003 | `planning/shared/PHASE-003-desktop-app.md` | Add note about assistant-ui integration in the "Message Streaming" section. |

### Plans to Keep Unchanged

| Plan | Reason |
|------|--------|
| SPRINT-B4 (Settings) | Settings UI is already implemented and works independently |
| SPRINT-A4 (UI Components) | shadcn/ui components remain the foundation |
| All completed sprints | Historical records, should not be modified |

### Migration Notes for SPRINT-C5

Update the sprint to reference:
- Use assistant-ui's `Thread` instead of custom `ChatPanel`
- Use `ComposerPrimitive` instead of custom `MissionInput`
- Use `MessagePrimitive` instead of custom `MessageList`
- File attachments can be added via assistant-ui's attachment primitives
- assistant-ui handles streaming, auto-scroll, and loading states automatically

---

## Phase 7: Main App Integration

### Update `apps/desktop/src/App.tsx`

Replace the placeholder UI with the chat interface:

```tsx
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Settings, MessageSquare } from 'lucide-react';
import { ChatPanel } from '@/panels/ChatPanel';
import { SettingsPanel } from '@/panels/SettingsPanel';

type Panel = 'chat' | 'settings';

function App() {
  const [activePanel, setActivePanel] = useState<Panel>('chat');

  return (
    <div className="flex h-screen flex-col bg-background">
      {/* Header */}
      <header className="flex h-14 items-center justify-between border-b border-border px-4">
        <div className="flex items-center gap-4">
          <h1 className="text-xl font-bold text-primary">AXORA</h1>
          <Badge variant="secondary">v0.1.0</Badge>
        </div>
        <div className="flex items-center gap-2">
          <Button
            variant={activePanel === 'chat' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setActivePanel('chat')}
          >
            <MessageSquare className="mr-2 h-4 w-4" />
            Chat
          </Button>
          <Button
            variant={activePanel === 'settings' ? 'secondary' : 'ghost'}
            size="sm"
            onClick={() => setActivePanel('settings')}
          >
            <Settings className="mr-2 h-4 w-4" />
            Settings
          </Button>
        </div>
      </header>

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        {activePanel === 'chat' ? (
          <ChatPanel />
        ) : (
          <div className="h-full overflow-auto p-6">
            <SettingsPanel />
          </div>
        )}
      </main>
    </div>
  );
}

export default App;
```

---

## Testing Checklist

### Tailwind v4 Migration
- [ ] `tailwind.config.js` removed
- [ ] Vite config includes `@tailwindcss/vite` plugin
- [ ] `globals.css` uses `@import "tailwindcss"`
- [ ] `@theme` directive defines all CSS variables
- [ ] All shadcn/ui components render correctly
- [ ] Dark mode works (toggle and system preference)
- [ ] No console errors about Tailwind directives

### assistant-ui Integration
- [ ] `pnpm install @assistant-ui/react @assistant-ui/react-ai-sdk` succeeds
- [ ] `pnpm install ai @ai-sdk/react @ai-sdk/openai` succeeds
- [ ] RuntimeProvider wraps chat components
- [ ] Thread component renders without errors

### Chat Functionality
- [ ] Chat messages render correctly (user and assistant)
- [ ] User messages appear on the right with user avatar
- [ ] Assistant messages appear on the left with bot avatar
- [ ] Composer auto-resizes with multi-line input
- [ ] Send button submits message
- [ ] Cancel button stops generation

### Streaming
- [ ] Streaming works with OpenAI API
- [ ] Streaming works with Ollama (local)
- [ ] Chinese providers (DeepSeek, Qwen) stream correctly
- [ ] Token-by-token rendering visible
- [ ] Cancel mid-stream works

### Settings Integration
- [ ] Model provider selection (OpenAI/Ollama/Anthropic) changes API endpoint
- [ ] API key is sent in Authorization header
- [ ] Base URL configuration works for custom endpoints
- [ ] Changes apply without page reload

### UI/UX
- [ ] Dark mode works correctly
- [ ] Welcome screen displays when no messages
- [ ] Auto-scroll to new messages works
- [ ] Copy message button works
- [ ] Regenerate button works
- [ ] No visual regressions in existing components
- [ ] Markdown renders correctly (code blocks, lists, links)

### No Regressions
- [ ] Settings panel works as before
- [ ] All existing shadcn/ui components function
- [ ] Zustand stores persist correctly
- [ ] Tauri commands still work

---

## File Structure Summary

```
apps/desktop/src/
├── components/
│   ├── ui/                    # Existing shadcn/ui (unchanged)
│   └── chat/                  # NEW: assistant-ui components
│       ├── Thread.tsx
│       ├── Composer.tsx
│       ├── UserMessage.tsx
│       ├── AssistantMessage.tsx
│       ├── MarkdownText.tsx
│       ├── ActionBar.tsx
│       ├── WelcomeScreen.tsx
│       └── RuntimeProvider.tsx
├── hooks/
│   └── useOpenAICompatibleRuntime.ts  # NEW
├── panels/
│   ├── SettingsPanel.tsx      # Existing (unchanged)
│   └── ChatPanel.tsx          # NEW: Main chat panel
├── store/
│   ├── settings-store.ts      # Existing (unchanged)
│   └── chat-store.ts          # NEW: Chat UI state
├── types/
│   └── settings.ts            # Existing (unchanged)
├── styles/
│   ├── globals.css            # MODIFIED: Tailwind v4
│   └── styles.css             # Existing (Tauri defaults)
├── App.tsx                    # MODIFIED: Add chat panel
└── main.tsx                   # Existing (unchanged)
```

---

## Dependencies Summary

### New Dependencies to Add

```bash
# Core assistant-ui
pnpm add @assistant-ui/react @assistant-ui/react-ai-sdk

# AI SDK for OpenAI-compatible APIs
pnpm add ai @ai-sdk/react @ai-sdk/openai

# Ollama provider
pnpm add ollama-ai-provider

# Markdown rendering
pnpm add react-markdown react-syntax-highlighter remark-gfm

# Dev dependencies
pnpm add -D @types/react-syntax-highlighter

# Tailwind v4
pnpm add -D tailwindcss@4 @tailwindcss/vite
```

### Dependencies to Remove

```bash
# Remove v3 tailwind
pnpm remove tailwindcss
```

---

## Key Implementation Notes

1. **assistant-ui State Management**: assistant-ui handles its own message state internally. Use the `useChatRuntime` hook to interact with it, and use Zustand only for UI state (sidebar open/closed, conversation list).

2. **OpenAI Compatibility**: The `AssistantChatTransport` from `@assistant-ui/react-ai-sdk` is designed for OpenAI-compatible APIs. It automatically handles message formatting and streaming.

3. **Chinese Providers**: Since most Chinese AI providers (DeepSeek, Qwen, Moonshot) offer OpenAI-compatible APIs, configure them as `provider: 'openai'` with custom `baseUrl`.

4. **Tailwind v4 Compatibility**: shadcn/ui components use CSS variables that need to be preserved. The `@theme` directive in v4 maps to these variables.

5. **No Backend Changes Required**: This is a frontend-only integration. The existing settings store API endpoints don't need modification.

---

## References

- **assistant-ui Docs**: https://www.assistant-ui.com/docs
- **assistant-ui Thread Component**: https://www.assistant-ui.com/docs/ui/thread
- **AI SDK Integration**: https://www.assistant-ui.com/docs/runtimes/ai-sdk/v6
- **Tailwind CSS v4**: https://tailwindcss.com/docs/v4-beta
- **shadcn/ui**: https://ui.shadcn.com
- **AXORA AGENTS.md**: `/Users/noasantos/Fluri/axora/AGENTS.md`
