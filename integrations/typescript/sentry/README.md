# @axora/logger-sentry

Sentry adapter for AXORA Wide Events.

## Install

```bash
pnpm add @axora/logger-sentry @sentry/node
```

## Usage

```typescript
import { Logger } from '@axora/logger-core';
import { SentrySink } from '@axora/logger-sentry';

const logger = new Logger({
  service: 'my-api',
  sinks: [new SentrySink()],
});
```
