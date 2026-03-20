import { beforeEach, describe, expect, it, vi } from 'vitest';

import { SentrySink } from './SentrySink.js';

const sentry = vi.hoisted(() => ({
  captureException: vi.fn(),
  addBreadcrumb: vi.fn(),
  withScope: vi.fn((fn: (scope: any) => void) => {
    fn({
      setTag: vi.fn(),
      setExtras: vi.fn(),
      setLevel: vi.fn(),
    });
  }),
}));

vi.mock('@sentry/node', () => sentry);

beforeEach(() => {
  sentry.captureException.mockClear();
  sentry.addBreadcrumb.mockClear();
  sentry.withScope.mockClear();
});

describe('SentrySink', () => {
  it('captures errors with tags and extras', async () => {
    const sink = new SentrySink();
    await sink.export({
      event_id: '2f64f7ef-efc9-4b9f-8d70-1a0f1e4b6f2d',
      service: 'svc',
      environment: 'production',
      timestamp_start: '2026-03-19T00:00:00.000Z',
      timestamp_end: '2026-03-19T00:00:00.500Z',
      duration_ms: 500,
      level: 'error',
      operation: 'job.fail',
      status: 'error',
      context: { attempt: 1 },
      error: { type: 'Error', message: 'boom', stack: 'stack' },
      meta: { sdk_version: '0.1.0', sdk_language: 'typescript' },
    });

    expect(sentry.withScope).toHaveBeenCalledTimes(1);
    expect(sentry.captureException).toHaveBeenCalledTimes(1);
    expect(sentry.addBreadcrumb).not.toHaveBeenCalled();
  });

  it('adds breadcrumbs for non-error events', async () => {
    const sink = new SentrySink();
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

    expect(sentry.addBreadcrumb).toHaveBeenCalledTimes(1);
    expect(sentry.captureException).not.toHaveBeenCalled();
  });
});
