# @openakta/logger-posthog

PostHog adapter for OPENAKTA Wide Events.

## Install

```bash
pnpm add @openakta/logger-posthog posthog-node
```

## Usage

```typescript
import { Logger } from '@openakta/logger-core';
import { PosthogSink } from '@openakta/logger-posthog';

const logger = new Logger({
  service: 'my-api',
  sinks: [new PosthogSink({ client })],
});
```
