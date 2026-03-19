//! Diff-first patch protocol and deterministic application.

use crate::error::AgentError;
use crate::Result;
use axora_cache::{apply_patch, parse_unified_diff, Schema, ToonSerializer};
use blake3::hash;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Compact control plane opcode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MetaGlyphOpcode {
    /// Read files or symbols.
    Read,
    /// Dispatch a patch operation.
    Patch,
    /// Run verification.
    Test,
    /// Request debugging.
    Debug,
}

/// Compact instruction emitted by the control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetaGlyphCommand {
    /// Opcode.
    pub opcode: MetaGlyphOpcode,
    /// Operand or target.
    pub operand: String,
}

impl MetaGlyphCommand {
    /// Render the symbolic command.
    pub fn render(&self) -> String {
        let op = match self.opcode {
            MetaGlyphOpcode::Read => "⟦READ⟧",
            MetaGlyphOpcode::Patch => "⟦PATCH⟧",
            MetaGlyphOpcode::Test => "⟦TEST⟧",
            MetaGlyphOpcode::Debug => "⟦DEBUG⟧",
        };

        format!("{op} {}", self.operand)
    }
}

/// Validation fact carried alongside patching operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationFact {
    /// Fact key.
    pub key: String,
    /// Fact value.
    pub value: String,
}

/// Summary of an AST target.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AstSummary {
    /// File path.
    pub file_path: String,
    /// Symbol path or scope path.
    pub symbol_path: String,
    /// Human-readable kind.
    pub kind: String,
    /// Start line.
    pub start_line: usize,
    /// End line.
    pub end_line: usize,
}

/// Symbol map entry delivered to a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolMap {
    /// File path.
    pub file_path: String,
    /// Symbol path.
    pub symbol_path: String,
    /// Referenced symbols.
    pub references: Vec<String>,
}

/// Retrieval hit delivered to a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RetrievalHit {
    /// File path.
    pub file_path: String,
    /// Symbol path.
    pub symbol_path: String,
    /// Start line.
    pub start_line: usize,
    /// End line.
    pub end_line: usize,
    /// Minimal snippet.
    pub snippet: String,
    /// Base revision for the snippet.
    pub base_revision: String,
}

/// Span reference inside a context pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextSpan {
    /// File path.
    pub file_path: String,
    /// Start line.
    pub start_line: usize,
    /// End line.
    pub end_line: usize,
    /// Symbol path.
    pub symbol_path: String,
}

/// LLM-facing context pack.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPack {
    /// Pack identifier.
    pub id: String,
    /// Owning task.
    pub task_id: String,
    /// Target files.
    pub target_files: Vec<String>,
    /// Relevant symbols.
    pub symbols: Vec<String>,
    /// Relevant spans.
    pub spans: Vec<ContextSpan>,
    /// Retrieval hits.
    pub retrieval_hits: Vec<RetrievalHit>,
    /// AST summaries.
    pub ast_summaries: Vec<AstSummary>,
    /// Symbol maps.
    pub symbol_maps: Vec<SymbolMap>,
    /// Validation facts.
    pub validation_facts: Vec<ValidationFact>,
    /// Base revision used for the pack.
    pub base_revision: String,
}

impl ContextPack {
    fn schema() -> Schema {
        let mut schema = Schema::new();
        for field in [
            "id",
            "task_id",
            "target_files",
            "symbols",
            "spans",
            "retrieval_hits",
            "ast_summaries",
            "symbol_maps",
            "validation_facts",
            "base_revision",
            "file_path",
            "start_line",
            "end_line",
            "symbol_path",
            "snippet",
            "base_revision",
            "kind",
            "references",
            "key",
            "value",
        ] {
            schema.add_field(field);
        }
        schema
    }

    /// Serialize the pack into TOON for the LLM boundary.
    pub fn to_toon(&self) -> Result<String> {
        let json =
            serde_json::to_string(self).map_err(|e| AgentError::Serialization(e.to_string()))?;
        let serializer = ToonSerializer::new(Self::schema());
        Ok(serializer
            .encode(&json)
            .map_err(|e| AgentError::Serialization(e.to_string()))?)
    }
}

/// Patch output format accepted by the runtime.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatchFormat {
    /// `git diff --unified=0`
    UnifiedDiffZero,
    /// SEARCH/REPLACE block.
    AstSearchReplace,
}

/// AST-aware search/replace block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchReplaceBlock {
    /// Target file path.
    pub file_path: String,
    /// Target symbol path.
    pub symbol_path: Option<String>,
    /// Start line hint.
    pub start_line: Option<usize>,
    /// End line hint.
    pub end_line: Option<usize>,
    /// Search content.
    pub search: String,
    /// Replacement content.
    pub replace: String,
}

/// Patch envelope sent over the transport layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchEnvelope {
    /// Owning task.
    pub task_id: String,
    /// Files expected to change.
    pub target_files: Vec<String>,
    /// Patch format.
    pub format: PatchFormat,
    /// Unified diff text when applicable.
    pub patch_text: Option<String>,
    /// Search/replace blocks when applicable.
    pub search_replace_blocks: Vec<SearchReplaceBlock>,
    /// Base revision used to build the patch.
    pub base_revision: String,
    /// Validation facts.
    pub validation: Vec<ValidationFact>,
}

/// Deterministic patch application status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatchApplyStatus {
    /// Patch applied successfully.
    Applied,
    /// Patch conflicted with current content.
    Conflict,
    /// Patch is invalid.
    Invalid,
    /// Patch was built from a stale base.
    StaleBase,
}

/// Receipt returned after deterministic application.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PatchReceipt {
    /// Owning task.
    pub task_id: String,
    /// Final status.
    pub status: PatchApplyStatus,
    /// Revision after the attempted application.
    pub applied_revision: String,
    /// Human-readable message.
    pub message: String,
    /// Files touched by the patch.
    pub affected_files: Vec<String>,
}

/// Result of validating a patch-producing model output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    /// Whether the output is accepted.
    pub accepted: bool,
    /// Validation message.
    pub message: String,
    /// Supplemental facts.
    pub facts: Vec<ValidationFact>,
}

/// Validated agent output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidatedAgentOutput {
    /// Accepted format.
    pub format: PatchFormat,
    /// Original output.
    pub raw_output: String,
    /// Parsed search/replace blocks when applicable.
    pub search_replace_blocks: Vec<SearchReplaceBlock>,
}

/// Validator that enforces diff-only outputs.
#[derive(Debug, Clone)]
pub struct DiffOutputValidator {
    max_plaintext_bytes: usize,
}

impl DiffOutputValidator {
    /// Create a validator with the default plaintext limit.
    pub fn new(max_plaintext_bytes: usize) -> Self {
        Self {
            max_plaintext_bytes,
        }
    }

    /// Validate a model output and classify it as an accepted patch format.
    pub fn validate(&self, output: &str) -> Result<ValidatedAgentOutput> {
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return Err(AgentError::DiffRequired("empty output".to_string()).into());
        }

        if Self::looks_like_unified_diff(trimmed) {
            let parsed = parse_unified_diff(trimmed)
                .map_err(|e| AgentError::DiffRequired(format!("invalid unified diff: {e}")))?;

            if parsed
                .hunks
                .iter()
                .flat_map(|h| &h.lines)
                .any(|line| matches!(line, axora_cache::DiffLine::Context(_)))
            {
                return Err(AgentError::DiffRequired(
                    "context lines are forbidden; require git diff --unified=0".to_string(),
                )
                .into());
            }

            return Ok(ValidatedAgentOutput {
                format: PatchFormat::UnifiedDiffZero,
                raw_output: trimmed.to_string(),
                search_replace_blocks: Vec::new(),
            });
        }

        let blocks = Self::parse_search_replace_blocks(trimmed)?;
        if !blocks.is_empty() {
            return Ok(ValidatedAgentOutput {
                format: PatchFormat::AstSearchReplace,
                raw_output: trimmed.to_string(),
                search_replace_blocks: blocks,
            });
        }

        if trimmed.len() > self.max_plaintext_bytes {
            return Err(AgentError::DiffRequired(
                "full-file or free-form output detected; only unified diff or SEARCH/REPLACE blocks are accepted"
                    .to_string(),
            )
            .into());
        }

        Err(AgentError::DiffRequired(
            "output does not match an accepted patch format".to_string(),
        )
        .into())
    }

    fn looks_like_unified_diff(output: &str) -> bool {
        output.starts_with("--- ") && output.contains("\n+++ ") && output.contains("\n@@ ")
    }

    fn parse_search_replace_blocks(output: &str) -> Result<Vec<SearchReplaceBlock>> {
        let mut blocks = Vec::new();
        let lines: Vec<&str> = output.lines().collect();
        let mut cursor = 0usize;

        while cursor < lines.len() {
            let line = lines[cursor];
            if !line.starts_with("<<<<<<< SEARCH") {
                cursor += 1;
                continue;
            }

            let header = line.trim_start_matches("<<<<<<< SEARCH").trim();
            let file_path = if header.is_empty() {
                return Err(AgentError::DiffRequired(
                    "SEARCH block must declare a target file path".to_string(),
                )
                .into());
            } else {
                header.to_string()
            };

            cursor += 1;
            let mut search_lines = Vec::new();
            while cursor < lines.len() && lines[cursor] != "=======" {
                search_lines.push(lines[cursor]);
                cursor += 1;
            }

            if cursor == lines.len() || lines[cursor] != "=======" {
                return Err(AgentError::DiffRequired(
                    "SEARCH/REPLACE block missing ======= separator".to_string(),
                )
                .into());
            }

            cursor += 1;
            let mut replace_lines = Vec::new();
            while cursor < lines.len() && !lines[cursor].starts_with(">>>>>>> REPLACE") {
                replace_lines.push(lines[cursor]);
                cursor += 1;
            }

            if cursor == lines.len() {
                return Err(AgentError::DiffRequired(
                    "SEARCH/REPLACE block missing >>>>>>> REPLACE terminator".to_string(),
                )
                .into());
            }

            blocks.push(SearchReplaceBlock {
                file_path,
                symbol_path: None,
                start_line: None,
                end_line: None,
                search: search_lines.join("\n"),
                replace: replace_lines.join("\n"),
            });
            cursor += 1;
        }

        Ok(blocks)
    }
}

impl Default for DiffOutputValidator {
    fn default() -> Self {
        Self::new(100)
    }
}

/// Deterministic patch applier running outside the model.
#[derive(Debug, Clone, Default)]
pub struct DeterministicPatchApplier;

impl DeterministicPatchApplier {
    /// Apply a patch envelope to the local workspace and return a receipt.
    pub fn apply_to_workspace(&self, workspace_root: &Path, envelope: &PatchEnvelope) -> Result<PatchReceipt> {
        match envelope.format {
            PatchFormat::UnifiedDiffZero => self.apply_unified_diff(workspace_root, envelope),
            PatchFormat::AstSearchReplace => self.apply_search_replace(workspace_root, envelope),
        }
    }

    fn apply_unified_diff(&self, workspace_root: &Path, envelope: &PatchEnvelope) -> Result<PatchReceipt> {
        let patch_text = envelope.patch_text.as_deref().ok_or_else(|| {
            AgentError::PatchApplication("missing patch_text for unified diff envelope".to_string())
        })?;
        let diff = parse_unified_diff(patch_text)
            .map_err(|e| AgentError::PatchApplication(format!("invalid diff: {e}")))?;

        let mut affected_files = Vec::new();
        let mut new_revision = envelope.base_revision.clone();

        for target in diff.target_files() {
            let rel_path = normalize_diff_path(&target);
            let path = workspace_root.join(&rel_path);
            let original = fs::read_to_string(&path).map_err(|e| {
                AgentError::PatchApplication(format!("failed reading {}: {}", path.display(), e))
            })?;
            let current_revision = revision_for_content(&original);
            if current_revision != envelope.base_revision {
                return Ok(PatchReceipt {
                    task_id: envelope.task_id.clone(),
                    status: PatchApplyStatus::StaleBase,
                    applied_revision: current_revision,
                    message: format!(
                        "stale base revision for {}",
                        path.strip_prefix(workspace_root).unwrap_or(&path).display()
                    ),
                    affected_files,
                });
            }

            let patched = apply_patch(&original, patch_text);
            if !patched.success {
                return Ok(PatchReceipt {
                    task_id: envelope.task_id.clone(),
                    status: PatchApplyStatus::Conflict,
                    applied_revision: current_revision,
                    message: patched.error.unwrap_or_else(|| "patch conflict".to_string()),
                    affected_files,
                });
            }

            fs::write(&path, &patched.content).map_err(|e| {
                AgentError::PatchApplication(format!("failed writing {}: {}", path.display(), e))
            })?;

            new_revision = revision_for_content(&patched.content);
            affected_files.push(rel_path);
        }

        Ok(PatchReceipt {
            task_id: envelope.task_id.clone(),
            status: PatchApplyStatus::Applied,
            applied_revision: new_revision,
            message: "patch applied".to_string(),
            affected_files,
        })
    }

    fn apply_search_replace(&self, workspace_root: &Path, envelope: &PatchEnvelope) -> Result<PatchReceipt> {
        let mut affected_files = Vec::new();
        let mut final_revision = envelope.base_revision.clone();

        for block in &envelope.search_replace_blocks {
            let path = workspace_root.join(&block.file_path);
            let original = fs::read_to_string(&path).map_err(|e| {
                AgentError::PatchApplication(format!("failed reading {}: {}", path.display(), e))
            })?;
            let current_revision = revision_for_content(&original);
            if current_revision != envelope.base_revision {
                return Ok(PatchReceipt {
                    task_id: envelope.task_id.clone(),
                    status: PatchApplyStatus::StaleBase,
                    applied_revision: current_revision,
                    message: format!("stale base revision for {}", block.file_path),
                    affected_files,
                });
            }

            let occurrences = original.matches(&block.search).count();
            if occurrences != 1 {
                let status = if occurrences == 0 {
                    PatchApplyStatus::Conflict
                } else {
                    PatchApplyStatus::Invalid
                };
                return Ok(PatchReceipt {
                    task_id: envelope.task_id.clone(),
                    status,
                    applied_revision: current_revision,
                    message: format!(
                        "expected exactly one match for SEARCH block in {}, found {}",
                        block.file_path, occurrences
                    ),
                    affected_files,
                });
            }

            let updated = original.replacen(&block.search, &block.replace, 1);
            fs::write(&path, &updated).map_err(|e| {
                AgentError::PatchApplication(format!("failed writing {}: {}", path.display(), e))
            })?;
            final_revision = revision_for_content(&updated);
            affected_files.push(block.file_path.clone());
        }

        Ok(PatchReceipt {
            task_id: envelope.task_id.clone(),
            status: PatchApplyStatus::Applied,
            applied_revision: final_revision,
            message: "search/replace patch applied".to_string(),
            affected_files,
        })
    }
}

fn normalize_diff_path(path: &str) -> String {
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

fn revision_for_content(content: &str) -> String {
    hash(content.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_context_pack_serializes_to_toon() {
        let pack = ContextPack {
            id: "pack-1".to_string(),
            task_id: "task-1".to_string(),
            target_files: vec!["src/lib.rs".to_string()],
            symbols: vec!["crate::do_work".to_string()],
            spans: vec![ContextSpan {
                file_path: "src/lib.rs".to_string(),
                start_line: 10,
                end_line: 20,
                symbol_path: "crate::do_work".to_string(),
            }],
            retrieval_hits: vec![RetrievalHit {
                file_path: "src/lib.rs".to_string(),
                symbol_path: "crate::do_work".to_string(),
                start_line: 10,
                end_line: 20,
                snippet: "fn do_work() {}".to_string(),
                base_revision: "rev-1".to_string(),
            }],
            ast_summaries: vec![AstSummary {
                file_path: "src/lib.rs".to_string(),
                symbol_path: "crate::do_work".to_string(),
                kind: "function".to_string(),
                start_line: 10,
                end_line: 20,
            }],
            symbol_maps: vec![SymbolMap {
                file_path: "src/lib.rs".to_string(),
                symbol_path: "crate::do_work".to_string(),
                references: vec!["crate::helper".to_string()],
            }],
            validation_facts: vec![ValidationFact {
                key: "base_revision".to_string(),
                value: "rev-1".to_string(),
            }],
            base_revision: "rev-1".to_string(),
        };

        let toon = pack.to_toon().unwrap();
        assert!(!toon.is_empty());
        assert!(toon.contains("{"));
    }

    #[test]
    fn test_diff_validator_rejects_plaintext_file_dump() {
        let validator = DiffOutputValidator::default();
        let full_file = "fn main() {}\n".repeat(32);

        let err = validator.validate(&full_file).unwrap_err().to_string();
        assert!(err.contains("diff-only"));
    }

    #[test]
    fn test_diff_validator_accepts_zero_context_diff() {
        let validator = DiffOutputValidator::default();
        let diff = "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
";

        let validated = validator.validate(diff).unwrap();
        assert_eq!(validated.format, PatchFormat::UnifiedDiffZero);
    }

    #[test]
    fn test_diff_validator_accepts_search_replace_blocks() {
        let validator = DiffOutputValidator::default();
        let block = "\
<<<<<<< SEARCH src/lib.rs
fn old() {}
=======
fn new() {}
>>>>>>> REPLACE";

        let validated = validator.validate(block).unwrap();
        assert_eq!(validated.format, PatchFormat::AstSearchReplace);
        assert_eq!(validated.search_replace_blocks.len(), 1);
    }

    #[test]
    fn test_deterministic_patch_applier_detects_stale_base() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("src/lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn old() {}\n").unwrap();

        let envelope = PatchEnvelope {
            task_id: "task-1".to_string(),
            target_files: vec!["src/lib.rs".to_string()],
            format: PatchFormat::UnifiedDiffZero,
            patch_text: Some(
                "\
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1 +1 @@
-fn old() {}
+fn new() {}
"
                .to_string(),
            ),
            search_replace_blocks: Vec::new(),
            base_revision: "stale".to_string(),
            validation: Vec::new(),
        };

        let receipt = DeterministicPatchApplier
            .apply_to_workspace(temp_dir.path(), &envelope)
            .unwrap();
        assert_eq!(receipt.status, PatchApplyStatus::StaleBase);
    }

    #[test]
    fn test_deterministic_patch_applier_applies_search_replace() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("src/lib.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        let original = "fn old() {}\n";
        fs::write(&file_path, original).unwrap();

        let envelope = PatchEnvelope {
            task_id: "task-1".to_string(),
            target_files: vec!["src/lib.rs".to_string()],
            format: PatchFormat::AstSearchReplace,
            patch_text: None,
            search_replace_blocks: vec![SearchReplaceBlock {
                file_path: "src/lib.rs".to_string(),
                symbol_path: Some("crate::old".to_string()),
                start_line: Some(1),
                end_line: Some(1),
                search: "fn old() {}".to_string(),
                replace: "fn new() {}".to_string(),
            }],
            base_revision: revision_for_content(original),
            validation: Vec::new(),
        };

        let receipt = DeterministicPatchApplier
            .apply_to_workspace(temp_dir.path(), &envelope)
            .unwrap();

        assert_eq!(receipt.status, PatchApplyStatus::Applied);
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "fn new() {}\n");
    }
}
