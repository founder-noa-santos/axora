import type { Sink, WideEventPayload } from '@axora/logger-core';
import * as Sentry from '@sentry/node';

function severityLevel(level: WideEventPayload['level']): Sentry.SeverityLevel {
  return level as Sentry.SeverityLevel;
}

export class SentrySink implements Sink {
  async export(event: WideEventPayload): Promise<void> {
    const isError = event.status === 'error' || event.status === 'timeout';

    if (isError) {
      Sentry.withScope((scope) => {
        scope.setTag('service', event.service);
        scope.setTag('environment', event.environment);
        scope.setTag('operation', event.operation);
        scope.setTag('axora.event_id', event.event_id);
        scope.setExtras(event.context);
        scope.setLevel(event.level === 'fatal' ? 'fatal' : severityLevel(event.level));

        const error = new Error(event.error.message ?? event.operation);
        error.name = event.error.type ?? 'AxoraError';
        if (event.error.stack) {
          error.stack = event.error.stack;
        }

        Sentry.captureException(error);
      });
      return;
    }

    Sentry.addBreadcrumb({
      category: event.operation,
      message: event.operation,
      level: severityLevel(event.level),
      data: {
        ...event.context,
        duration_ms: event.duration_ms,
        axora_event_id: event.event_id,
      },
      timestamp: Date.parse(event.timestamp_start) / 1000,
    });
  }
}
