import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useProgressStore } from '../progress-store';

// Mock the WebSocket module
vi.mock('../../api/progress-websocket', () => {
  return {
    progressWS: {
      connect: vi.fn(),
      disconnect: vi.fn(),
      onEvent: vi.fn(),
      offEvent: vi.fn(),
      isConnected: vi.fn().mockReturnValue(false),
    },
  };
});

describe('ProgressStore', () => {
  beforeEach(() => {
    useProgressStore.setState({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: false,
    });
    vi.clearAllMocks();
  });

  it('should initialize with empty state', () => {
    const state = useProgressStore.getState();

    expect(state.activeMissions.size).toBe(0);
    expect(state.completedMissions.length).toBe(0);
    expect(state.failedMissions.length).toBe(0);
    expect(state.workers.size).toBe(0);
    expect(state.blockers.length).toBe(0);
    expect(state.isConnected).toBe(false);
  });

  it('should connect WebSocket', () => {
    const state = useProgressStore.getState();
    state.connectWebSocket('ws://localhost:8080');

    // The store sets isConnected to true when connecting
    expect(useProgressStore.getState().isConnected).toBe(true);
  });

  it('should clear completed missions', () => {
    useProgressStore.setState({
      completedMissions: [
        {
          missionId: 'mission-1',
          progress: 100,
          status: 'completed' as const,
          completedAt: Date.now(),
        },
      ],
      failedMissions: [
        {
          missionId: 'mission-2',
          progress: 50,
          status: 'failed' as const,
          message: 'Error',
        },
      ],
    });

    const state = useProgressStore.getState();
    state.clearCompleted();

    const updatedState = useProgressStore.getState();
    expect(updatedState.completedMissions.length).toBe(0);
    expect(updatedState.failedMissions.length).toBe(0);
  });

  it('should clear individual blocker', () => {
    useProgressStore.setState({
      blockers: [
        {
          id: 'blocker-1',
          missionId: 'mission-1',
          reason: 'Stalled',
          stalledSince: Date.now(),
          severity: 'warning',
        },
        {
          id: 'blocker-2',
          missionId: 'mission-2',
          reason: 'Failed',
          stalledSince: Date.now(),
          severity: 'error',
        },
      ],
    });

    const state = useProgressStore.getState();
    state.clearBlocker('blocker-1');

    const updatedState = useProgressStore.getState();
    expect(updatedState.blockers.length).toBe(1);
    expect(updatedState.blockers[0].id).toBe('blocker-2');
  });

  it('should calculate progress summary correctly', () => {
    const activeMissions = new Map();
    activeMissions.set('mission-1', {
      missionId: 'mission-1',
      progress: 50,
      status: 'running' as const,
    });

    useProgressStore.setState({
      activeMissions,
      completedMissions: [
        {
          missionId: 'mission-2',
          progress: 100,
          status: 'completed' as const,
          completedAt: Date.now(),
        },
      ],
      failedMissions: [],
    });

    const state = useProgressStore.getState();
    const summary = state.getProgressSummary();

    expect(summary.totalMissions).toBe(2);
    expect(summary.activeMissions).toBe(1);
    expect(summary.completedMissions).toBe(1);
    expect(summary.failedMissions).toBe(0);
    expect(summary.overallProgress).toBe(75);
  });

  it('should handle empty progress summary', () => {
    const state = useProgressStore.getState();
    const summary = state.getProgressSummary();

    expect(summary.totalMissions).toBe(0);
    expect(summary.activeMissions).toBe(0);
    expect(summary.completedMissions).toBe(0);
    expect(summary.failedMissions).toBe(0);
    expect(summary.overallProgress).toBe(0);
  });
});
