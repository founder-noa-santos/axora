/**
 * REST API Client Tests
 * 
 * Tests for the REST API client including error handling and retry logic.
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import {
  RestClient,
  ApiError,
  createRestClient,
} from '../rest-client';
import { configureApi } from '../config';

describe('RestClient', () => {
  const testBaseUrl = 'http://localhost:3000/api';
  let client: RestClient;

  beforeEach(() => {
    client = new RestClient(testBaseUrl);
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    // Reset to default config
    configureApi({ retryAttempts: 3, retryDelay: 1000 });
  });

  describe('Constructor', () => {
    it('should create client with base URL', () => {
      const customClient = new RestClient('http://custom-url.com/api');
      expect(customClient).toBeDefined();
    });

    it('should create client with custom headers', () => {
      const customClient = new RestClient('http://custom-url.com/api', {
        'Authorization': 'Bearer token123',
      });
      expect(customClient).toBeDefined();
    });
  });

  describe('API Error', () => {
    it('should create ApiError with status and code', () => {
      const error = new ApiError('Not found', 404, 'NOT_FOUND');
      expect(error.message).toBe('Not found');
      expect(error.status).toBe(404);
      expect(error.code).toBe('NOT_FOUND');
      expect(error.name).toBe('ApiError');
    });

    it('should identify network errors', () => {
      const networkError = new ApiError('Network error', 0, 'NETWORK_ERROR');
      expect(networkError.isNetworkError()).toBe(true);
      expect(networkError.isClientError()).toBe(false);
      expect(networkError.isServerError()).toBe(false);
    });

    it('should identify client errors (4xx)', () => {
      const clientError = new ApiError('Bad request', 400, 'BAD_REQUEST');
      expect(clientError.isClientError()).toBe(true);
      expect(clientError.isRetryable()).toBe(false);
    });

    it('should identify server errors (5xx)', () => {
      const serverError = new ApiError('Internal error', 500, 'INTERNAL_ERROR');
      expect(serverError.isServerError()).toBe(true);
      expect(serverError.isRetryable()).toBe(true);
    });
  });

  describe('Mission Endpoints', () => {
    it('should submit mission successfully', async () => {
      const mockResponse = {
        missionId: 'test-123',
        message: 'Mission submitted',
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.submitMission('Test mission content');
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        `${testBaseUrl}/missions`,
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ content: 'Test mission content', attachments: undefined }),
        })
      );
    });

    it('should submit mission with attachments', async () => {
      const mockResponse = {
        missionId: 'test-123',
        message: 'Mission submitted',
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const attachments = [{ name: 'file.txt', content: 'test' }];
      const result = await client.submitMission('Test mission', attachments);
      expect(result).toEqual(mockResponse);
    });

    it('should get mission status', async () => {
      const mockResponse = {
        missionId: 'test-123',
        progress: 50,
        status: 'running',
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.getMission('test-123');
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        `${testBaseUrl}/missions/test-123`,
        expect.objectContaining({ method: 'GET' })
      );
    });

    it('should list missions', async () => {
      const mockResponse = {
        missions: [
          { missionId: '1', status: 'completed', progress: 100 },
          { missionId: '2', status: 'running', progress: 50 },
        ],
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.listMissions();
      expect(result).toEqual(mockResponse);
      expect(result.missions).toHaveLength(2);
    });

    it('should cancel mission', async () => {
      const mockResponse = { success: true };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.cancelMission('test-123');
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        `${testBaseUrl}/missions/test-123`,
        expect.objectContaining({ method: 'DELETE' })
      );
    });
  });

  describe('Worker Endpoints', () => {
    it('should list workers', async () => {
      const mockResponse = {
        workers: [
          { workerId: 'worker-1', status: 'idle' },
          { workerId: 'worker-2', status: 'busy' },
        ],
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.listWorkers();
      expect(result).toEqual(mockResponse);
      expect(result.workers).toHaveLength(2);
    });

    it('should get worker details', async () => {
      const mockResponse = {
        worker: {
          workerId: 'worker-1',
          status: 'idle',
          lastHeartbeat: Date.now(),
        },
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.getWorker('worker-1');
      expect(result).toEqual(mockResponse);
    });
  });

  describe('Settings Endpoints', () => {
    it('should get settings', async () => {
      const mockResponse = {
        model: { provider: 'ollama', model: 'qwen2.5-coder:7b' },
        tokens: { maxTokensPerRequest: 4096 },
        workers: { minWorkers: 2, maxWorkers: 10 },
        theme: { mode: 'dark', accentColor: 'electric-purple' },
        advanced: { enableLogging: true, logLevel: 'info' },
      };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.getSettings();
      expect(result).toEqual(mockResponse);
    });

    it('should update settings', async () => {
      const mockResponse = { success: true };

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => mockResponse,
      });

      const result = await client.updateSettings({
        theme: { mode: 'light', accentColor: 'blue' },
      });
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        `${testBaseUrl}/settings`,
        expect.objectContaining({
          method: 'PUT',
          body: JSON.stringify({
            settings: { theme: { mode: 'light', accentColor: 'blue' } },
          }),
        })
      );
    });
  });

  describe('Error Handling', () => {
    it('should throw ApiError on HTTP error', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        json: async () => ({ message: 'Mission not found', code: 'NOT_FOUND' }),
      });

      await expect(client.getMission('invalid-id')).rejects.toThrow(ApiError);
      await expect(client.getMission('invalid-id')).rejects.toMatchObject({
        status: 404,
        code: 'NOT_FOUND',
      });
    });

    it('should handle network errors', async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error('Network error'));

      await expect(client.getMission('test-123')).rejects.toThrow(ApiError);
    }, 10000);

    it('should handle timeout errors', async () => {
      configureApi({ timeout: 100, retryAttempts: 0 });

      global.fetch = vi.fn().mockImplementation(() => {
        return new Promise((_, reject) => {
          setTimeout(() => reject(new Error('Timeout')), 200);
        });
      });

      // The request will fail with network error due to abort
      await expect(client.getMission('test-123')).rejects.toThrow();
    });

    it('should handle missing response body on error', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => {
          throw new Error('Invalid JSON');
        },
      });

      await expect(client.getMission('test-123')).rejects.toThrow(ApiError);
    }, 10000);
  });

  describe('Retry Logic', () => {
    it('should retry on server error', async () => {
      configureApi({ retryAttempts: 2, retryDelay: 10 });

      // Fail twice, succeed on third attempt
      global.fetch = vi.fn()
        .mockRejectedValueOnce(new ApiError('Server error', 500, 'INTERNAL_ERROR'))
        .mockRejectedValueOnce(new ApiError('Server error', 500, 'INTERNAL_ERROR'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ missionId: 'test-123', progress: 50, status: 'running' }),
        });

      const result = await client.getMission('test-123');
      expect(result).toBeDefined();
      expect(global.fetch).toHaveBeenCalledTimes(3);
    });

    it('should not retry on client error (4xx)', async () => {
      configureApi({ retryAttempts: 3, retryDelay: 10 });

      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 404,
        json: async () => ({ message: 'Not found' }),
      });

      await expect(client.getMission('invalid')).rejects.toThrow();
      expect(global.fetch).toHaveBeenCalledTimes(1); // No retries
    });

    it('should retry on network error', async () => {
      configureApi({ retryAttempts: 2, retryDelay: 10 });

      global.fetch = vi.fn()
        .mockRejectedValueOnce(new Error('Network error'))
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          json: async () => ({ workers: [] }),
        });

      const result = await client.listWorkers();
      expect(result).toBeDefined();
      expect(global.fetch).toHaveBeenCalledTimes(3);
    });

    it('should respect skipRetry option', async () => {
      configureApi({ retryAttempts: 3, retryDelay: 10 });

      global.fetch = vi.fn().mockRejectedValue(
        new ApiError('Server error', 500, 'INTERNAL_ERROR')
      );

      // Access private method via any cast for testing
      const anyClient = client as any;
      await expect(
        anyClient.request('/test', { skipRetry: true })
      ).rejects.toThrow();
      expect(global.fetch).toHaveBeenCalledTimes(1); // No retries
    });
  });

  describe('createRestClient', () => {
    it('should create client with default config', () => {
      const client = createRestClient();
      expect(client).toBeDefined();
    });

    it('should create client with custom URL', () => {
      const client = createRestClient('http://custom.com/api');
      expect(client).toBeDefined();
    });

    it('should create client with custom headers', () => {
      const client = createRestClient('http://custom.com/api', {
        'X-Custom-Header': 'value',
      });
      expect(client).toBeDefined();
    });
  });
});
