/**
 * Progress Dashboard Types
 * 
 * Real-time progress tracking for missions and workers.
 */

/**
 * Mission progress status
 */
export type MissionStatus = 'pending' | 'running' | 'completed' | 'failed';

/**
 * Worker status
 */
export type WorkerStatusType = 'idle' | 'busy' | 'unhealthy' | 'failed';

/**
 * Blocker alert severity
 */
export type BlockerSeverity = 'warning' | 'error';

/**
 * Mission progress event
 */
export interface MissionProgressEvent {
  missionId: string;
  progress: number;  // 0-100
  eta?: number;      // Seconds remaining
  status: MissionStatus;
  message?: string;
  startedAt?: number;
  completedAt?: number;
}

/**
 * Worker status event
 */
export interface WorkerStatusEvent {
  workerId: string;
  status: WorkerStatusType;
  currentTask?: string;
  lastHeartbeat: number;
  healthScore?: number;  // 0-100
}

/**
 * Blocker alert
 */
export interface BlockerAlert {
  id: string;
  missionId: string;
  reason: string;
  stalledSince: number;
  severity: BlockerSeverity;
  resolved?: boolean;
  resolvedAt?: number;
}

/**
 * WebSocket event types
 */
export type WebSocketEvent =
  | { type: 'mission:started'; payload: { missionId: string; message?: string } }
  | { type: 'mission:progress'; payload: MissionProgressEvent }
  | { type: 'mission:completed'; payload: { missionId: string; result: string; completedAt: number } }
  | { type: 'mission:failed'; payload: { missionId: string; error: string; failedAt: number } }
  | { type: 'worker:status'; payload: WorkerStatusEvent }
  | { type: 'blocker:alert'; payload: BlockerAlert }
  | { type: 'blocker:resolved'; payload: { blockerId: string; resolvedAt: number } };

/**
 * Progress summary for display
 */
export interface ProgressSummary {
  totalMissions: number;
  activeMissions: number;
  completedMissions: number;
  failedMissions: number;
  overallProgress: number;  // 0-100
}

/**
 * Format duration from seconds to human-readable string
 */
export function formatDuration(seconds: number): string {
  if (seconds < 0) return '—';
  
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  
  if (minutes === 0) {
    return `${remainingSeconds}s`;
  }
  
  const hours = Math.floor(minutes / 60);
  const remainingMinutes = minutes % 60;
  
  if (hours === 0) {
    return `${minutes}m ${remainingSeconds}s`;
  }
  
  return `${hours}h ${remainingMinutes}m`;
}

/**
 * Calculate ETA from progress and elapsed time
 */
export function calculateETA(progress: number, elapsedSeconds: number): number | undefined {
  if (progress <= 0 || progress >= 100) return undefined;
  if (elapsedSeconds <= 0) return undefined;
  
  const rate = progress / elapsedSeconds;
  if (rate <= 0) return undefined;
  
  const totalEstimated = elapsedSeconds / rate;
  const remaining = totalEstimated - elapsedSeconds;
  
  return Math.max(0, Math.floor(remaining));
}
