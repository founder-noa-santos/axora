# @axora/logger-core

Canonical AXORA Wide Event SDK for TypeScript.

## Install

```bash
pnpm add @axora/logger-core
```

## Usage

```typescript
import { Logger, ConsoleSink } from '@axora/logger-core';

const logger = new Logger({
  service: 'my-api',
  environment: 'production',
  sinks: [new ConsoleSink()],
  defaultContext: { region: 'eu-west-1' },
});

const event = logger.startEvent('user.login');
event.appendContext({ user_id: 'usr_123', method: 'oauth2' });
await event.emit({ status: 'ok' });
```
