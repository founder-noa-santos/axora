using Axora.Logger.Sinks;

namespace Axora.Logger.Posthog;

public interface IPosthogClientLike
{
    void Capture(string distinctId, string eventName, IDictionary<string, object?> properties, DateTimeOffset timestamp);
    void Shutdown();
}

public sealed class PosthogSink : ISink
{
    private readonly IPosthogClientLike _client;

    public PosthogSink(IPosthogClientLike client)
    {
        _client = client;
    }

    public Task ExportAsync(WideEventPayload @event)
    {
        var distinctId = @event.Context.TryGetValue("user_id", out var userId) && userId is not null
            ? userId.ToString()!
            : $"service:{@event.Service}";

        var properties = new Dictionary<string, object?>(@event.Context)
        {
            ["axora_event_id"] = @event.EventId,
            ["axora_service"] = @event.Service,
            ["status"] = @event.Status,
            ["level"] = @event.Level,
            ["duration_ms"] = @event.DurationMs,
        };

        if (@event.Error.Message is not null)
        {
            properties["error_message"] = @event.Error.Message;
        }

        _client.Capture(distinctId, @event.Operation, properties, DateTimeOffset.Parse(@event.TimestampStart));
        return Task.CompletedTask;
    }

    public Task FlushAsync()
    {
        _client.Shutdown();
        return Task.CompletedTask;
    }
}
