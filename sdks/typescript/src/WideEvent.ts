import { performance } from 'node:perf_hooks';
import { randomUUID } from 'node:crypto';

import type { EventStatus, LogLevel, WideEventPayload, Sink } from './types.js';
import { cloneStructured, deepFreeze, normalizeEnvironment } from './internal/snapshot.js';
import { SDK_VERSION } from './version.js';

const DEFAULT_ERROR = { type: null, message: null, stack: null };

function normalizeError(err: Error | unknown): WideEventPayload['error'] {
  if (err instanceof Error) {
    return {
      type: err.constructor?.name ?? 'Error',
      message: err.message || null,
      stack: err.stack ?? null,
    };
  }

  if (typeof err === 'string') {
    return { type: 'Error', message: err, stack: null };
  }

  if (err === null || err === undefined) {
    return { ...DEFAULT_ERROR };
  }

  return {
    type: typeof err,
    message: (() => {
      try {
        return JSON.stringify(err);
      } catch {
        return String(err);
      }
    })(),
    stack: null,
  };
}

export class WideEvent {
  private readonly event_id = randomUUID();
  private readonly timestamp_start = new Date();
  private readonly start_monotonic_ms = performance.now();
  private readonly sinks: readonly Sink[];
  private readonly service: string;
  private readonly environment: ReturnType<typeof normalizeEnvironment>;
  private context: Record<string, unknown> = {};
  private error: WideEventPayload['error'] = { ...DEFAULT_ERROR };
  private level: LogLevel = 'info';
  private status: EventStatus = 'ok';
  private finalized = false;
  private emitPromise: Promise<void> | null = null;

  constructor(
    private readonly operation: string,
    service: string,
    environment: string,
    sinks: Sink[],
  ) {
    this.service = service;
    this.environment = normalizeEnvironment(environment);
    this.sinks = [...sinks];
  }

  private assertMutable() {
    if (this.finalized) {
      throw new Error('WideEvent has already been finalized');
    }
  }

  /** Merge arbitrary key-value context into the event. Chainable. */
  appendContext(fields: Record<string, unknown>): this {
    this.assertMutable();
    Object.assign(this.context, cloneStructured(fields));
    return this;
  }

  /** Set structured error. Chainable. */
  setError(err: Error | unknown): this {
    this.assertMutable();
    this.level = 'error';
    this.status = 'error';
    this.error = normalizeError(err);
    return this;
  }

  private buildPayload(overrides?: Partial<Pick<WideEventPayload, 'level' | 'status'>>): WideEventPayload {
    const timestamp_end = new Date();
    const payload: WideEventPayload = {
      event_id: this.event_id,
      service: this.service,
      environment: this.environment,
      timestamp_start: this.timestamp_start.toISOString(),
      timestamp_end: timestamp_end.toISOString(),
      duration_ms: Number((performance.now() - this.start_monotonic_ms).toFixed(3)),
      level: overrides?.level ?? this.level,
      operation: this.operation,
      status: overrides?.status ?? this.status,
      context: cloneStructured(this.context),
      error: cloneStructured(this.error),
      meta: { sdk_version: SDK_VERSION, sdk_language: 'typescript' },
    };

    return deepFreeze(payload);
  }

  /** Finalizes the event and dispatches to all sinks. */
  async emit(overrides?: Partial<Pick<WideEventPayload, 'level' | 'status'>>): Promise<void> {
    if (this.emitPromise) {
      return this.emitPromise;
    }

    const payload = this.buildPayload(overrides);
    this.finalized = true;

    this.emitPromise = Promise.allSettled(
      this.sinks.map((sink) => Promise.resolve().then(() => sink.export(payload))),
    ).then(() => undefined);

    return this.emitPromise;
  }
}
