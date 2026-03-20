import { describe, expect, it, vi } from 'vitest';

import { DatadogSink } from './DatadogSink.js';

describe('DatadogSink', () => {
  it('writes the wide event as a single JSON log line', async () => {
    const write = vi.spyOn(process.stdout, 'write').mockReturnValue(true as never);
    const sink = new DatadogSink();

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
      context: { attempt: 2 },
      error: { type: null, message: null, stack: null },
      meta: { sdk_version: '0.1.0', sdk_language: 'typescript' },
    });

    expect(write).toHaveBeenCalledTimes(1);
    const output = String(write.mock.calls[0][0]);
    expect(JSON.parse(output)).toMatchObject({
      date: '2026-03-19T00:00:00.000Z',
      status: 'info',
      service: 'svc',
      message: 'job.run',
      duration: 500,
      attempt: 2,
      'dd.axora_event_id': '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
      'dd.env': 'production',
    });
  });
});
