using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace Openakta.Logger;

public sealed record WideEventPayload
{
    [JsonPropertyName("event_id")] public string EventId { get; init; } = string.Empty;
    [JsonPropertyName("service")] public string Service { get; init; } = string.Empty;
    [JsonPropertyName("environment")] public string Environment { get; init; } = string.Empty;
    [JsonPropertyName("timestamp_start")] public string TimestampStart { get; init; } = string.Empty;
    [JsonPropertyName("timestamp_end")] public string TimestampEnd { get; init; } = string.Empty;
    [JsonPropertyName("duration_ms")] public double DurationMs { get; init; }
    [JsonPropertyName("level")] public string Level { get; init; } = string.Empty;
    [JsonPropertyName("operation")] public string Operation { get; init; } = string.Empty;
    [JsonPropertyName("status")] public string Status { get; init; } = string.Empty;
    [JsonPropertyName("context")] public Dictionary<string, object?> Context { get; init; } = new();
    [JsonPropertyName("error")] public ErrorDetail Error { get; init; } = new(null, null, null);
    [JsonPropertyName("meta")] public SdkMeta Meta { get; init; } = new("0.1.0", "csharp");

    public sealed record ErrorDetail(
        [property: JsonPropertyName("type")] string? Type,
        [property: JsonPropertyName("message")] string? Message,
        [property: JsonPropertyName("stack")] string? Stack);

    public sealed record SdkMeta(
        [property: JsonPropertyName("sdk_version")] string SdkVersion,
        [property: JsonPropertyName("sdk_language")] string SdkLanguage);
}
