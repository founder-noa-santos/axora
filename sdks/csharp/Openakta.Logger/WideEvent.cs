using System.Collections.Concurrent;
using System.Diagnostics;
using System.Text.Json;
using Openakta.Logger.Sinks;

namespace Openakta.Logger;

public sealed class WideEvent
{
    private const string SdkVersion = "0.1.0";

    private readonly string _eventId = Guid.NewGuid().ToString();
    private readonly DateTimeOffset _timestampStart = DateTimeOffset.UtcNow;
    private readonly Stopwatch _stopwatch = Stopwatch.StartNew();
    private readonly string _operation;
    private readonly string _service;
    private readonly string _environment;
    private readonly IReadOnlyList<ISink> _sinks;
    private readonly ConcurrentDictionary<string, object?> _context = new();
    private WideEventPayload.ErrorDetail _error = new(null, null, null);
    private string _level = "info";
    private string _status = "ok";
    private bool _finalized;
    private Task? _emitTask;

    internal WideEvent(string operation, string service, string environment, IReadOnlyList<ISink> sinks)
    {
        _operation = operation;
        _service = service;
        _environment = environment;
        _sinks = sinks;
    }

    private void EnsureMutable()
    {
        if (_finalized)
        {
            throw new InvalidOperationException("WideEvent has already been finalized");
        }
    }

    public WideEvent AppendContext(IDictionary<string, object?> fields)
    {
        EnsureMutable();
        foreach (var (key, value) in fields)
        {
            _context[key] = CloneValue(value);
        }

        return this;
    }

    public WideEvent SetError(Exception ex)
    {
        EnsureMutable();
        _level = "error";
        _status = "error";
        _error = new WideEventPayload.ErrorDetail(ex.GetType().Name, ex.Message, ex.StackTrace);
        return this;
    }

    private static object? ConvertJsonElement(JsonElement element)
    {
        return element.ValueKind switch
        {
            JsonValueKind.Object => element.EnumerateObject().ToDictionary(property => property.Name, property => ConvertJsonElement(property.Value)),
            JsonValueKind.Array => element.EnumerateArray().Select(ConvertJsonElement).ToList(),
            JsonValueKind.String => element.GetString(),
            JsonValueKind.Number when element.TryGetInt64(out var intValue) => intValue,
            JsonValueKind.Number when element.TryGetDecimal(out var decimalValue) => decimalValue,
            JsonValueKind.Number => element.GetDouble(),
            JsonValueKind.True => true,
            JsonValueKind.False => false,
            JsonValueKind.Null => null,
            _ => element.GetRawText(),
        };
    }

    private static object? CloneValue(object? value)
    {
        if (value is null)
        {
            return null;
        }

        using var document = JsonDocument.Parse(JsonSerializer.Serialize(value));
        return ConvertJsonElement(document.RootElement);
    }

    private static Dictionary<string, object?> CloneContext(IDictionary<string, object?> fields)
    {
        var cloned = new Dictionary<string, object?>();
        foreach (var (key, value) in fields)
        {
            cloned[key] = CloneValue(value);
        }

        return cloned;
    }

    private WideEventPayload BuildPayload(string? level = null, string? status = null)
    {
        var endTime = DateTimeOffset.UtcNow;
        return new WideEventPayload
        {
            EventId = _eventId,
            Service = _service,
            Environment = _environment,
            TimestampStart = _timestampStart.ToString("O"),
            TimestampEnd = endTime.ToString("O"),
            DurationMs = Math.Round(_stopwatch.Elapsed.TotalMilliseconds, 3, MidpointRounding.AwayFromZero),
            Level = level ?? _level,
            Operation = _operation,
            Status = status ?? _status,
            Context = CloneContext(_context),
            Error = _error,
            Meta = new WideEventPayload.SdkMeta(SdkVersion, "csharp"),
        };
    }

    private Task SafeExport(ISink sink, WideEventPayload payload)
    {
        try
        {
            return sink.ExportAsync(payload).ContinueWith(_ => { }, TaskContinuationOptions.ExecuteSynchronously);
        }
        catch
        {
            return Task.CompletedTask;
        }
    }

    public Task EmitAsync(string? level = null, string? status = null)
    {
        if (_emitTask is not null)
        {
            return _emitTask;
        }

        var payload = BuildPayload(level, status);
        _finalized = true;
        _emitTask = Task.WhenAll(_sinks.Select(sink => SafeExport(sink, payload)));
        return _emitTask;
    }
}
