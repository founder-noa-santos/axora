using System.Globalization;
using System.Text.RegularExpressions;
using Markdig;
using Markdig.Syntax;
using Markdig.Syntax.Inlines;

namespace OpenAkta.AktaDocs;

public static class LintEngine
{
    private static readonly Regex DocId = new(@"^[a-z0-9][a-z0-9._-]*$", RegexOptions.Compiled);
    private static readonly Regex IsoDate = new(
        @"^\d{4}-\d{2}-\d{2}(?:T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?)?$",
        RegexOptions.Compiled);

    private static readonly Regex QuestionStart = new(
        @"^(how|what|when|where|why|who|which|can|should|does|is|are)\b",
        RegexOptions.IgnoreCase | RegexOptions.Compiled);

    private static readonly Regex H23 = new(@"^(#{2,3})\s+(.+)$", RegexOptions.Compiled);

    private static readonly string[] DocTypes =
    {
        "adr", "business_rule", "feature", "guide", "reference", "explanation", "research", "meta",
        "changelog", "technical", "other"
    };

    private static readonly MarkdownPipeline Pipeline = new MarkdownPipelineBuilder()
        .UseAdvancedExtensions()
        .Build();

    public static LintResult LintFiles(IReadOnlyList<string> paths, AktaConfig cfg, string cwd)
    {
        var diags = new List<Diagnostic>();
        foreach (var p in paths)
        {
            var abs = Path.GetFullPath(p);
            string rel;
            try
            {
                rel = Path.GetRelativePath(cwd, abs);
            }
            catch
            {
                rel = Path.GetFileName(abs);
            }

            rel = rel.Replace('\\', '/');
            if (string.IsNullOrEmpty(rel)) rel = Path.GetFileName(abs);

            string raw;
            try
            {
                raw = File.ReadAllText(abs);
            }
            catch
            {
                continue;
            }

            Dictionary<string, object?> data;
            string body;
            try
            {
                (data, body) = Frontmatter.Parse(raw);
            }
            catch (InvalidDataException)
            {
                Push(diags, rel, "META-001", RuleSeverity.MapSeverity(RuleSeverity.Effective(cfg, "META-001")),
                    1, 1, "Invalid frontmatter YAML.");
                continue;
            }

            LintMeta(rel, raw, data, cfg, diags);
            if (!Frontmatter.HasBlock(raw))
                continue;

            MarkdownDocument? doc;
            try
            {
                doc = Markdown.Parse(body, Pipeline) as MarkdownDocument;
            }
            catch
            {
                Push(diags, rel, "META-001", RuleSeverity.MapSeverity(RuleSeverity.Effective(cfg, "META-001")),
                    1, 1, "Markdown body failed to parse.");
                continue;
            }

            if (doc == null) continue;

            var bodyStartLine = Frontmatter.BodyStartLine(raw, body);
            var lineOffset = bodyStartLine - 1;

            LintMetaQuick(rel, body, data, doc, cfg, diags, lineOffset);
            LintStruct008(rel, body, data, cfg, diags, lineOffset);
            LintContent001(rel, body, cfg, diags, lineOffset);
        }

        diags.Sort(CompareDiag);
        var err = diags.Count(d => d.Severity == "error");
        var warn = diags.Count(d => d.Severity == "warn");
        return new LintResult(diags, new LintSummary(err, warn));
    }

    private static int CompareDiag(Diagnostic a, Diagnostic b)
    {
        var c = string.Compare(a.File, b.File, StringComparison.Ordinal);
        if (c != 0) return c;
        if (a.Line != b.Line) return a.Line.CompareTo(b.Line);
        if (a.Column != b.Column) return a.Column.CompareTo(b.Column);
        return string.Compare(a.RuleId, b.RuleId, StringComparison.Ordinal);
    }

    private static void Push(
        List<Diagnostic> diags, string file, string rule, string? severity,
        int line, int col, string msg, int? endLine = null, int? endCol = null)
    {
        if (severity == null) return;
        diags.Add(new Diagnostic(file, line, col, rule, severity, msg, endLine, endCol));
    }

    private static void LintMeta(
        string file, string fileContent, IReadOnlyDictionary<string, object?> data, AktaConfig cfg,
        List<Diagnostic> diags)
    {
        var r1 = RuleSeverity.Effective(cfg, "META-001");
        if (!Frontmatter.HasBlock(fileContent))
        {
            Push(diags, file, "META-001", RuleSeverity.MapSeverity(r1), 1, 1,
                "Missing YAML frontmatter (expected leading --- block).");
            return;
        }

        var r2 = RuleSeverity.Effective(cfg, "META-002");
        data.TryGetValue("doc_id", out var docIdObj);
        var docId = docIdObj?.ToString();
        if (string.IsNullOrWhiteSpace(docId))
        {
            Push(diags, file, "META-002", RuleSeverity.MapSeverity(r2), 2, 1,
                "Frontmatter must include non-empty string `doc_id`.");
        }
        else if (!DocId.IsMatch(docId.Trim()))
        {
            Push(diags, file, "META-002", RuleSeverity.MapSeverity(r2), 2, 1,
                $"doc_id must match pattern {DocId}: got \"{docId}\".");
        }

        var r3 = RuleSeverity.Effective(cfg, "META-003");
        data.TryGetValue("doc_type", out var docTypeObj);
        var docType = docTypeObj?.ToString();
        if (string.IsNullOrWhiteSpace(docType))
        {
            Push(diags, file, "META-003", RuleSeverity.MapSeverity(r3), 2, 1,
                $"Frontmatter must include string `doc_type` (one of: {string.Join(", ", DocTypes)}).");
        }
        else if (!DocTypes.Contains(docType.Trim(), StringComparer.Ordinal))
        {
            Push(diags, file, "META-003", RuleSeverity.MapSeverity(r3), 2, 1, $"Invalid doc_type \"{docType}\".");
        }

        var r4 = RuleSeverity.Effective(cfg, "META-004");
        data.TryGetValue("date", out var dateVal);
        var dateStr = CoerceDateString(dateVal);
        if (string.IsNullOrEmpty(dateStr))
        {
            Push(diags, file, "META-004", RuleSeverity.MapSeverity(r4), 2, 1,
                "Frontmatter must include ISO8601 `date` (YYYY-MM-DD or full instant).");
        }
        else if (!IsoDate.IsMatch(dateStr))
        {
            if (!DateTime.TryParse(dateStr, CultureInfo.InvariantCulture,
                    DateTimeStyles.RoundtripKind, out _)
                && !DateTimeOffset.TryParse(dateStr, CultureInfo.InvariantCulture,
                    DateTimeStyles.RoundtripKind, out _))
            {
                Push(diags, file, "META-004", RuleSeverity.MapSeverity(r4), 2, 1,
                    $"date must be ISO8601: got \"{dateStr}\".");
            }
        }
    }

    private static string? CoerceDateString(object? v)
    {
        if (v == null) return null;
        return v switch
        {
            string s when !string.IsNullOrWhiteSpace(s) => s.Trim(),
            DateTime dt => dt.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
            DateTimeOffset dto => dto.ToString("yyyy-MM-dd", CultureInfo.InvariantCulture),
            _ => v.ToString()?.Trim()
        };
    }

    private static void LintMetaQuick(
        string file, string body, IReadOnlyDictionary<string, object?> data, MarkdownDocument doc,
        AktaConfig cfg, List<Diagnostic> diags, int lineOffset)
    {
        var sev = RuleSeverity.Effective(cfg, "META-QUICK");
        var eff = RuleSeverity.MapSeverity(sev);
        if (eff == null) return;
        if (data.TryGetValue("doc_type", out var dt) && string.Equals(dt?.ToString(), "changelog", StringComparison.Ordinal))
            return;

        var minW = (int)RuleSeverity.NumberOption(cfg, "META-QUICK", "min_words", 40);
        var maxW = (int)RuleSeverity.NumberOption(cfg, "META-QUICK", "max_words", 80);

        var quick = FindQuickFence(doc);
        if (quick == null)
        {
            Push(diags, file, "META-QUICK", eff, 1 + lineOffset, 1,
                "After the first H1, expected a fenced code block with language tag `quick` (```quick).");
            return;
        }

        var text = FenceLiteral(quick).Trim();
        var words = WordCount.Count(text);
        var line = FenceQuickOpenLine1(body) + lineOffset;
        const int col = 1;
        if (words < minW || words > maxW)
        {
            Push(diags, file, "META-QUICK", eff, line, col,
                $"Quick Answer block must be {minW}–{maxW} words; found {words}.");
        }
    }

    private static int FenceQuickOpenLine1(string body)
    {
        var idx = body.IndexOf("```quick", StringComparison.Ordinal);
        if (idx < 0) idx = body.IndexOf("``` quick", StringComparison.Ordinal);
        if (idx < 0) return 1;
        var n = 1;
        for (var i = 0; i < idx; i++)
        {
            if (body[i] == '\n') n++;
        }

        return n;
    }

    private static string FenceLiteral(FencedCodeBlock f)
    {
        var sb = new System.Text.StringBuilder();
        foreach (var line in f.Lines)
            sb.Append(line);
        return sb.ToString();
    }

    private static FencedCodeBlock? FindQuickFence(MarkdownDocument doc)
    {
        var foundH1 = false;
        foreach (var block in doc)
        {
            if (!foundH1)
            {
                if (block is HeadingBlock { Level: 1 })
                    foundH1 = true;
                continue;
            }

            if (block is HeadingBlock) return null;
            if (block is FencedCodeBlock fcb)
            {
                var info = TrimInfo(fcb.Info);
                return info == "quick" ? fcb : null;
            }

            if (block is ParagraphBlock pb)
            {
                if (ParagraphBlank(pb)) continue;
                return null;
            }

            return null;
        }

        return null;
    }

    private static string TrimInfo(string? info)
    {
        if (string.IsNullOrWhiteSpace(info)) return "";
        var parts = info.Trim().Split(' ', StringSplitOptions.RemoveEmptyEntries);
        return parts.Length > 0 ? parts[0] : "";
    }

    private static bool ParagraphBlank(ParagraphBlock p)
    {
        if (p.Inline == null) return true;
        for (var il = p.Inline.FirstChild; il != null; il = il.NextSibling)
        {
            switch (il)
            {
                case LiteralInline lit:
                {
                    foreach (var ch in lit.Content.AsSpan())
                    {
                        if (!char.IsWhiteSpace(ch)) return false;
                    }

                    break;
                }
                case LineBreakInline:
                    break;
                default:
                    return false;
            }
        }

        return true;
    }

    private static void LintStruct008(
        string file, string body, IReadOnlyDictionary<string, object?> data, AktaConfig cfg,
        List<Diagnostic> diags, int lineOffset)
    {
        var sev = RuleSeverity.Effective(cfg, "STRUCT-008");
        var eff = RuleSeverity.MapSeverity(sev);
        if (eff == null) return;
        if (data.TryGetValue("doc_type", out var dt) && string.Equals(dt?.ToString(), "changelog", StringComparison.Ordinal))
            return;

        var minW = (int)RuleSeverity.NumberOption(cfg, "STRUCT-008", "min_words", 150);
        var maxW = (int)RuleSeverity.NumberOption(cfg, "STRUCT-008", "max_words", 300);

        var hs = CollectH23(body);
        hs.Sort((a, b) => a.Line0.CompareTo(b.Line0));

        var lines = body.Split(new[] { "\r\n", "\n", "\r" }, StringSplitOptions.None);

        foreach (var h in hs)
        {
            var firstContent0 = h.Line0 + 1;
            HeadingInfo? nxt = null;
            foreach (var x in hs)
            {
                if (x.Line0 > h.Line0 && x.Depth <= h.Depth)
                {
                    nxt = x;
                    break;
                }
            }

            var endExclusive = nxt == null ? lines.Length : nxt.Value.Line0;
            if (firstContent0 > endExclusive)
            {
                Push(diags, file, "STRUCT-008", eff, h.Line0 + 1 + lineOffset, 1,
                    "Section has no body; cannot satisfy STRUCT-008 word range.",
                    h.Line0 + 1 + lineOffset, null);
                continue;
            }

            var sb = new System.Text.StringBuilder();
            for (var i = firstContent0; i < endExclusive; i++)
            {
                if (i > firstContent0) sb.Append('\n');
                sb.Append(lines[i]);
            }

            var wc = WordCount.Count(sb.ToString());
            var line = h.Line0 + 1 + lineOffset;
            if (wc < minW || wc > maxW)
            {
                Push(diags, file, "STRUCT-008", eff, line, 1,
                    $"H{h.Depth} section must be {minW}–{maxW} words; found {wc}.",
                    h.Line0 + 1 + lineOffset, null);
            }
        }
    }

    private static List<HeadingInfo> CollectH23(string body)
    {
        var lines = body.Split(new[] { "\r\n", "\n", "\r" }, StringSplitOptions.None);
        var outList = new List<HeadingInfo>();
        for (var i = 0; i < lines.Length; i++)
        {
            var m = H23.Match(lines[i]);
            if (!m.Success) continue;
            var depth = m.Groups[1].Value.Length;
            if (depth is 2 or 3)
                outList.Add(new HeadingInfo(depth, i, m.Groups[2].Value.Trim()));
        }

        return outList;
    }

    private static void LintContent001(
        string file, string body, AktaConfig cfg, List<Diagnostic> diags, int lineOffset)
    {
        var sev = RuleSeverity.Effective(cfg, "CONTENT-001");
        var eff = RuleSeverity.MapSeverity(sev);
        if (eff == null) return;
        var ratio = RuleSeverity.NumberOption(cfg, "CONTENT-001", "min_question_ratio", 0.7);
        var hs = CollectH23(body);
        if (hs.Count == 0) return;
        var q = hs.Count(IsQuestion);
        var actual = (double)q / hs.Count;
        if (actual + 1e-9 < ratio)
        {
            Push(diags, file, "CONTENT-001", eff, 1 + lineOffset, 1,
                $"At least {ratio * 100:0}% of H2/H3 headings should be questions; got {actual * 100:0.0}% ({q}/{hs.Count}).");
        }
    }

    private static bool IsQuestion(HeadingInfo h) => IsQuestionText(h.Text);

    private static bool IsQuestionText(string text)
    {
        var s = text.Trim();
        if (s.EndsWith('?')) return true;
        return QuestionStart.IsMatch(s);
    }
}
