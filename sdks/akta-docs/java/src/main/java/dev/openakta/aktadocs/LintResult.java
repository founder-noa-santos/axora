package dev.openakta.aktadocs;

import java.util.List;

public record LintResult(List<Diagnostic> diagnostics, LintSummary summary) {}
