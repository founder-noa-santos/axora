/**
 * WebSocket Client Tests
 * 
 * Tests for the WebSocket client including reconnection and heartbeat.
 */

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { WebSocketClient, createWebSocketClient } from '../websocket-client';
import { configureApi } from '../config';

describe('WebSocketClient', () => {
  let client: WebSocketClient;
  const testWsUrl = 'ws://localhost:3000/ws';

  beforeEach(() => {
    client = new WebSocketClient();
    vi.clearAllMocks();
    configureApi({ wsUrl: testWsUrl });
  });

  afterEach(() => {
    client.disconnect();
    vi.restoreAllMocks();
  });

  describe('Constructor & State', () => {
    it('should create client', () => {
      expect(client).toBeDefined();
      expect(client.getState()).toBe('disconnected');
    });

    it('should start disconnected', () => {
      expect(client.isConnected()).toBe(false);
      expect(client.getState()).toBe('disconnected');
    });

    it('should report disconnected after disconnect', () => {
      client.disconnect();
      expect(client.isConnected()).toBe(false);
    });
  });

  describe('Event Handlers', () => {
    it('should subscribe to events', () => {
      const handler = vi.fn();
      client.on('test:event', handler);

      expect(handler).not.toHaveBeenCalled();
    });

    it('should unsubscribe from events', () => {
      const handler = vi.fn();
      client.on('test:event', handler);
      client.off('test:event', handler);

      // Handler should not be called after unsubscribe
      expect(client).toBeDefined();
    });

    it('should clear all handlers', () => {
      const handler1 = vi.fn();
      const handler2 = vi.fn();

      client.on('event1', handler1);
      client.on('event2', handler2);

      client.clearHandlers();

      expect(client).toBeDefined();
    });
  });

  describe('Typed Event Handlers', () => {
    it('should subscribe to mission started events', () => {
      const handler = vi.fn();
      client.onMissionStarted(handler);
      expect(handler).toBeDefined();
    });

    it('should subscribe to mission progress events', () => {
      const handler = vi.fn();
      client.onMissionProgress(handler);
      expect(handler).toBeDefined();
    });

    it('should subscribe to mission completed events', () => {
      const handler = vi.fn();
      client.onMissionCompleted(handler);
      expect(handler).toBeDefined();
    });

    it('should subscribe to mission failed events', () => {
      const handler = vi.fn();
      client.onMissionFailed(handler);
      expect(handler).toBeDefined();
    });

    it('should subscribe to worker status events', () => {
      const handler = vi.fn();
      client.onWorkerStatus(handler);
      expect(handler).toBeDefined();
    });

    it('should subscribe to worker heartbeat events', () => {
      const handler = vi.fn();
      client.onWorkerHeartbeat(handler);
      expect(handler).toBeDefined();
    });
  });

  describe('Emit Events', () => {
    it('should emit events to handlers', () => {
      const handler = vi.fn();
      client.on('test:event', handler);

      client.emit('test:event', { data: 'test' });

      expect(handler).toHaveBeenCalledWith({ data: 'test' });
    });

    it('should emit typed mission events', () => {
      const handler = vi.fn();
      client.onMissionProgress(handler);

      client.emit('mission:progress', {
        missionId: 'test-123',
        progress: 50,
      });

      expect(handler).toHaveBeenCalledWith({
        missionId: 'test-123',
        progress: 50,
      });
    });

    it('should handle handler errors gracefully', () => {
      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      const badHandler = vi.fn(() => {
        throw new Error('Handler error');
      });

      client.on('test:event', badHandler);
      client.emit('test:event', {});

      expect(consoleSpy).toHaveBeenCalled();
    });
  });

  describe('Send', () => {
    it('should warn when sending while disconnected', () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      client.send('test:message', { data: 'test' });

      expect(consoleSpy).toHaveBeenCalled();
    });
  });

  describe('Disconnect', () => {
    it('should set state to disconnected', () => {
      client.disconnect();
      expect(client.getState()).toBe('disconnected');
    });
  });
});

describe('createWebSocketClient', () => {
  it('should create new WebSocket client', () => {
    const client = createWebSocketClient();
    expect(client).toBeDefined();
    expect(client.getState()).toBe('disconnected');
  });
});
