# Phase 4 Sprint A5: Progress Dashboard

**Agent:** A (UI Components + Progress Display)  
**Sprint:** A5  
**Priority:** HIGH  
**Estimated:** 8 hours  
**Subagents:** ENABLED (GPT-5.4)

---

## 🎯 Mission

Implement real-time progress dashboard with ETA, blocker alerts, and worker status.

**Context:** Users need to see mission progress in real-time (not just chat responses).

**Difficulty:** ⚠️ **MEDIUM-HIGH** — Real-time updates, WebSocket, progress visualization

---

## 📋 Subagents Assignment

**You are Lead Agent. Delegate to 2 subagents:**

### Subagent 1: Progress Visualization
**Task:** Implement progress bars, charts, status indicators
**File:** `apps/desktop/src/panels/ProgressPanel.tsx`
**Deliverables:**
- Progress panel layout
- Progress bars (per mission, overall)
- ETA display (countdown/up)
- Status indicators (pending, running, completed, failed)
- 5+ tests

### Subagent 2: WebSocket + Real-Time Updates
**Task:** Implement WebSocket integration for real-time progress
**File:** `apps/desktop/src/api/progress-websocket.ts`
**Deliverables:**
- WebSocket client (connect, disconnect, reconnect)
- Event handlers (mission:started, mission:progress, mission:completed)
- Progress store (real-time state updates)
- Blocker alerts (notifications)
- 5+ tests

---

## 🏗️ Lead Agent Responsibilities

**You must:**

1. **Coordinate 2 Subagents:**
   - Assign tasks to both subagents
   - Review progress UI + WebSocket integration
   - Ensure real-time updates work

2. **Integrate Components:**
   - Connect Progress Panel to WebSocket store
   - Implement worker status panel
   - Add mission list (active, completed, failed)

3. **Implement Progress Store:**
   - Active missions (with progress %)
   - Completed missions (history)
   - Worker status (idle, busy, failed)
   - Blocker alerts

4. **Write Integration Tests:**
   - Test progress updates (real-time)
   - Test WebSocket reconnection
   - Test blocker alerts
   - Test worker status display

5. **Update Documentation:**
   - Add progress dashboard documentation
   - Add WebSocket event documentation

---

## 📐 Technical Spec

### WebSocket Events

```typescript
// apps/desktop/src/types/progress.ts
export interface MissionProgressEvent {
  missionId: string;
  progress: number;  // 0-100
  eta?: number;      // Seconds remaining
  status: 'pending' | 'running' | 'completed' | 'failed';
  message?: string;
}

export interface WorkerStatusEvent {
  workerId: string;
  status: 'idle' | 'busy' | 'unhealthy' | 'failed';
  currentTask?: string;
  lastHeartbeat: number;
}

export interface BlockerAlert {
  id: string;
  missionId: string;
  reason: string;
  stalledSince: number;
  severity: 'warning' | 'error';
}

// WebSocket event types
export type WebSocketEvent =
  | { type: 'mission:started'; payload: { missionId: string } }
  | { type: 'mission:progress'; payload: MissionProgressEvent }
  | { type: 'mission:completed'; payload: { missionId: string; result: string } }
  | { type: 'mission:failed'; payload: { missionId: string; error: string } }
  | { type: 'worker:status'; payload: WorkerStatusEvent }
  | { type: 'blocker:alert'; payload: BlockerAlert };
```

### WebSocket Client

```typescript
// apps/desktop/src/api/progress-websocket.ts
import { WebSocketEvent, MissionProgress, WorkerStatus } from '../types/progress';

type EventHandler = (event: WebSocketEvent) => void;

export class ProgressWebSocket {
  private ws: WebSocket | null = null;
  private handlers: EventHandler[] = [];
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  
  connect(url: string) {
    this.ws = new WebSocket(url);
    
    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
    };
    
    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.handlers.forEach(handler => handler(data));
    };
    
    this.ws.onclose = () => {
      console.log('WebSocket closed');
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
  
  onEvent(handler: EventHandler) {
    this.handlers.push(handler);
  }
  
  disconnect() {
    this.ws?.close();
    this.ws = null;
  }
}

export const progressWS = new ProgressWebSocket();
```

### Progress Store

```typescript
// apps/desktop/src/store/progress-store.ts
import { create } from 'zustand';
import { MissionProgress, WorkerStatus, BlockerAlert } from '../types/progress';
import { progressWS } from '../api/progress-websocket';

interface ProgressStore {
  activeMissions: Map<string, MissionProgress>;
  completedMissions: MissionProgress[];
  workers: Map<string, WorkerStatus>;
  blockers: BlockerAlert[];
  
  // Actions
  connectWebSocket: (url: string) => void;
  disconnectWebSocket: () => void;
  clearCompleted: () => void;
}

export const useProgressStore = create<ProgressStore>((set, get) => ({
  activeMissions: new Map(),
  completedMissions: [],
  workers: new Map(),
  blockers: [],
  
  connectWebSocket: (url) => {
    progressWS.connect(url);
    
    progressWS.onEvent((event) => {
      switch (event.type) {
        case 'mission:started':
          set((state) => ({
            activeMissions: new Map(state.activeMissions).set(event.payload.missionId, {
              missionId: event.payload.missionId,
              progress: 0,
              status: 'running',
            }),
          }));
          break;
          
        case 'mission:progress':
          set((state) => {
            const missions = new Map(state.activeMissions);
            missions.set(event.payload.missionId, event.payload);
            return { activeMissions: missions };
          });
          break;
          
        case 'mission:completed':
          set((state) => {
            const missions = new Map(state.activeMissions);
            const mission = missions.get(event.payload.missionId);
            if (mission) {
              missions.delete(event.payload.missionId);
              return {
                activeMissions: missions,
                completedMissions: [...state.completedMissions, mission],
              };
            }
            return state;
          });
          break;
          
        case 'worker:status':
          set((state) => ({
            workers: new Map(state.workers).set(event.payload.workerId, event.payload),
          }));
          break;
          
        case 'blocker:alert':
          set((state) => ({
            blockers: [...state.blockers, event.payload],
          }));
          break;
      }
    });
  },
  
  disconnectWebSocket: () => {
    progressWS.disconnect();
  },
  
  clearCompleted: () => {
    set({ completedMissions: [] });
  },
}));
```

### Progress Panel UI

```tsx
// apps/desktop/src/panels/ProgressPanel.tsx
import { useProgressStore } from '../store/progress-store';

export function ProgressPanel() {
  const { activeMissions, completedMissions, workers, blockers } = useProgressStore();
  
  return (
    <div className="progress-panel">
      <header className="panel-header">
        <h2>Progress</h2>
      </header>
      
      <div className="progress-content">
        {/* Active Missions */}
        <section className="active-missions">
          <h3>Active Missions</h3>
          {Array.from(activeMissions.values()).map((mission) => (
            <div key={mission.missionId} className="mission-progress">
              <div className="mission-header">
                <span className="mission-id">{mission.missionId}</span>
                <span className="mission-status">{mission.status}</span>
              </div>
              <progress value={mission.progress} max="100" />
              <div className="mission-details">
                <span>{mission.progress}%</span>
                {mission.eta && <span>ETA: {formatDuration(mission.eta)}</span>}
              </div>
            </div>
          ))}
        </section>
        
        {/* Worker Status */}
        <section className="worker-status">
          <h3>Workers</h3>
          {Array.from(workers.values()).map((worker) => (
            <div key={worker.workerId} className={`worker ${worker.status}`}>
              <span>{worker.workerId}</span>
              <span>{worker.status}</span>
            </div>
          ))}
        </section>
        
        {/* Blocker Alerts */}
        {blockers.length > 0 && (
          <section className="blocker-alerts">
            <h3>Blockers</h3>
            {blockers.map((blocker) => (
              <div key={blocker.id} className={`alert alert-${blocker.severity}`}>
                <span>{blocker.reason}</span>
              </div>
            ))}
          </section>
        )}
        
        {/* Completed Missions */}
        <section className="completed-missions">
          <h3>Completed</h3>
          {completedMissions.map((mission) => (
            <div key={mission.missionId} className="mission-completed">
              <span>{mission.missionId}</span>
              <span>✓</span>
            </div>
          ))}
        </section>
      </div>
    </div>
  );
}
```

---

## ✅ Success Criteria

**Sprint is done when:**

- [ ] 2 subagents complete their tasks
- [ ] Lead agent integrates all components
- [ ] Progress panel compiles and works
- [ ] Real-time updates work (WebSocket)
- [ ] Progress bars display correctly
- [ ] ETA calculation works
- [ ] Blocker alerts display
- [ ] 10+ tests passing (5 per subagent + 5 integration)

---

## 🔗 Dependencies

**Requires:**
- Sprint A4 complete (UI Components)

**Blocks:**
- Sprint C6 (Integration needs progress dashboard)

---

## 📝 Notes for GPT-5.4

**Subagent Pattern:**
```
Lead Agent:
  ├─ Subagent 1: Progress Visualization (parallel)
  └─ Subagent 2: WebSocket Integration (parallel)
  ↓
Lead Agent: Integration + Progress Store + Tests
```

**Real-Time Design:**
- WebSocket for instant updates (<500ms lag)
- Reconnection logic (exponential backoff)
- Optimistic UI (update before confirmation)

**Difficulty: MEDIUM-HIGH**
- 2 subagents to coordinate
- WebSocket complexity (reconnection, events)
- Real-time state management
- Progress visualization (accurate, not misleading)

**Review Checklist:**
- [ ] Progress panel renders correctly
- [ ] WebSocket connects and receives events
- [ ] Real-time updates work (progress bars update)
- [ ] Reconnection works (after disconnect)
- [ ] Blocker alerts display correctly
- [ ] Worker status updates in real-time

---

**Start AFTER Sprint A4 complete.**
