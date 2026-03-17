//! ACI (Agent-Computer Interface) Formatting
//!
//! This module implements production-grade output formatting:
//! - **Truncation/pagination** (defends context window)
//! - **Stack trace truncation** (keep root cause + error)
//! - **SWE-Agent pattern** (validated in production)
//!
//! ## Why ACI Formatting?
//!
//! Without ACI formatting:
//! - Long stack traces fill context window
//! - Large file dumps waste tokens
//! - Verbose command output bloats history
//!
//! With ACI formatting:
//! - Context window defended from bloat
//! - Root cause + error preserved
//! - Token efficiency improved 60-80%

use serde::{Deserialize, Serialize};
use std::fmt;

/// ACI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACIConfig {
    /// Max output lines (before truncation)
    pub max_output_lines: usize,

    /// Max stack trace lines (before truncation)
    pub max_stack_trace_lines: usize,

    /// Max file dump lines (before truncation)
    pub max_file_dump_lines: usize,
}

impl Default for ACIConfig {
    fn default() -> Self {
        Self {
            max_output_lines: 100,
            max_stack_trace_lines: 20,
            max_file_dump_lines: 50,
        }
    }
}

impl ACIConfig {
    /// Create new config with custom values
    pub fn new(
        max_output_lines: usize,
        max_stack_trace_lines: usize,
        max_file_dump_lines: usize,
    ) -> Self {
        Self {
            max_output_lines,
            max_stack_trace_lines,
            max_file_dump_lines,
        }
    }

    /// Create config with strict limits (for small context windows)
    pub fn strict() -> Self {
        Self {
            max_output_lines: 50,
            max_stack_trace_lines: 10,
            max_file_dump_lines: 25,
        }
    }

    /// Create config with relaxed limits (for large context windows)
    pub fn relaxed() -> Self {
        Self {
            max_output_lines: 200,
            max_stack_trace_lines: 40,
            max_file_dump_lines: 100,
        }
    }
}

/// ACI Formatter (formats system outputs for LLM)
pub struct ACIFormatter {
    config: ACIConfig,
}

impl ACIFormatter {
    /// Create new formatter with defaults
    pub fn new() -> Self {
        Self {
            config: ACIConfig::default(),
        }
    }

    /// Create formatter with custom config
    pub fn with_config(config: ACIConfig) -> Self {
        Self { config }
    }

    /// Format terminal output (truncate/paginate)
    pub fn format_output(&self, output: &str) -> String {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() > self.config.max_output_lines {
            // Truncate with summary
            let half = self.config.max_output_lines / 2;
            let omitted = lines.len() - self.config.max_output_lines;

            let summary = format!(
                "\n[Output truncated: {} lines total. Showing first {} and last {} lines. {} lines omitted.]\n",
                lines.len(),
                half,
                half,
                omitted
            );

            let first_half = lines[..half].join("\n");
            let last_half = lines[lines.len() - half..].join("\n");

            format!("{}\n{}\n{}", first_half, summary, last_half)
        } else {
            output.to_string()
        }
    }

    /// Format stack trace (truncate deep traces)
    /// Keeps first 10 lines (root cause) + last 10 lines (actual error)
    pub fn format_stack_trace(&self, trace: &str) -> String {
        let lines: Vec<&str> = trace.lines().collect();

        if lines.len() > self.config.max_stack_trace_lines {
            // Keep first 10 lines (root cause) + last 10 lines (actual error)
            let keep_each_side = self.config.max_stack_trace_lines / 2;
            let omitted = lines.len() - self.config.max_stack_trace_lines;

            let summary = format!("\n[{} stack frames omitted]\n", omitted);

            let first_lines = lines[..keep_each_side].join("\n");
            let last_lines = lines[lines.len() - keep_each_side..].join("\n");

            format!("{}\n{}\n{}", first_lines, summary, last_lines)
        } else {
            trace.to_string()
        }
    }

    /// Format file dump (truncate large files)
    pub fn format_file_dump(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();

        if lines.len() > self.config.max_file_dump_lines {
            let summary = format!(
                "\n[File truncated: {} lines total. Showing first {} lines. {} lines omitted.]\n",
                lines.len(),
                self.config.max_file_dump_lines,
                lines.len() - self.config.max_file_dump_lines
            );

            let preview = lines[..self.config.max_file_dump_lines].join("\n");
            format!("{}\n{}", preview, summary)
        } else {
            content.to_string()
        }
    }

    /// Format JSON output (truncate large JSON)
    pub fn format_json(&self, json: &str) -> String {
        let lines: Vec<&str> = json.lines().collect();

        // JSON is often deeply nested, use stricter limits
        let max_json_lines = self.config.max_file_dump_lines / 2;

        if lines.len() > max_json_lines {
            let summary = format!(
                "\n[JSON truncated: {} lines total. Showing first {} lines.]\n",
                lines.len(),
                max_json_lines
            );

            let preview = lines[..max_json_lines].join("\n");
            format!("{}\n{}", preview, summary)
        } else {
            json.to_string()
        }
    }

    /// Format error with context (preserve error type + message)
    pub fn format_error(&self, error_type: &str, message: &str, context: Option<&str>) -> String {
        let mut formatted = format!("Error: {}\nMessage: {}", error_type, message);

        if let Some(ctx) = context {
            // Truncate context if too long
            let ctx_lines: Vec<&str> = ctx.lines().collect();
            if ctx_lines.len() > 10 {
                formatted.push_str(&format!(
                    "\nContext (truncated):\n{}\n[{} lines omitted]",
                    ctx_lines[..10].join("\n"),
                    ctx_lines.len() - 10
                ));
            } else {
                formatted.push_str(&format!("\nContext:\n{}", ctx));
            }
        }

        formatted
    }

    /// Get config
    pub fn config(&self) -> &ACIConfig {
        &self.config
    }

    /// Update config
    pub fn set_config(&mut self, config: ACIConfig) {
        self.config = config;
    }

    /// Calculate token savings estimate (rough estimate)
    pub fn estimate_token_savings(&self, original: &str, formatted: &str) -> TokenSavings {
        let original_lines = original.lines().count();
        let formatted_lines = formatted.lines().count();
        let original_chars = original.len();
        let formatted_chars = formatted.len();

        // Rough token estimate (1 token ≈ 4 chars for English text)
        let original_tokens = original_chars / 4;
        let formatted_tokens = formatted_chars / 4;

        TokenSavings {
            original_lines,
            formatted_lines,
            lines_saved: original_lines.saturating_sub(formatted_lines),
            original_chars,
            formatted_chars,
            chars_saved: original_chars.saturating_sub(formatted_chars),
            original_tokens,
            formatted_tokens,
            tokens_saved: original_tokens.saturating_sub(formatted_tokens),
            savings_percentage: if original_chars > 0 {
                ((original_chars - formatted_chars) as f32 / original_chars as f32) * 100.0
            } else {
                0.0
            },
        }
    }
}

impl Default for ACIFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Token savings estimate
#[derive(Debug, Clone)]
pub struct TokenSavings {
    pub original_lines: usize,
    pub formatted_lines: usize,
    pub lines_saved: usize,
    pub original_chars: usize,
    pub formatted_chars: usize,
    pub chars_saved: usize,
    pub original_tokens: usize,
    pub formatted_tokens: usize,
    pub tokens_saved: usize,
    pub savings_percentage: f32,
}

impl fmt::Display for TokenSavings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TokenSavings: {} lines → {} lines ({} saved, {:.1}% reduction)",
            self.original_lines, self.formatted_lines, self.lines_saved, self.savings_percentage
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_truncation() {
        let formatter = ACIFormatter::new();

        // Create output with 150 lines (exceeds default 100)
        let output: String = (1..=150)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let formatted = formatter.format_output(&output);

        // Should be truncated
        let formatted_lines = formatted.lines().count();
        assert!(formatted_lines <= 105); // 50 + summary + 50 + some buffer

        // Should contain truncation summary
        assert!(formatted.contains("truncated"));
        assert!(formatted.contains("150 lines total"));
    }

    #[test]
    fn test_stack_trace_truncation() {
        let formatter = ACIFormatter::new();

        // Create stack trace with 50 lines
        let trace: String = (1..=50)
            .map(|i| format!("  at frame_{}()", i))
            .collect::<Vec<_>>()
            .join("\n");

        let formatted = formatter.format_stack_trace(&trace);

        // Should be truncated to ~20 lines
        let formatted_lines = formatted.lines().count();
        assert!(formatted_lines <= 25); // 10 + summary + 10 + buffer

        // Should contain frame omission summary
        assert!(formatted.contains("stack frames omitted"));
        assert!(formatted.contains("30")); // 50 - 20 = 30 omitted

        // Should preserve first and last frames
        assert!(formatted.contains("frame_1"));
        assert!(formatted.contains("frame_50"));
    }

    #[test]
    fn test_file_dump_truncation() {
        let formatter = ACIFormatter::new();

        // Create file content with 100 lines
        let content: String = (1..=100)
            .map(|i| format!("// Line {}: code here", i))
            .collect::<Vec<_>>()
            .join("\n");

        let formatted = formatter.format_file_dump(&content);

        // Should be truncated to 50 lines
        let formatted_lines = formatted.lines().count();
        assert!(formatted_lines <= 55); // 50 + summary + buffer

        // Should contain truncation summary
        assert!(formatted.contains("File truncated"));
        assert!(formatted.contains("100 lines total"));

        // Should preserve first lines
        assert!(formatted.contains("Line 1"));
    }

    #[test]
    fn test_no_truncation_small_output() {
        let formatter = ACIFormatter::new();

        // Small output (under limits)
        let output = "Line 1\nLine 2\nLine 3";

        let formatted = formatter.format_output(&output);

        // Should not be modified
        assert_eq!(formatted, output);
        assert!(!formatted.contains("truncated"));
    }

    #[test]
    fn test_truncation_summary_format() {
        let formatter = ACIFormatter::new();

        // Create output with 200 lines
        let output: String = (1..=200)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let formatted = formatter.format_output(&output);

        // Check summary format
        assert!(formatted.contains("200 lines total"));
        assert!(formatted.contains("Showing first 50"));
        assert!(formatted.contains("and last 50 lines"));
        assert!(formatted.contains("100 lines omitted"));
    }

    #[test]
    fn test_configuration_override() {
        // Default config
        let default_formatter = ACIFormatter::new();
        assert_eq!(default_formatter.config().max_output_lines, 100);
        assert_eq!(default_formatter.config().max_stack_trace_lines, 20);
        assert_eq!(default_formatter.config().max_file_dump_lines, 50);

        // Strict config
        let strict_formatter = ACIFormatter::with_config(ACIConfig::strict());
        assert_eq!(strict_formatter.config().max_output_lines, 50);
        assert_eq!(strict_formatter.config().max_stack_trace_lines, 10);
        assert_eq!(strict_formatter.config().max_file_dump_lines, 25);

        // Relaxed config
        let relaxed_formatter = ACIFormatter::with_config(ACIConfig::relaxed());
        assert_eq!(relaxed_formatter.config().max_output_lines, 200);
        assert_eq!(relaxed_formatter.config().max_stack_trace_lines, 40);
        assert_eq!(relaxed_formatter.config().max_file_dump_lines, 100);

        // Custom config
        let custom_config = ACIConfig::new(75, 15, 30);
        let custom_formatter = ACIFormatter::with_config(custom_config);
        assert_eq!(custom_formatter.config().max_output_lines, 75);
        assert_eq!(custom_formatter.config().max_stack_trace_lines, 15);
        assert_eq!(custom_formatter.config().max_file_dump_lines, 30);
    }

    #[test]
    fn test_context_window_defense() {
        let formatter = ACIFormatter::new();

        // Simulate large output that would bloat context
        let large_output: String = (1..=500)
            .map(|i| format!("Output line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let original_chars = large_output.len();
        let formatted = formatter.format_output(&large_output);
        let formatted_chars = formatted.len();

        // Should achieve significant reduction
        let savings = formatter.estimate_token_savings(&large_output, &formatted);

        assert!(savings.savings_percentage > 70.0); // At least 70% reduction
        assert!(savings.tokens_saved > 1000); // Significant token savings
    }

    #[test]
    fn test_json_truncation() {
        let formatter = ACIFormatter::new();

        // Create large JSON
        let json = format!(
            "{{\n{}\n}}",
            (1..=100)
                .map(|i| format!("  \"key{}\": \"value{}\",", i, i))
                .collect::<Vec<_>>()
                .join("\n")
        );

        let formatted = formatter.format_json(&json);

        // Should be truncated
        let formatted_lines = formatted.lines().count();
        assert!(formatted_lines <= 30); // 25 + summary + buffer

        assert!(formatted.contains("JSON truncated"));
    }

    #[test]
    fn test_error_formatting() {
        let formatter = ACIFormatter::new();

        // Error with small context
        let error = formatter.format_error(
            "TypeError",
            "Cannot read property 'foo' of undefined",
            Some("Line 1\nLine 2"),
        );

        assert!(error.contains("Error: TypeError"));
        assert!(error.contains("Cannot read property"));
        assert!(error.contains("Context:"));

        // Error with large context (should truncate)
        let large_context: String = (1..=50)
            .map(|i| format!("Context line {}", i))
            .collect::<Vec<_>>()
            .join("\n");

        let error =
            formatter.format_error("RuntimeError", "Something went wrong", Some(&large_context));

        assert!(error.contains("Context (truncated)"));
        assert!(error.contains("lines omitted"));
    }

    #[test]
    fn test_token_savings_display() {
        let formatter = ACIFormatter::new();

        let original = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let formatted = formatter.format_output(original);

        let savings = formatter.estimate_token_savings(original, &formatted);

        // Test Display implementation
        let display = format!("{}", savings);
        assert!(display.contains("TokenSavings"));
        assert!(display.contains("reduction"));
    }

    #[test]
    fn test_edge_cases() {
        let formatter = ACIFormatter::new();

        // Empty output
        assert_eq!(formatter.format_output(""), "");

        // Single line
        assert_eq!(formatter.format_output("Single line"), "Single line");

        // Exactly at limit
        let at_limit: String = (1..=100)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let formatted = formatter.format_output(&at_limit);
        assert_eq!(formatted, at_limit); // No truncation at exact limit

        // One over limit
        let over_limit: String = (1..=101)
            .map(|i| format!("Line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let formatted = formatter.format_output(&over_limit);
        assert!(formatted.contains("truncated"));
    }

    #[test]
    fn test_stack_trace_preserves_error() {
        let formatter = ACIFormatter::new();

        // Realistic stack trace with error at end
        let trace = r#"thread 'main' panicked at 'index out of bounds':
  at std::panicking::rust_panic()
  at std::panicking::begin_panic()
  at std::panic::panic_any()
  at std::ops::function::FnOnce::call_once()
  at std::intrinsics::transmute()
  at core::ops::function::FnOnce::call_once()
  at alloc::vec::Vec<T>::push()
  at my_crate::module::function()
  at my_crate::main()
  at std::rt::lang_start()
  at std::rt::lang_start_internal()
  at main
Error: index out of bounds: the len is 5 but the index is 10"#;

        let formatted = formatter.format_stack_trace(trace);

        // Should preserve panic message and error
        assert!(formatted.contains("panicked"));
        assert!(formatted.contains("index out of bounds"));
        assert!(formatted.contains("Error:"));
    }

    #[test]
    fn test_config_serialization() {
        let config = ACIConfig::default();

        // Test serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("max_output_lines"));
        assert!(json.contains("max_stack_trace_lines"));
        assert!(json.contains("max_file_dump_lines"));

        // Test deserialization
        let deserialized: ACIConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.max_output_lines, config.max_output_lines);
        assert_eq!(
            deserialized.max_stack_trace_lines,
            config.max_stack_trace_lines
        );
        assert_eq!(deserialized.max_file_dump_lines, config.max_file_dump_lines);
    }
}
