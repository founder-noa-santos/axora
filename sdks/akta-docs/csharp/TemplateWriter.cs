namespace OpenAkta.AktaDocs;

public static class TemplateWriter
{
    private static readonly HashSet<string> Kinds = new(StringComparer.Ordinal)
    {
        "adr", "business_rule", "feature", "guide", "reference", "explanation", "research", "meta",
        "changelog", "technical", "other"
    };

    private const string QuickAdr =
        "This ADR records one architecture decision including context, the decision itself, "
        + "and consequences for teams. It explains why we chose this path and which trade-offs "
        + "we accept for maintenance, operations, and future migrations. Readers should leave "
        + "with a clear yes or no on scope.";

    private const string QuickBr =
        "This document defines one business rule covering actors, scope, and enforcement. "
        + "It states the invariant in plain language and points to validation or audit hooks. "
        + "The goal is to align product, legal, and engineering without ambiguous edge cases.";

    private static string SectionBlock()
    {
        var parts = new string[200];
        for (var i = 0; i < 200; i++)
            parts[i] = "w" + (i + 1);
        return string.Join(" ", parts);
    }

    public static void Write(string kind, string outputPath, string title, string slug, string docId, string date)
    {
        if (!Kinds.Contains(kind))
            throw new ArgumentException("Invalid kind: " + kind);

        string body;
        if (kind == "adr")
        {
            body =
                "---\n" +
                "doc_id: " + docId + "\n" +
                "doc_type: adr\n" +
                "date: " + date + "\n" +
                "---\n\n" +
                "# " + title + "\n\n```quick\n" +
                QuickAdr +
                "\n```\n\n" +
                "## Why does this decision matter for the product?\n\n" +
                SectionBlock() + "\n\n" +
                "## What constraints shaped the available options?\n\n" +
                SectionBlock() + "\n\n" +
                "## Which option did we select and why?\n\n" +
                SectionBlock() + "\n";
        }
        else if (kind == "business_rule")
        {
            body =
                "---\n" +
                "doc_id: " + docId + "\n" +
                "doc_type: business_rule\n" +
                "date: " + date + "\n" +
                "---\n\n" +
                "# " + title + "\n\n```quick\n" +
                QuickBr +
                "\n```\n\n" +
                "## Who must follow this rule and when?\n\n" +
                SectionBlock() + "\n\n" +
                "## What is the exact rule or invariant?\n\n" +
                SectionBlock() + "\n\n" +
                "## How do we validate or audit compliance?\n\n" +
                SectionBlock() + "\n";
        }
        else
        {
            var qg =
                "This page documents " + slug +
                " for the repository. It orients readers before deeper sections and keeps token use predictable for retrieval. Skim the quick answer first, then jump to the question headings that match your task.";
            body =
                "---\n" +
                "doc_id: " + docId + "\n" +
                "doc_type: " + kind + "\n" +
                "date: " + date + "\n" +
                "---\n\n" +
                "# " + title + "\n\n```quick\n" +
                qg +
                "\n```\n\n" +
                "## What problem does this page solve?\n\n" +
                SectionBlock() + "\n\n" +
                "## What are the key facts or steps?\n\n" +
                SectionBlock() + "\n\n" +
                "## Where should readers go next?\n\n" +
                SectionBlock() + "\n";
        }

        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(outputPath))!);
        File.WriteAllText(outputPath, body);
    }
}
