using Axora.Logger.Sinks;

namespace Axora.Logger.Sentry;

public interface ISentryScopeBridge
{
    void SetTag(string key, string value);
    void SetExtras(IDictionary<string, object?> extras);
    void SetLevel(string level);
}

public interface ISentryBridge
{
    void WithScope(Action<ISentryScopeBridge> callback);
    void CaptureException(Exception exception);
    void AddBreadcrumb(string category, string message, string level, DateTimeOffset timestamp, IDictionary<string, object?> data);
}

public sealed class SentrySink : ISink
{
    private readonly ISentryBridge _bridge;

    public SentrySink(ISentryBridge bridge)
    {
        _bridge = bridge;
    }

    public Task ExportAsync(WideEventPayload @event)
    {
        if (@event.Status is "error" or "timeout")
        {
            _bridge.WithScope(scope =>
            {
                scope.SetTag("service", @event.Service);
                scope.SetTag("environment", @event.Environment);
                scope.SetTag("operation", @event.Operation);
                scope.SetTag("axora.event_id", @event.EventId);
                scope.SetExtras(@event.Context);
                scope.SetLevel(@event.Level == "fatal" ? "fatal" : @event.Level);
            });

            var error = new InvalidOperationException(@event.Error.Message ?? @event.Operation);
            _bridge.CaptureException(error);
            return Task.CompletedTask;
        }

        var data = new Dictionary<string, object?>(@event.Context)
        {
            ["duration_ms"] = @event.DurationMs,
            ["axora_event_id"] = @event.EventId,
        };

        _bridge.AddBreadcrumb(
            @event.Operation,
            @event.Operation,
            @event.Level,
            DateTimeOffset.Parse(@event.TimestampStart),
            data);

        return Task.CompletedTask;
    }
}
