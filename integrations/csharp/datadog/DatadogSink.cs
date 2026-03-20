using System.Text.Json;
using Axora.Logger.Sinks;

namespace Axora.Logger.Datadog;

public sealed class DatadogSink : ISink
{
    public Task ExportAsync(WideEventPayload @event)
    {
        var logEntry = new Dictionary<string, object?>
        {
            ["date"] = @event.TimestampStart,
            ["status"] = @event.Level,
            ["service"] = @event.Service,
            ["message"] = @event.Operation,
            ["duration"] = @event.DurationMs,
            ["dd.axora_event_id"] = @event.EventId,
            ["dd.env"] = @event.Environment,
        };

        foreach (var (key, value) in @event.Context)
        {
            logEntry[key] = value;
        }

        if (@event.Error.Message is not null)
        {
            logEntry["error"] = new Dictionary<string, object?>
            {
                ["kind"] = @event.Error.Type,
                ["message"] = @event.Error.Message,
                ["stack"] = @event.Error.Stack,
            };
        }

        Console.Out.WriteLine(JsonSerializer.Serialize(logEntry));
        return Task.CompletedTask;
    }
}
