import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { ProgressWebSocket } from '../progress-websocket';

// Mock WebSocket
class MockWebSocket {
  public onopen: (() => void) | null = null;
  public onmessage: ((event: MessageEvent) => void) | null = null;
  public onclose: (() => void) | null = null;
  public onerror: ((event: Event) => void) | null = null;
  public readyState: number = WebSocket.OPEN;
  
  constructor(public url: string) {}
  
  close() {
    this.readyState = WebSocket.CLOSED;
    this.onclose?.();
  }
  
  send() {}
}

vi.stubGlobal('WebSocket', MockWebSocket);

describe('ProgressWebSocket', () => {
  let ws: ProgressWebSocket;

  beforeEach(() => {
    vi.useFakeTimers();
    ws = new ProgressWebSocket();
    vi.clearAllMocks();
  });

  afterEach(() => {
    ws.disconnect();
    vi.useRealTimers();
  });

  it('should connect to WebSocket server', () => {
    ws.connect('ws://localhost:8080');

    expect(ws.isConnected()).toBe(true);
  });

  it('should register event handler', () => {
    const handler = vi.fn();
    ws.onEvent(handler);

    ws.connect('ws://localhost:8080');

    // Simulate message
    const eventData = {
      type: 'mission:started',
      payload: { missionId: 'test-mission' },
    };

    // Get the mock WebSocket instance
    const mockWs = (ws as any).ws;
    mockWs?.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify(eventData),
    }));

    expect(handler).toHaveBeenCalledWith(eventData);
  });

  it('should handle multiple event handlers', () => {
    const handler1 = vi.fn();
    const handler2 = vi.fn();

    ws.onEvent(handler1);
    ws.onEvent(handler2);

    ws.connect('ws://localhost:8080');

    const eventData = {
      type: 'mission:progress',
      payload: { missionId: 'test', progress: 50, status: 'running' },
    };

    const mockWs = (ws as any).ws;
    mockWs?.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify(eventData),
    }));

    expect(handler1).toHaveBeenCalledWith(eventData);
    expect(handler2).toHaveBeenCalledWith(eventData);
  });

  it('should remove event handler', () => {
    const handler = vi.fn();
    ws.onEvent(handler);
    ws.offEvent(handler);

    ws.connect('ws://localhost:8080');

    const eventData = {
      type: 'mission:completed',
      payload: { missionId: 'test', result: 'success', completedAt: Date.now() },
    };

    const mockWs = (ws as any).ws;
    mockWs?.onmessage?.(new MessageEvent('message', {
      data: JSON.stringify(eventData),
    }));

    expect(handler).not.toHaveBeenCalled();
  });

  it('should disconnect from WebSocket server', () => {
    ws.connect('ws://localhost:8080');
    ws.disconnect();

    expect(ws.isConnected()).toBe(false);
  });

  it('should attempt reconnection on close', () => {
    ws.connect('ws://localhost:8080');

    // Simulate close
    const mockWs = (ws as any).ws;
    mockWs?.onclose?.();

    // Fast-forward timers
    vi.advanceTimersByTime(2000);

    // Should have attempted reconnection
    expect(ws.readyState).toBe(WebSocket.OPEN);
  });

  it('should use exponential backoff for reconnection', () => {
    ws.connect('ws://localhost:8080');

    // First close
    let mockWs = (ws as any).ws;
    mockWs?.onclose?.();
    vi.advanceTimersByTime(2000);

    // Second close
    mockWs = (ws as any).ws;
    mockWs?.onclose?.();
    vi.advanceTimersByTime(4000);

    // Should still be reconnecting
    expect(ws.readyState).toBe(WebSocket.OPEN);
  });

  it('should stop reconnecting after max attempts', () => {
    ws.connect('ws://localhost:8080');

    // Simulate 5 closes (max attempts)
    for (let i = 0; i < 5; i++) {
      const mockWs = (ws as any).ws;
      mockWs?.onclose?.();
      vi.advanceTimersByTime(10000);
    }

    // Should not attempt again after max
    const mockWs = (ws as any).ws;
    mockWs?.onclose?.();
    vi.advanceTimersByTime(30000);

    // Connection should be closed
    expect(ws.isConnected()).toBe(false);
  });

  it('should reset reconnect attempts on successful connection', () => {
    ws.connect('ws://localhost:8080');

    // Simulate close and reconnect
    let mockWs = (ws as any).ws;
    mockWs?.onclose?.();
    vi.advanceTimersByTime(2000);

    // Simulate successful connection (onopen)
    mockWs = (ws as any).ws;
    mockWs?.onopen?.();

    // Close again
    mockWs?.onclose?.();
    vi.advanceTimersByTime(2000);

    // Should still be reconnecting
    expect(ws.readyState).toBe(WebSocket.OPEN);
  });

  it('should handle malformed JSON gracefully', () => {
    const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const handler = vi.fn();

    ws.onEvent(handler);
    ws.connect('ws://localhost:8080');

    const mockWs = (ws as any).ws;
    mockWs?.onmessage?.(new MessageEvent('message', {
      data: 'invalid json',
    }));

    expect(consoleErrorSpy).toHaveBeenCalledWith(
      '[ProgressWebSocket] Failed to parse message:',
      expect.any(Error)
    );
    expect(handler).not.toHaveBeenCalled();

    consoleErrorSpy.mockRestore();
  });

  it('should return correct readyState', () => {
    expect(ws.readyState).toBe(WebSocket.CLOSED);

    ws.connect('ws://localhost:8080');
    expect(ws.readyState).toBe(WebSocket.OPEN);

    ws.disconnect();
    expect(ws.readyState).toBe(WebSocket.CLOSED);
  });
});
