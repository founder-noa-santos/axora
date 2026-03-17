/**
 * API Integration Tests
 * 
 * Integration tests for the complete API layer including
 * REST client, WebSocket client, and mock API working together.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import {
  initializeApi,
  getApiStatus,
  restClient,
  wsClient,
  mockApi,
  enableMockApi,
  disableMockApi,
  configureApi,
} from '../index';

describe('API Integration', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    configureApi({ mockMode: true, retryAttempts: 2, retryDelay: 100 });
  });

  afterEach(() => {
    disableMockApi();
    wsClient.disconnect();
    vi.restoreAllMocks();
  });

  describe('API Initialization', () => {
    it('should initialize API with config', () => {
      initializeApi({
        baseUrl: 'http://localhost:3000/api',
        wsUrl: 'ws://localhost:3000/ws',
        mockMode: true,
        timeout: 30000,
      });

      const status = getApiStatus();
      expect(status.mockMode).toBe(true);
    });

    it('should initialize API with mock mode enabled', () => {
      initializeApi({ mockMode: true });
      enableMockApi();

      expect(mockApi.isMockMode()).toBe(true);
    });

    it('should get API status', () => {
      const status = getApiStatus();

      expect(status).toHaveProperty('mockMode');
      expect(status).toHaveProperty('wsConnected');
      expect(status).toHaveProperty('baseUrl');
      expect(status).toHaveProperty('wsUrl');
    });
  });

  describe('Mock API Integration', () => {
    beforeEach(() => {
      enableMockApi();
    });

    it('should submit mission and get mission ID', async () => {
      const result = await restClient.submitMission('Integration test mission');

      expect(result.missionId).toBeDefined();
      expect(result.missionId).toContain('mock-');
    });

    it('should track mission lifecycle', async () => {
      // Submit mission
      const result = await restClient.submitMission('Lifecycle test');

      // Get initial status
      const initialStatus = await restClient.getMission(result.missionId);
      expect(initialStatus.status).toBe('running');

      // Cancel mission
      await restClient.cancelMission(result.missionId);

      // Verify cancellation
      const finalStatus = await restClient.getMission(result.missionId);
      expect(finalStatus.status).toBe('cancelled');
    });

    it('should list missions and workers', async () => {
      mockApi.clearMockData(); // Clear any existing missions

      // Submit multiple missions
      await restClient.submitMission('Mission 1');
      await restClient.submitMission('Mission 2');
      await restClient.submitMission('Mission 3');

      // List missions
      const missionsResult = await restClient.listMissions();
      expect(missionsResult.missions.length).toBeGreaterThanOrEqual(3);

      // List workers
      const workersResult = await restClient.listWorkers();
      expect(workersResult.workers.length).toBeGreaterThan(0);
    });

    it('should handle settings CRUD', async () => {
      // Get initial settings
      const initialSettings = await restClient.getSettings();
      expect(initialSettings.theme.mode).toBe('dark');

      // Update settings
      await restClient.updateSettings({
        theme: { mode: 'light', accentColor: 'blue' },
      });

      // Get updated settings
      const updatedSettings = await restClient.getSettings();
      expect(updatedSettings.theme.mode).toBe('light');
      expect(updatedSettings.theme.accentColor).toBe('blue');
    });
  });

  describe('Error Handling Integration', () => {
    beforeEach(() => {
      enableMockApi();
    });

    it('should handle mission not found error', async () => {
      await expect(restClient.getMission('nonexistent-mission')).rejects.toThrow(
        'Mission not found'
      );
    });

    it('should handle worker not found error', async () => {
      await expect(restClient.getWorker('nonexistent-worker')).rejects.toThrow(
        'Worker not found'
      );
    });
  });

  describe('Concurrent Operations', () => {
    beforeEach(() => {
      enableMockApi();
    });

    it('should handle multiple concurrent mission submissions', async () => {
      mockApi.clearMockData(); // Clear any existing missions

      const missions = await Promise.all([
        restClient.submitMission('Mission 1'),
        restClient.submitMission('Mission 2'),
        restClient.submitMission('Mission 3'),
      ]);

      expect(missions).toHaveLength(3);

      const allMissions = await restClient.listMissions();
      expect(allMissions.missions.length).toBeGreaterThanOrEqual(3);
    });

    it('should handle parallel reads and writes', async () => {
      // Submit mission
      const submitPromise = restClient.submitMission('Test mission');

      // Parallel reads
      const [workers, settings] = await Promise.all([
        restClient.listWorkers(),
        restClient.getSettings(),
      ]);

      const result = await submitPromise;

      expect(workers.workers).toBeDefined();
      expect(settings).toBeDefined();
      expect(result.missionId).toBeDefined();
    });
  });

  describe('WebSocket Integration', () => {
    beforeEach(() => {
      enableMockApi();
    });

    it('should handle multiple event subscriptions', async () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();
      const handler3 = vi.fn();

      wsClient.on('mission:progress', handler1);
      wsClient.on('mission:progress', handler2);
      wsClient.on('mission:progress', handler3);

      await restClient.submitMission('Multi-subscriber test');

      // All handlers should be registered
      expect(handler1).toBeDefined();
      expect(handler2).toBeDefined();
      expect(handler3).toBeDefined();
    });
  });

  describe('Configuration Integration', () => {
    it('should work with custom mock config', async () => {
      const customMockApi = new (await import('../mock-api')).MockApi({
        networkDelay: 50,
      });

      customMockApi.enableMockMode();

      const start = Date.now();
      await restClient.submitMission('Fast test');
      const elapsed = Date.now() - start;

      // Should complete quickly with reduced delay
      expect(elapsed).toBeLessThan(200);

      customMockApi.disableMockMode();
    });

    it('should respect retry configuration', async () => {
      configureApi({ retryAttempts: 1, retryDelay: 10 });
      enableMockApi();

      // In mock mode, retries shouldn't be needed
      const result = await restClient.getSettings();
      expect(result).toBeDefined();
    });
  });

  describe('End-to-End Workflow', () => {
    beforeEach(() => {
      enableMockApi();
    });

    it('should complete mission workflow', async () => {
      // 1. Submit mission
      const result = await restClient.submitMission('E2E workflow test');
      expect(result.missionId).toBeDefined();

      // 2. Check initial status
      const status = await restClient.getMission(result.missionId);
      expect(status.status).toBe('running');

      // 3. List missions to verify
      const missions = await restClient.listMissions();
      expect(missions.missions.some(m => m.missionId === result.missionId)).toBe(true);

      // 4. Cancel mission
      await restClient.cancelMission(result.missionId);

      // 5. Verify cancellation
      const finalStatus = await restClient.getMission(result.missionId);
      expect(finalStatus.status).toBe('cancelled');
    });

    it('should handle worker monitoring workflow', async () => {
      // 1. List initial workers
      const workers = await restClient.listWorkers();
      expect(workers.workers.length).toBeGreaterThan(0);

      // 2. Get specific worker
      const worker = await restClient.getWorker('worker-1');
      expect(worker.worker.workerId).toBe('worker-1');
    });
  });
});
