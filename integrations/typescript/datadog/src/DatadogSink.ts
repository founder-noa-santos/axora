import type { Sink, WideEventPayload } from '@axora/logger-core';

export class DatadogSink implements Sink {
  async export(event: WideEventPayload): Promise<void> {
    const logEntry: Record<string, unknown> = {
      date: event.timestamp_start,
      status: event.level,
      service: event.service,
      message: event.operation,
      duration: event.duration_ms,
      ...event.context,
      'dd.axora_event_id': event.event_id,
      'dd.env': event.environment,
    };

    if (event.error.message) {
      logEntry.error = {
        kind: event.error.type,
        message: event.error.message,
        stack: event.error.stack,
      };
    }

    process.stdout.write(`${JSON.stringify(logEntry)}\n`);
  }
}
