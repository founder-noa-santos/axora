# @openakta/logger-otel

OpenTelemetry adapter for OPENAKTA Wide Events.

## Install

```bash
pnpm add @openakta/logger-otel @opentelemetry/api-logs @opentelemetry/sdk-logs
```

## Usage

```typescript
import { Logger } from '@openakta/logger-core';
import { OtelSink } from '@openakta/logger-otel';

const logger = new Logger({
  service: 'my-api',
  sinks: [new OtelSink({ provider })],
});
```
