using YamlDotNet.Serialization;

namespace OpenAkta.AktaDocs;

public class AktaConfig
{
    [YamlMember(Alias = "schema_version")]
    public string SchemaVersion { get; set; } = "";

    public ProjectCfg Project { get; set; } = new();

    public PathsCfg Paths { get; set; } = new();

    public LinterCfg Linter { get; set; } = new();

    public ScaffoldCfg Scaffold { get; set; } = new();

    public ChangelogCfg Changelog { get; set; } = new();
}

public class ProjectCfg
{
    public string Name { get; set; } = "";

    public string? Slug { get; set; }
}

public class PathsCfg
{
    [YamlMember(Alias = "docs_root")]
    public string DocsRoot { get; set; } = "./akta-docs";

    [YamlMember(Alias = "include_globs")]
    public List<string> IncludeGlobs { get; set; } = new() { "**/*.md" };

    [YamlMember(Alias = "exclude_globs")]
    public List<string> ExcludeGlobs { get; set; } =
        new() { "**/node_modules/**", "**/.git/**", "**/99-archive/**" };
}

public class LinterCfg
{
    [YamlMember(Alias = "default_severity")]
    public string DefaultSeverity { get; set; } = "error";

    public Dictionary<string, RuleOptions> Rules { get; set; } = new();
}

public class RuleOptions
{
    public string? Severity { get; set; }

    [YamlMember(Alias = "min_words")]
    public int? MinWords { get; set; }

    [YamlMember(Alias = "max_words")]
    public int? MaxWords { get; set; }

    [YamlMember(Alias = "min_question_ratio")]
    public double? MinQuestionRatio { get; set; }
}

public class ScaffoldCfg
{
    [YamlMember(Alias = "create_readme_in_each_folder")]
    public bool CreateReadmeInEachFolder { get; set; } = true;

    public bool Gitkeep { get; set; }
}

public class ChangelogCfg
{
    [YamlMember(Alias = "default_target")]
    public string? DefaultTarget { get; set; }

    [YamlMember(Alias = "entry_template")]
    public string EntryTemplate { get; set; } = "compact";

    [YamlMember(Alias = "summary_max_length")]
    public int SummaryMaxLength { get; set; } = 200;
}
