//! Default procedural skill seeding for batteries-included AXORA runtimes.

use crate::procedural_store::{ProceduralStore, Skill};
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;

const DEFAULT_SKILL_SEED_NAME: &str = "builtin_skill_library";
const DEFAULT_SKILL_SEED_VERSION: &str = "v1";

const RUST_TEST_WRITING: &str = include_str!("../assets/skills/RUST_TEST_WRITING.md");
const CARGO_REPAIR: &str = include_str!("../assets/skills/CARGO_REPAIR.md");
const DIFF_REVIEW: &str = include_str!("../assets/skills/DIFF_REVIEW.md");
const MERGE_CONFLICT: &str = include_str!("../assets/skills/MERGE_CONFLICT_RESOLUTION.md");
const SAFE_PATCH: &str = include_str!("../assets/skills/SAFE_PATCH_APPLICATION.md");
const REPO_EXPLORATION: &str = include_str!("../assets/skills/REPO_EXPLORATION.md");
const JWT_DEBUGGING: &str = include_str!("../assets/skills/JWT_DEBUGGING.md");

/// Errors raised while seeding built-in procedural skills.
#[derive(Debug, Error)]
pub enum SkillSeederError {
    /// SQLite checkpoint error.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    /// Procedural store write error.
    #[error("procedural store error: {0}")]
    Procedural(#[from] crate::procedural_store::ProceduralError),
}

/// Outcome summary for a skill seed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillSeedReport {
    /// Seed name used for checkpointing.
    pub seed_name: String,
    /// Seed version applied to the database.
    pub seed_version: String,
    /// Number of skills written into the procedural store.
    pub seeded_skills: usize,
    /// Whether the seed was skipped because the version already existed.
    pub skipped: bool,
}

/// Seeder for AXORA's built-in procedural skill library.
pub struct SkillSeeder;

impl SkillSeeder {
    /// Seed the default skill library into the procedural store and storage registry.
    pub async fn seed_defaults(
        store: &ProceduralStore,
        checkpoint_db_path: &Path,
    ) -> Result<SkillSeedReport, SkillSeederError> {
        let connection = Connection::open(checkpoint_db_path)?;
        connection.execute_batch(include_str!("../../axora-storage/migrations/0001_init.sql"))?;
        connection.execute_batch(include_str!("../../axora-storage/migrations/0002_memory_domains.sql"))?;
        connection.execute_batch(include_str!("../../axora-storage/migrations/0003_runtime_seeds.sql"))?;

        let current_version: Option<String> = connection
            .query_row(
                "SELECT version FROM runtime_seed_versions WHERE seed_name = ?1",
                [DEFAULT_SKILL_SEED_NAME],
                |row| row.get(0),
            )
            .ok();

        if current_version.as_deref() == Some(DEFAULT_SKILL_SEED_VERSION) {
            return Ok(SkillSeedReport {
                seed_name: DEFAULT_SKILL_SEED_NAME.to_string(),
                seed_version: DEFAULT_SKILL_SEED_VERSION.to_string(),
                seeded_skills: 0,
                skipped: true,
            });
        }

        let mut seeded = 0usize;
        for skill in builtin_skills() {
            store.store(skill.clone()).await?;
            connection.execute(
                "INSERT OR REPLACE INTO procedural_skills (id, path, status, trigger_summary, metadata, created_at, updated_at)
                 VALUES (?1, ?2, 'active', ?3, ?4, datetime('now'), datetime('now'))",
                params![
                    skill.metadata.skill_id,
                    store
                        .skills_dir()
                        .join(format!("{}.md", skill.metadata.skill_id))
                        .to_string_lossy()
                        .to_string(),
                    skill.metadata.triggers.join(", "),
                    serde_json::to_string(&skill.metadata).unwrap_or_else(|_| "{}".to_string())
                ],
            )?;
            connection.execute(
                "INSERT OR REPLACE INTO memory_utility_scores (skill_id, utility_score, retrieval_count, success_count, failure_count, updated_at)
                 VALUES (?1, ?2, 0, 0, 0, datetime('now'))",
                params![skill.metadata.skill_id, skill.metadata.utility_score],
            )?;
            seeded += 1;
        }

        connection.execute(
            "INSERT OR REPLACE INTO runtime_seed_versions (seed_name, version, applied_at)
             VALUES (?1, ?2, datetime('now'))",
            params![DEFAULT_SKILL_SEED_NAME, DEFAULT_SKILL_SEED_VERSION],
        )?;

        Ok(SkillSeedReport {
            seed_name: DEFAULT_SKILL_SEED_NAME.to_string(),
            seed_version: DEFAULT_SKILL_SEED_VERSION.to_string(),
            seeded_skills: seeded,
            skipped: false,
        })
    }
}

fn builtin_skills() -> Vec<Skill> {
    [
        RUST_TEST_WRITING,
        CARGO_REPAIR,
        DIFF_REVIEW,
        MERGE_CONFLICT,
        SAFE_PATCH,
        REPO_EXPLORATION,
        JWT_DEBUGGING,
    ]
    .into_iter()
    .map(|content| Skill::from_skill_md(content).expect("embedded builtin skill must be valid"))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::SkillSeeder;
    use crate::ProceduralStore;
    use tempfile::TempDir;

    #[tokio::test]
    async fn seeds_builtins_only_once() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("axora.db");
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();

        let first = SkillSeeder::seed_defaults(&store, &db_path).await.unwrap();
        let second = SkillSeeder::seed_defaults(&store, &db_path).await.unwrap();

        assert!(!first.skipped);
        assert_eq!(first.seeded_skills, 7);
        assert!(second.skipped);
    }
}
