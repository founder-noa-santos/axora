# TypeScript Example

```typescript
import { Logger, ConsoleSink } from '@openakta/logger-core';
import { OtelSink } from '@openakta/logger-otel';

const logger = new Logger({
  service: 'billing-api',
  environment: 'production',
  sinks: [new ConsoleSink(), new OtelSink(provider)],
  defaultContext: { region: 'eu-west-1' },
});

const event = logger.startEvent('user.login');
event.appendContext({ user_id: 'usr_123', method: 'oauth2' });
await event.emit({ status: 'ok' });

await logger.trace('payment.capture', async (trace) => {
  trace.appendContext({ amount: 99.99, currency: 'EUR' });
  return await processPayment();
});
```

Use `trace()` when the operation already has a clear async boundary. Use `startEvent()` when you need to accumulate context across multiple steps before finalization.
