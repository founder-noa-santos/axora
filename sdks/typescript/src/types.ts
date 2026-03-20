export type LogLevel = 'info' | 'warn' | 'error' | 'fatal';
export type EventStatus = 'ok' | 'error' | 'timeout' | 'cancelled';
export type EnvironmentName = 'production' | 'staging' | 'development';

export interface WideEventPayload {
  event_id: string;
  service: string;
  environment: EnvironmentName;
  timestamp_start: string;
  timestamp_end: string;
  duration_ms: number;
  level: LogLevel;
  operation: string;
  status: EventStatus;
  context: Record<string, unknown>;
  error: {
    type: string | null;
    message: string | null;
    stack: string | null;
  };
  meta: {
    sdk_version: string;
    sdk_language: string;
  };
}

export interface SinkConfig {
  flush?: 'sync' | 'async';
}

export interface LoggerOptions {
  service?: string;
  environment?: EnvironmentName;
  sinks?: Sink[];
  defaultContext?: Record<string, unknown>;
}

export interface HttpSinkOptions {
  url?: string;
  headers?: Record<string, string>;
  timeoutMs?: number;
  token?: string;
}

export interface Sink {
  export(event: WideEventPayload): Promise<void>;
  flush?(): Promise<void>;
}
