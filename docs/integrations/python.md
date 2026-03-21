# Python Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `openakta-logger-otel` | OpenTelemetry logs | Accepts an SDK logger or provider object |
| `openakta-logger-sentry` | Sentry errors and breadcrumbs | Uses the Sentry scope API |
| `openakta-logger-datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `openakta-logger-posthog` | PostHog capture events | Uses `distinct_id` plus `shutdown()` |

## Initialization

```python
from openakta_logger import Logger
from openakta_logger_otel import OtelSink
from openakta_logger_sentry import SentrySink
from openakta_logger_datadog import DatadogSink
from openakta_logger_posthog import PosthogSink

logger = Logger(
    service="billing-api",
    sinks=[OtelSink(provider), SentrySink(), DatadogSink(), PosthogSink(client)],
)
```

Adapters are additive. They translate the finalized Wide Event payload into the vendor transport without changing the core lifecycle.
