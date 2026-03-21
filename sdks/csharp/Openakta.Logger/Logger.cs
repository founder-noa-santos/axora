using Openakta.Logger.Sinks;

namespace Openakta.Logger;

public sealed class Logger
{
    private readonly string _service;
    private readonly string _environment;
    private readonly IReadOnlyList<ISink> _sinks;
    private readonly Dictionary<string, object?> _defaultContext;

    public Logger(LoggerOptions options)
    {
        _service = ResolveService(options.Service);
        _environment = ResolveEnvironment(options.Environment);
        _sinks = options.Sinks?.ToList() ?? [];
        _defaultContext = new Dictionary<string, object?>(options.DefaultContext ?? new Dictionary<string, object?>());
    }

    internal Logger(string service, string environment, IReadOnlyList<ISink> sinks, IReadOnlyDictionary<string, object?> defaultContext)
    {
        _service = service;
        _environment = environment;
        _sinks = sinks;
        _defaultContext = new Dictionary<string, object?>(defaultContext);
    }

    public static LoggerBuilder Builder() => new();

    public WideEvent StartEvent(string operation)
    {
        var eventInstance = new WideEvent(operation, _service, _environment, _sinks);
        if (_defaultContext.Count > 0)
        {
            eventInstance.AppendContext(_defaultContext);
        }

        return eventInstance;
    }

    public async Task<T> TraceAsync<T>(string operation, Func<WideEvent, Task<T>> fn)
    {
        var eventInstance = StartEvent(operation);
        try
        {
            var result = await fn(eventInstance).ConfigureAwait(false);
            await eventInstance.EmitAsync(status: "ok").ConfigureAwait(false);
            return result;
        }
        catch (Exception ex)
        {
            eventInstance.SetError(ex);
            await eventInstance.EmitAsync().ConfigureAwait(false);
            throw;
        }
    }

    private static string ResolveService(string? candidate)
    {
        var resolved = !string.IsNullOrWhiteSpace(candidate) ? candidate : Environment.GetEnvironmentVariable("OPENAKTA_SERVICE");
        if (string.IsNullOrWhiteSpace(resolved))
        {
            throw new InvalidOperationException("Logger requires a service name or OPENAKTA_SERVICE");
        }

        return resolved;
    }

    private static string ResolveEnvironment(string? candidate)
    {
        var resolved = !string.IsNullOrWhiteSpace(candidate) ? candidate : Environment.GetEnvironmentVariable("OPENAKTA_ENV");
        return resolved is "production" or "staging" or "development" ? resolved : "production";
    }
}
