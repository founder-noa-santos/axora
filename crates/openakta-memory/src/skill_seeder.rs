//! Built-in skill corpus synchronization.

use crate::procedural_store::{Result, SkillCatalog, SkillCorpusIngestor};
use std::path::{Path, PathBuf};

/// Outcome summary for a built-in skill sync.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSeedReport {
    /// Number of built-in skills discovered.
    pub discovered_skills: usize,
    /// Number of documents present in the catalog after sync.
    pub catalog_documents: usize,
}

impl SkillCorpusIngestor {
    /// Sync the built-in bundled skill corpus into the catalog.
    pub async fn sync_builtin_skills(
        catalog: &SkillCatalog,
        skills_root: impl AsRef<Path>,
    ) -> Result<SkillSeedReport> {
        let ingestor = SkillCorpusIngestor::new(skills_root);
        let discovered = ingestor.sync(catalog).await?;
        let catalog_documents = catalog.list_documents()?.len();
        Ok(SkillSeedReport {
            discovered_skills: discovered.len(),
            catalog_documents,
        })
    }
}

/// Return the default bundled skill corpus root inside the crate.
pub fn builtin_skill_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::procedural_store::SkillCatalog;
    use tempfile::TempDir;

    #[tokio::test]
    async fn syncs_builtin_skill_corpus() {
        let temp_dir = TempDir::new().unwrap();
        let catalog = SkillCatalog::new(temp_dir.path().join("skills.db")).unwrap();

        let report = SkillCorpusIngestor::sync_builtin_skills(&catalog, builtin_skill_root())
            .await
            .unwrap();

        assert!(report.discovered_skills > 0);
        assert_eq!(report.discovered_skills, report.catalog_documents);
    }
}
