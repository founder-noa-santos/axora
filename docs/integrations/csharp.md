# C# Integration Guide

## Packages

| Package | Transport | Notes |
| --- | --- | --- |
| `Axora.Logger.Otel` | OpenTelemetry logs | Requires a configured provider |
| `Axora.Logger.Sentry` | Sentry errors and breadcrumbs | Uses scope and capture bridges |
| `Axora.Logger.Datadog` | Datadog stdout JSON | Emits canonical JSON lines in v1 |
| `Axora.Logger.Posthog` | PostHog capture events | Uses `distinct_id` plus shutdown support |

## Initialization

```csharp
using Axora.Logger;
using Axora.Logger.Sinks;

var logger = Logger.Builder()
    .Service("billing-api")
    .AddSink(new ConsoleSink())
    .Build();
```

C# integration packages reference `Axora.Logger` and the vendor SDK they bridge to. The final payload remains canonical in all cases.
