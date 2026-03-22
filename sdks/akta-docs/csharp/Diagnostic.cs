using System.Text.Json.Serialization;

namespace OpenAkta.AktaDocs;

public sealed record Diagnostic(
    [property: JsonPropertyName("file")] string File,
    [property: JsonPropertyName("line")] int Line,
    [property: JsonPropertyName("column")] int Column,
    [property: JsonPropertyName("rule_id")] string RuleId,
    [property: JsonPropertyName("severity")] string Severity,
    [property: JsonPropertyName("message")] string Message,
    [property: JsonPropertyName("end_line")] int? EndLine,
    [property: JsonPropertyName("end_column")] int? EndColumn);
