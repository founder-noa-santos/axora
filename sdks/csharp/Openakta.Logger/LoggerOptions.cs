using System.Collections.Generic;

namespace Openakta.Logger;

public sealed record LoggerOptions(
    string? Service = null,
    string? Environment = null,
    IReadOnlyList<Sinks.ISink>? Sinks = null,
    IReadOnlyDictionary<string, object?>? DefaultContext = null);
