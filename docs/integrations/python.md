# Python Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `axora-logger-otel` | OpenTelemetry logs | Accepts an SDK logger or provider object |
| `axora-logger-sentry` | Sentry errors and breadcrumbs | Uses the Sentry scope API |
| `axora-logger-datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `axora-logger-posthog` | PostHog capture events | Uses `distinct_id` plus `shutdown()` |

## Initialization

```python
from axora_logger import Logger
from axora_logger_otel import OtelSink
from axora_logger_sentry import SentrySink
from axora_logger_datadog import DatadogSink
from axora_logger_posthog import PosthogSink

logger = Logger(
    service="billing-api",
    sinks=[OtelSink(provider), SentrySink(), DatadogSink(), PosthogSink(client)],
)
```

Adapters are additive. They translate the finalized Wide Event payload into the vendor transport without changing the core lifecycle.
