/**
 * AXORA Mock API Server
 * 
 * Mock API for development before Phase 3 backend is ready.
 * Provides the same interface as the real API with simulated responses and events.
 */

import { restClient } from './rest-client';
import { wsClient } from './websocket-client';
import type {
  Mission,
  Worker,
  WorkerStatus,
  SubmitMissionResponse,
  GetMissionResponse,
  ListMissionsResponse,
  CancelMissionResponse,
  ListWorkersResponse,
  GetWorkerResponse,
  GetSettingsResponse,
  UpdateSettingsResponse,
} from './types';

/**
 * Mock API configuration
 */
interface MockApiConfig {
  /** Simulated network delay in milliseconds */
  networkDelay: number;
  /** Delay variation for realism (±ms) */
  delayVariation: number;
  /** Mission progress interval in milliseconds */
  progressInterval: number;
  /** Worker status update interval in milliseconds */
  workerStatusInterval: number;
}

const defaultMockConfig: MockApiConfig = {
  networkDelay: 200, // 200ms base delay
  delayVariation: 100, // ±100ms variation
  progressInterval: 1000, // 1 second
  workerStatusInterval: 5000, // 5 seconds
};

/**
 * Original REST client methods (for restoration)
 */
interface OriginalMethods {
  submitMission: typeof restClient.submitMission;
  getMission: typeof restClient.getMission;
  listMissions: typeof restClient.listMissions;
  cancelMission: typeof restClient.cancelMission;
  listWorkers: typeof restClient.listWorkers;
  getWorker: typeof restClient.getWorker;
  getSettings: typeof restClient.getSettings;
  updateSettings: typeof restClient.updateSettings;
}

/**
 * Mock API Server for development
 */
export class MockApi {
  private missions = new Map<string, Mission>();
  private workers = new Map<string, Worker>();
  private settings: GetSettingsResponse | null = null;
  private progressIntervals = new Map<string, ReturnType<typeof setInterval>>();
  private config: MockApiConfig;
  private originalMethods: OriginalMethods | null = null;
  private enabled = false;

  constructor(config?: Partial<MockApiConfig>) {
    this.config = { ...defaultMockConfig, ...config };
    this.initializeWorkers();
  }

  /**
   * Initialize mock workers
   */
  private initializeWorkers(): void {
    const mockWorkers: Worker[] = [
      {
        workerId: 'worker-1',
        status: 'idle',
        lastHeartbeat: Date.now(),
        capabilities: ['general', 'documentation'],
      },
      {
        workerId: 'worker-2',
        status: 'busy',
        currentMissionId: 'mock-mission-001',
        lastHeartbeat: Date.now(),
        capabilities: ['implementation', 'testing'],
      },
      {
        workerId: 'worker-3',
        status: 'idle',
        lastHeartbeat: Date.now(),
        capabilities: ['review', 'analysis'],
      },
    ];

    mockWorkers.forEach((worker) => {
      this.workers.set(worker.workerId, worker);
    });
  }

  /**
   * Simulate network delay
   */
  private async simulateDelay(): Promise<void> {
    const delay =
      this.config.networkDelay +
      (Math.random() * 2 - 1) * this.config.delayVariation;
    return new Promise((resolve) => setTimeout(resolve, Math.max(0, delay)));
  }

  /**
   * Enable mock API mode
   */
  enableMockMode(): void {
    if (this.enabled) {
      return;
    }

    this.enabled = true;

    // Save original methods
    this.originalMethods = {
      submitMission: restClient.submitMission,
      getMission: restClient.getMission,
      listMissions: restClient.listMissions,
      cancelMission: restClient.cancelMission,
      listWorkers: restClient.listWorkers,
      getWorker: restClient.getWorker,
      getSettings: restClient.getSettings,
      updateSettings: restClient.updateSettings,
    };

    // Override REST client methods
    restClient.submitMission = this.mockSubmitMission.bind(this);
    restClient.getMission = this.mockGetMission.bind(this);
    restClient.listMissions = this.mockListMissions.bind(this);
    restClient.cancelMission = this.mockCancelMission.bind(this);
    restClient.listWorkers = this.mockListWorkers.bind(this);
    restClient.getWorker = this.mockGetWorker.bind(this);
    restClient.getSettings = this.mockGetSettings.bind(this);
    restClient.updateSettings = this.mockUpdateSettings.bind(this);

    // Start simulating WebSocket events
    this.simulateWebSocketEvents();

    console.log('[MockAPI] Mock mode enabled');
  }

  /**
   * Disable mock API mode (restore real API)
   */
  disableMockMode(): void {
    if (!this.enabled || !this.originalMethods) {
      return;
    }

    // Restore original methods
    restClient.submitMission = this.originalMethods.submitMission;
    restClient.getMission = this.originalMethods.getMission;
    restClient.listMissions = this.originalMethods.listMissions;
    restClient.cancelMission = this.originalMethods.cancelMission;
    restClient.listWorkers = this.originalMethods.listWorkers;
    restClient.getWorker = this.originalMethods.getWorker;
    restClient.getSettings = this.originalMethods.getSettings;
    restClient.updateSettings = this.originalMethods.updateSettings;

    // Stop simulating WebSocket events
    this.stopSimulatingEvents();

    this.enabled = false;
    console.log('[MockAPI] Mock mode disabled, using real API');
  }

  /**
   * Check if mock mode is enabled
   */
  isMockMode(): boolean {
    return this.enabled;
  }

  /**
   * Start simulating WebSocket events
   */
  private simulateWebSocketEvents(): void {
    // Simulate worker status updates
    const workerInterval = setInterval(() => {
      this.simulateWorkerStatusUpdate();
    }, this.config.workerStatusInterval);

    // Simulate worker heartbeats
    const heartbeatInterval = setInterval(() => {
      this.simulateWorkerHeartbeat();
    }, this.config.workerStatusInterval / 2);

    this.progressIntervals.set('workers' as any, workerInterval);
    this.progressIntervals.set('heartbeats' as any, heartbeatInterval);
  }

  /**
   * Stop simulating WebSocket events
   */
  private stopSimulatingEvents(): void {
    this.progressIntervals.forEach((interval) => {
      clearInterval(interval);
    });
    this.progressIntervals.clear();
  }

  /**
   * Simulate worker status update
   */
  private simulateWorkerStatusUpdate(): void {
    const workerIds = Array.from(this.workers.keys());
    const randomWorkerId = workerIds[Math.floor(Math.random() * workerIds.length)];
    const worker = this.workers.get(randomWorkerId);

    if (worker && !worker.currentMissionId) {
      // Only update idle workers
      const statuses: WorkerStatus[] = ['idle', 'idle', 'idle', 'busy'];
      const newStatus = statuses[Math.floor(Math.random() * statuses.length)];

      worker.status = newStatus;
      worker.lastHeartbeat = Date.now();

      wsClient.emit('worker:status', {
        workerId: randomWorkerId,
        status: newStatus,
        lastHeartbeat: Date.now(),
      });
    }
  }

  /**
   * Simulate worker heartbeat
   */
  private simulateWorkerHeartbeat(): void {
    const workerIds = Array.from(this.workers.keys());
    workerIds.forEach((workerId) => {
      const worker = this.workers.get(workerId);
      if (worker) {
        worker.lastHeartbeat = Date.now();
        wsClient.emit('worker:heartbeat', {
          workerId,
          timestamp: Date.now(),
          health: 'healthy',
        });
      }
    });
  }

  /**
   * Simulate mission progress
   */
  private simulateMissionProgress(missionId: string): void {
    let progress = 0;
    const steps = [
      'Initializing...',
      'Analyzing requirements...',
      'Planning implementation...',
      'Writing code...',
      'Running tests...',
      'Finalizing...',
    ];

    const interval = setInterval(async () => {
      const mission = this.missions.get(missionId);
      if (!mission) {
        clearInterval(interval);
        return;
      }

      // Increment progress
      progress += Math.random() * 15 + 5; // 5-20% per step

      if (progress >= 100) {
        progress = 100;
        clearInterval(interval);
        this.progressIntervals.delete(missionId);

        // Mark as completed
        mission.status = 'completed';
        mission.progress = 100;
        mission.completedAt = Date.now();
        mission.result = 'Mission completed successfully (mock)';
        mission.updatedAt = Date.now();

        // Emit completion event
        wsClient.emit('mission:completed', {
          missionId,
          result: mission.result,
        });
      } else {
        const currentStep = steps[Math.min(Math.floor(progress / 20), steps.length - 1)];
        mission.progress = progress;
        mission.updatedAt = Date.now();

        // Emit progress event
        wsClient.emit('mission:progress', {
          missionId,
          progress: Math.round(progress),
          eta: Math.max(0, Math.round((100 - progress) / 10)),
          currentStep,
        });
      }
    }, this.config.progressInterval);

    this.progressIntervals.set(missionId, interval);
  }

  // ============ Mock REST Endpoints ============

  /**
   * Mock submit mission
   */
  private async mockSubmitMission(
    content: string,
    attachments?: any[]
  ): Promise<SubmitMissionResponse> {
    await this.simulateDelay();

    const missionId = `mock-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const mission: Mission = {
      missionId,
      content,
      status: 'running',
      progress: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
    };

    this.missions.set(missionId, mission);

    // Emit started event
    wsClient.emit('mission:started', {
      missionId,
      status: 'running',
    });

    // Start simulating progress
    this.simulateMissionProgress(missionId);

    return {
      missionId,
      message: 'Mission submitted (mock mode)',
    };
  }

  /**
   * Mock get mission
   */
  private async mockGetMission(missionId: string): Promise<GetMissionResponse> {
    await this.simulateDelay();

    const mission = this.missions.get(missionId);
    if (!mission) {
      throw new Error(`Mission not found: ${missionId}`);
    }

    return {
      missionId: mission.missionId,
      progress: mission.progress,
      status: mission.status,
      content: mission.content,
      result: mission.result,
      error: mission.error,
    };
  }

  /**
   * Mock list missions
   */
  private async mockListMissions(): Promise<ListMissionsResponse> {
    await this.simulateDelay();

    return {
      missions: Array.from(this.missions.values()),
    };
  }

  /**
   * Mock cancel mission
   */
  private async mockCancelMission(missionId: string): Promise<CancelMissionResponse> {
    await this.simulateDelay();

    const mission = this.missions.get(missionId);
    if (!mission) {
      throw new Error(`Mission not found: ${missionId}`);
    }

    // Stop progress simulation
    const interval = this.progressIntervals.get(missionId);
    if (interval) {
      clearInterval(interval);
      this.progressIntervals.delete(missionId);
    }

    mission.status = 'cancelled';
    mission.updatedAt = Date.now();

    // Emit cancelled event
    wsClient.emit('mission:cancelled', {
      missionId,
    });

    return {
      success: true,
    };
  }

  /**
   * Mock list workers
   */
  private async mockListWorkers(): Promise<ListWorkersResponse> {
    await this.simulateDelay();

    return {
      workers: Array.from(this.workers.values()),
    };
  }

  /**
   * Mock get worker
   */
  private async mockGetWorker(workerId: string): Promise<GetWorkerResponse> {
    await this.simulateDelay();

    const worker = this.workers.get(workerId);
    if (!worker) {
      throw new Error(`Worker not found: ${workerId}`);
    }

    return {
      worker,
    };
  }

  /**
   * Mock get settings
   */
  private async mockGetSettings(): Promise<GetSettingsResponse> {
    await this.simulateDelay();

    if (this.settings) {
      return this.settings;
    }

    // Return default settings
    return {
      model: {
        provider: 'ollama',
        model: 'qwen2.5-coder:7b',
        baseUrl: 'http://localhost:11434',
      },
      tokens: {
        maxTokensPerRequest: 4096,
        maxContextTokens: 8192,
        tokenBudget: 100000,
      },
      workers: {
        minWorkers: 2,
        maxWorkers: 10,
        healthCheckInterval: 30,
      },
      theme: {
        mode: 'dark',
        accentColor: 'electric-purple',
      },
      advanced: {
        enableLogging: true,
        logLevel: 'info',
        autoUpdate: true,
      },
    };
  }

  /**
   * Mock update settings
   */
  private async mockUpdateSettings(
    settings: Partial<GetSettingsResponse>
  ): Promise<UpdateSettingsResponse> {
    await this.simulateDelay();

    this.settings = {
      ...(this.settings || (await this.mockGetSettings())),
      ...settings,
    } as GetSettingsResponse;

    return {
      success: true,
    };
  }

  /**
   * Clear all mock data
   */
  clearMockData(): void {
    this.missions.clear();
    this.settings = null;
    this.progressIntervals.forEach((interval) => clearInterval(interval));
    this.progressIntervals.clear();
  }

  /**
   * Get mock mission by ID
   */
  getMission(missionId: string): Mission | undefined {
    return this.missions.get(missionId);
  }

  /**
   * Get all mock missions
   */
  getAllMissions(): Mission[] {
    return Array.from(this.missions.values());
  }

  /**
   * Get mock worker by ID
   */
  getWorker(workerId: string): Worker | undefined {
    return this.workers.get(workerId);
  }

  /**
   * Get all mock workers
   */
  getAllWorkers(): Worker[] {
    return Array.from(this.workers.values());
  }
}

// Create default mock API instance
export const mockApi = new MockApi();

/**
 * Enable mock API mode
 */
export function enableMockApi(): void {
  mockApi.enableMockMode();
}

/**
 * Disable mock API mode
 */
export function disableMockApi(): void {
  mockApi.disableMockMode();
}

/**
 * Create a new mock API instance with custom configuration
 * @param config - Mock API configuration
 * @returns New mock API instance
 */
export function createMockApi(config?: Partial<MockApiConfig>): MockApi {
  return new MockApi(config);
}
