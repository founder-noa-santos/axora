import { describe, expect, it, vi } from 'vitest';

import { OtelSink } from './OtelSink.js';

describe('OtelSink', () => {
  it('maps the canonical wide event into a log record', async () => {
    const emit = vi.fn();
    const provider = {
      getLogger: vi.fn().mockReturnValue({ emit }),
    };

    const sink = new OtelSink({ provider });
    await sink.export({
      event_id: '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
      service: 'svc',
      environment: 'production',
      timestamp_start: '2026-03-19T00:00:00.000Z',
      timestamp_end: '2026-03-19T00:00:00.500Z',
      duration_ms: 500,
      level: 'warn',
      operation: 'job.run',
      status: 'timeout',
      context: { attempt: 2 },
      error: { type: 'TimeoutError', message: 'slow', stack: 'stack' },
      meta: { sdk_version: '0.1.0', sdk_language: 'typescript' },
    });

    expect(provider.getLogger).toHaveBeenCalledWith('axora-logger', '0.1.0');
    expect(emit).toHaveBeenCalledTimes(1);
    expect(emit.mock.calls[0][0]).toMatchObject({
      severityText: 'WARN',
      body: 'job.run',
      attributes: {
        'axora.event_id': '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
        'axora.operation': 'job.run',
        'axora.status': 'timeout',
        'axora.duration_ms': 500,
        'axora.ctx.attempt': 2,
        'exception.type': 'TimeoutError',
        'exception.message': 'slow',
        'service.name': 'svc',
        'deployment.environment.name': 'production',
      },
    });
  });
});
