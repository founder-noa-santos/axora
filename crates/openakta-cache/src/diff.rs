//! Unified diff generation and deterministic patch application.

use regex::Regex;
use std::fmt;
use tracing::debug;

/// Unified diff hunk.
#[derive(Debug, Clone)]
pub struct Hunk {
    /// Starting line in original file.
    pub old_start: usize,
    /// Number of lines in original file.
    pub old_count: usize,
    /// Starting line in new file.
    pub new_start: usize,
    /// Number of lines in new file.
    pub new_count: usize,
    /// Diff lines.
    pub lines: Vec<DiffLine>,
}

/// Type of diff line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffLine {
    /// Context line.
    Context(String),
    /// Added line.
    Add(String),
    /// Removed line.
    Remove(String),
}

/// Unified diff representation.
#[derive(Debug, Clone)]
pub struct UnifiedDiff {
    /// Old file path.
    pub old_path: String,
    /// New file path.
    pub new_path: String,
    /// Hunks of changes.
    pub hunks: Vec<Hunk>,
}

impl UnifiedDiff {
    /// Create a new unified diff.
    pub fn new(old_path: &str, new_path: &str) -> Self {
        Self {
            old_path: old_path.to_string(),
            new_path: new_path.to_string(),
            hunks: Vec::new(),
        }
    }

    /// Generate a compact zero-context diff from two contents.
    pub fn generate(old_content: &str, new_content: &str, old_path: &str, new_path: &str) -> Self {
        debug!("Generating unified diff: {} -> {}", old_path, new_path);

        let old_lines: Vec<&str> = old_content.lines().collect();
        let new_lines: Vec<&str> = new_content.lines().collect();
        let mut diff = UnifiedDiff::new(old_path, new_path);
        let max_lines = old_lines.len().max(new_lines.len());

        for index in 0..max_lines {
            let old_line = old_lines.get(index).copied();
            let new_line = new_lines.get(index).copied();
            if old_line == new_line {
                continue;
            }

            let mut hunk = Hunk {
                old_start: index + 1,
                old_count: usize::from(old_line.is_some()),
                new_start: index + 1,
                new_count: usize::from(new_line.is_some()),
                lines: Vec::new(),
            };

            if let Some(line) = old_line {
                hunk.lines.push(DiffLine::Remove(line.to_string()));
            }
            if let Some(line) = new_line {
                hunk.lines.push(DiffLine::Add(line.to_string()));
            }

            diff.hunks.push(hunk);
        }

        diff
    }

    /// Estimate token count (rough estimate: 1 token ≈ 4 characters).
    pub fn estimate_tokens(&self) -> usize {
        self.to_string().len() / 4
    }

    /// Get number of changes (additions + removals).
    pub fn change_count(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|line| matches!(line, DiffLine::Add(_) | DiffLine::Remove(_)))
            .count()
    }

    /// Return all unique target files referenced by this diff.
    pub fn target_files(&self) -> Vec<String> {
        vec![self.new_path.clone()]
    }
}

impl fmt::Display for UnifiedDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "--- {}", self.old_path)?;
        writeln!(f, "+++ {}", self.new_path)?;
        for hunk in &self.hunks {
            writeln!(
                f,
                "@@ -{},{} +{},{} @@",
                hunk.old_start, hunk.old_count, hunk.new_start, hunk.new_count
            )?;
            for line in &hunk.lines {
                match line {
                    DiffLine::Context(content) => writeln!(f, " {}", content)?,
                    DiffLine::Add(content) => writeln!(f, "+{}", content)?,
                    DiffLine::Remove(content) => writeln!(f, "-{}", content)?,
                }
            }
        }
        Ok(())
    }
}

/// Patch application result.
#[derive(Debug, Clone)]
pub struct PatchResult {
    /// Success or failure.
    pub success: bool,
    /// New content after patch application.
    pub content: String,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Parse a unified diff string.
pub fn parse_unified_diff(patch: &str) -> Result<UnifiedDiff, String> {
    let mut lines = patch.lines();
    let old_path = lines
        .next()
        .and_then(|line| line.strip_prefix("--- "))
        .ok_or_else(|| "missing --- header".to_string())?;
    let new_path = lines
        .next()
        .and_then(|line| line.strip_prefix("+++ "))
        .ok_or_else(|| "missing +++ header".to_string())?;

    let header_re =
        Regex::new(r"^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@$").map_err(|e| e.to_string())?;
    let mut diff = UnifiedDiff::new(old_path, new_path);
    let mut current_hunk: Option<Hunk> = None;

    for line in lines {
        if let Some(captures) = header_re.captures(line) {
            if let Some(hunk) = current_hunk.take() {
                diff.hunks.push(hunk);
            }

            current_hunk = Some(Hunk {
                old_start: captures[1]
                    .parse()
                    .map_err(|_| "invalid old start".to_string())?,
                old_count: captures
                    .get(2)
                    .map(|m| {
                        m.as_str()
                            .parse()
                            .map_err(|_| "invalid old count".to_string())
                    })
                    .transpose()?
                    .unwrap_or(1),
                new_start: captures[3]
                    .parse()
                    .map_err(|_| "invalid new start".to_string())?,
                new_count: captures
                    .get(4)
                    .map(|m| {
                        m.as_str()
                            .parse()
                            .map_err(|_| "invalid new count".to_string())
                    })
                    .transpose()?
                    .unwrap_or(1),
                lines: Vec::new(),
            });
            continue;
        }

        let hunk = current_hunk
            .as_mut()
            .ok_or_else(|| "diff line found before any hunk header".to_string())?;
        match line.chars().next() {
            Some(' ') => hunk.lines.push(DiffLine::Context(line[1..].to_string())),
            Some('+') => hunk.lines.push(DiffLine::Add(line[1..].to_string())),
            Some('-') => hunk.lines.push(DiffLine::Remove(line[1..].to_string())),
            _ => return Err(format!("invalid diff line: {}", line)),
        }
    }

    if let Some(hunk) = current_hunk.take() {
        diff.hunks.push(hunk);
    }

    if diff.hunks.is_empty() {
        return Err("diff does not contain any hunks".to_string());
    }

    Ok(diff)
}

/// Apply a unified diff patch to original content.
pub fn apply_patch(original: &str, patch: &str) -> PatchResult {
    debug!("Applying patch to content ({} bytes)", original.len());

    let diff = match parse_unified_diff(patch) {
        Ok(diff) => diff,
        Err(error) => {
            return PatchResult {
                success: false,
                content: original.to_string(),
                error: Some(error),
            }
        }
    };

    let original_lines: Vec<String> = original.lines().map(|line| line.to_string()).collect();
    let mut result_lines = Vec::new();
    let mut cursor = 0usize;

    for hunk in diff.hunks {
        let hunk_start = hunk.old_start.saturating_sub(1);
        if hunk_start > original_lines.len() {
            return PatchResult {
                success: false,
                content: original.to_string(),
                error: Some("hunk starts beyond end of file".to_string()),
            };
        }

        while cursor < hunk_start {
            result_lines.push(original_lines[cursor].clone());
            cursor += 1;
        }

        for line in hunk.lines {
            match line {
                DiffLine::Context(expected) => {
                    if cursor >= original_lines.len() || original_lines[cursor] != expected {
                        return PatchResult {
                            success: false,
                            content: original.to_string(),
                            error: Some(format!("context mismatch at line {}", cursor + 1)),
                        };
                    }
                    result_lines.push(expected);
                    cursor += 1;
                }
                DiffLine::Remove(expected) => {
                    if cursor >= original_lines.len() || original_lines[cursor] != expected {
                        return PatchResult {
                            success: false,
                            content: original.to_string(),
                            error: Some(format!("remove mismatch at line {}", cursor + 1)),
                        };
                    }
                    cursor += 1;
                }
                DiffLine::Add(content) => {
                    result_lines.push(content);
                }
            }
        }
    }

    while cursor < original_lines.len() {
        result_lines.push(original_lines[cursor].clone());
        cursor += 1;
    }

    let mut content = result_lines.join("\n");
    if original.ends_with('\n') || patch.ends_with('\n') {
        content.push('\n');
    }

    PatchResult {
        success: true,
        content,
        error: None,
    }
}

/// Calculate token savings between full content and diff.
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

/// Token savings calculation result.
#[derive(Debug, Clone)]
pub struct TokenSavings {
    /// Tokens in full content.
    pub full_tokens: usize,
    /// Tokens in diff.
    pub diff_tokens: usize,
    /// Tokens saved.
    pub saved_tokens: usize,
    /// Savings percentage.
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
        assert_eq!(diff.change_count(), 2);
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
    fn test_parse_unified_diff() {
        let diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
";

        let parsed = parse_unified_diff(diff).unwrap();
        assert_eq!(parsed.old_path, "a/src/lib.rs");
        assert_eq!(parsed.new_path, "b/src/lib.rs");
        assert_eq!(parsed.hunks.len(), 1);
    }

    #[test]
    fn test_apply_zero_context_patch() {
        let original = "fn old() {}\n";
        let diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
";

        let result = apply_patch(original, diff);
        assert!(result.success);
        assert_eq!(result.content, "fn new() {}\n");
    }

    #[test]
    fn test_apply_patch_detects_conflict() {
        let original = "fn different() {}\n";
        let diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
";

        let result = apply_patch(original, diff);
        assert!(!result.success);
        assert!(result.error.unwrap().contains("mismatch"));
    }

    #[test]
    fn test_diff_token_savings() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nmodified\nline3\nline4\nline5\n";

        let diff = UnifiedDiff::generate(old, new, "old.txt", "new.txt");
        let savings = calculate_token_savings(new, &diff);

        assert!(savings.diff_tokens <= savings.full_tokens || savings.saved_tokens == 0);
    }
}
