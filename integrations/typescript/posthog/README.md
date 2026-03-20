# @axora/logger-posthog

PostHog adapter for AXORA Wide Events.

## Install

```bash
pnpm add @axora/logger-posthog posthog-node
```

## Usage

```typescript
import { Logger } from '@axora/logger-core';
import { PosthogSink } from '@axora/logger-posthog';

const logger = new Logger({
  service: 'my-api',
  sinks: [new PosthogSink({ client })],
});
```
