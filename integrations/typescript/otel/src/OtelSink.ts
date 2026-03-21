import type { Sink, WideEventPayload } from '@openakta/logger-core';
import { SeverityNumber } from '@opentelemetry/api-logs';

export interface OtelLoggerLike {
  emit(record: Record<string, unknown>): void;
}

export interface OtelLoggerProviderLike {
  getLogger(name: string, version?: string): OtelLoggerLike;
}

export interface OtelSinkOptions {
  provider: OtelLoggerProviderLike;
  loggerName?: string;
}

const SEVERITY_MAP: Record<WideEventPayload['level'], SeverityNumber> = {
  info: SeverityNumber.INFO,
  warn: SeverityNumber.WARN,
  error: SeverityNumber.ERROR,
  fatal: SeverityNumber.FATAL,
};

export class OtelSink implements Sink {
  private readonly logger: OtelLoggerLike;

  constructor(options: OtelSinkOptions) {
    this.logger = options.provider.getLogger(options.loggerName ?? 'openakta-logger', '0.1.0');
  }

  async export(event: WideEventPayload): Promise<void> {
    const attributes: Record<string, unknown> = {
      'openakta.event_id': event.event_id,
      'openakta.operation': event.operation,
      'openakta.status': event.status,
      'openakta.duration_ms': event.duration_ms,
      'service.name': event.service,
      'deployment.environment.name': event.environment,
    };

    for (const [key, value] of Object.entries(event.context)) {
      attributes[`openakta.ctx.${key}`] = value;
    }

    if (event.error.message) {
      attributes['exception.type'] = event.error.type;
      attributes['exception.message'] = event.error.message;
      attributes['exception.stacktrace'] = event.error.stack;
    }

    this.logger.emit({
      severityNumber: SEVERITY_MAP[event.level] ?? SeverityNumber.INFO,
      severityText: event.level.toUpperCase(),
      body: event.operation,
      attributes,
      timestamp: Date.parse(event.timestamp_end) * 1_000_000,
      observedTimestamp: Date.parse(event.timestamp_start) * 1_000_000,
      resource: {
        attributes: {
          'service.name': event.service,
          'deployment.environment.name': event.environment,
        },
      },
    });
  }
}
