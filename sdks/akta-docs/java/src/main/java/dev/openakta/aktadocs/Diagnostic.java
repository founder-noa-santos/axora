package dev.openakta.aktadocs;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.annotation.JsonProperty;

@JsonInclude(JsonInclude.Include.NON_NULL)
public record Diagnostic(
        String file,
        int line,
        int column,
        @JsonProperty("rule_id") String ruleId,
        String severity,
        String message,
        @JsonProperty("end_line") Integer endLine,
        @JsonProperty("end_column") Integer endColumn,
        @JsonProperty("doc_url") String docUrl
) {
    public Diagnostic(
            String file,
            int line,
            int column,
            String ruleId,
            String severity,
            String message,
            Integer endLine,
            Integer endColumn
    ) {
        this(file, line, column, ruleId, severity, message, endLine, endColumn, null);
    }
}
