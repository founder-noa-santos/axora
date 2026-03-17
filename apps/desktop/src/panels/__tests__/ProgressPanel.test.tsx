import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { ProgressPanel } from '../ProgressPanel';
import { useProgressStore } from '../../store/progress-store';

// Mock the store
vi.mock('../../store/progress-store', () => ({
  useProgressStore: vi.fn(),
}));

describe('ProgressPanel', () => {
  beforeEach(() => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });
  });

  it('should render ProgressPanel header', () => {
    render(<ProgressPanel />);

    expect(screen.getByText('Progress Dashboard')).toBeInTheDocument();
  });

  it('should show connected status', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('Connected')).toBeInTheDocument();
  });

  it('should show disconnected status', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: false,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('Disconnected')).toBeInTheDocument();
  });

  it('should show empty state for active missions', () => {
    render(<ProgressPanel />);

    expect(screen.getByText('No active missions')).toBeInTheDocument();
  });

  it('should show active missions', () => {
    const activeMissions = new Map();
    activeMissions.set('mission-1', {
      missionId: 'mission-1',
      progress: 50,
      status: 'running' as const,
      eta: 300,
      message: 'Processing...',
    });

    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions,
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 1,
        activeMissions: 1,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 50,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('mission-1')).toBeInTheDocument();
    expect(screen.getByText('50%')).toBeInTheDocument();
    expect(screen.getByText('ETA: 5m 0s')).toBeInTheDocument();
    expect(screen.getByText('Processing...')).toBeInTheDocument();
  });

  it('should show empty state for workers', () => {
    render(<ProgressPanel />);

    expect(screen.getByText('No workers available')).toBeInTheDocument();
  });

  it('should show workers with status', () => {
    const workers = new Map();
    workers.set('worker-1', {
      workerId: 'worker-1',
      status: 'busy' as const,
      currentTask: 'Processing mission-1',
      lastHeartbeat: Date.now(),
      healthScore: 95,
    });

    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers,
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('worker-1')).toBeInTheDocument();
    expect(screen.getByText('busy')).toBeInTheDocument();
    expect(screen.getByText('Processing mission-1')).toBeInTheDocument();
  });

  it('should show empty state for completed missions', () => {
    render(<ProgressPanel />);

    expect(screen.getByText('No completed missions')).toBeInTheDocument();
  });

  it('should show completed missions', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [
        {
          missionId: 'mission-1',
          progress: 100,
          status: 'completed' as const,
          completedAt: Date.now(),
        },
        {
          missionId: 'mission-2',
          progress: 100,
          status: 'completed' as const,
          completedAt: Date.now(),
        },
      ],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 2,
        activeMissions: 0,
        completedMissions: 2,
        failedMissions: 0,
        overallProgress: 100,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('mission-1')).toBeInTheDocument();
    expect(screen.getByText('mission-2')).toBeInTheDocument();
  });

  it('should show empty state for failed missions', () => {
    render(<ProgressPanel />);

    expect(screen.getByText('No failed missions')).toBeInTheDocument();
  });

  it('should show failed missions', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [
        {
          missionId: 'mission-1',
          progress: 25,
          status: 'failed' as const,
          message: 'Connection timeout',
          failedAt: Date.now(),
        },
      ],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 1,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 1,
        overallProgress: 25,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('mission-1')).toBeInTheDocument();
    expect(screen.getByText('Connection timeout')).toBeInTheDocument();
  });

  it('should show blocker alerts', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [
        {
          id: 'blocker-1',
          missionId: 'mission-1',
          reason: 'Task stalled for 5 minutes',
          stalledSince: Date.now(),
          severity: 'warning',
        },
        {
          id: 'blocker-2',
          missionId: 'mission-2',
          reason: 'Worker unresponsive',
          stalledSince: Date.now(),
          severity: 'error',
        },
      ],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('Blockers (2)')).toBeInTheDocument();
    expect(screen.getByText('Task stalled for 5 minutes')).toBeInTheDocument();
    expect(screen.getByText('Worker unresponsive')).toBeInTheDocument();
  });

  it('should not show blockers section when all resolved', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [
        {
          id: 'blocker-1',
          missionId: 'mission-1',
          reason: 'Task stalled',
          stalledSince: Date.now(),
          severity: 'warning',
          resolved: true,
          resolvedAt: Date.now(),
        },
      ],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 0,
        activeMissions: 0,
        completedMissions: 0,
        failedMissions: 0,
        overallProgress: 0,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.queryByText('Blockers')).not.toBeInTheDocument();
  });

  it('should show overall progress', () => {
    vi.mocked(useProgressStore).mockReturnValue({
      activeMissions: new Map(),
      completedMissions: [],
      failedMissions: [],
      workers: new Map(),
      blockers: [],
      isConnected: true,
      connectWebSocket: vi.fn(),
      disconnectWebSocket: vi.fn(),
      clearCompleted: vi.fn(),
      clearBlocker: vi.fn(),
      getProgressSummary: vi.fn().mockReturnValue({
        totalMissions: 5,
        activeMissions: 2,
        completedMissions: 2,
        failedMissions: 1,
        overallProgress: 60,
      }),
    });

    render(<ProgressPanel />);

    expect(screen.getByText('2 active')).toBeInTheDocument();
    expect(screen.getByText('2 completed')).toBeInTheDocument();
    expect(screen.getByText('1 failed')).toBeInTheDocument();
  });
});
