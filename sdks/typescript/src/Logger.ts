import type { LoggerOptions, Sink } from './types.js';
import { cloneStructured, normalizeEnvironment } from './internal/snapshot.js';
import { WideEvent } from './WideEvent.js';

function resolveService(service?: string): string {
  const resolved = service?.trim() || process.env.OPENAKTA_SERVICE?.trim();
  if (!resolved) {
    throw new Error('Logger requires a service name or OPENAKTA_SERVICE');
  }

  return resolved;
}

function resolveEnvironment(environment?: string): string {
  return normalizeEnvironment(environment ?? process.env.OPENAKTA_ENV);
}

export class Logger {
  private readonly service: string;
  private readonly environment: string;
  private readonly sinks: readonly Sink[];
  private readonly defaultContext: Record<string, unknown>;

  constructor(options: LoggerOptions) {
    this.service = resolveService(options.service);
    this.environment = resolveEnvironment(options.environment);
    this.sinks = [...(options.sinks ?? [])];
    this.defaultContext = cloneStructured(options.defaultContext ?? {});
  }

  /** Create a new WideEvent lifecycle for one operation. */
  startEvent(operation: string): WideEvent {
    const event = new WideEvent(operation, this.service, this.environment, [...this.sinks]);
    if (Object.keys(this.defaultContext).length > 0) {
      event.appendContext(this.defaultContext);
    }
    return event;
  }

  /** Wrap an async or sync function in a WideEvent lifecycle automatically. */
  async trace<T>(
    operation: string,
    fn: (event: WideEvent) => Promise<T> | T,
  ): Promise<T> {
    const event = this.startEvent(operation);

    try {
      const result = await fn(event);
      await event.emit({ status: 'ok' });
      return result;
    } catch (err) {
      event.setError(err);
      await event.emit();
      throw err;
    }
  }
}
