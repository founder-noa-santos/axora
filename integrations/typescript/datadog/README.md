# @axora/logger-datadog

Datadog adapter for AXORA Wide Events.

## Install

```bash
pnpm add @axora/logger-datadog dd-trace
```

## Usage

```typescript
import { Logger } from '@axora/logger-core';
import { DatadogSink } from '@axora/logger-datadog';

const logger = new Logger({
  service: 'my-api',
  sinks: [new DatadogSink()],
});
```
