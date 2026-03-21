# TypeScript Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `@openakta/logger-otel` | OpenTelemetry logs | Requires a configured `LoggerProvider` |
| `@openakta/logger-sentry` | Sentry errors and breadcrumbs | Uses `withScope` and `captureException` |
| `@openakta/logger-datadog` | Datadog stdout JSON | Ships canonical JSON lines in v1 |
| `@openakta/logger-posthog` | PostHog capture events | Uses `distinctId` plus `shutdown()` |

## Initialization

```typescript
import { Logger, ConsoleSink } from '@openakta/logger-core';
import { OtelSink } from '@openakta/logger-otel';
import { SentrySink } from '@openakta/logger-sentry';
import { DatadogSink } from '@openakta/logger-datadog';
import { PosthogSink } from '@openakta/logger-posthog';

const logger = new Logger({
  service: 'billing-api',
  sinks: [
    new ConsoleSink(),
    new OtelSink(provider),
    new SentrySink(),
    new DatadogSink(),
    new PosthogSink({ client }),
  ],
});
```

The adapter packages are sidecars. They depend on the core SDK, but they do not change the canonical payload shape.
