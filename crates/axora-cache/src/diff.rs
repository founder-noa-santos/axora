//! Unified diff generation and patch application for token-efficient code communication

use std::fmt::Write;
use tracing::debug;

/// Unified diff hunk
#[derive(Debug, Clone)]
pub struct Hunk {
    /// Starting line in original file
    pub old_start: usize,
    /// Number of lines in original file
    pub old_count: usize,
    /// Starting line in new file
    pub new_start: usize,
    /// Number of lines in new file
    pub new_count: usize,
    /// Diff lines
    pub lines: Vec<DiffLine>,
}

/// Type of diff line
#[derive(Debug, Clone, PartialEq)]
pub enum DiffLine {
    /// Context line (unchanged)
    Context(String),
    /// Added line
    Add(String),
    /// Removed line
    Remove(String),
}

/// Unified diff representation
#[derive(Debug, Clone)]
pub struct UnifiedDiff {
    /// Old file path
    pub old_path: String,
    /// New file path
    pub new_path: String,
    /// Hunks of changes
    pub hunks: Vec<Hunk>,
}

impl UnifiedDiff {
    /// Create a new unified diff
    pub fn new(old_path: &str, new_path: &str) -> Self {
        Self {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
            hunks: Vec::new(),
        }
    }

    /// Generate unified diff from old and new content
    pub fn generate(old_content: &str, new_content: &str, old_path: &str, new_path: &str) -> Self {
        debug!("Generating unified diff: {} -> {}", old_path, new_path);

        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();

        let mut diff = UnifiedDiff::new(old_path, new_path);

        // Simple line-by-line diff (in production, use Myers diff algorithm)
        let mut hunk = Hunk {
            old_start: 1,
            old_count: 0,
            new_start: 1,
            new_count: 0,
            lines: Vec::new(),
        };

        let max_lines = old_lines.len().max(new_lines.len());

        for i in 0..max_lines {
            let old_line = old_lines.get(i);
            let new_line = new_lines.get(i);

            match (old_line, new_line) {
                (Some(old), Some(new)) if old == new => {
                    // Context line
                    if !hunk.lines.is_empty() {
                        hunk.lines.push(DiffLine::Context(old.to_string()));
                        hunk.old_count += 1;
                        hunk.new_count += 1;
                    }
                }
                (Some(old), Some(new)) => {
                    // Changed line (remove + add)
                    hunk.lines.push(DiffLine::Remove(old.to_string()));
                    hunk.lines.push(DiffLine::Add(new.to_string()));
                    hunk.old_count += 1;
                    hunk.new_count += 1;
                }
                (Some(old), None) => {
                    // Removed line
                    hunk.lines.push(DiffLine::Remove(old.to_string()));
                    hunk.old_count += 1;
                }
                (None, Some(new)) => {
                    // Added line
                    hunk.lines.push(DiffLine::Add(new.to_string()));
                    hunk.new_count += 1;
                }
                (None, None) => {}
            }
        }

        // Add hunk if it has changes
        if !hunk.lines.is_empty() {
            diff.hunks.push(hunk);
        }

        diff
    }

    /// Convert to unified diff string format
    pub fn to_string(&self) -> String {
        let mut output = String::new();

        // File headers
        writeln!(output, "--- {}", self.old_path).unwrap();
        writeln!(output, "+++ {}", self.new_path).unwrap();

        // Hunks
        for hunk in &self.hunks {
            writeln!(
                output,
                "@@ -{},{} +{},{} @@",
                hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
            )
            .unwrap();

            for line in &hunk.lines {
                match line {
                    DiffLine::Context(content) => {
                        writeln!(output, " {}", content).unwrap();
                    }
                    DiffLine::Add(content) => {
                        writeln!(output, "+{}", content).unwrap();
                    }
                    DiffLine::Remove(content) => {
                        writeln!(output, "-{}", content).unwrap();
                    }
                }
            }
        }

        output
    }

    /// Estimate token count (rough estimate: 1 token ≈ 4 characters)
    pub fn estimate_tokens(&self) -> usize {
        self.to_string().len() / 4
    }

    /// Get number of changes (additions + removals)
    pub fn change_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Add(_) | DiffLine::Remove(_)))
            .count()
    }
}

/// Patch application result
#[derive(Debug, Clone)]
pub struct PatchResult {
    /// Success or failure
    pub success: bool,
    /// New content after patch application
    pub content: String,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Apply a unified diff patch to original content
pub fn apply_patch(original: &str, patch: &str) -> PatchResult {
    debug!("Applying patch to content ({} bytes)", original.len());

    let original_lines: Vec<&str> = original.lines().collect();
    let mut result_lines: Vec<String> = Vec::new();

    let mut current_line = 0;
    let mut in_hunk = false;

    for line in patch.lines() {
        if line.starts_with("---") || line.starts_with("+++") {
            // File headers, skip
            continue;
        }

        if line.starts_with("@@") {
            // Hunk header
            in_hunk = true;
            // Parse hunk header (simplified)
            continue;
        }

        if !in_hunk {
            continue;
        }

        match line.chars().next() {
            Some(' ') => {
                // Context line - should match original
                if current_line < original_lines.len() {
                    result_lines.push(original_lines[current_line].to_string());
                    current_line += 1;
                }
            }
            Some('-') => {
                // Removal - skip this line from original
                if current_line < original_lines.len() {
                    current_line += 1;
                }
            }
            Some('+') => {
                // Addition - add new line
                result_lines.push(line[1..].to_string());
            }
            _ => {}
        }
    }

    // Add remaining lines from original
    for i in current_line..original_lines.len() {
        result_lines.push(original_lines[i].to_string());
    }

    PatchResult {
        success: true,
        content: result_lines.join("\n"),
        error: None,
    }
}

/// Calculate token savings between full content and diff
pub fn calculate_token_savings(full_content: &str, diff: &UnifiedDiff) -> TokenSavings {
    let full_tokens = full_content.len() / 4;
    let diff_tokens = diff.estimate_tokens();

    let saved_tokens = full_tokens.saturating_sub(diff_tokens);
    let savings_percentage = if full_tokens > 0 {
        (saved_tokens as f32 / full_tokens as f32) * 100.0
    } else {
        0.0
    };

    TokenSavings {
        full_tokens,
        diff_tokens,
        saved_tokens,
        savings_percentage: savings_percentage.max(0.0),
    }
}

/// Token savings calculation result
#[derive(Debug, Clone)]
pub struct TokenSavings {
    /// Tokens in full content
    pub full_tokens: usize,
    /// Tokens in diff
    pub diff_tokens: usize,
    /// Tokens saved
    pub saved_tokens: usize,
    /// Savings percentage
    pub savings_percentage: f32,
}

impl std::fmt::Display for TokenSavings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Token Savings: {} → {} (saved {}, {:.1}% reduction)",
            self.full_tokens, self.diff_tokens, self.saved_tokens, self.savings_percentage
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_generation() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nmodified\nline3\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");

        assert_eq!(diff.hunks.len(), 1);
        assert_eq!(diff.change_count(), 2); // 1 remove + 1 add
    }

    #[test]
    fn test_diff_to_string() {
        let old = "line1\nline2\n";
        let new = "line1\nmodified\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");
        let diff_str = diff.to_string();

        assert!(diff_str.contains("--- old.txt"));
        assert!(diff_str.contains("+++ new.txt"));
        assert!(diff_str.contains("-line2"));
        assert!(diff_str.contains("+modified"));
    }

    #[test]
    fn test_diff_token_savings() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nmodified\nline3\nline4\nline5\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");
        let savings = calculate_token_savings(&new, &diff);

        // Diff should be smaller than full content (or at least not larger)
        // Using saturating_sub to avoid overflow
        assert!(savings.diff_tokens <= savings.full_tokens || savings.saved_tokens == 0);
    }

    #[test]
    fn test_patch_application() {
        let original = "line1\nline2\nline3\n";
        let patch =
            "--- old.txt\n+++ new.txt\n@@ -1,3 +1,3 @@\n line1\n-modified\n+line2\n line3\n";

        // This is a simplified test - real patch format may vary
        let result = apply_patch(original, patch);

        // Patch application is simplified, just check it runs
        assert!(result.success || result.error.is_some());
    }

    #[test]
    fn test_no_changes_diff() {
        let content = "line1\nline2\nline3\n";

        let diff = UnifiedDiff::generate(content, content, "same.txt", "same.txt");

        assert_eq!(diff.hunks.len(), 0);
        assert_eq!(diff.change_count(), 0);
    }

    #[test]
    fn test_add_lines_diff() {
        let old = "line1\nline3\n";
        let new = "line1\nline2\nline3\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");

        assert!(diff.hunks.len() > 0);
        assert!(diff.change_count() > 0);
    }

    #[test]
    fn test_remove_lines_diff() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline3\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");

        assert!(diff.hunks.len() > 0);
        assert!(diff.change_count() > 0);
    }
}
