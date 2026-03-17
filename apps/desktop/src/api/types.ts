/**
 * AXORA API Types
 * 
 * Type definitions for all API requests, responses, and WebSocket events.
 */

// ============ Mission Types ============

export type MissionStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

export interface Mission {
  missionId: string;
  content: string;
  status: MissionStatus;
  progress: number; // 0-100
  createdAt: number; // timestamp
  updatedAt?: number; // timestamp
  completedAt?: number; // timestamp
  result?: string;
  error?: string;
}

export interface SubmitMissionRequest {
  content: string;
  attachments?: any[];
}

export interface SubmitMissionResponse {
  missionId: string;
  message: string;
}

export interface GetMissionResponse {
  missionId: string;
  progress: number;
  status: MissionStatus;
  content?: string;
  result?: string;
  error?: string;
}

export interface ListMissionsResponse {
  missions: Mission[];
}

export interface CancelMissionResponse {
  success: boolean;
}

// ============ Worker Types ============

export type WorkerStatus = 'idle' | 'busy' | 'offline' | 'error';

export interface Worker {
  workerId: string;
  status: WorkerStatus;
  currentMissionId?: string;
  lastHeartbeat: number;
  capabilities?: string[];
}

export interface ListWorkersResponse {
  workers: Worker[];
}

export interface GetWorkerResponse {
  worker: Worker;
}

// ============ Settings Types ============

export interface Settings {
  model: {
    provider: 'ollama' | 'openai' | 'anthropic';
    model: string;
    baseUrl?: string;
    apiKey?: string;
  };
  tokens: {
    maxTokensPerRequest: number;
    maxContextTokens: number;
    tokenBudget: number;
  };
  workers: {
    minWorkers: number;
    maxWorkers: number;
    healthCheckInterval: number;
  };
  theme: {
    mode: 'light' | 'dark' | 'system';
    accentColor: string;
  };
  advanced: {
    enableLogging: boolean;
    logLevel: 'debug' | 'info' | 'warn' | 'error';
    autoUpdate: boolean;
  };
}

export type GetSettingsResponse = Settings;

export interface UpdateSettingsRequest {
  settings: Partial<Settings>;
}

export interface UpdateSettingsResponse {
  success: boolean;
}

// ============ WebSocket Event Types ============

export type WebSocketEventType =
  | 'mission:started'
  | 'mission:progress'
  | 'mission:completed'
  | 'mission:failed'
  | 'mission:cancelled'
  | 'worker:status'
  | 'worker:heartbeat'
  | 'ping'
  | 'pong';

export interface WebSocketEvent<T = any> {
  type: WebSocketEventType;
  payload: T;
  timestamp: number;
}

export interface MissionStartedEvent {
  missionId: string;
  status: 'running';
}

export interface MissionProgressEvent {
  missionId: string;
  progress: number; // 0-100
  eta?: number; // estimated time remaining in seconds
  currentStep?: string;
}

export interface MissionCompletedEvent {
  missionId: string;
  result: string;
}

export interface MissionFailedEvent {
  missionId: string;
  error: string;
  code?: string;
}

export interface WorkerStatusEvent {
  workerId: string;
  status: WorkerStatus;
  currentMissionId?: string;
  lastHeartbeat: number;
}

export interface WorkerHeartbeatEvent {
  workerId: string;
  timestamp: number;
  health: 'healthy' | 'degraded' | 'unhealthy';
}

// ============ Error Types ============

export interface ApiErrorResponse {
  message: string;
  code?: string;
  details?: Record<string, string>;
}
