# AXORA API Layer Documentation

**Version:** 1.0.0  
**Created:** 2026-03-17  
**Status:** Active

---

## 📋 Overview

The AXORA API Layer provides a unified interface for frontend-backend communication, supporting both REST and WebSocket protocols. It includes a mock API for development before the Phase 3 backend is ready.

**Features:**
- ✅ Type-safe REST API client
- ✅ Real-time WebSocket client with auto-reconnection
- ✅ Mock API for development
- ✅ Exponential backoff retry logic
- ✅ Comprehensive error handling
- ✅ Mission and worker management
- ✅ Settings synchronization

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Frontend Application                  │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                      API Layer                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ REST Client  │  │ WebSocket    │  │ Mock API     │  │
│  │              │  │ Client       │  │              │  │
│  │ - Missions   │  │ - Events     │  │ - Dev Mode   │  │
│  │ - Workers    │  │ - Real-time  │  │ - Simulation │  │
│  │ - Settings   │  │ - Heartbeat  │  │ - Testing    │  │
│  └──────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                  Backend (Phase 3)                       │
│  - Coordinator Agent                                     │
│  - Worker Pool                                           │
│  - Settings Service                                      │
└─────────────────────────────────────────────────────────┘
```

---

## 📦 Installation

The API layer is part of the AXORA desktop application:

```bash
cd apps/desktop
pnpm install
```

---

## 🚀 Quick Start

### 1. Initialize API

```typescript
import { initializeApi, enableMockApi } from './api';

// Initialize with mock mode (recommended for development)
initializeApi({
  mockMode: true,
  baseUrl: 'http://localhost:3000/api',
  wsUrl: 'ws://localhost:3000/ws',
});

// Enable mock API
enableMockApi();
```

### 2. Submit a Mission

```typescript
import { restClient } from './api';

// Submit a new mission
const result = await restClient.submitMission(
  'Implement authentication system'
);

console.log(`Mission ${result.missionId} submitted!`);
```

### 3. Listen for Progress

```typescript
import { wsClient } from './api';

// Subscribe to mission progress
wsClient.onMissionProgress((event) => {
  console.log(`Progress: ${event.payload.progress}%`);
  console.log(`Current step: ${event.payload.currentStep}`);
});

// Subscribe to mission completion
wsClient.onMissionCompleted((event) => {
  console.log(`Mission completed: ${event.payload.result}`);
});
```

---

## 📖 API Reference

### Configuration

```typescript
import { configureApi, getApiConfig, ApiConfig } from './api';

// Get current configuration
const config = getApiConfig();

// Update configuration
configureApi({
  baseUrl: 'http://localhost:3000/api',
  wsUrl: 'ws://localhost:3000/ws',
  mockMode: true,
  timeout: 30000,        // 30 seconds
  retryAttempts: 3,
  retryDelay: 1000,      // 1 second
});
```

**Configuration Options:**

| Option | Type | Default | Description |
|--------|------|---------|-------------|
| `baseUrl` | `string` | `http://localhost:3000/api` | REST API base URL |
| `wsUrl` | `string` | `ws://localhost:3000/ws` | WebSocket URL |
| `mockMode` | `boolean` | `true` | Enable mock API mode |
| `timeout` | `number` | `30000` | Request timeout (ms) |
| `retryAttempts` | `number` | `3` | Number of retry attempts |
| `retryDelay` | `number` | `1000` | Base retry delay (ms) |

---

### REST Client

```typescript
import { restClient, createRestClient } from './api';
```

#### Mission Endpoints

**Submit Mission**
```typescript
const result = await restClient.submitMission(
  'Mission content',
  [attachments]  // optional
);
// Returns: { missionId: string, message: string }
```

**Get Mission Status**
```typescript
const status = await restClient.getMission(missionId);
// Returns: { missionId, progress, status, content?, result?, error? }
```

**List Missions**
```typescript
const missions = await restClient.listMissions();
// Returns: { missions: Mission[] }
```

**Cancel Mission**
```typescript
const result = await restClient.cancelMission(missionId);
// Returns: { success: boolean }
```

#### Worker Endpoints

**List Workers**
```typescript
const workers = await restClient.listWorkers();
// Returns: { workers: Worker[] }
```

**Get Worker**
```typescript
const worker = await restClient.getWorker(workerId);
// Returns: { worker: Worker }
```

#### Settings Endpoints

**Get Settings**
```typescript
const settings = await restClient.getSettings();
// Returns: Settings object
```

**Update Settings**
```typescript
await restClient.updateSettings({
  theme: { mode: 'dark', accentColor: 'blue' },
});
// Returns: { success: boolean }
```

---

### WebSocket Client

```typescript
import { wsClient, createWebSocketClient } from './api';
```

#### Connection Management

**Connect**
```typescript
wsClient.connect('ws://localhost:3000/ws');
// or use config URL: wsClient.connect()
```

**Disconnect**
```typescript
wsClient.disconnect();
```

**Check Connection Status**
```typescript
const isConnected = wsClient.isConnected();
const state = wsClient.getState(); // 'disconnected' | 'connecting' | 'connected' | 'reconnecting'
```

#### Event Subscription

**Generic Event Handler**
```typescript
wsClient.on('mission:progress', (event) => {
  console.log(event.payload);
});

// Unsubscribe
wsClient.off('mission:progress', handler);
```

**Typed Event Handlers**
```typescript
// Mission events
wsClient.onMissionStarted((event) => { /* ... */ });
wsClient.onMissionProgress((event) => { /* ... */ });
wsClient.onMissionCompleted((event) => { /* ... */ });
wsClient.onMissionFailed((event) => { /* ... */ });

// Worker events
wsClient.onWorkerStatus((event) => { /* ... */ });
wsClient.onWorkerHeartbeat((event) => { /* ... */ });
```

#### Event Types

**Mission Started**
```typescript
{
  type: 'mission:started',
  payload: {
    missionId: string,
    status: 'running'
  }
}
```

**Mission Progress**
```typescript
{
  type: 'mission:progress',
  payload: {
    missionId: string,
    progress: number,      // 0-100
    eta?: number,          // seconds remaining
    currentStep?: string   // current step description
  }
}
```

**Mission Completed**
```typescript
{
  type: 'mission:completed',
  payload: {
    missionId: string,
    result: string
  }
}
```

**Worker Status**
```typescript
{
  type: 'worker:status',
  payload: {
    workerId: string,
    status: 'idle' | 'busy' | 'offline' | 'error',
    currentMissionId?: string,
    lastHeartbeat: number
  }
}
```

---

### Mock API

```typescript
import { mockApi, enableMockApi, disableMockApi, createMockApi } from './api';
```

#### Enable/Disable Mock Mode

```typescript
// Enable mock API
enableMockApi();

// Disable mock API (use real API)
disableMockApi();

// Check if mock mode is enabled
const isMock = mockApi.isMockMode();
```

#### Custom Mock Configuration

```typescript
const customMockApi = createMockApi({
  networkDelay: 200,        // Base delay (ms)
  delayVariation: 100,      // Variation (±ms)
  progressInterval: 1000,   // Progress update interval (ms)
  workerStatusInterval: 5000, // Worker status update interval (ms)
});

customMockApi.enableMockMode();
```

#### Mock Data Access

```typescript
// Get mock mission
const mission = mockApi.getMission(missionId);

// Get all missions
const missions = mockApi.getAllMissions();

// Get all workers
const workers = mockApi.getAllWorkers();

// Clear all mock data
mockApi.clearMockData();
```

---

## 🔧 Error Handling

### ApiError Class

```typescript
import { ApiError } from './api';

try {
  await restClient.getMission('invalid-id');
} catch (error) {
  if (error instanceof ApiError) {
    console.log(`Status: ${error.status}`);
    console.log(`Code: ${error.code}`);
    console.log(`Message: ${error.message}`);
    
    // Check error type
    if (error.isNetworkError()) {
      // Handle network error
    } else if (error.isClientError()) {
      // Handle 4xx error
    } else if (error.isServerError()) {
      // Handle 5xx error
    }
  }
}
```

### Error Types

| Error Type | Status | Retryable | Description |
|------------|--------|-----------|-------------|
| Network Error | 0 | ✅ | Connection failed, timeout |
| Client Error | 4xx | ❌ | Bad request, not found, unauthorized |
| Server Error | 5xx | ✅ | Internal error, unavailable |

### Retry Logic

The REST client automatically retries failed requests:

```typescript
// Configure retry behavior
configureApi({
  retryAttempts: 3,      // Number of retries
  retryDelay: 1000,      // Base delay (ms)
});

// Retry uses exponential backoff with jitter:
// delay = baseDelay * 2^attempt + random jitter (±25%)
```

---

## 📝 Usage Examples

### Example 1: Mission Progress Dashboard

```typescript
import { restClient, wsClient, enableMockApi } from './api';

class MissionDashboard {
  constructor() {
    this.setupEventListeners();
  }

  setupEventListeners() {
    wsClient.onMissionStarted((event) => {
      console.log(`Mission ${event.payload.missionId} started`);
      this.addMission(event.payload.missionId);
    });

    wsClient.onMissionProgress((event) => {
      this.updateProgress(
        event.payload.missionId,
        event.payload.progress,
        event.payload.currentStep
      );
    });

    wsClient.onMissionCompleted((event) => {
      this.completeMission(event.payload.missionId, event.payload.result);
    });
  }

  async submitMission(content: string) {
    const result = await restClient.submitMission(content);
    return result.missionId;
  }

  private addMission(missionId: string) {
    // Add mission to UI
  }

  private updateProgress(missionId: string, progress: number, step: string) {
    // Update progress bar
  }

  private completeMission(missionId: string, result: string) {
    // Mark as complete
  }
}

// Usage
enableMockApi();
const dashboard = new MissionDashboard();
dashboard.submitMission('Implement feature X');
```

### Example 2: Worker Monitor

```typescript
import { restClient, wsClient } from './api';

class WorkerMonitor {
  private workers = new Map<string, any>();

  constructor() {
    this.startMonitoring();
  }

  async startMonitoring() {
    // Initial worker list
    const response = await restClient.listWorkers();
    response.workers.forEach(worker => {
      this.workers.set(worker.workerId, worker);
    });

    // Listen for updates
    wsClient.onWorkerStatus((event) => {
      const { workerId, status } = event.payload;
      this.updateWorkerStatus(workerId, status);
    });

    wsClient.onWorkerHeartbeat((event) => {
      const { workerId, timestamp } = event.payload;
      this.updateHeartbeat(workerId, timestamp);
    });
  }

  private updateWorkerStatus(workerId: string, status: string) {
    const worker = this.workers.get(workerId);
    if (worker) {
      worker.status = status;
      this.render();
    }
  }

  private updateHeartbeat(workerId: string, timestamp: number) {
    const worker = this.workers.get(workerId);
    if (worker) {
      worker.lastHeartbeat = timestamp;
      this.render();
    }
  }

  private render() {
    // Update UI
  }
}
```

### Example 3: Settings Sync

```typescript
import { restClient, wsClient } from './api';
import { useSettingsStore } from '../store/settings';

class SettingsSync {
  constructor() {
    this.syncSettings();
  }

  async syncSettings() {
    try {
      // Load from API
      const settings = await restClient.getSettings();
      useSettingsStore.getState().setSettings(settings);
    } catch (error) {
      console.error('Failed to sync settings:', error);
    }
  }

  async updateSettings(settings: Partial<Settings>) {
    try {
      await restClient.updateSettings(settings);
      useSettingsStore.getState().updateSettings(settings);
    } catch (error) {
      console.error('Failed to update settings:', error);
      throw error;
    }
  }
}
```

---

## 🧪 Testing

### Running Tests

```bash
cd apps/desktop
pnpm test
```

### Test Files

- `rest-client.test.ts` — REST API client tests
- `websocket-client.test.ts` — WebSocket client tests
- `mock-api.test.ts` — Mock API tests
- `integration.test.ts` — Integration tests

### Writing Tests

```typescript
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { restClient, enableMockApi } from './api';

describe('My Feature', () => {
  beforeEach(() => {
    enableMockApi();
  });

  it('should work', async () => {
    const result = await restClient.submitMission('Test');
    expect(result.missionId).toBeDefined();
  });
});
```

---

## 🎯 Best Practices

### 1. Always Use Mock Mode for Development

```typescript
// ✅ Good
enableMockApi();

// ❌ Bad (unless backend is ready)
disableMockApi();
```

### 2. Handle Errors Gracefully

```typescript
// ✅ Good
try {
  await restClient.getMission(id);
} catch (error) {
  if (error instanceof ApiError) {
    showError(error.message);
  }
}

// ❌ Bad
await restClient.getMission(id); // May throw
```

### 3. Clean Up Event Listeners

```typescript
// ✅ Good (for long-lived components)
componentWillUnmount() {
  wsClient.off('mission:progress', this.handler);
}

// ❌ Bad (memory leak)
wsClient.on('mission:progress', handler); // Never cleaned up
```

### 4. Use Typed Event Handlers

```typescript
// ✅ Good
wsClient.onMissionProgress((event) => {
  console.log(event.payload.progress); // Type-safe
});

// ❌ Bad (no type safety)
wsClient.on('mission:progress', (event) => {
  console.log(event.payload.progress);
});
```

---

## 🔗 Related Documents

- [Phase 3 Master Plan](../../../planning/phase-3/PHASE-3-MASTER-PLAN.md)
- [Sprint B5 Specification](../../../planning/phase-4/agent-b/SPRINT-B5-API-INTEGRATION.md)
- [Settings Types](../types/settings.ts)

---

## 📝 Changelog

### 1.0.0 (2026-03-17)

- ✅ Initial release
- ✅ REST API client with retry logic
- ✅ WebSocket client with reconnection
- ✅ Mock API for development
- ✅ Comprehensive test suite

---

**Last Updated:** 2026-03-17  
**Maintained By:** Agent B (API Integration + Configuration)
