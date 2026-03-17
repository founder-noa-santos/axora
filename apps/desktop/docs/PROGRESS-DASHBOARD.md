# Progress Dashboard

Real-time progress dashboard for monitoring mission execution with WebSocket-based live updates.

---

## 📋 Overview

The Progress Dashboard provides real-time visibility into mission execution, worker status, and blocker alerts. It uses WebSocket connections to receive instant updates from the backend.

**Features:**
- Real-time mission progress tracking
- Worker status monitoring
- Blocker alerts with severity levels
- ETA calculations
- Overall progress summary

---

## 🏗️ Architecture

```
┌─────────────────┐
│  ProgressPanel  │
│   (UI Component)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ProgressStore   │
│  (Zustand Store)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ ProgressWebSocket│
│  (WebSocket Client)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Backend WS    │
│    Server       │
└─────────────────┘
```

---

## 📦 Components

### 1. ProgressPanel

**Location:** `src/panels/ProgressPanel.tsx`

Main UI component displaying:
- Overall progress summary
- Active missions with progress bars
- Worker status indicators
- Blocker alerts
- Completed/Failed mission history

**Usage:**
```tsx
import { ProgressPanel } from '@/panels/ProgressPanel';

function App() {
  return <ProgressPanel />;
}
```

### 2. ProgressWebSocket

**Location:** `src/api/progress-websocket.ts`

WebSocket client with:
- Automatic reconnection (exponential backoff)
- Event handler registration
- Connection state management

**Usage:**
```typescript
import { progressWS } from '@/api/progress-websocket';

progressWS.connect('ws://localhost:8080/progress');
progressWS.onEvent((event) => {
  console.log('Received event:', event);
});
progressWS.disconnect();
```

### 3. ProgressStore

**Location:** `src/store/progress-store.ts`

Zustand store managing:
- Active missions state
- Completed/Failed missions history
- Worker status
- Blocker alerts
- WebSocket connection state

**Usage:**
```typescript
import { useProgressStore } from '@/store/progress-store';

const { activeMissions, workers, blockers } = useProgressStore();
```

---

## 🔌 WebSocket Events

### Event Types

#### mission:started
```typescript
{
  type: 'mission:started',
  payload: {
    missionId: string,
    message?: string
  }
}
```

#### mission:progress
```typescript
{
  type: 'mission:progress',
  payload: {
    missionId: string,
    progress: number,  // 0-100
    eta?: number,      // Seconds remaining
    status: 'running',
    message?: string
  }
}
```

#### mission:completed
```typescript
{
  type: 'mission:completed',
  payload: {
    missionId: string,
    result: string,
    completedAt: number
  }
}
```

#### mission:failed
```typescript
{
  type: 'mission:failed',
  payload: {
    missionId: string,
    error: string,
    failedAt: number
  }
}
```

#### worker:status
```typescript
{
  type: 'worker:status',
  payload: {
    workerId: string,
    status: 'idle' | 'busy' | 'unhealthy' | 'failed',
    currentTask?: string,
    lastHeartbeat: number,
    healthScore?: number
  }
}
```

#### blocker:alert
```typescript
{
  type: 'blocker:alert',
  payload: {
    id: string,
    missionId: string,
    reason: string,
    stalledSince: number,
    severity: 'warning' | 'error'
  }
}
```

#### blocker:resolved
```typescript
{
  type: 'blocker:resolved',
  payload: {
    blockerId: string,
    resolvedAt: number
  }
}
```

---

## 🎨 UI Components

### Progress Bars
- Display mission progress (0-100%)
- Color-coded by status
- Show ETA when available

### Worker Status Indicators
- **Idle**: Gray indicator
- **Busy**: Purple indicator (primary color)
- **Unhealthy**: Yellow/Warning indicator
- **Failed**: Red/Destructive indicator

### Blocker Alerts
- Warning level: Yellow border
- Error level: Red border
- Dismissible with X button

### Status Badges
- Connection status (Connected/Disconnected)
- Mission status (pending, running, completed, failed)
- Worker status

---

## 🔧 Configuration

### WebSocket URL

Set the WebSocket URL when connecting:

```typescript
const state = useProgressStore.getState();
state.connectWebSocket('ws://localhost:8080/progress');
```

### Reconnection Settings

The WebSocket client automatically reconnects with exponential backoff:
- Max attempts: 5
- Initial delay: 2 seconds
- Max delay: 30 seconds
- Backoff multiplier: 2x

---

## 📊 Progress Summary

The dashboard calculates overall progress:

```typescript
const summary = useProgressStore.getState().getProgressSummary();
// {
//   totalMissions: number,
//   activeMissions: number,
//   completedMissions: number,
//   failedMissions: number,
//   overallProgress: number  // 0-100
// }
```

---

## 🧪 Testing

### Run Tests

```bash
cd apps/desktop
pnpm vitest run src/panels/__tests__/ProgressPanel.test.tsx
pnpm vitest run src/store/__tests__/progress-store.test.ts
pnpm vitest run src/api/__tests__/progress-websocket.test.ts
pnpm vitest run src/types/__tests__/progress.test.ts
```

### Test Coverage

- **ProgressWebSocket**: Connection, reconnection, event handling
- **ProgressStore**: State updates, event processing, summary calculation
- **ProgressPanel**: Rendering, empty states, status display
- **Utilities**: Duration formatting, ETA calculation

---

## 📝 Utilities

### formatDuration

Convert seconds to human-readable format:

```typescript
import { formatDuration } from '@/types/progress';

formatDuration(5);      // "5s"
formatDuration(90);     // "1m 30s"
formatDuration(3660);   // "1h 1m"
```

### calculateETA

Estimate time remaining based on progress:

```typescript
import { calculateETA } from '@/types/progress';

calculateETA(50, 100);  // 100 (seconds remaining)
```

---

## 🔗 Dependencies

- **zustand**: State management
- **@radix-ui/react-progress**: Progress bar component
- **@radix-ui/react-badge**: Status badges
- **@radix-ui/react-alert**: Alert dialogs
- **lucide-react**: Icons

---

## 🚨 Error Handling

### WebSocket Disconnection
- Automatic reconnection with exponential backoff
- Status badge shows "Disconnected" state
- Events queued until reconnection

### Malformed Messages
- Caught and logged to console
- Do not crash the application
- Other events continue processing

---

## 📈 Performance

- **Update Frequency**: Real-time (<500ms latency)
- **Reconnection Delay**: 2s - 30s (exponential backoff)
- **State Updates**: Optimistic UI updates
- **Memory**: Automatic cleanup on disconnect

---

## 🔐 Security

- WebSocket URL should use `wss://` in production
- No sensitive data in WebSocket messages
- Connection state visible to user

---

## 📚 Related Documentation

- [Architecture Ledger](../../docs/ARCHITECTURE-LEDGER.md)
- [WebSocket Events](#websocket-events)
- [UI Components Guide](./UI-COMPONENTS-GUIDE.md)

---

**Last Updated:** 2026-03-17  
**Sprint:** A5
