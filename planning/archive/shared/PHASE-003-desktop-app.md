# Phase 3: Desktop Application & Technology Research

**Status:** 📋 PLANNED  
**Priority:** HIGH  
**Estimated Effort:** 5-7 days

## Summary

Research and implement the desktop application. Currently using Tauri v2, but we should evaluate alternatives.

## Current State

### What Exists
- Basic Tauri v2 scaffold in `apps/desktop/`
- Default template UI (not functional)
- No gRPC client integration
- No connection to daemon

---

## Part A: Technology Research

### Tauri v2 Evaluation (Current Choice)

**Pros:**
- ✅ Small bundle size (~10MB vs Electron's ~150MB)
- ✅ Better performance (uses system webview, not bundled Chromium)
- ✅ Rust backend for native functionality
- ✅ Good security model
- ✅ Active development, v2 released in 2024

**Cons:**
- ⚠️ v2 is relatively new (less mature ecosystem)
- ⚠️ iOS/Android support still in alpha
- ⚠️ Some plugins may not be v2-compatible yet
- ⚠️ Requires Rust knowledge for backend code

**Verdict:** ✅ **KEEP** - Good choice for this project since we're already using Rust

---

### Alternative 1: Electron

**Pros:**
- Mature ecosystem, huge community
- Cross-platform with consistent behavior
- Large plugin ecosystem
- Easy to find developers

**Cons:**
- Large bundle size (~150MB minimum)
- High memory usage
- Slower performance
- Security concerns (Chromium vulnerabilities)

**Verdict:** ❌ Not recommended - Overkill for this use case

---

### Alternative 2: Wails (Go + Frontend)

**Pros:**
- Similar to Tauri but uses Go
- Smaller learning curve for web developers
- Good performance

**Cons:**
- Would require learning Go
- Smaller ecosystem than Tauri
- Less mature

**Verdict:** ❌ Not worth switching - We're already invested in Rust

---

### Alternative 3: Pure Web App

**Pros:**
- No desktop app to maintain
- Easy updates
- Accessible from anywhere

**Cons:**
- Can't access native features easily
- Requires hosting
- Less integrated experience

**Verdict:** ❌ Not suitable - We want native integration

---

### Alternative 4: Rust-native UI (Iced, Dioxus, Tauri 2.0)

**Iced:**
- Pure Rust, Elm-like architecture
- Very early stage, limited widgets
- Steep learning curve

**Dioxus:**
- React-like syntax for Rust
- Can target web, desktop, mobile
- Still in beta

**Verdict:** ⚠️ **Consider for future** - Not mature enough yet

---

## Recommendation

**Stay with Tauri v2** because:
1. Already set up in the project
2. Leverages existing Rust expertise
3. Best performance/size tradeoff
4. Good security model for a coding tool
5. Active development and good documentation

---

## Part B: Desktop Implementation Plan

### Step 1: Set Up gRPC Client

Create `apps/desktop/src/lib/grpc.ts`:

```typescript
import { createPromiseClient } from '@connectrpc/connect';
import { createGrpcWebTransport } from '@connectrpc/connect-web';
import { CollectiveService } from '../proto/collective/v1/core_connectweb';

const transport = createGrpcWebTransport({
  baseUrl: 'http://localhost:50051',
});

export const collectiveClient = createPromiseClient(
  CollectiveService,
  transport
);
```

### Step 2: Create React Query Setup

```typescript
// apps/desktop/src/lib/query.ts
import { QueryClient } from '@tanstack/react-query';

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchInterval: 5000, // Poll every 5 seconds
      retry: 2,
    },
  },
});
```

### Step 3: Implement Agent Hooks

```typescript
// apps/desktop/src/hooks/useAgents.ts
import { useQuery, useMutation } from '@tanstack/react-query';
import { collectiveClient } from '../lib/grpc';

export function useAgents() {
  return useQuery({
    queryKey: ['agents'],
    queryFn: () => collectiveClient.listAgents({}),
  });
}

export function useRegisterAgent() {
  return useMutation({
    mutationFn: (data: { name: string; role: string }) =>
      collectiveClient.registerAgent(data),
  });
}
```

### Step 4: Create UI Components

#### Agent List Component
```tsx
// apps/desktop/src/components/AgentList.tsx
export function AgentList() {
  const { data, isLoading } = useAgents();
  
  if (isLoading) return <div>Loading...</div>;
  
  return (
    <div className="agent-list">
      {data?.agents.map(agent => (
        <AgentCard key={agent.id} agent={agent} />
      ))}
    </div>
  );
}
```

#### Task Dashboard
```tsx
// apps/desktop/src/components/TaskDashboard.tsx
export function TaskDashboard() {
  const { data: tasks } = useTasks();
  
  return (
    <div className="task-dashboard">
      <TaskColumn status="pending" tasks={tasks?.pending} />
      <TaskColumn status="in-progress" tasks={tasks?.inProgress} />
      <TaskColumn status="completed" tasks={tasks?.completed} />
    </div>
  );
}
```

### Step 5: Message Streaming

```typescript
// apps/desktop/src/hooks/useMessageStream.ts
export function useMessageStream(agentId: string) {
  const [messages, setMessages] = useState<Message[]>([]);
  
  useEffect(() => {
    const stream = collectiveClient.streamMessages({ agentId });
    
    for await (const message of stream) {
      setMessages(prev => [...prev, message]);
    }
  }, [agentId]);
  
  return messages;
}
```

### Step 6: Tauri Commands (Optional Native Features)

```rust
// apps/desktop/src-tauri/src/main.rs
use tauri::command;

#[command]
fn get_system_info() -> String {
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![get_system_info])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Dependencies to Add

### Frontend (apps/desktop/package.json)
```json
{
  "dependencies": {
    "@connectrpc/connect": "^1.4.0",
    "@connectrpc/connect-web": "^1.4.0",
    "@tanstack/react-query": "^5.17.0",
    "@tauri-apps/plugin-shell": "^2.0.0-alpha",
    "react-router-dom": "^6.21.0"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2.0.0-alpha",
    "bufbuild/protobuf": "^1.8.0"
  }
}
```

### Rust Backend (apps/desktop/src-tauri/Cargo.toml)
```toml
[dependencies]
tauri = { version = "2.0.0-alpha", features = [] }
tonic = "0.11"
prost = "0.12"
```

---

## UI/UX Design

### Layout
```
┌─────────────────────────────────────────────────────────┐
│  OPENAKTA                           [Settings] [Profile]   │
├────────────┬────────────────────────────────────────────┤
│            │                                            │
│  Agents    │           Main Content Area                │
│  Tasks     │                                            │
│  Messages  │   - Agent list / details                   │
│  Settings  │   - Task board                             │
│            │   - Message stream                         │
│            │                                            │
├────────────┴────────────────────────────────────────────┤
│  Status: ● Connected to daemon (localhost:50051)        │
└─────────────────────────────────────────────────────────┘
```

### Color Scheme
- Primary: `#6366f1` (Indigo)
- Background: `#0f172a` (Dark slate)
- Surface: `#1e293b` (Slate)
- Success: `#22c55e` (Green)
- Warning: `#f59e0b` (Amber)
- Error: `#ef4444` (Red)

---

## Acceptance Criteria

- [ ] Desktop app connects to daemon via gRPC
- [ ] Can list/register/unregister agents
- [ ] Can create/view tasks
- [ ] Real-time message streaming works
- [ ] UI is responsive and modern
- [ ] Build succeeds for macOS, Windows, Linux

---

## Timeline

| Week | Task |
|------|------|
| 1 | Set up gRPC client, React Query |
| 2 | Implement agent management UI |
| 3 | Implement task dashboard |
| 4 | Message streaming, polish, testing |

---

## Related Phases

- Phase 1: ✅ Daemon Build Fixes (completed)
- Phase 2: 🔄 Storage Implementation (in progress)
- Phase 4: 📋 Agent System (pending)
- Phase 5: 📋 Integration & Testing (pending)
