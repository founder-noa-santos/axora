//! Heuristic doc reconciliation for LivingDocs-driven sync.

use crate::{DocUpdate, LivingDocs, UpdateType};
use std::path::{Path, PathBuf};

/// Reconciliation outcome for a changed file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconcileDecision {
    /// No documentation change is required.
    Noop,
    /// Documentation should be updated.
    UpdateRequired,
    /// Change is significant and should be reviewed.
    ReviewRequired,
}

/// Draft patch generated for a documentation target.
#[derive(Debug, Clone)]
pub struct DocPatch {
    /// Target file path.
    pub target: PathBuf,
    /// Patch body or suggested content.
    pub content: String,
}

/// Configures heuristic doc target routing.
#[derive(Debug, Clone)]
pub struct DocReconcilerConfig {
    /// Repository root used to locate common docs.
    pub repo_root: PathBuf,
}

impl DocReconcilerConfig {
    /// Create a new reconciler config.
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }
}

/// Reconciles code changes into document updates.
pub struct DocReconciler {
    living_docs: LivingDocs,
    config: DocReconcilerConfig,
}

impl DocReconciler {
    /// Create a new reconciler.
    pub fn new(config: DocReconcilerConfig) -> Self {
        Self {
            living_docs: LivingDocs::new(),
            config,
        }
    }

    /// Create a reconciler with an existing LivingDocs index.
    pub fn with_living_docs(config: DocReconcilerConfig, living_docs: LivingDocs) -> Self {
        Self {
            living_docs,
            config,
        }
    }

    /// Access the underlying LivingDocs registry.
    pub fn living_docs(&self) -> &LivingDocs {
        &self.living_docs
    }

    /// Access the underlying LivingDocs registry mutably.
    pub fn living_docs_mut(&mut self) -> &mut LivingDocs {
        &mut self.living_docs
    }

    /// Reconcile a change and return a decision plus draft patches.
    pub fn reconcile_change(
        &mut self,
        file: &Path,
        old_content: &str,
        new_content: &str,
    ) -> (ReconcileDecision, Vec<DocPatch>) {
        let updates = self
            .living_docs
            .on_code_change(file, old_content, new_content);

        if updates.is_empty() {
            return (ReconcileDecision::Noop, Vec::new());
        }

        let decision = if updates.iter().any(|update| {
            matches!(
                update.update_type,
                UpdateType::FlagForReview | UpdateType::Deprecate
            )
        }) {
            ReconcileDecision::ReviewRequired
        } else {
            ReconcileDecision::UpdateRequired
        };

        let patches = updates
            .iter()
            .map(|update| DocPatch {
                target: self.resolve_target(file, update),
                content: self.draft_patch(file, update),
            })
            .collect();

        (decision, patches)
    }

    fn resolve_target(&self, file: &Path, update: &DocUpdate) -> PathBuf {
        if update.doc_id.starts_with("pending:") {
            return self.config.repo_root.join("AGENTS.md");
        }

        let file_name = file.file_name().and_then(|name| name.to_str()).unwrap_or("");
        if file_name.eq_ignore_ascii_case("cargo.toml") || file_name.ends_with(".rs") {
            return self.config.repo_root.join("README.md");
        }

        self.config.repo_root.join("docs/ARCHITECTURE-LEDGER.md")
    }

    fn draft_patch(&self, file: &Path, update: &DocUpdate) -> String {
        format!(
            "## Auto-generated Documentation Update\n\n- Source: `{}`\n- Reason: {}\n- Suggested Change:\n\n{}\n",
            file.display(),
            update.reason,
            update.suggested_changes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{DocReconciler, DocReconcilerConfig, ReconcileDecision};
    use std::path::Path;

    #[test]
    fn flags_significant_undocumented_change_for_review() {
        let root = tempfile::tempdir().unwrap();
        let mut reconciler = DocReconciler::new(DocReconcilerConfig::new(root.path()));
        let new_content = "pub fn important_change() {}\n".repeat(30);

        let (decision, patches) = reconciler.reconcile_change(
            Path::new("src/new_module.rs"),
            "",
            &new_content,
        );

        assert_eq!(decision, ReconcileDecision::ReviewRequired);
        assert!(!patches.is_empty());
    }
}
