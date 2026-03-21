import { describe, expect, it, vi } from 'vitest';

import { PosthogSink } from './PosthogSink.js';

describe('PosthogSink', () => {
  it('captures the wide event with the user id when available', async () => {
    const capture = vi.fn();
    const shutdown = vi.fn();
    const sink = new PosthogSink({
      client: { capture, shutdown },
    });

    await sink.export({
      event_id: '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
      service: 'svc',
      environment: 'production',
      timestamp_start: '2026-03-19T00:00:00.000Z',
      timestamp_end: '2026-03-19T00:00:00.500Z',
      duration_ms: 500,
      level: 'info',
      operation: 'job.run',
      status: 'ok',
      context: { user_id: 'usr_123', attempt: 2 },
      error: { type: null, message: null, stack: null },
      meta: { sdk_version: '0.1.0', sdk_language: 'typescript' },
    });

    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture.mock.calls[0][0]).toMatchObject({
      distinctId: 'usr_123',
      event: 'job.run',
      properties: {
        user_id: 'usr_123',
        attempt: 2,
        openakta_event_id: '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
        openakta_service: 'svc',
        status: 'ok',
        level: 'info',
        duration_ms: 500,
      },
    });

    await sink.flush();
    expect(shutdown).toHaveBeenCalledTimes(1);
  });
});
