import { afterEach, describe, expect, it, vi } from 'vitest';

import { Logger, WideEvent } from '../index.js';
import type { Sink, WideEventPayload } from '../types.js';

class CaptureSink implements Sink {
  public readonly events: WideEventPayload[] = [];

  async export(event: WideEventPayload): Promise<void> {
    this.events.push(event);
  }
}

afterEach(() => {
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

describe('Logger core', () => {
  it('snapshots context, seals the event, and emits only once', async () => {
    const sink = new CaptureSink();
    const logger = new Logger({
      service: 'my-api',
      environment: 'staging',
      sinks: [sink],
      defaultContext: { region: 'eu-west-1' },
    });

    const details = { step: 1 };
    const event = logger.startEvent('user.login');
    event.appendContext({ details });

    await event.emit();
    details.step = 2;

    expect(sink.events).toHaveLength(1);
    expect(sink.events[0]).toMatchObject({
      service: 'my-api',
      environment: 'staging',
      operation: 'user.login',
      status: 'ok',
      context: {
        region: 'eu-west-1',
        details: { step: 1 },
      },
    });
    expect(Object.isFrozen(sink.events[0])).toBe(true);

    await event.emit({ status: 'timeout' });
    expect(sink.events).toHaveLength(1);

    expect(() => event.appendContext({ later: true })).toThrow(/finalized/i);
    expect(() => event.setError(new Error('nope'))).toThrow(/finalized/i);
  });

  it('falls back to environment variables when service and environment are omitted', async () => {
    vi.stubEnv('OPENAKTA_SERVICE', 'env-service');
    vi.stubEnv('OPENAKTA_ENV', 'development');

    const sink = new CaptureSink();
    const logger = new Logger({ sinks: [sink] });
    await logger.trace('task.run', async () => undefined);

    expect(sink.events[0]).toMatchObject({
      service: 'env-service',
      environment: 'development',
      operation: 'task.run',
      status: 'ok',
    });
  });

  it('swallows sink failures and still resolves the trace wrapper', async () => {
    const sink = {
      async export() {
        throw new Error('sink failed');
      },
    } satisfies Sink;

    const logger = new Logger({ service: 'svc', sinks: [sink] });
    await expect(
      logger.trace('job.run', async (event) => {
        event.appendContext({ work: 'done' });
        return 42;
      }),
    ).resolves.toBe(42);
  });

  it('captures errors and emits an error payload from trace', async () => {
    const sink = new CaptureSink();
    const logger = new Logger({ service: 'svc', sinks: [sink] });

    await expect(
      logger.trace('job.fail', async (event) => {
        event.appendContext({ attempt: 1 });
        throw new Error('boom');
      }),
    ).rejects.toThrow('boom');

    expect(sink.events[0]).toMatchObject({
      level: 'error',
      status: 'error',
      error: {
        type: 'Error',
        message: 'boom',
      },
      context: {
        attempt: 1,
      },
    });
  });
});
