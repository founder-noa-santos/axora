using System.Collections.Generic;
using Openakta.Logger.Sinks;

namespace Openakta.Logger;

public sealed class LoggerBuilder
{
    private string? _service;
    private string? _environment;
    private readonly List<ISink> _sinks = [];
    private Dictionary<string, object?> _defaultContext = [];

    public LoggerBuilder Service(string service)
    {
        _service = service;
        return this;
    }

    public LoggerBuilder Environment(string environment)
    {
        _environment = environment;
        return this;
    }

    public LoggerBuilder AddSink(ISink sink)
    {
        _sinks.Add(sink);
        return this;
    }

    public LoggerBuilder Sinks(IEnumerable<ISink> sinks)
    {
        _sinks.Clear();
        _sinks.AddRange(sinks);
        return this;
    }

    public LoggerBuilder DefaultContext(IDictionary<string, object?> context)
    {
        _defaultContext = new Dictionary<string, object?>(context);
        return this;
    }

    public Logger Build()
    {
        return new Logger(
            ResolveService(_service),
            ResolveEnvironment(_environment),
            _sinks,
            _defaultContext);
    }

    private static string ResolveService(string? candidate)
    {
        var resolved = !string.IsNullOrWhiteSpace(candidate) ? candidate : System.Environment.GetEnvironmentVariable("OPENAKTA_SERVICE");
        if (string.IsNullOrWhiteSpace(resolved))
        {
            throw new InvalidOperationException("Logger requires a service name or OPENAKTA_SERVICE");
        }

        return resolved;
    }

    private static string ResolveEnvironment(string? candidate)
    {
        var resolved = !string.IsNullOrWhiteSpace(candidate) ? candidate : System.Environment.GetEnvironmentVariable("OPENAKTA_ENV");
        return resolved is "production" or "staging" or "development" ? resolved : "production";
    }
}
