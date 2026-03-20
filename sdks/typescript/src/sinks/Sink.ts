import type { WideEventPayload } from '../types.js';

export interface Sink {
  /**
   * Called once when the WideEvent lifecycle ends.
   * MUST be non-blocking from the caller's perspective.
   */
  export(event: WideEventPayload): Promise<void>;

  /**
   * Optional graceful flush hook.
   */
  flush?(): Promise<void>;
}
