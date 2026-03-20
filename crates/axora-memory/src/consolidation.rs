//! Consolidation of episodic trajectories into authored skills.

use crate::episodic_store::{EpisodicMemory, EpisodicStore};
use crate::procedural_store::{Skill, SkillCatalog, SkillStep};
use chrono::Utc;
use thiserror::Error;

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
    /// Auto-accept into the catalog.
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
}

impl ConsolidationPipeline {
    /// Create a new consolidation pipeline.
    pub fn new(
        episodic_store: EpisodicStore,
        skill_catalog: SkillCatalog,
        distillation_model: LightweightLLM,
    ) -> Self {
        Self {
            episodic_store,
            skill_catalog,
            distillation_model,
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
                ObservationActionPair::new(
                    observation,
                    action,
                    memory.success,
                )
            })
            .collect()
    }

    /// Distill an authored skill from a trajectory.
    pub async fn distill(&self, trajectory: &[ObservationActionPair]) -> Result<Skill> {
        let skill_id = format!("SKILL_{}", Utc::now().timestamp());
        let name = format!("Consolidated {}", self.distillation_model.model);
        let domain = "general";
        let tags = trajectory
            .iter()
            .flat_map(|pair| pair.observation.split_whitespace().take(3))
            .map(|part| part.to_lowercase())
            .collect::<Vec<_>>();
        let steps = trajectory
            .iter()
            .enumerate()
            .map(|(index, pair)| SkillStep {
                order: (index + 1) as u32,
                description: pair.observation.chars().take(100).collect(),
                command: if pair.action.trim().is_empty() {
                    None
                } else {
                    Some(pair.action.clone())
                },
                validation: None,
            })
            .collect::<Vec<_>>();
        let mut skill = Skill::new(&skill_id, &name, domain, tags, steps);
        skill.metadata.summary = trajectory
            .first()
            .map(|pair| pair.observation.clone())
            .unwrap_or_else(|| "Consolidated skill".to_string());
        Ok(skill)
    }

    /// Validate a skill and optionally write it into the catalog.
    pub async fn validate(&self, skill: &Skill, mode: ValidationMode) -> Result<bool> {
        match mode {
            ValidationMode::Review => Ok(self.generate_validation_report(skill).passed),
            ValidationMode::TeacherStudent => {
                let verifier = TeacherVerifier::new();
                if verifier.verify(skill).await? {
                    let document = skill.to_document(format!("generated://{}", skill.metadata.skill_id));
                    self.skill_catalog
                        .upsert_document(&document)
                        .map_err(|err| ConsolidationError::Procedural(err.to_string()))?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
        }
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
            tokio::time::sleep(self.check_interval).await;
        }
    }

    /// Expose the pipeline for tests and callers.
    pub fn pipeline(&self) -> &ConsolidationPipeline {
        &self.pipeline
    }
}
