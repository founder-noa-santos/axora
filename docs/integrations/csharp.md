# C# Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `Openakta.Logger.Otel` | OpenTelemetry logs | Requires a configured provider |
| `Openakta.Logger.Sentry` | Sentry errors and breadcrumbs | Uses scope and capture bridges |
| `Openakta.Logger.Datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `Openakta.Logger.Posthog` | PostHog capture events | Uses `distinct_id` plus shutdown support |

## Initialization

```csharp
using Openakta.Logger;
using Openakta.Logger.Sinks;

var logger = Logger.Builder()
    .Service("billing-api")
    .AddSink(new ConsoleSink())
    .Build();
```

C# integration packages reference `Openakta.Logger` and the vendor SDK they bridge to. The final payload remains canonical in all cases.
