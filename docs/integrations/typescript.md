# TypeScript Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `@axora/logger-otel` | OpenTelemetry logs | Requires a configured `LoggerProvider` |
| `@axora/logger-sentry` | Sentry errors and breadcrumbs | Uses `withScope` and `captureException` |
| `@axora/logger-datadog` | Datadog stdout JSON | Ships canonical JSON lines in v1 |
| `@axora/logger-posthog` | PostHog capture events | Uses `distinctId` plus `shutdown()` |

## Initialization

```typescript
import { Logger, ConsoleSink } from '@axora/logger-core';
import { OtelSink } from '@axora/logger-otel';
import { SentrySink } from '@axora/logger-sentry';
import { DatadogSink } from '@axora/logger-datadog';
import { PosthogSink } from '@axora/logger-posthog';

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
