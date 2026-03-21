//! Consolidation of episodic trajectories into authored skills.

use crate::episodic_store::{EpisodicMemory, EpisodicStore};
use crate::procedural_store::{Skill, SkillCatalog, SkillCorpusIngestor, SkillStep};
use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;
use tokio::fs;

/// Consolidation errors.
#[derive(Debug, Error)]
pub enum ConsolidationError {
    /// Episodic store error.
    #[error("episodic error: {0}")]
    Episodic(String),

    /// Procedural-memory error.
    #[error("procedural error: {0}")]
    Procedural(String),
}

type Result<T> = std::result::Result<T, ConsolidationError>;

/// Signals that a session may be worth consolidating.
#[derive(Debug, Clone, PartialEq)]
pub enum ConsolidationTrigger {
    /// High-success trajectory.
    Success {
        /// Session identifier.
        session_id: String,
        /// Fraction of successful actions in the session.
        success_rate: f32,
    },
    /// Repeated pattern seen across enough steps.
    Frequency {
        /// Session identifier.
        session_id: String,
        /// Number of memories observed.
        occurrence_count: u32,
    },
}

/// Observation/action pair extracted from episodic memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationActionPair {
    /// Observed state.
    pub observation: String,
    /// Action executed.
    pub action: String,
    /// Whether the action succeeded.
    pub success: Option<bool>,
}

impl ObservationActionPair {
    /// Create a new pair.
    pub fn new(observation: &str, action: &str, success: Option<bool>) -> Self {
        Self {
            observation: observation.to_string(),
            action: action.to_string(),
            success,
        }
    }
}

/// Validation mode for synthesized skills.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Create a report only.
    Review,
    /// Run the local teacher-student verifier before acceptance.
    TeacherStudent,
}

/// Validation outcome for a synthesized skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationReport {
    /// Skill identifier.
    pub skill_id: String,
    /// Whether the skill passed validation.
    pub passed: bool,
    /// Human-readable notes.
    pub notes: Vec<String>,
    /// Follow-up actions.
    pub recommendations: Vec<String>,
}

/// Lightweight local model handle used during distillation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LightweightLLM {
    model: String,
}

impl LightweightLLM {
    /// Create a new local distillation model handle.
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
        }
    }
}

/// Local verifier used for teacher-student validation mode.
#[derive(Debug, Clone, Default)]
pub struct TeacherVerifier;

impl TeacherVerifier {
    /// Create a new verifier.
    pub fn new() -> Self {
        Self
    }

    /// Verify a synthesized skill.
    pub async fn verify(&self, skill: &Skill) -> Result<bool> {
        Ok(!skill.steps.is_empty() && !skill.metadata.tags.is_empty())
    }
}

/// Consolidates episodic trajectories into skill documents.
pub struct ConsolidationPipeline {
    episodic_store: EpisodicStore,
    skill_catalog: SkillCatalog,
    distillation_model: LightweightLLM,
    skill_corpus_root: PathBuf,
}

impl ConsolidationPipeline {
    /// Create a new consolidation pipeline.
    pub fn new(
        episodic_store: EpisodicStore,
        skill_catalog: SkillCatalog,
        distillation_model: LightweightLLM,
        skill_corpus_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            episodic_store,
            skill_catalog,
            distillation_model,
            skill_corpus_root: skill_corpus_root.into(),
        }
    }

    /// Determine whether a session warrants consolidation.
    pub async fn check_triggers(&self, session_id: &str) -> Result<Option<ConsolidationTrigger>> {
        let memories = self
            .episodic_store
            .retrieve_trajectory(session_id)
            .await
            .map_err(|err| ConsolidationError::Episodic(err.to_string()))?;
        if memories.len() < 3 {
            return Ok(None);
        }

        let success_rate = memories
            .iter()
            .filter(|memory| memory.success == Some(true))
            .count() as f32
            / memories.len() as f32;
        if success_rate >= 0.7 {
            return Ok(Some(ConsolidationTrigger::Success {
                session_id: session_id.to_string(),
                success_rate,
            }));
        }

        Ok(Some(ConsolidationTrigger::Frequency {
            session_id: session_id.to_string(),
            occurrence_count: memories.len() as u32,
        }))
    }

    /// Extract an ordered trajectory from episodic memories.
    pub fn extract_trajectory(&self, memories: &[EpisodicMemory]) -> Vec<ObservationActionPair> {
        memories
            .iter()
            .filter(|memory| !memory.content.trim().is_empty())
            .map(|memory| {
                let (observation, action) = extract_observation_action(&memory.content);
                ObservationActionPair::new(observation, action, memory.success)
            })
            .collect()
    }

    /// Distill an authored skill from a trajectory.
    pub async fn distill(
        &self,
        session_id: &str,
        trajectory: &[ObservationActionPair],
    ) -> Result<Skill> {
        let normalized = trajectory
            .iter()
            .map(|pair| {
                format!(
                    "{} => {} => {}",
                    pair.observation.trim().to_lowercase(),
                    pair.action.trim().to_lowercase(),
                    pair.success.unwrap_or(false)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let digest = blake3::hash(normalized.as_bytes()).to_hex().to_string();
        let skill_id = format!(
            "distilled_{}_{}",
            sanitize_identifier(session_id),
            &digest[..12]
        );
        let name = format!("Distilled {}", self.distillation_model.model);
        let domain = "general";

        let mut seen_tags = HashSet::new();
        let mut tags = trajectory
            .iter()
            .flat_map(|pair| pair.observation.split_whitespace().take(4))
            .map(|token| token.to_lowercase())
            .filter(|token| !token.is_empty() && seen_tags.insert(token.clone()))
            .collect::<Vec<_>>();
        if tags.is_empty() {
            tags.push("distilled".to_string());
        }

        let steps = trajectory
            .iter()
            .enumerate()
            .map(|(index, pair)| SkillStep {
                order: (index + 1) as u32,
                description: pair.observation.chars().take(160).collect(),
                command: if pair.action.trim().is_empty() {
                    None
                } else {
                    Some(pair.action.clone())
                },
                validation: pair.success.filter(|success| !success).map(|_| {
                    "Re-run and verify the observation matches the expected output".to_string()
                }),
            })
            .collect::<Vec<_>>();

        let mut skill = Skill::new(&skill_id, &name, domain, tags, steps);
        skill.metadata.summary = trajectory
            .first()
            .map(|pair| pair.observation.clone())
            .unwrap_or_else(|| "Consolidated skill".to_string());
        Ok(skill)
    }

    /// Validate a skill prior to persistence.
    pub async fn validate(&self, skill: &Skill, mode: ValidationMode) -> Result<bool> {
        match mode {
            ValidationMode::Review => Ok(self.generate_validation_report(skill).passed),
            ValidationMode::TeacherStudent => TeacherVerifier::new().verify(skill).await,
        }
    }

    /// Persist a distilled skill to disk and resync the catalog.
    pub async fn persist_skill(&self, skill: &Skill) -> Result<PathBuf> {
        let skill_dir = self
            .skill_corpus_root
            .join("distilled")
            .join(&skill.metadata.skill_id);
        fs::create_dir_all(&skill_dir)
            .await
            .map_err(|err| ConsolidationError::Procedural(err.to_string()))?;

        let skill_path = skill_dir.join("SKILL.md");
        fs::write(&skill_path, skill.to_skill_markdown())
            .await
            .map_err(|err| ConsolidationError::Procedural(err.to_string()))?;

        let document = skill.to_document(skill_path.to_string_lossy().to_string());
        self.skill_catalog
            .upsert_document(&document)
            .map_err(|err| ConsolidationError::Procedural(err.to_string()))?;

        SkillCorpusIngestor::new(&self.skill_corpus_root)
            .sync(&self.skill_catalog)
            .await
            .map_err(|err| ConsolidationError::Procedural(err.to_string()))?;

        Ok(skill_path)
    }

    /// Generate a validation report.
    pub fn generate_validation_report(&self, skill: &Skill) -> ValidationReport {
        let mut notes = Vec::new();
        let mut recommendations = Vec::new();

        if skill.steps.is_empty() {
            notes.push("No procedural steps".to_string());
            recommendations.push("Add at least one concrete step".to_string());
        }
        if skill.metadata.tags.is_empty() {
            notes.push("No retrieval tags".to_string());
            recommendations.push("Extract or author retrieval tags".to_string());
        }
        if skill.metadata.summary.trim().is_empty() {
            notes.push("Missing summary".to_string());
            recommendations.push("Add a compact retrieval summary".to_string());
        }

        ValidationReport {
            skill_id: skill.metadata.skill_id.clone(),
            passed: notes.is_empty(),
            notes,
            recommendations,
        }
    }
}

fn extract_observation_action(content: &str) -> (&str, &str) {
    let mut observation = content.trim();
    let mut action = "inspect";

    for line in content.lines() {
        if let Some(value) = line.strip_prefix("Action: ") {
            action = value.trim();
        } else if let Some(value) = line.strip_prefix("Output: ") {
            observation = value.trim();
        }
    }

    (observation, action)
}

/// Background worker that periodically checks sessions for consolidation.
pub struct ConsolidationWorker {
    pipeline: ConsolidationPipeline,
    check_interval: std::time::Duration,
}

impl ConsolidationWorker {
    /// Create a new worker.
    pub fn new(pipeline: ConsolidationPipeline, check_interval: std::time::Duration) -> Self {
        Self {
            pipeline,
            check_interval,
        }
    }

    /// Start the worker loop.
    pub async fn start(&self) {
        loop {
            let session_ids = match self.pipeline.episodic_store.list_session_ids().await {
                Ok(session_ids) => session_ids,
                Err(_) => {
                    tokio::time::sleep(self.check_interval).await;
                    continue;
                }
            };

            for session_id in session_ids {
                let Ok(Some(_trigger)) = self.pipeline.check_triggers(&session_id).await else {
                    continue;
                };
                let Ok(memories) = self
                    .pipeline
                    .episodic_store
                    .retrieve_trajectory(&session_id)
                    .await
                else {
                    continue;
                };
                let trajectory = self.pipeline.extract_trajectory(&memories);
                if trajectory.is_empty() {
                    continue;
                }
                let Ok(skill) = self.pipeline.distill(&session_id, &trajectory).await else {
                    continue;
                };
                let Ok(true) = self
                    .pipeline
                    .validate(&skill, ValidationMode::TeacherStudent)
                    .await
                else {
                    continue;
                };
                let _ = self.pipeline.persist_skill(&skill).await;
            }

            tokio::time::sleep(self.check_interval).await;
        }
    }

    /// Expose the pipeline for tests and callers.
    pub fn pipeline(&self) -> &ConsolidationPipeline {
        &self.pipeline
    }
}

fn sanitize_identifier(input: &str) -> String {
    let sanitized = input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_lowercase();
    if sanitized.is_empty() {
        "session".to_string()
    } else {
        sanitized
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic_store::EpisodicStoreConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn distill_uses_stable_skill_ids() {
        let temp_dir = TempDir::new().unwrap();
        let store = EpisodicStore::new(EpisodicStoreConfig::in_memory())
            .await
            .unwrap();
        let catalog = SkillCatalog::new(temp_dir.path().join("skills.db")).unwrap();
        let pipeline = ConsolidationPipeline::new(
            store,
            catalog,
            LightweightLLM::new("haiku"),
            temp_dir.path().join("skills"),
        );
        let trajectory = vec![
            ObservationActionPair::new("Inspect auth flow", "read_file src/auth.rs", Some(true)),
            ObservationActionPair::new("Patch login bug", "apply_patch", Some(true)),
        ];

        let first = pipeline.distill("session-1", &trajectory).await.unwrap();
        let second = pipeline.distill("session-1", &trajectory).await.unwrap();

        assert_eq!(first.metadata.skill_id, second.metadata.skill_id);
    }

    #[tokio::test]
    async fn persist_skill_writes_skill_markdown() {
        let temp_dir = TempDir::new().unwrap();
        let store = EpisodicStore::new(EpisodicStoreConfig::in_memory())
            .await
            .unwrap();
        let catalog = SkillCatalog::new(temp_dir.path().join("skills.db")).unwrap();
        let pipeline = ConsolidationPipeline::new(
            store,
            catalog,
            LightweightLLM::new("haiku"),
            temp_dir.path().join("skills"),
        );
        let trajectory = vec![
            ObservationActionPair::new("Inspect auth flow", "read_file src/auth.rs", Some(true)),
            ObservationActionPair::new("Patch login bug", "apply_patch", Some(true)),
            ObservationActionPair::new("Verify cargo test", "cargo test -p auth", Some(true)),
        ];
        let skill = pipeline.distill("session-2", &trajectory).await.unwrap();

        let path = pipeline.persist_skill(&skill).await.unwrap();
        let content = std::fs::read_to_string(&path).unwrap();

        assert!(path.ends_with("SKILL.md"));
        assert!(content.contains("skill_id"));
        assert!(content.contains("# Distilled haiku"));
    }
}
