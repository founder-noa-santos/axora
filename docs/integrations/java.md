# Java Integration Guide

## Modules

| Module | Transport | Notes |
| --- | --- | --- |
| `com.axora.logger.otel` | OpenTelemetry logs | Bridge-based adapter for a configured provider |
| `com.axora.logger.sentry` | Sentry errors and breadcrumbs | Uses scope and capture bridges |
| `com.axora.logger.datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `com.axora.logger.posthog` | PostHog capture events | Uses `distinct_id` plus flush support |

## Initialization

```java
import com.axora.logger.Logger;
import com.axora.logger.sinks.ConsoleSink;

Logger logger = Logger.builder()
    .service("billing-api")
    .addSink(new ConsoleSink())
    .build();
```

Java integration modules depend on the published `com.axora:logger-core` artifact. They are thin adapters and should not re-implement lifecycle logic.
