# @openakta/logger-datadog

Datadog adapter for OPENAKTA Wide Events.

## Install

```bash
pnpm add @openakta/logger-datadog dd-trace
```

## Usage

```typescript
import { Logger } from '@openakta/logger-core';
import { DatadogSink } from '@openakta/logger-datadog';

const logger = new Logger({
  service: 'my-api',
  sinks: [new DatadogSink()],
});
```
