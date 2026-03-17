import { create } from 'zustand';
import {
  MissionProgressEvent,
  WorkerStatusEvent,
  BlockerAlert,
  WebSocketEvent,
} from '../types/progress';
import { progressWS } from '../api/progress-websocket';

interface ProgressStore {
  // State
  activeMissions: Map<string, MissionProgressEvent>;
  completedMissions: MissionProgressEvent[];
  failedMissions: MissionProgressEvent[];
  workers: Map<string, WorkerStatusEvent>;
  blockers: BlockerAlert[];
  isConnected: boolean;

  // Actions
  connectWebSocket: (url: string) => void;
  disconnectWebSocket: () => void;
  clearCompleted: () => void;
  clearBlocker: (blockerId: string) => void;
  getProgressSummary: () => {
    totalMissions: number;
    activeMissions: number;
    completedMissions: number;
    failedMissions: number;
    overallProgress: number;
  };
}

export const useProgressStore = create<ProgressStore>((set, get) => ({
  // Initial state
  activeMissions: new Map(),
  completedMissions: [],
  failedMissions: [],
  workers: new Map(),
  blockers: [],
  isConnected: false,

  connectWebSocket: (url: string) => {
    const handler = (event: WebSocketEvent) => {
      switch (event.type) {
        case 'mission:started': {
          set((state) => {
            const missions = new Map(state.activeMissions);
            missions.set(event.payload.missionId, {
              missionId: event.payload.missionId,
              progress: 0,
              status: 'running',
              message: event.payload.message,
              startedAt: Date.now(),
            });
            return { activeMissions: missions };
          });
          break;
        }

        case 'mission:progress': {
          set((state) => {
            const missions = new Map(state.activeMissions);
            missions.set(event.payload.missionId, event.payload);
            return { activeMissions: missions };
          });
          break;
        }

        case 'mission:completed': {
          set((state) => {
            const missions = new Map(state.activeMissions);
            const mission = missions.get(event.payload.missionId);
            if (mission) {
              missions.delete(event.payload.missionId);
              const completedMission: MissionProgressEvent = {
                ...mission,
                ...event.payload,
                status: 'completed',
                progress: 100,
                completedAt: event.payload.completedAt,
              };
              return {
                activeMissions: missions,
                completedMissions: [...state.completedMissions, completedMission],
              };
            }
            return state;
          });
          break;
        }

        case 'mission:failed': {
          set((state) => {
            const missions = new Map(state.activeMissions);
            const mission = missions.get(event.payload.missionId);
            if (mission) {
              missions.delete(event.payload.missionId);
              const failedMission: MissionProgressEvent = {
                ...mission,
                ...event.payload,
                status: 'failed',
                message: event.payload.error,
                completedAt: event.payload.failedAt,
              };
              return {
                activeMissions: missions,
                failedMissions: [...state.failedMissions, failedMission],
              };
            }
            return state;
          });
          break;
        }

        case 'worker:status': {
          set((state) => ({
            workers: new Map(state.workers).set(
              event.payload.workerId,
              event.payload
            ),
          }));
          break;
        }

        case 'blocker:alert': {
          set((state) => ({
            blockers: [...state.blockers, event.payload],
          }));
          break;
        }

        case 'blocker:resolved': {
          set((state) => ({
            blockers: state.blockers.map((b) =>
              b.id === event.payload.blockerId
                ? { ...b, resolved: true, resolvedAt: event.payload.resolvedAt }
                : b
            ),
          }));
          break;
        }
      }
    };

    progressWS.onEvent(handler);
    progressWS.connect(url);

    set({ isConnected: true });

    // Store handler for cleanup
    (get() as unknown as { _handler?: typeof handler })._handler = handler;
  },

  disconnectWebSocket: () => {
    const state = get() as unknown as { _handler?: (event: WebSocketEvent) => void };
    if (state._handler) {
      progressWS.offEvent(state._handler);
    }
    progressWS.disconnect();
    set({ isConnected: false });
  },

  clearCompleted: () => {
    set({ completedMissions: [], failedMissions: [] });
  },

  clearBlocker: (blockerId: string) => {
    set((state) => ({
      blockers: state.blockers.filter((b) => b.id !== blockerId),
    }));
  },

  getProgressSummary: () => {
    const state = get();
    const total =
      state.activeMissions.size +
      state.completedMissions.length +
      state.failedMissions.length;

    let totalProgress = 0;
    state.activeMissions.forEach((mission) => {
      totalProgress += mission.progress;
    });
    totalProgress += state.completedMissions.length * 100;

    const overallProgress = total > 0 ? Math.round(totalProgress / total) : 0;

    return {
      totalMissions: total,
      activeMissions: state.activeMissions.size,
      completedMissions: state.completedMissions.length,
      failedMissions: state.failedMissions.length,
      overallProgress,
    };
  },
}));
