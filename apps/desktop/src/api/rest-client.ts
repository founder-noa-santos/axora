/**
 * AXORA REST API Client
 * 
 * Type-safe REST client for missions, workers, and settings endpoints.
 * Includes error handling and exponential backoff retry logic.
 */

import { apiConfig, getApiConfig } from './config';
import type {
  SubmitMissionRequest,
  SubmitMissionResponse,
  GetMissionResponse,
  ListMissionsResponse,
  CancelMissionResponse,
  ListWorkersResponse,
  GetWorkerResponse,
  GetSettingsResponse,
  UpdateSettingsRequest,
  UpdateSettingsResponse,
  ApiErrorResponse,
} from './types';

/**
 * HTTP methods supported by the REST client
 */
type HttpMethod = 'GET' | 'POST' | 'PUT' | 'DELETE';

/**
 * Options for REST API requests
 */
interface RequestOptions {
  method?: HttpMethod;
  body?: any;
  headers?: Record<string, string>;
  skipRetry?: boolean;
}

/**
 * API Error class for handling HTTP and API errors
 */
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public code?: string,
    public details?: Record<string, string>
  ) {
    super(message);
    this.name = 'ApiError';
  }

  /**
   * Check if error is a network error (status 0)
   */
  isNetworkError(): boolean {
    return this.status === 0;
  }

  /**
   * Check if error is a client error (4xx)
   */
  isClientError(): boolean {
    return this.status >= 400 && this.status < 500;
  }

  /**
   * Check if error is a server error (5xx)
   */
  isServerError(): boolean {
    return this.status >= 500 && this.status < 600;
  }

  /**
   * Check if error is retryable
   */
  isRetryable(): boolean {
    // Network errors and 5xx errors are retryable
    return this.isNetworkError() || this.isServerError();
  }
}

/**
 * Calculate retry delay with exponential backoff and jitter
 * @param attempt - Current retry attempt (0-based)
 * @param baseDelay - Base delay in milliseconds
 * @returns Delay in milliseconds
 */
function calculateRetryDelay(attempt: number, baseDelay: number): number {
  // Exponential backoff: baseDelay * 2^attempt
  const exponentialDelay = baseDelay * Math.pow(2, attempt);
  // Add jitter (±25% randomness)
  const jitter = exponentialDelay * 0.25 * (Math.random() * 2 - 1);
  return Math.max(0, exponentialDelay + jitter);
}

/**
 * Sleep for specified milliseconds
 * @param ms - Milliseconds to sleep
 */
function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

/**
 * REST API Client for AXORA backend
 */
export class RestClient {
  private baseUrl: string;
  private defaultHeaders: Record<string, string>;

  constructor(baseUrl: string, headers?: Record<string, string>) {
    this.baseUrl = baseUrl;
    this.defaultHeaders = headers || {};
  }

  /**
   * Make a REST API request with error handling and retry logic
   * @param endpoint - API endpoint (without base URL)
   * @param options - Request options
   * @returns Promise resolving to response data
   */
  private async request<T>(endpoint: string, options: RequestOptions = {}): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const { method = 'GET', body, headers = {}, skipRetry = false } = options;

    const mergedHeaders: Record<string, string> = {
      'Content-Type': 'application/json',
      ...this.defaultHeaders,
      ...headers,
    };

    let lastError: ApiError | null = null;
    const maxAttempts = skipRetry ? 1 : apiConfig.retryAttempts + 1;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), apiConfig.timeout);

        const response = await fetch(url, {
          method,
          headers: mergedHeaders,
          body: body ? JSON.stringify(body) : undefined,
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        // Parse response body
        const responseData = await response.json().catch(
          (): ApiErrorResponse => ({
            message: response.statusText || 'Unknown error',
            code: `HTTP_${response.status}`,
          })
        );

        // Handle non-OK responses
        if (!response.ok) {
          throw new ApiError(
            responseData.message || 'Request failed',
            response.status,
            responseData.code,
            responseData.details
          );
        }

        return responseData as T;
      } catch (error) {
        // Handle timeout errors
        if (error instanceof Error && error.name === 'AbortError') {
          throw new ApiError('Request timeout', 0, 'TIMEOUT');
        }

        // Handle API errors
        if (error instanceof ApiError) {
          lastError = error;

          // Don't retry client errors (4xx)
          if (!error.isRetryable() || skipRetry) {
            throw error;
          }

          // Check if we have retries remaining
          if (attempt < maxAttempts - 1) {
            const delay = calculateRetryDelay(attempt, apiConfig.retryDelay);
            console.warn(
              `API request failed (attempt ${attempt + 1}/${maxAttempts}), retrying in ${Math.round(delay)}ms...`
            );
            await sleep(delay);
            continue;
          }
        } else {
          // Handle network errors
          lastError = new ApiError('Network error', 0, 'NETWORK_ERROR');
          
          if (attempt < maxAttempts - 1 && !skipRetry) {
            const delay = calculateRetryDelay(attempt, apiConfig.retryDelay);
            console.warn(
              `Network error (attempt ${attempt + 1}/${maxAttempts}), retrying in ${Math.round(delay)}ms...`
            );
            await sleep(delay);
            continue;
          }
        }

        // All retries exhausted
        if (lastError) {
          throw lastError;
        }
        throw new ApiError('Unknown error', 0);
      }
    }

    throw new ApiError('Unexpected error', 0);
  }

  // ============ Mission Endpoints ============

  /**
   * Submit a new mission
   * @param content - Mission content/description
   * @param attachments - Optional attachments
   * @returns Mission ID and confirmation message
   */
  async submitMission(
    content: string,
    attachments?: any[]
  ): Promise<SubmitMissionResponse> {
    const request: SubmitMissionRequest = { content, attachments };
    return this.request<SubmitMissionResponse>('/missions', {
      method: 'POST',
      body: request,
    });
  }

  /**
   * Get mission status and details
   * @param missionId - Mission ID
   * @returns Mission details including progress and status
   */
  async getMission(missionId: string): Promise<GetMissionResponse> {
    return this.request<GetMissionResponse>(`/missions/${missionId}`);
  }

  /**
   * List all missions
   * @returns List of missions
   */
  async listMissions(): Promise<ListMissionsResponse> {
    return this.request<ListMissionsResponse>('/missions');
  }

  /**
   * Cancel a running mission
   * @param missionId - Mission ID
   * @returns Confirmation of cancellation
   */
  async cancelMission(missionId: string): Promise<CancelMissionResponse> {
    return this.request<CancelMissionResponse>(`/missions/${missionId}`, {
      method: 'DELETE',
    });
  }

  // ============ Worker Endpoints ============

  /**
   * List all workers
   * @returns List of workers
   */
  async listWorkers(): Promise<ListWorkersResponse> {
    return this.request<ListWorkersResponse>('/workers');
  }

  /**
   * Get worker details
   * @param workerId - Worker ID
   * @returns Worker details
   */
  async getWorker(workerId: string): Promise<GetWorkerResponse> {
    return this.request<GetWorkerResponse>(`/workers/${workerId}`);
  }

  // ============ Settings Endpoints ============

  /**
   * Get current settings
   * @returns Application settings
   */
  async getSettings(): Promise<GetSettingsResponse> {
    return this.request<GetSettingsResponse>('/settings');
  }

  /**
   * Update settings
   * @param settings - Settings to update
   * @returns Confirmation of update
   */
  async updateSettings(
    settings: Partial<GetSettingsResponse>
  ): Promise<UpdateSettingsResponse> {
    const request: UpdateSettingsRequest = { settings };
    return this.request<UpdateSettingsResponse>('/settings', {
      method: 'PUT',
      body: request,
    });
  }
}

// Create default REST client instance
export const restClient = new RestClient(apiConfig.baseUrl);

/**
 * Create a new REST client with custom configuration
 * @param baseUrl - Custom base URL
 * @param headers - Custom headers
 * @returns New REST client instance
 */
export function createRestClient(
  baseUrl?: string,
  headers?: Record<string, string>
): RestClient {
  return new RestClient(baseUrl || apiConfig.baseUrl, headers);
}
