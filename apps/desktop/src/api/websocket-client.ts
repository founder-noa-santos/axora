/**
 * AXORA WebSocket Client
 * 
 * Real-time event client with automatic reconnection and heartbeat.
 * Supports typed event handlers for mission and worker events.
 */

import { apiConfig } from './config';
import type {
  WebSocketEvent,
  WebSocketEventType,
  MissionStartedEvent,
  MissionProgressEvent,
  MissionCompletedEvent,
  MissionFailedEvent,
  WorkerStatusEvent,
  WorkerHeartbeatEvent,
} from './types';

/**
 * Event handler function type
 */
type EventHandler<T = any> = (event: T) => void;

/**
 * WebSocket connection state
 */
type ConnectionState = 'disconnected' | 'connecting' | 'connected' | 'reconnecting';

/**
 * WebSocket Client for real-time events
 */
export class WebSocketClient {
  private ws: WebSocket | null = null;
  private url: string | null = null;
  private handlers: Map<string, EventHandler[]> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private heartbeatInterval: ReturnType<typeof setInterval> | null = null;
  private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;
  private state: ConnectionState = 'disconnected';
  private shouldReconnect = true;

  /**
   * Get current connection state
   */
  getState(): ConnectionState {
    return this.state;
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.state === 'connected';
  }

  /**
   * Connect to WebSocket server
   * @param url - WebSocket URL (uses config if not provided)
   */
  connect(url?: string): void {
    const wsUrl = url || apiConfig.wsUrl;
    this.url = wsUrl;
    this.shouldReconnect = true;
    this.state = 'connecting';

    try {
      this.ws = new WebSocket(wsUrl);

      this.ws.onopen = () => {
        console.log('[WebSocket] Connected');
        this.state = 'connected';
        this.reconnectAttempts = 0;
        this.startHeartbeat();
        this.emit('connection:open', { connected: true });
      };

      this.ws.onmessage = (event) => {
        try {
          const data = JSON.parse(event.data);
          const { type, payload, timestamp } = data;

          // Handle ping-pong
          if (type === 'ping') {
            this.sendPong();
            return;
          }

          // Emit event to handlers
          this.emit(type, {
            type,
            payload,
            timestamp: timestamp || Date.now(),
          });
        } catch (error) {
          console.error('[WebSocket] Failed to parse message:', error);
        }
      };

      this.ws.onclose = (event) => {
        console.log('[WebSocket] Closed:', event.code, event.reason);
        this.state = 'disconnected';
        this.stopHeartbeat();
        this.emit('connection:close', { code: event.code, reason: event.reason });

        // Attempt reconnection if not manually disconnected
        if (this.shouldReconnect) {
          this.attemptReconnect();
        }
      };

      this.ws.onerror = (error) => {
        console.error('[WebSocket] Error:', error);
        this.emit('connection:error', { error });
      };
    } catch (error) {
      console.error('[WebSocket] Failed to connect:', error);
      this.state = 'disconnected';
      this.attemptReconnect();
    }
  }

  /**
   * Attempt to reconnect with exponential backoff
   */
  private attemptReconnect(): void {
    if (!this.shouldReconnect) {
      return;
    }

    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error(
        `[WebSocket] Max reconnection attempts reached (${this.maxReconnectAttempts})`
      );
      this.emit('connection:failed', {
        attempts: this.reconnectAttempts,
      });
      return;
    }

    this.reconnectAttempts++;
    this.state = 'reconnecting';

    // Exponential backoff with max 30 seconds
    const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
    console.log(
      `[WebSocket] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts}/${this.maxReconnectAttempts})`
    );

    this.reconnectTimeout = setTimeout(() => {
      if (this.url) {
        this.connect(this.url);
      }
    }, delay);
  }

  /**
   * Start heartbeat (ping-pong)
   */
  private startHeartbeat(): void {
    this.stopHeartbeat(); // Clear existing interval

    this.heartbeatInterval = setInterval(() => {
      if (this.ws && this.ws.readyState === WebSocket.OPEN) {
        this.ws.send(JSON.stringify({ type: 'ping', timestamp: Date.now() }));
      }
    }, 30000); // 30 seconds
  }

  /**
   * Stop heartbeat
   */
  private stopHeartbeat(): void {
    if (this.heartbeatInterval) {
      clearInterval(this.heartbeatInterval);
      this.heartbeatInterval = null;
    }
  }

  /**
   * Send pong response
   */
  private sendPong(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({ type: 'pong', timestamp: Date.now() }));
    }
  }

  /**
   * Send a message to the WebSocket server
   * @param type - Event type
   * @param payload - Event payload
   */
  send<T = any>(type: string, payload: T): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('[WebSocket] Cannot send message: not connected');
      return;
    }

    const message = JSON.stringify({
      type,
      payload,
      timestamp: Date.now(),
    });
    this.ws.send(message);
  }

  /**
   * Subscribe to an event type
   * @param eventType - Event type to subscribe to
   * @param handler - Event handler function
   */
  on<T = any>(eventType: WebSocketEventType | string, handler: EventHandler<T>): void {
    if (!this.handlers.has(eventType)) {
      this.handlers.set(eventType, []);
    }
    this.handlers.get(eventType)!.push(handler);
  }

  /**
   * Unsubscribe from an event type
   * @param eventType - Event type to unsubscribe from
   * @param handler - Event handler function to remove
   */
  off<T = any>(eventType: WebSocketEventType | string, handler: EventHandler<T>): void {
    const handlers = this.handlers.get(eventType);
    if (handlers) {
      const index = handlers.indexOf(handler);
      if (index > -1) {
        handlers.splice(index, 1);
      }
    }
  }

  /**
   * Clear all event handlers
   */
  clearHandlers(): void {
    this.handlers.clear();
  }

  /**
   * Emit an event to all registered handlers (public for mock API)
   * @param eventType - Event type
   * @param payload - Event payload
   */
  emit<T = any>(eventType: string, payload: T): void {
    const handlers = this.handlers.get(eventType);
    if (handlers) {
      handlers.forEach((handler) => {
        try {
          handler(payload);
        } catch (error) {
          console.error(`[WebSocket] Error in event handler for ${eventType}:`, error);
        }
      });
    }
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    this.shouldReconnect = false;
    this.stopHeartbeat();

    if (this.reconnectTimeout) {
      clearTimeout(this.reconnectTimeout);
      this.reconnectTimeout = null;
    }

    if (this.ws) {
      this.ws.close(1000, 'Client disconnected');
      this.ws = null;
    }

    this.state = 'disconnected';
  }

  // ============ Typed Event Handlers ============

  /**
   * Subscribe to mission started events
   * @param handler - Event handler
   */
  onMissionStarted(handler: EventHandler<MissionStartedEvent>): void {
    this.on('mission:started', handler);
  }

  /**
   * Subscribe to mission progress events
   * @param handler - Event handler
   */
  onMissionProgress(handler: EventHandler<MissionProgressEvent>): void {
    this.on('mission:progress', handler);
  }

  /**
   * Subscribe to mission completed events
   * @param handler - Event handler
   */
  onMissionCompleted(handler: EventHandler<MissionCompletedEvent>): void {
    this.on('mission:completed', handler);
  }

  /**
   * Subscribe to mission failed events
   * @param handler - Event handler
   */
  onMissionFailed(handler: EventHandler<MissionFailedEvent>): void {
    this.on('mission:failed', handler);
  }

  /**
   * Subscribe to worker status events
   * @param handler - Event handler
   */
  onWorkerStatus(handler: EventHandler<WorkerStatusEvent>): void {
    this.on('worker:status', handler);
  }

  /**
   * Subscribe to worker heartbeat events
   * @param handler - Event handler
   */
  onWorkerHeartbeat(handler: EventHandler<WorkerHeartbeatEvent>): void {
    this.on('worker:heartbeat', handler);
  }
}

// Create default WebSocket client instance
export const wsClient = new WebSocketClient();

/**
 * Create a new WebSocket client
 * @returns New WebSocket client instance
 */
export function createWebSocketClient(): WebSocketClient {
  return new WebSocketClient();
}
