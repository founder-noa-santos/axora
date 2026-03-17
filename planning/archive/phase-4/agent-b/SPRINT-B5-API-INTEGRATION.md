# Phase 4 Sprint B5: API Integration Layer

**Agent:** B (API Integration + Configuration)  
**Sprint:** B5  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement API integration layer with REST client, WebSocket client, error handling, and mock API for development.

**Context:** Frontend needs to communicate with Phase 3 backend (Coordinator, Workers, Settings).

**Difficulty:** ⚠️ **MEDIUM** — API client, error handling, mock server, retry logic

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 3 subagents:**

### Subagent 1: REST API Client
**Task:** Implement REST API client for missions, workers, settings
**File:** `apps/desktop/src/api/rest-client.ts`
**Deliverables:**
- API client (missions, workers, settings endpoints)
- Request/response types (TypeScript interfaces)
- Error handling (HTTP errors, API errors)
- Retry logic (exponential backoff)
- 5+ tests

### Subagent 2: WebSocket Client
**Task:** Implement WebSocket client for real-time events
**File:** `apps/desktop/src/api/websocket-client.ts`
**Deliverables:**
- WebSocket connection management
- Event handlers (typed events)
- Reconnection logic (exponential backoff)
- Heartbeat/ping-pong
- 5+ tests

### Subagent 3: Mock API Server
**Task:** Implement mock API for development (before Phase 3 complete)
**File:** `apps/desktop/src/api/mock-api.ts`
**Deliverables:**
- Mock REST endpoints (same interface as real API)
- Mock WebSocket events (simulate progress)
- Toggle between mock/real API
- Configurable delays (simulate network latency)
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 3 Subagents:**
   - Assign tasks to all 3 subagents
   - Review REST + WebSocket + mock API
   - Ensure consistent interface

2. **Integrate Components:**
   - Create unified API module (`apps/desktop/src/api/index.ts`)
   - Export REST client, WebSocket client, mock API
   - Add API configuration (base URL, mock mode)

3. **Implement Type Safety:**
   - Request/response types for all endpoints
   - WebSocket event types
   - Error types (HTTP, API, network)

4. **Write Integration Tests:**
   - Test REST API calls
   - Test WebSocket events
   - Test mock API (same interface as real)
   - Test error handling

5. **Update Documentation:**
   - Add API documentation
   - Add usage examples
   - Add mock API guide

---

## 📐 Technical Spec

### API Configuration

```typescript
// apps/desktop/src/api/config.ts
export interface ApiConfig {
  baseUrl: string;
  wsUrl: string;
  mockMode: boolean;
  timeout: number;
  retryAttempts: number;
  retryDelay: number;
}

export const defaultConfig: ApiConfig = {
  baseUrl: 'http://localhost:3000/api',
  wsUrl: 'ws://localhost:3000/ws',
  mockMode: true, // Default to mock for development
  timeout: 30000, // 30 seconds
  retryAttempts: 3,
  retryDelay: 1000, // 1 second
};

export let apiConfig = { ...defaultConfig };

export function configureApi(config: Partial<ApiConfig>) {
  apiConfig = { ...apiConfig, ...config };
}

export function getApiConfig(): ApiConfig {
  return apiConfig;
}
```

### REST API Client

```typescript
// apps/desktop/src/api/rest-client.ts
import { apiConfig } from './config';

type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

interface RequestOptions {
  method?: HttpMethod;
  body?: any;
  headers?: Record<string, string>;
}

class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

export class RestClient {
  private baseUrl: string;
  
  constructor(baseUrl: string) {
    this.baseUrl = baseUrl;
  }
  
  private async request<T>(endpoint: string, options: RequestOptions = {}): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const { method = 'GET', body, headers = {} } = options;
    
    try {
      const response = await fetch(url, {
        method,
        headers: {
          'Content-Type': 'application/json',
          ...headers,
        },
        body: body ? JSON.stringify(body) : undefined,
      });
      
      if (!response.ok) {
        const error = await response.json().catch(() => ({ message: response.statusText }));
        throw new ApiError(error.message, response.status, error.code);
      }
      
      return response.json();
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }
      throw new ApiError('Network error', 0);
    }
  }
  
  // Mission endpoints
  async submitMission(content: string, attachments?: any[]) {
    return this.request<{ missionId: string; message: string }>('/missions', {
      method: 'POST',
      body: { content, attachments },
    });
  }
  
  async getMission(missionId: string) {
    return this.request<{ missionId: string; progress: number; status: string }>
      (`/missions/${missionId}`);
  }
  
  async listMissions() {
    return this.request<{ missions: any[] }>('/missions');
  }
  
  async cancelMission(missionId: string) {
    return this.request<{ success: boolean }>(`/missions/${missionId}`, {
      method: 'DELETE',
    });
  }
  
  // Worker endpoints
  async listWorkers() {
    return this.request<{ workers: any[] }>('/workers');
  }
  
  async getWorker(workerId: string) {
    return this.request<{ worker: any }>(`/workers/${workerId}`);
  }
  
  // Settings endpoints
  async getSettings() {
    return this.request<any>('/settings');
  }
  
  async updateSettings(settings: any) {
    return this.request<{ success: boolean }>('/settings', {
      method: 'PUT',
      body: settings,
    });
  }
}

export const restClient = new RestClient(apiConfig.baseUrl);
```

### WebSocket Client

```typescript
// apps/desktop/src/api/websocket-client.ts
import { WebSocketEvent } from '../types/progress';

type EventHandler = (event: WebSocketEvent) => void;

export class WebSocketClient {
  private ws: WebSocket | null = null;
  private handlers: Map<string, EventHandler[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private heartbeatInterval: NodeJS.Timeout | null = null;
  
  connect(url: string) {
    this.ws = new WebSocket(url);
    
    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
      this.startHeartbeat();
    };
    
    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.emit(data.type, data.payload);
    };
    
    this.ws.onclose = () => {
      console.log('WebSocket closed');
      this.stopHeartbeat();
      this.attemptReconnect(url);
    };
    
    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }
  
  private attemptReconnect(url: string) {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
      console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);
      setTimeout(() => this.connect(url), delay);
    }
  }
  
  private startHeartbeat() {
    this.heartbeatInterval = setInterval(() => {
      this.ws?.send(JSON.stringify({ type: 'ping' }));
    }, 30000); // 30 seconds
  }
  
  private stopHeartbeat() {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }
  
  on(eventType: string, handler: EventHandler) {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, []);
    }
    this.handlers.get(eventType)!.push(handler);
  }
  
  off(eventType: string, handler: EventHandler) {
    const handlers = this.handlers.get(eventType);
    if (handlers) {
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    }
  }
  
  private emit(eventType: string, payload: any) {
    const handlers = this.handlers.get(eventType);
    if (handlers) {
      handlers.forEach(handler => handler({ type: eventType, payload }));
    }
  }
  
  disconnect() {
    this.stopHeartbeat();
    this.ws?.close();
    this.ws = null;
  }
}

export const wsClient = new WebSocketClient();
```

### Mock API Server

```typescript
// apps/desktop/src/api/mock-api.ts
import { restClient } from './rest-client';
import { wsClient } from './websocket-client';

class MockApi {
  private missions = new Map<string, any>();
  private workers = new Map<string, any>();
  
  enableMockMode() {
    // Override REST client methods
    restClient.submitMission = this.mockSubmitMission.bind(this);
    restClient.getMission = this.mockGetMission.bind(this);
    restClient.listMissions = this.mockListMissions.bind(this);
    restClient.listWorkers = this.mockListWorkers.bind(this);
    
    // Simulate WebSocket events
    this.simulateWebSocketEvents();
  }
  
  private async mockSubmitMission(content: string, attachments?: any[]) {
    const missionId = `mock-${Date.now()}`;
    const mission = {
      missionId,
      content,
      status: 'running',
      progress: 0,
      createdAt: Date.now(),
    };
    this.missions.set(missionId, mission);
    
    // Simulate progress
    this.simulateMissionProgress(missionId);
    
    return {
      missionId,
      message: 'Mission submitted (mock mode)',
    };
  }
  
  private async mockGetMission(missionId: string) {
    const mission = this.missions.get(missionId);
    if (!mission) {
      throw new Error('Mission not found');
    }
    return mission;
  }
  
  private async mockListMissions() {
    return {
      missions: Array.from(this.missions.values()),
    };
  }
  
  private async mockListWorkers() {
    return {
      workers: [
        { workerId: 'worker-1', status: 'idle' },
        { workerId: 'worker-2', status: 'busy' },
        { workerId: 'worker-3', status: 'idle' },
      ],
    };
  }
  
  private simulateMissionProgress(missionId: string) {
    let progress = 0;
    const interval = setInterval(() => {
      progress += Math.random() * 20;
      if (progress >= 100) {
        progress = 100;
        clearInterval(interval);
        this.missions.get(missionId).status = 'completed';
        wsClient.emit('mission:completed', { missionId, result: 'Success (mock)' });
      } else {
        this.missions.get(missionId).progress = progress;
        wsClient.emit('mission:progress', {
          missionId,
          progress,
          eta: Math.max(0, (100 - progress) / 10),
        });
      }
    }, 1000);
  }
  
  private simulateWebSocketEvents() {
    // Simulate worker status updates
    setInterval(() => {
      const workers = ['worker-1', 'worker-2', 'worker-3'];
      const workerId = workers[Math.floor(Math.random() * workers.length)];
      const statuses = ['idle', 'busy', 'idle', 'idle'];
      const status = statuses[Math.floor(Math.random() * statuses.length)];
      
      wsClient.emit('worker:status', {
        workerId,
        status,
        lastHeartbeat: Date.now(),
      });
    }, 5000);
  }
}

export const mockApi = new MockApi();

export function enableMockApi() {
  mockApi.enableMockMode();
}

export function disableMockApi() {
  // Restore real API (would need to save original methods)
  console.log('Mock API disabled - using real API');
}
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 3 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] REST API client works
- [ ] WebSocket client works
- [ ] Mock API works (same interface as real)
- [ ] Error handling works (HTTP, API, network)
- [ ] Retry logic works (exponential backoff)
- [ ] 15+ tests passing (5 per subagent + 5 integration)

---

## 🔗 Dependencies

**Requires:**
- API contract defined (Phase 3)

**Blocks:**
- Sprint C6 (Integration needs API layer)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: REST Client (parallel)
  ├─ Subagent 2: WebSocket Client (parallel)
  └─ Subagent 3: Mock API (parallel)
  ↓
Lead Agent: Integration + Type Safety + Tests
```

**API Design:**
- Same interface for mock and real API
- Type-safe (TypeScript interfaces)
- Error handling (graceful degradation)
- Retry logic (transient failures)

**Difficulty: MEDIUM**
- 3 subagents to coordinate
- Mock/real API switching
- WebSocket reconnection logic
- Type safety across all endpoints

**Review Checklist:**
- [ ] REST API client works (all endpoints)
- [ ] WebSocket client works (connect, events, reconnect)
- [ ] Mock API works (same interface as real)
- [ ] Error handling works (HTTP, API, network)
- [ ] Retry logic works (exponential backoff)
- [ ] Type safety (all requests/responses typed)

---

**Start AFTER API contract is defined (Phase 3).**
