using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace OpenAkta.AktaDocs;

public static class ScaffoldRunner
{
    private static readonly ISerializer YamlSerializer =
        new SerializerBuilder()
            .WithNamingConvention(UnderscoredNamingConvention.Instance)
            .ConfigureDefaultValuesHandling(DefaultValuesHandling.OmitNull)
            .Build();

    private static readonly string[] SectionDirs =
    {
        "00-meta", "01-adrs", "02-business-core", "03-business-logic", "04-research", "05-features",
        "06-technical", "07-guides", "08-references", "09-explanations", "10-changelog", "99-archive"
    };

    public static string Slugify(string name)
    {
        var s = System.Text.RegularExpressions.Regex.Replace(name.ToLowerInvariant(), "[^a-z0-9]+", "-");
        s = System.Text.RegularExpressions.Regex.Replace(s, "^-|-$", "");
        return string.IsNullOrEmpty(s) ? "project" : s;
    }

    public static AktaConfig DefaultConfig(string projectName)
    {
        var slug = Slugify(projectName);
        return new AktaConfig
        {
            SchemaVersion = "1.0.0",
            Project = new ProjectCfg { Name = projectName, Slug = slug },
            Paths = new PathsCfg { DocsRoot = "./akta-docs" },
            Linter = new LinterCfg
            {
                DefaultSeverity = "error",
                Rules = new Dictionary<string, RuleOptions>
                {
                    ["META-QUICK"] = new RuleOptions { Severity = "off" },
                    ["STRUCT-008"] = new RuleOptions { Severity = "off" },
                    ["CONTENT-001"] = new RuleOptions { Severity = "off" }
                }
            },
            Changelog = new ChangelogCfg { DefaultTarget = "akta-docs/10-changelog/CHANGELOG.md" }
        };
    }

    public sealed record Report(string Root, string DocsRoot, IReadOnlyList<string> CreatedPaths, string ConfigPath);

    public static Report Run(
        string root,
        string projectName,
        bool force,
        bool dryRun,
        bool createReadmeInEachFolder,
        bool gitkeep)
    {
        var rootP = Path.GetFullPath(root);
        var configPath = Path.Combine(rootP, ".akta-config.yaml");
        var cfg = DefaultConfig(projectName);
        cfg.Scaffold = new ScaffoldCfg
        {
            CreateReadmeInEachFolder = createReadmeInEachFolder,
            Gitkeep = gitkeep
        };

        var docsRoot = Path.Combine(rootP, "akta-docs");
        var created = new List<string>();

        if (!dryRun && File.Exists(configPath) && !force)
            throw new IOException($"{configPath} already exists; use --force to overwrite.");

        var slug = !string.IsNullOrWhiteSpace(cfg.Project.Slug) ? cfg.Project.Slug! : Slugify(projectName);
        var today = DateTime.UtcNow.ToString("yyyy-MM-dd");

        foreach (var dir in SectionDirs)
        {
            var full = Path.Combine(docsRoot, dir);
            if (!dryRun)
                Directory.CreateDirectory(full);
            created.Add(Path.Combine("akta-docs", dir));

            if (createReadmeInEachFolder)
            {
                var docId = slug + "." + System.Text.RegularExpressions.Regex.Replace(dir, "[^a-z0-9]+", "-") + "-readme";
                var body =
                    "---\n" +
                    "doc_id: " + docId + "\n" +
                    "doc_type: meta\n" +
                    "date: " + today + "\n" +
                    "---\n\n" +
                    "# " + dir + "\n\n" +
                    "Placeholder content for this section. Replace with architecture and business documentation aligned with OPENAKTA GEO standards.\n";
                if (!dryRun)
                    File.WriteAllText(Path.Combine(full, "README.md"), body);
                created.Add(Path.Combine("akta-docs", dir, "README.md"));
            }

            if (gitkeep && !dryRun)
                File.WriteAllText(Path.Combine(full, ".gitkeep"), "");
        }

        var changelogPath = Path.Combine(docsRoot, "10-changelog", "CHANGELOG.md");
        var initial =
            "---\n" +
            "doc_id: " + slug + ".changelog\n" +
            "doc_type: changelog\n" +
            "date: " + today + "\n" +
            "---\n\n" +
            "# Changelog\n\n" +
            ChangelogAppend.Anchor + "\n";
        if (!dryRun)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(changelogPath)!);
            File.WriteAllText(changelogPath, initial);
        }

        created.Add("akta-docs/10-changelog/CHANGELOG.md");

        if (!dryRun)
        {
            var yaml = YamlSerializer.Serialize(cfg);
            File.WriteAllText(configPath, yaml);
        }

        return new Report(rootP, docsRoot, created, configPath);
    }
}
