package dev.openakta.aktadocs;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Set;

public final class TemplateWriter {

    private static final Set<String> KINDS =
            Set.of(
                    "adr",
                    "business_rule",
                    "feature",
                    "guide",
                    "reference",
                    "explanation",
                    "research",
                    "meta",
                    "changelog",
                    "technical",
                    "other");

    private static final String QUICK_ADR =
            "This ADR records one architecture decision including context, the decision itself, "
                    + "and consequences for teams. It explains why we chose this path and which trade-offs "
                    + "we accept for maintenance, operations, and future migrations. Readers should leave "
                    + "with a clear yes or no on scope.";

    private static final String QUICK_BR =
            "This document defines one business rule covering actors, scope, and enforcement. "
                    + "It states the invariant in plain language and points to validation or audit hooks. "
                    + "The goal is to align product, legal, and engineering without ambiguous edge cases.";

    private TemplateWriter() {}

    public static void write(
            String kind, Path outputPath, String title, String slug, String docId, String date)
            throws IOException {
        if (!KINDS.contains(kind)) {
            throw new IllegalArgumentException("Invalid kind: " + kind);
        }
        Files.createDirectories(outputPath.getParent());
        String body;
        if ("adr".equals(kind)) {
            body =
                    "---\n"
                            + "doc_id: "
                            + docId
                            + "\n"
                            + "doc_type: adr\n"
                            + "date: "
                            + date
                            + "\n"
                            + "---\n\n"
                            + "# "
                            + title
                            + "\n\n```quick\n"
                            + QUICK_ADR
                            + "\n```\n\n"
                            + "## Why does this decision matter for the product?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## What constraints shaped the available options?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## Which option did we select and why?\n\n"
                            + sectionBlock()
                            + "\n";
        } else if ("business_rule".equals(kind)) {
            body =
                    "---\n"
                            + "doc_id: "
                            + docId
                            + "\n"
                            + "doc_type: business_rule\n"
                            + "date: "
                            + date
                            + "\n"
                            + "---\n\n"
                            + "# "
                            + title
                            + "\n\n```quick\n"
                            + QUICK_BR
                            + "\n```\n\n"
                            + "## Who must follow this rule and when?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## What is the exact rule or invariant?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## How do we validate or audit compliance?\n\n"
                            + sectionBlock()
                            + "\n";
        } else {
            String qg =
                    "This page documents "
                            + slug
                            + " for the repository. It orients readers before deeper sections and keeps token use predictable for retrieval. Skim the quick answer first, then jump to the question headings that match your task.";
            body =
                    "---\n"
                            + "doc_id: "
                            + docId
                            + "\n"
                            + "doc_type: "
                            + kind
                            + "\n"
                            + "date: "
                            + date
                            + "\n"
                            + "---\n\n"
                            + "# "
                            + title
                            + "\n\n```quick\n"
                            + qg
                            + "\n```\n\n"
                            + "## What problem does this page solve?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## What are the key facts or steps?\n\n"
                            + sectionBlock()
                            + "\n\n"
                            + "## Where should readers go next?\n\n"
                            + sectionBlock()
                            + "\n";
        }
        Files.writeString(outputPath, body);
    }

    private static String sectionBlock() {
        StringBuilder sb = new StringBuilder();
        for (int i = 1; i <= 200; i++) {
            if (i > 1) sb.append(' ');
            sb.append('w').append(i);
        }
        return sb.toString();
    }
}
