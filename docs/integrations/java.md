# Java Integration Guide

## Modules

| Module | Transport | Notes |
| --- | --- | --- |
| `com.openakta.logger.otel` | OpenTelemetry logs | Bridge-based adapter for a configured provider |
| `com.openakta.logger.sentry` | Sentry errors and breadcrumbs | Uses scope and capture bridges |
| `com.openakta.logger.datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `com.openakta.logger.posthog` | PostHog capture events | Uses `distinct_id` plus flush support |

## Initialization

```java
import com.openakta.logger.Logger;
import com.openakta.logger.sinks.ConsoleSink;

Logger logger = Logger.builder()
    .service("billing-api")
    .addSink(new ConsoleSink())
    .build();
```

Java integration modules depend on the published `com.openakta:logger-core` artifact. They are thin adapters and should not re-implement lifecycle logic.
