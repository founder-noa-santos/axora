# @axora/logger-otel

OpenTelemetry adapter for AXORA Wide Events.

## Install

```bash
pnpm add @axora/logger-otel @opentelemetry/api-logs @opentelemetry/sdk-logs
```

## Usage

```typescript
import { Logger } from '@axora/logger-core';
import { OtelSink } from '@axora/logger-otel';

const logger = new Logger({
  service: 'my-api',
  sinks: [new OtelSink({ provider })],
});
```
