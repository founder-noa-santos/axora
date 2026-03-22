using Microsoft.Extensions.FileSystemGlobbing;
using Microsoft.Extensions.FileSystemGlobbing.Abstractions;

namespace OpenAkta.AktaDocs;

public static class GlobPaths
{
    public static List<string> ExpandLintPaths(AktaConfig cfg, string cwd)
    {
        var root = Path.GetFullPath(Path.Combine(cwd, cfg.Paths.DocsRoot));
        if (!Directory.Exists(root)) return new List<string>();

        var matcher = new Matcher(StringComparison.OrdinalIgnoreCase);
        foreach (var g in cfg.Paths.IncludeGlobs)
            matcher.AddInclude(g);
        foreach (var ex in cfg.Paths.ExcludeGlobs)
            matcher.AddExclude(ex);

        var dir = new DirectoryInfoWrapper(new DirectoryInfo(root));
        var result = matcher.Execute(dir);
        var list = new List<string>();
        foreach (var file in result.Files)
        {
            var full = Path.GetFullPath(Path.Combine(root, file.Path));
            if (full.EndsWith(".md", StringComparison.OrdinalIgnoreCase))
                list.Add(full);
        }

        list.Sort(StringComparer.Ordinal);
        return list;
    }
}
