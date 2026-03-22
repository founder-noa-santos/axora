package dev.openakta.aktadocs;

import com.fasterxml.jackson.annotation.JsonProperty;

public record LintSummary(
        @JsonProperty("error_count") int errorCount,
        @JsonProperty("warn_count") int warnCount
) {}
