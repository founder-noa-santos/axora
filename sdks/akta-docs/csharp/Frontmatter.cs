using System.Text.RegularExpressions;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace OpenAkta.AktaDocs;

public static class Frontmatter
{
    private static readonly Regex Block =
        new(@"^---\r?\n(.*?)\r?\n---\r?\n", RegexOptions.Singleline);

    private static readonly IDeserializer YamlDeserializer =
        new DeserializerBuilder()
            .IgnoreUnmatchedProperties()
            .WithNamingConvention(NullNamingConvention.Instance)
            .Build();

    public static bool HasBlock(string raw) => raw.TrimStart().StartsWith("---", StringComparison.Ordinal);

    public static (Dictionary<string, object?> Data, string Body) Parse(string raw)
    {
        var m = Block.Match(raw);
        if (!m.Success)
            return (new Dictionary<string, object?>(), raw);

        var yamlBlock = m.Groups[1].Value.Trim();
        var body = raw[m.Length..];
        if (string.IsNullOrEmpty(yamlBlock))
            return (new Dictionary<string, object?>(), body);

        Dictionary<string, object?> dict;
        try
        {
            dict = YamlDeserializer.Deserialize<Dictionary<string, object?>>(new StringReader(yamlBlock))
                   ?? new Dictionary<string, object?>();
        }
        catch (Exception e)
        {
            throw new InvalidDataException("Invalid frontmatter YAML.", e);
        }

        return (dict, body);
    }

    public static int BodyStartLine(string fileContent, string body)
    {
        var idx = fileContent.IndexOf(body, StringComparison.Ordinal);
        if (idx < 0) return 1;
        var n = 0;
        for (var i = 0; i < idx; i++)
        {
            if (fileContent[i] == '\n') n++;
        }

        return n + 1;
    }
}
