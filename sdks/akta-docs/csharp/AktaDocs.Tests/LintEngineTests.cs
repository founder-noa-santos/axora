using OpenAkta.AktaDocs;
using Xunit.Sdk;

namespace AktaDocs.Tests;

public class LintEngineTests
{
    private static bool StrictFixtures()
    {
        var ci = Environment.GetEnvironmentVariable("CI");
        var skipFixtures = Environment.GetEnvironmentVariable("SKIP_FIXTURES");
        return string.Equals(ci, "true", StringComparison.OrdinalIgnoreCase) && skipFixtures != "1";
    }

    private static string FixturesDir()
    {
        // net7.0 -> bin -> AktaDocs.Tests -> csharp -> sdks/akta-docs -> typescript/tests/fixtures
        var d = Path.GetFullPath(Path.Combine(
            AppContext.BaseDirectory,
            "..", "..", "..", "..", "..",
            "typescript", "tests", "fixtures"));
        return d;
    }

    private static string RequireFixture(string name)
    {
        var fixture = Path.Combine(FixturesDir(), name);
        if (File.Exists(fixture))
        {
            return fixture;
        }

        if (StrictFixtures())
        {
            throw new XunitException($"Missing shared TS fixture: {fixture}");
        }

        throw new SkipException($"Shared TS fixtures not present: {fixture}");
    }

    private static AktaConfig Strict()
    {
        return new AktaConfig
        {
            SchemaVersion = "1.0.0",
            Project = new ProjectCfg { Name = "Test", Slug = "test" },
            Paths = new PathsCfg
            {
                DocsRoot = "./akta-docs",
                IncludeGlobs = new List<string> { "**/*.md" },
                ExcludeGlobs = new List<string>()
            },
            Linter = new LinterCfg { DefaultSeverity = "error", Rules = new Dictionary<string, RuleOptions>() },
            Scaffold = new ScaffoldCfg(),
            Changelog = new ChangelogCfg()
        };
    }

    [Fact]
    public void WordCount_tokens()
    {
        Assert.Equal(3, WordCount.Count("a b c"));
        Assert.Equal(0, WordCount.Count(""));
    }

    [Fact]
    public void Meta001_no_frontmatter()
    {
        var fx = FixturesDir();
        var f = RequireFixture("no-frontmatter.md");

        var cwd = Path.GetFullPath(Path.Combine(fx, ".."));
        var r = LintEngine.LintFiles(new[] { f }, Strict(), cwd);
        Assert.Contains(r.Diagnostics, d => d.RuleId == "META-001");
    }

    [Fact]
    public void Compliant_passes()
    {
        var fx = FixturesDir();
        var f = RequireFixture("compliant.md");

        var cwd = Path.GetFullPath(Path.Combine(fx, ".."));
        var r = LintEngine.LintFiles(new[] { f }, Strict(), cwd);
        Assert.Empty(r.Diagnostics.Where(d => d.Severity == "error"));
    }

    [Fact]
    public void Struct008_short_section()
    {
        var fx = FixturesDir();
        var f = RequireFixture("short-section.md");

        var cwd = Path.GetFullPath(Path.Combine(fx, ".."));
        var r = LintEngine.LintFiles(new[] { f }, Strict(), cwd);
        Assert.Contains(r.Diagnostics, d => d.RuleId == "STRUCT-008");
    }
}
