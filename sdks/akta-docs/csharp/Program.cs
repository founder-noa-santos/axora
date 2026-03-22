using System.CommandLine;
using System.CommandLine.Builder;
using System.CommandLine.Invocation;
using System.CommandLine.Parsing;
using System.Text.Json;

namespace OpenAkta.AktaDocs;

public static class Program
{
    public static async Task<int> Main(string[] args)
    {
        var configOption = new Option<FileInfo?>(new[] { "--config", "-c" }, "Path to .akta-config.yaml");
        var formatOption = new Option<string>("--format", () => "default", "Lint output: default or json");

        var root = new RootCommand("OPENAKTA documentation linter and scaffolding (GEO / AI context)")
        {
            Name = "akta-docs"
        };
        root.AddGlobalOption(configOption);
        root.AddGlobalOption(formatOption);

        var init = new Command("init", "Scaffold akta-docs layout and .akta-config.yaml");
        var rootDir = new Option<DirectoryInfo>(
            "--root",
            () => new DirectoryInfo(Environment.CurrentDirectory),
            "Repository root");
        var forceOpt = new Option<bool>("--force", () => false, "Overwrite existing config");
        var dryRunOpt = new Option<bool>("--dry-run", () => false, "Do not write files");
        var projectNameOpt = new Option<string?>("--project-name", "Project display name");
        var skipReadmeOpt = new Option<bool>("--skip-readme", () => false, "Do not write README in each folder");
        var gitkeepOpt = new Option<bool>("--gitkeep", () => false, "Write .gitkeep in each folder");
        init.AddOption(rootDir);
        init.AddOption(forceOpt);
        init.AddOption(dryRunOpt);
        init.AddOption(projectNameOpt);
        init.AddOption(skipReadmeOpt);
        init.AddOption(gitkeepOpt);

        Handler.SetHandler(init, (InvocationContext ctx) =>
        {
            try
            {
                var r = ctx.ParseResult.GetValueForOption(rootDir)!;
                var rr = r.FullName;
                var name = ctx.ParseResult.GetValueForOption(projectNameOpt);
                if (string.IsNullOrWhiteSpace(name))
                    name = Path.GetFileName(rr.TrimEnd(Path.DirectorySeparatorChar));
                if (string.IsNullOrWhiteSpace(name))
                    name = "openakta-project";

                var rep = ScaffoldRunner.Run(
                    rr,
                    name,
                    ctx.ParseResult.GetValueForOption(forceOpt),
                    ctx.ParseResult.GetValueForOption(dryRunOpt),
                    !ctx.ParseResult.GetValueForOption(skipReadmeOpt),
                    ctx.ParseResult.GetValueForOption(gitkeepOpt));
                ctx.Console.WriteLine($"Created docs tree under {rep.DocsRoot}");
                ctx.Console.WriteLine($"Wrote {rep.ConfigPath}");
                ctx.ExitCode = 0;
            }
            catch (IOException ex)
            {
                Console.Error.WriteLine(ex.Message);
                ctx.ExitCode = 2;
            }
        });

        var lint = new Command("lint", "Lint markdown files");
        var pathsArg = new Argument<string[]>("paths", "Files or directories")
        {
            Arity = ArgumentArity.ZeroOrMore
        };
        var maxWarningsOpt = new Option<int>("--max-warnings", () => -1, "-1 = unlimited");
        var ruleOpt = new Option<string[]>("--rule", "Only these rules (repeatable)") { Arity = ArgumentArity.ZeroOrMore };
        lint.AddArgument(pathsArg);
        lint.AddOption(maxWarningsOpt);
        lint.AddOption(ruleOpt);

        Handler.SetHandler(lint, (InvocationContext ctx) =>
        {
            try
            {
                var cwd = Environment.CurrentDirectory;
                var cfgPath = ConfigLoader.ResolveConfigPath(cwd, ctx.ParseResult.GetValueForOption(configOption)?.FullName);
                var cfg = ConfigLoader.Load(cfgPath);
                var paths = ctx.ParseResult.GetValueForArgument(pathsArg) ?? Array.Empty<string>();
                List<string> files;
                if (paths.Length > 0)
                    files = ExpandLintInputs(paths, cwd);
                else
                    files = GlobPaths.ExpandLintPaths(cfg, cwd);

                var res = LintEngine.LintFiles(files, cfg, cwd);
                var diags = res.Diagnostics.ToList();
                var rules = ctx.ParseResult.GetValueForOption(ruleOpt);
                if (rules is { Length: > 0 })
                    diags.RemoveAll(d => !rules.Contains(d.RuleId, StringComparer.Ordinal));

                var fmt = ctx.ParseResult.GetValueForOption(formatOption) ?? "default";
                if (string.Equals(fmt, "json", StringComparison.OrdinalIgnoreCase))
                {
                    ctx.Console.WriteLine(JsonSerializer.Serialize(diags,
                        new JsonSerializerOptions { WriteIndented = true }));
                }
                else
                {
                    foreach (var d in diags)
                        ctx.Console.WriteLine(FormatDefault(d, cwd));
                }

                var errors = diags.Count(d => d.Severity == "error");
                var warnings = diags.Count(d => d.Severity == "warn");
                var cap = ctx.ParseResult.GetValueForOption(maxWarningsOpt);
                var warnCap = cap < 0 ? long.MaxValue : cap;
                ctx.ExitCode = errors > 0 || warnings > warnCap ? 1 : 0;
            }
            catch (ConfigException ex)
            {
                Console.Error.WriteLine(ex.Message);
                ctx.ExitCode = 2;
            }
        });

        var create = new Command("create", "Create a linter-friendly template");
        var kindArg = new Argument<string>("kind", "Template kind");
        var outputArg = new Argument<string>("output_path", "Output .md path");
        var titleOpt = new Option<string>("--title") { IsRequired = true };
        var slugOpt = new Option<string>("--slug") { IsRequired = true };
        var docIdOpt = new Option<string?>("--doc-id", "Explicit doc_id (default: slug + date)");
        create.AddArgument(kindArg);
        create.AddArgument(outputArg);
        create.AddOption(titleOpt);
        create.AddOption(slugOpt);
        create.AddOption(docIdOpt);

        Handler.SetHandler(create, (InvocationContext ctx) =>
        {
            try
            {
                var kind = ctx.ParseResult.GetValueForArgument(kindArg);
                var outputPath = ctx.ParseResult.GetValueForArgument(outputArg);
                var title = ctx.ParseResult.GetValueForOption(titleOpt)!;
                var slug = ctx.ParseResult.GetValueForOption(slugOpt)!;
                var cwd = Environment.CurrentDirectory;
                var date = DateTime.UtcNow.ToString("yyyy-MM-dd");
                var docId = ctx.ParseResult.GetValueForOption(docIdOpt);
                if (string.IsNullOrWhiteSpace(docId))
                    docId = ReSlug(slug) + "." + date;

                TemplateWriter.Write(kind, Path.Combine(cwd, outputPath), title, slug, docId, date);
                ctx.Console.WriteLine($"Wrote {outputPath}");
                ctx.ExitCode = 0;
            }
            catch (ArgumentException ex)
            {
                Console.Error.WriteLine(ex.Message);
                ctx.ExitCode = 2;
            }
        });

        var changelog = new Command("changelog", "Append and manage changelog entries");
        var append = new Command("append", "Append changelog entry from JSON");
        var fileOpt = new Option<FileInfo>("--file") { IsRequired = true };
        var payloadOpt = new Option<FileInfo?>("--payload", "JSON file (else stdin)");
        var dryRunAppendOpt = new Option<bool>("--dry-run", () => false, "Print size only");
        append.AddOption(fileOpt);
        append.AddOption(payloadOpt);
        append.AddOption(dryRunAppendOpt);

        Handler.SetHandler(append, (InvocationContext ctx) =>
        {
            try
            {
                var cwd = Environment.CurrentDirectory;
                var cfgPath = ConfigLoader.ResolveConfigPath(cwd, ctx.ParseResult.GetValueForOption(configOption)?.FullName);
                var template = "compact";
                try
                {
                    var cfg = ConfigLoader.Load(cfgPath);
                    template = cfg.Changelog.EntryTemplate;
                }
                catch
                {
                    /* default */
                }

                var file = ctx.ParseResult.GetValueForOption(fileOpt)!;
                var targetPath = Path.GetFullPath(file.FullName);
                string raw;
                var payload = ctx.ParseResult.GetValueForOption(payloadOpt);
                if (payload != null)
                    raw = File.ReadAllText(payload.FullName);
                else
                    raw = new StreamReader(Console.OpenStandardInput()).ReadToEnd();

                var r = ChangelogAppend.Append(
                    targetPath,
                    raw,
                    template,
                    ctx.ParseResult.GetValueForOption(dryRunAppendOpt));
                var pfx = ctx.ParseResult.GetValueForOption(dryRunAppendOpt) ? "[dry-run] " : "";
                ctx.Console.WriteLine(
                    $"{pfx}Wrote {r.BytesWritten} bytes to {r.Target}" + (r.Created ? " (created)" : ""));
                ctx.ExitCode = 0;
            }
            catch (ChangelogException ex)
            {
                Console.Error.WriteLine(ex.Message);
                ctx.ExitCode = 2;
            }
        });

        changelog.AddCommand(append);

        root.AddCommand(init);
        root.AddCommand(lint);
        root.AddCommand(create);
        root.AddCommand(changelog);

        var parser = new CommandLineBuilder(root)
            .UseDefaults()
            .Build();

        try
        {
            return await parser.InvokeAsync(args);
        }
        catch (ConfigException ex)
        {
            Console.Error.WriteLine(ex.Message);
            return 2;
        }
        catch (ChangelogException ex)
        {
            Console.Error.WriteLine(ex.Message);
            return 2;
        }
    }

    private static string FormatDefault(Diagnostic d, string cwd)
    {
        var p = Path.GetFullPath(d.File);
        string rel;
        try
        {
            rel = Path.GetRelativePath(cwd, p).Replace('\\', '/');
        }
        catch
        {
            rel = d.File;
        }

        return $"{rel}:{d.Line}:{d.Column} {d.Severity} {d.RuleId} {d.Message}";
    }

    private static List<string> ExpandLintInputs(string[] paths, string cwd)
    {
        var o = new List<string>();
        foreach (var p in paths)
        {
            var abs = Path.GetFullPath(Path.Combine(cwd, p));
            if (Directory.Exists(abs))
            {
                foreach (var f in Directory.EnumerateFiles(abs, "*.md", SearchOption.AllDirectories))
                {
                    if (f.Contains($"{Path.DirectorySeparatorChar}node_modules{Path.DirectorySeparatorChar}",
                            StringComparison.OrdinalIgnoreCase))
                        continue;
                    o.Add(f);
                }
            }
            else
            {
                o.Add(abs);
            }
        }

        return o.Distinct().OrderBy(x => x, StringComparer.Ordinal).ToList();
    }

    private static string ReSlug(string slug)
    {
        var s = System.Text.RegularExpressions.Regex.Replace(slug.ToLowerInvariant(), "[^a-z0-9]+", "-");
        return System.Text.RegularExpressions.Regex.Replace(s, "^-|-$", "");
    }
}
