# @openakta/logger-sentry

Sentry adapter for OPENAKTA Wide Events.

## Install

```bash
pnpm add @openakta/logger-sentry @sentry/node
```

## Usage

```typescript
import { Logger } from '@openakta/logger-core';
import { SentrySink } from '@openakta/logger-sentry';

const logger = new Logger({
  service: 'my-api',
  sinks: [new SentrySink()],
});
```
