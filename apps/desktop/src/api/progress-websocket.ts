import { WebSocketEvent } from '../types/progress';

type EventHandler = (event: WebSocketEvent) => void;

export class ProgressWebSocket {
  private ws: WebSocket | null = null;
  private handlers: EventHandler[] = [];
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private url: string | null = null;

  /**
   * Connect to WebSocket server
   * @param url WebSocket server URL
   */
  connect(url: string) {
    this.url = url;
    this.ws = new WebSocket(url);

    this.ws.onopen = () => {
      console.log('[ProgressWebSocket] Connected');
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event) => {
      try {
        const data = JSON.parse(event.data);
        this.handlers.forEach((handler) => handler(data));
      } catch (error) {
        console.error('[ProgressWebSocket] Failed to parse message:', error);
      }
    };

    this.ws.onclose = () => {
      console.log('[ProgressWebSocket] Closed');
      this.attemptReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('[ProgressWebSocket] Error:', error);
    };
  }

  /**
   * Attempt to reconnect with exponential backoff
   */
  private attemptReconnect() {
    if (!this.url) return;

    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
      console.log(
        `[ProgressWebSocket] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`
      );
      setTimeout(() => this.connect(this.url!), delay);
    } else {
      console.error('[ProgressWebSocket] Max reconnection attempts reached');
    }
  }

  /**
   * Register event handler
   * @param handler Event handler function
   */
  onEvent(handler: EventHandler) {
    this.handlers.push(handler);
  }

  /**
   * Remove event handler
   * @param handler Event handler function to remove
   */
  offEvent(handler: EventHandler) {
    const index = this.handlers.indexOf(handler);
    if (index > -1) {
      this.handlers.splice(index, 1);
    }
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect() {
    this.ws?.close();
    this.ws = null;
    this.url = null;
    this.reconnectAttempts = 0;
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws?.readyState === WebSocket.OPEN;
  }

  /**
   * Get current connection state
   */
  get readyState(): number {
    return this.ws?.readyState ?? WebSocket.CLOSED;
  }
}

export const progressWS = new ProgressWebSocket();
