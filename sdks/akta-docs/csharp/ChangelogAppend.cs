using System.Text.Json;
using System.Text.Json.Serialization;

namespace OpenAkta.AktaDocs;

public sealed class ChangelogException : IOException
{
    public ChangelogException(string message, Exception? inner = null) : base(message, inner) { }
}

public sealed class ChangelogEntryPayload
{
    [JsonPropertyName("schema_version")]
    public string SchemaVersion { get; set; } = "";

    [JsonPropertyName("doc_id")]
    public string DocId { get; set; } = "";

    public string Timestamp { get; set; } = "";

    [JsonPropertyName("change_type")]
    public string ChangeType { get; set; } = "";

    public string Summary { get; set; } = "";

    public string? Details { get; set; }
}

public static class ChangelogAppend
{
    public const string Anchor = "<!-- akta-changelog-append -->";

    public sealed record Result(string Target, int BytesWritten, bool Created);

    public static Result Append(string targetPath, string jsonPayload, string template, bool dryRun)
    {
        ChangelogEntryPayload entry;
        try
        {
            entry = JsonSerializer.Deserialize<ChangelogEntryPayload>(jsonPayload)
                    ?? throw new ChangelogException("Invalid changelog payload: empty JSON.");
        }
        catch (JsonException e)
        {
            throw new ChangelogException("Invalid changelog payload: not valid JSON.", e);
        }

        if (string.IsNullOrWhiteSpace(entry.ChangeType) || string.IsNullOrWhiteSpace(entry.Summary))
            throw new ChangelogException("Invalid changelog payload: missing required fields.");

        var line = $"- **{entry.ChangeType}** ({entry.Timestamp}) {entry.Summary}";
        if (string.Equals(template, "detailed", StringComparison.Ordinal) &&
            !string.IsNullOrWhiteSpace(entry.Details))
            line += "\n  " + entry.Details!.Replace("\n", "\n  ", StringComparison.Ordinal);

        var block = "\n" + line + "\n";

        string outText;
        bool created;
        if (!File.Exists(targetPath))
        {
            created = true;
            var date = entry.Timestamp.Length >= 10 ? entry.Timestamp[..10] : entry.Timestamp;
            outText =
                "---\n" +
                "doc_id: " + entry.DocId + "\n" +
                "doc_type: changelog\n" +
                "date: " + date + "\n" +
                "---\n\n" +
                "# Changelog\n\n" +
                Anchor +
                block;
        }
        else
        {
            created = false;
            var existing = File.ReadAllText(targetPath);
            if (existing.Contains(Anchor, StringComparison.Ordinal))
                outText = existing.Replace(Anchor, Anchor + block, StringComparison.Ordinal);
            else
            {
                var sep = existing.EndsWith('\n') ? "\n" : "\n\n";
                outText = existing + sep + block.TrimStart() + "\n";
            }
        }

        var bytes = System.Text.Encoding.UTF8.GetBytes(outText);
        if (dryRun)
            return new Result(targetPath, bytes.Length, created);

        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(targetPath))!);
        var tmp = Path.Combine(Path.GetTempPath(), $"akta-changelog-{Guid.NewGuid():N}.md");
        try
        {
            File.WriteAllText(tmp, outText);
            File.Move(tmp, targetPath, overwrite: true);
        }
        finally
        {
            try
            {
                if (File.Exists(tmp)) File.Delete(tmp);
            }
            catch
            {
                /* ignore */
            }
        }

        return new Result(targetPath, bytes.Length, created);
    }
}
