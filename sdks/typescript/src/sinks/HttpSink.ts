import type { Sink } from './Sink.js';
import type { HttpSinkOptions, WideEventPayload } from '../types.js';

const DEFAULT_TIMEOUT_MS = 5000;

export class HttpSink implements Sink {
  private readonly url: string;
  private readonly headers: Record<string, string>;
  private readonly timeoutMs: number;

  constructor(options: HttpSinkOptions = {}) {
    const resolvedUrl = options.url ?? process.env.OPENAKTA_SINK_URL;
    if (!resolvedUrl) {
      throw new Error('HttpSink requires a url option or OPENAKTA_SINK_URL');
    }

    const token = options.token ?? process.env.OPENAKTA_SINK_TOKEN;
    this.url = resolvedUrl;
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    this.headers = {
      'Content-Type': 'application/json',
      ...(options.headers ?? {}),
    };

    if (token && this.headers.Authorization === undefined) {
      this.headers.Authorization = `Bearer ${token}`;
    }
  }

  async export(event: WideEventPayload): Promise<void> {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), this.timeoutMs);

    try {
      const response = await fetch(this.url, {
        method: 'POST',
        headers: this.headers,
        body: JSON.stringify(event),
        signal: controller.signal,
      });

      if (!response.ok) {
        const details = await response.text().catch(() => '');
        throw new Error(`HTTP ${response.status} ${response.statusText}${details ? `: ${details}` : ''}`);
      }
    } finally {
      clearTimeout(timer);
    }
  }
}
