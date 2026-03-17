/**
 * AXORA API Configuration
 * 
 * Configuration for REST and WebSocket API connections.
 * Supports mock mode for development before Phase 3 backend is ready.
 */

export interface ApiConfig {
  /** Base URL for REST API calls */
  baseUrl: string;
  /** WebSocket URL for real-time events */
  wsUrl: string;
  /** Enable mock API mode for development */
  mockMode: boolean;
  /** Request timeout in milliseconds */
  timeout: number;
  /** Number of retry attempts for failed requests */
  retryAttempts: number;
  /** Base delay between retries in milliseconds */
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

export let apiConfig: ApiConfig = { ...defaultConfig };

/**
 * Configure API settings
 * @param config - Partial configuration to merge with defaults
 */
export function configureApi(config: Partial<ApiConfig>): void {
  apiConfig = { ...apiConfig, ...config };
}

/**
 * Get current API configuration
 * @returns Current API configuration
 */
export function getApiConfig(): ApiConfig {
  return apiConfig;
}

/**
 * Enable mock API mode
 */
export function enableMockMode(): void {
  apiConfig.mockMode = true;
}

/**
 * Disable mock API mode (use real API)
 */
export function disableMockMode(): void {
  apiConfig.mockMode = false;
}
