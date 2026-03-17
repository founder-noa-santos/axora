/**
 * AXORA API Module
 *
 * Unified API layer for REST and WebSocket communication.
 * Supports mock mode for development.
 *
 * @module api
 */

// Configuration
export {
  apiConfig,
  defaultConfig,
  configureApi,
  getApiConfig,
  enableMockMode,
  disableMockMode,
} from './config';

export type { ApiConfig } from './config';

// REST Client
export {
  RestClient,
  restClient,
  createRestClient,
  ApiError,
} from './rest-client';

// WebSocket Client
export {
  WebSocketClient,
  wsClient,
  createWebSocketClient,
} from './websocket-client';

// Mock API
export {
  MockApi,
  mockApi,
  enableMockApi,
  disableMockApi,
  createMockApi,
} from './mock-api';

// Types
export type {
  // Mission types
  Mission,
  MissionStatus,
  SubmitMissionRequest,
  SubmitMissionResponse,
  GetMissionResponse,
  ListMissionsResponse,
  CancelMissionResponse,
  // Worker types
  Worker,
  WorkerStatus,
  ListWorkersResponse,
  GetWorkerResponse,
  // Settings types
  Settings,
  GetSettingsResponse,
  UpdateSettingsRequest,
  UpdateSettingsResponse,
  // WebSocket types
  WebSocketEvent,
  WebSocketEventType,
  MissionStartedEvent,
  MissionProgressEvent,
  MissionCompletedEvent,
  MissionFailedEvent,
  WorkerStatusEvent,
  WorkerHeartbeatEvent,
  // Error types
  ApiErrorResponse,
} from './types';

// Internal imports for functions
import { apiConfig, configureApi, enableMockMode } from './config';
import { wsClient } from './websocket-client';
import type { ApiConfig } from './config';

/**
 * Initialize API with configuration
 * @param config - API configuration
 * @param autoConnect - Whether to automatically connect WebSocket
 *
 * @example
 * ```typescript
 * import { initializeApi, enableMockApi } from './api';
 *
 * // Initialize with mock mode
 * initializeApi({ mockMode: true });
 * enableMockApi();
 *
 * // Or initialize with real API
 * initializeApi({
 *   baseUrl: 'http://localhost:3000/api',
 *   wsUrl: 'ws://localhost:3000/ws',
 *   mockMode: false,
 * });
 * ```
 */
export function initializeApi(
  config?: Partial<ApiConfig>,
  autoConnect: boolean = false
): void {
  if (config) {
    configureApi(config);
  }

  if (config?.mockMode) {
    enableMockMode();
  }

  if (autoConnect && !config?.mockMode) {
    wsClient.connect();
  }
}

/**
 * Connect to WebSocket server
 * @param url - WebSocket URL (uses config if not provided)
 */
export function connectWebSocket(url?: string): void {
  wsClient.connect(url);
}

/**
 * Disconnect from WebSocket server
 */
export function disconnectWebSocket(): void {
  wsClient.disconnect();
}

/**
 * Get API status
 * @returns Current API status
 */
export function getApiStatus(): {
  mockMode: boolean;
  wsConnected: boolean;
  baseUrl: string;
  wsUrl: string;
} {
  return {
    mockMode: apiConfig.mockMode,
    wsConnected: wsClient.isConnected(),
    baseUrl: apiConfig.baseUrl,
    wsUrl: apiConfig.wsUrl,
  };
}
