using System.Collections.Generic;
using Axora.Logger.Sinks;

namespace Axora.Logger.Otel;

public interface IOtelLoggerLike
{
    void Emit(IDictionary<string, object?> record);
}

public interface IOtelLoggerProviderLike
{
    IOtelLoggerLike GetLogger(string name, string version);
}

public sealed class OtelSink : ISink
{
    private readonly IOtelLoggerLike _logger;

    public OtelSink(IOtelLoggerProviderLike provider)
    {
        _logger = provider.GetLogger("axora-logger", "0.1.0");
    }

    public Task ExportAsync(WideEventPayload @event)
    {
        var attributes = new Dictionary<string, object?>
        {
            ["axora.event_id"] = @event.EventId,
            ["axora.operation"] = @event.Operation,
            ["axora.status"] = @event.Status,
            ["axora.duration_ms"] = @event.DurationMs,
            ["service.name"] = @event.Service,
            ["deployment.environment.name"] = @event.Environment,
        };

        foreach (var (key, value) in @event.Context)
        {
            attributes[$"axora.ctx.{key}"] = value;
        }

        if (@event.Error.Message is not null)
        {
            attributes["exception.type"] = @event.Error.Type;
            attributes["exception.message"] = @event.Error.Message;
            attributes["exception.stacktrace"] = @event.Error.Stack;
        }

        _logger.Emit(
            new Dictionary<string, object?>
            {
                ["severityNumber"] = SeverityNumber(@event.Level),
                ["severityText"] = @event.Level.ToUpperInvariant(),
                ["body"] = @event.Operation,
                ["attributes"] = attributes,
                ["timestamp"] = DateTimeOffset.Parse(@event.TimestampEnd).ToUnixTimeMilliseconds() * 1_000_000,
                ["observedTimestamp"] = DateTimeOffset.Parse(@event.TimestampStart).ToUnixTimeMilliseconds() * 1_000_000,
                ["resource"] = new Dictionary<string, object?>
                {
                    ["attributes"] = new Dictionary<string, object?>
                    {
                        ["service.name"] = @event.Service,
                        ["deployment.environment.name"] = @event.Environment,
                    },
                },
            });

        return Task.CompletedTask;
    }

    private static int SeverityNumber(string level) => level switch
    {
        "info" => 9,
        "warn" => 13,
        "error" => 17,
        "fatal" => 21,
        _ => 9,
    };
}
