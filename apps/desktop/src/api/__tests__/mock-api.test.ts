/**
 * Mock API Tests
 * 
 * Tests for the mock API server including mock REST endpoints and WebSocket events.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { MockApi, createMockApi, enableMockApi, disableMockApi, mockApi } from '../mock-api';
import { restClient } from '../rest-client';
import { wsClient } from '../websocket-client';
import { configureApi } from '../config';

describe('MockApi', () => {
  let mockApiInstance: MockApi;

  beforeEach(() => {
    mockApiInstance = new MockApi();
    vi.clearAllMocks();
    configureApi({ mockMode: true });
  });

  afterEach(() => {
    mockApiInstance.disableMockMode();
    mockApiInstance.clearMockData();
    vi.restoreAllMocks();
  });

  describe('Constructor', () => {
    it('should create mock API with default config', () => {
      expect(mockApiInstance).toBeDefined();
      expect(mockApiInstance.isMockMode()).toBe(false);
    });

    it('should create mock API with custom config', () => {
      const customMockApi = new MockApi({
        networkDelay: 100,
        delayVariation: 50,
      });
      expect(customMockApi).toBeDefined();
      customMockApi.disableMockMode();
    });
  });

  describe('Enable/Disable Mock Mode', () => {
    it('should enable mock mode', () => {
      mockApiInstance.enableMockMode();
      expect(mockApiInstance.isMockMode()).toBe(true);
    });

    it('should disable mock mode', () => {
      mockApiInstance.enableMockMode();
      mockApiInstance.disableMockMode();
      expect(mockApiInstance.isMockMode()).toBe(false);
    });

    it('should not error when enabling twice', () => {
      mockApiInstance.enableMockMode();
      expect(() => mockApiInstance.enableMockMode()).not.toThrow();
    });

    it('should not error when disabling twice', () => {
      mockApiInstance.disableMockMode();
      expect(() => mockApiInstance.disableMockMode()).not.toThrow();
    });
  });

  describe('Mock Mission Endpoints', () => {
    beforeEach(() => {
      mockApiInstance.enableMockMode();
    });

    it('should submit mission', async () => {
      const result = await restClient.submitMission('Test mission');

      expect(result.missionId).toBeDefined();
      expect(result.missionId).toContain('mock-');
      expect(result.message).toContain('mock mode');
    });

    it('should get mission', async () => {
      const submitResult = await restClient.submitMission('Test mission');
      const mission = mockApiInstance.getMission(submitResult.missionId);

      expect(mission).toBeDefined();
      expect(mission?.content).toBe('Test mission');
      expect(mission?.status).toBe('running');
    });

    it('should get mission not found', async () => {
      await expect(restClient.getMission('nonexistent')).rejects.toThrow(
        'Mission not found'
      );
    });

    it('should list missions', async () => {
      await restClient.submitMission('Mission 1');
      await restClient.submitMission('Mission 2');

      const result = await restClient.listMissions();

      expect(result.missions).toHaveLength(2);
    });

    it('should cancel mission', async () => {
      const submitResult = await restClient.submitMission('Test mission');

      const cancelResult = await restClient.cancelMission(submitResult.missionId);

      expect(cancelResult.success).toBe(true);

      const mission = mockApiInstance.getMission(submitResult.missionId);
      expect(mission?.status).toBe('cancelled');
    });
  });

  describe('Mock Worker Endpoints', () => {
    beforeEach(() => {
      mockApiInstance.enableMockMode();
    });

    it('should list workers', async () => {
      const result = await restClient.listWorkers();

      expect(result.workers).toBeDefined();
      expect(result.workers.length).toBeGreaterThan(0);
    });

    it('should get worker', async () => {
      const result = await restClient.getWorker('worker-1');

      expect(result.worker).toBeDefined();
      expect(result.worker.workerId).toBe('worker-1');
    });

    it('should get worker not found', async () => {
      await expect(restClient.getWorker('nonexistent')).rejects.toThrow(
        'Worker not found'
      );
    });
  });

  describe('Mock Settings Endpoints', () => {
    beforeEach(() => {
      mockApiInstance.enableMockMode();
    });

    it('should get default settings', async () => {
      const result = await restClient.getSettings();

      expect(result).toBeDefined();
      expect(result.model.provider).toBe('ollama');
      expect(result.theme.mode).toBe('dark');
    });

    it('should update settings', async () => {
      const result = await restClient.updateSettings({
        theme: { mode: 'light', accentColor: 'blue' },
      });

      expect(result.success).toBe(true);

      // Get settings again to verify update
      const updatedSettings = await restClient.getSettings();
      expect(updatedSettings.theme.mode).toBe('light');
    });
  });

  describe('Mock Data Management', () => {
    beforeEach(() => {
      mockApiInstance.enableMockMode();
    });

    it('should clear mock data', async () => {
      await restClient.submitMission('Test mission');

      const missions = mockApiInstance.getAllMissions();
      expect(missions).toHaveLength(1);

      mockApiInstance.clearMockData();

      const clearedMissions = mockApiInstance.getAllMissions();
      expect(clearedMissions).toHaveLength(0);
    });

    it('should get all missions', async () => {
      await restClient.submitMission('Mission 1');
      await restClient.submitMission('Mission 2');

      const missions = mockApiInstance.getAllMissions();

      expect(missions).toHaveLength(2);
    });

    it('should get all workers', () => {
      const workers = mockApiInstance.getAllWorkers();

      expect(workers).toHaveLength(3);
      expect(workers.map(w => w.workerId)).toEqual([
        'worker-1',
        'worker-2',
        'worker-3',
      ]);
    });
  });
});

describe('enableMockApi / disableMockApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    disableMockApi();
  });

  it('should enable mock API via helper function', () => {
    enableMockApi();
    expect(mockApi.isMockMode()).toBe(true);
  });

  it('should disable mock API via helper function', () => {
    enableMockApi();
    disableMockApi();
    expect(mockApi.isMockMode()).toBe(false);
  });
});

describe('createMockApi', () => {
  it('should create new mock API instance', () => {
    const customMockApi = createMockApi({
      networkDelay: 50,
    });

    expect(customMockApi).toBeDefined();
    expect(customMockApi.isMockMode()).toBe(false);

    customMockApi.disableMockMode();
  });
});
