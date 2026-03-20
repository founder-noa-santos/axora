import { describe, expect, it, vi } from 'vitest';

import { HttpSink } from '../HttpSink.js';

describe('HttpSink', () => {
  it('uses explicit options before env fallbacks and sends the canonical payload', async () => {
    vi.stubEnv('AXORA_SINK_URL', 'https://env.invalid/logs');
    vi.stubEnv('AXORA_SINK_TOKEN', 'env-token');

    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      statusText: 'OK',
      text: vi.fn().mockResolvedValue(''),
    });
    vi.stubGlobal('fetch', fetchMock);

    const sink = new HttpSink({
      url: 'https://example.invalid/logs',
      token: 'explicit-token',
      headers: { 'X-Test': 'yes' },
      timeoutMs: 100,
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
      context: {},
      error: { type: null, message: null, stack: null },
      meta: { sdk_version: '0.1.0', sdk_language: 'typescript' },
    });

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0];
    expect(url).toBe('https://example.invalid/logs');
    expect(init).toMatchObject({
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: 'Bearer explicit-token',
        'X-Test': 'yes',
      },
    });
  });
});
