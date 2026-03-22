using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace OpenAkta.AktaDocs;

public sealed class ConfigException : IOException
{
    public ConfigException(string message, Exception? inner = null) : base(message, inner) { }
}

public static class ConfigLoader
{
    private static readonly IDeserializer Deserializer =
        new DeserializerBuilder()
            .IgnoreUnmatchedProperties()
            .WithNamingConvention(UnderscoredNamingConvention.Instance)
            .Build();

    public static AktaConfig Load(string path)
    {
        string raw;
        try
        {
            raw = File.ReadAllText(path);
        }
        catch (Exception e)
        {
            throw new ConfigException($"Cannot read config: {path}", e);
        }

        try
        {
            return Deserializer.Deserialize<AktaConfig>(raw)
                   ?? throw new ConfigException($"Invalid .akta-config.yaml: {path}");
        }
        catch (Exception e) when (e is not ConfigException)
        {
            throw new ConfigException($"Invalid YAML in {path}", e);
        }
    }

    public static string ResolveConfigPath(string cwd, string? explicitPath) =>
        Path.GetFullPath(explicitPath ?? Path.Combine(cwd, ".akta-config.yaml"));
}
