import type { Sink, WideEventPayload } from '@axora/logger-core';

export interface PosthogClientLike {
  capture(payload: {
    distinctId: string;
    event: string;
    timestamp: Date;
    properties: Record<string, unknown>;
  }): void;
  shutdown(): Promise<void> | void;
}

export interface PosthogSinkOptions {
  client: PosthogClientLike;
}

export class PosthogSink implements Sink {
  constructor(private readonly options: PosthogSinkOptions) {}

  async export(event: WideEventPayload): Promise<void> {
    const distinctId = (event.context.user_id as string | undefined) ?? `service:${event.service}`;

    this.options.client.capture({
      distinctId,
      event: event.operation,
      timestamp: new Date(event.timestamp_start),
      properties: {
        ...event.context,
        axora_event_id: event.event_id,
        axora_service: event.service,
        status: event.status,
        level: event.level,
        duration_ms: event.duration_ms,
        ...(event.error.message ? { error_message: event.error.message } : {}),
      },
    });
  }

  async flush(): Promise<void> {
    await this.options.client.shutdown();
  }
}
