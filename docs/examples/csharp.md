# C# Example

```csharp
using Axora.Logger;
using Axora.Logger.Sinks;

var logger = new LoggerBuilder()
    .Service("billing-api")
    .Environment("production")
    .AddSink(new ConsoleSink())
    .DefaultContext(new Dictionary<string, object?>
    {
        ["region"] = "eu-west-1",
    })
    .Build();

var wideEvent = logger.StartEvent("user.login");
wideEvent.AppendContext(new Dictionary<string, object?>
{
    ["user_id"] = "usr_123",
    ["method"] = "oauth2",
});
await wideEvent.EmitAsync(status: "ok");

await logger.TraceAsync("payment.capture", async traced =>
{
    traced.AppendContext(new Dictionary<string, object?>
    {
        ["amount"] = 99.99,
        ["currency"] = "EUR",
    });

    return await ProcessPayment();
});
```

The C# SDK uses `LoggerBuilder` for named configuration and `TraceAsync` for automatic lifecycle finalization.
