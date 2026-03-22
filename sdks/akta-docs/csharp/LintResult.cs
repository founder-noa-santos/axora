namespace OpenAkta.AktaDocs;

public sealed record LintSummary(int ErrorCount, int WarnCount);

public sealed record LintResult(IReadOnlyList<Diagnostic> Diagnostics, LintSummary Summary);
