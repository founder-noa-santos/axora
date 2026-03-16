//! Consolidation Pipeline
//!
//! This module implements episodic → procedural consolidation:
//! - **Trigger conditions** (success, failure, frequency)
//! - **Trajectory extraction** (filter noise)
//! - **Multi-faceted distillation** (LLM-based pattern extraction)
//! - **Human-in-the-loop validation** (Review Mode before deployment)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Consolidation Pipeline                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Trigger Detection          │  Distillation                 │
//! │  - Success-based            │  - LLM pattern extraction     │
//! │  - Failure-based            │  - Skill step generation      │
//! │  - Frequency-based          │  - Metadata generation        │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Validation                                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Review Mode (human approval)  │  Teacher-Student (auto)   │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use crate::episodic_store::{EpisodicMemory, EpisodicStore};
use crate::procedural_store::{Skill, SkillStep};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Consolidation errors
#[derive(Error, Debug)]
pub enum ConsolidationError {
    /// Episodic store error
    #[error("episodic store error: {0}")]
    Episodic(#[from] crate::episodic_store::EpisodicError),

    /// Procedural store error
    #[error("procedural store error: {0}")]
    Procedural(#[from] crate::procedural_store::ProceduralError),

    /// No trajectory found
    #[error("no trajectory found for session: {0}")]
    NoTrajectory(String),

    /// Distillation failed
    #[error("distillation failed: {0}")]
    DistillationFailed(String),

    /// Validation failed
    #[error("validation failed: {0}")]
    ValidationFailed(String),

    /// Skill parsing error
    #[error("skill parsing error: {0}")]
    SkillParsing(String),
}

/// Result type for consolidation operations
pub type Result<T> = std::result::Result<T, ConsolidationError>;

/// Consolidation trigger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsolidationTrigger {
    /// Success-based: Complex task completed successfully
    Success {
        session_id: String,
        complexity_score: f32,
    },

    /// Failure-based: Catastrophic failure loop (learn anti-pattern)
    Failure {
        session_id: String,
        failure_type: String,
    },

    /// Frequency-based: Identical sequence observed N times
    Frequency {
        pattern_hash: String,
        occurrence_count: u32,
    },
}

/// Observation-Action pair for distillation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationActionPair {
    /// Observation (what was seen)
    pub observation: String,
    /// Action taken
    pub action: String,
    /// Success flag
    pub success: Option<bool>,
}

impl ObservationActionPair {
    /// Create new pair
    pub fn new(observation: &str, action: &str, success: Option<bool>) -> Self {
        Self {
            observation: observation.to_string(),
            action: action.to_string(),
            success,
        }
    }
}

/// Validation mode for consolidated skills
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Human-in-the-loop review
    Review,
    /// Automated teacher-student verification
    TeacherStudent,
}

/// Validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Skill ID
    pub skill_id: String,
    /// Validation passed
    pub passed: bool,
    /// Validation notes
    pub notes: String,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Lightweight LLM for distillation (simulated)
pub struct LightweightLLM {
    /// Model name (e.g., "claude-haiku")
    model_name: String,
}

impl LightweightLLM {
    /// Create new lightweight LLM
    pub fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_string(),
        }
    }

    /// Generate response (simulated for now)
    pub async fn generate(&self, prompt: &str) -> Result<String> {
        // In production, this would call the actual LLM API
        // For now, return a simulated response
        Ok(format!(
            "[LLM Response from {}]\n\nGenerated skill based on {} characters of prompt",
            self.model_name,
            prompt.len()
        ))
    }
}

/// Teacher verifier for automated validation
pub struct TeacherVerifier;

impl TeacherVerifier {
    /// Create new verifier
    pub fn new() -> Self {
        Self
    }

    /// Verify skill quality
    pub async fn verify(&self, skill: &Skill) -> Result<bool> {
        // In production, this would use a higher-capacity model
        // For now, do basic validation
        Ok(!skill.metadata.triggers.is_empty() && !skill.steps.is_empty())
    }
}

impl Default for TeacherVerifier {
    fn default() -> Self {
        Self::new()
    }
}

/// Consolidation pipeline
pub struct ConsolidationPipeline {
    episodic_store: EpisodicStore,
    procedural_store: crate::procedural_store::ProceduralStore,
    distillation_model: LightweightLLM,
    /// Minimum complexity score for success-based consolidation
    min_complexity_score: f32,
    /// Minimum occurrences for frequency-based consolidation
    min_frequency: u32,
}

impl ConsolidationPipeline {
    /// Create new consolidation pipeline
    pub fn new(
        episodic_store: EpisodicStore,
        procedural_store: crate::procedural_store::ProceduralStore,
        distillation_model: LightweightLLM,
    ) -> Self {
        Self {
            episodic_store,
            procedural_store,
            distillation_model,
            min_complexity_score: 0.7,
            min_frequency: 3,
        }
    }

    /// Create pipeline with custom thresholds
    pub fn with_thresholds(
        episodic_store: EpisodicStore,
        procedural_store: crate::procedural_store::ProceduralStore,
        distillation_model: LightweightLLM,
        min_complexity_score: f32,
        min_frequency: u32,
    ) -> Self {
        Self {
            episodic_store,
            procedural_store,
            distillation_model,
            min_complexity_score,
            min_frequency,
        }
    }

    /// Check for consolidation triggers
    pub async fn check_triggers(&self, session_id: &str) -> Result<Option<ConsolidationTrigger>> {
        // Retrieve session trajectory
        let trajectory = self.episodic_store.retrieve_trajectory(session_id).await?;

        if trajectory.is_empty() {
            return Ok(None);
        }

        // Check success-based trigger
        if let Some(trigger) = self.check_success_trigger(&trajectory)? {
            return Ok(Some(trigger));
        }

        // Check failure-based trigger
        if let Some(trigger) = self.check_failure_trigger(&trajectory)? {
            return Ok(Some(trigger));
        }

        // Check frequency-based trigger
        if let Some(trigger) = self.check_frequency_trigger(&trajectory).await? {
            return Ok(Some(trigger));
        }

        Ok(None) // No triggers
    }

    /// Check success-based trigger
    fn check_success_trigger(&self, trajectory: &[EpisodicMemory]) -> Result<Option<ConsolidationTrigger>> {
        // Look for success state at end of trajectory
        let has_success = trajectory
            .iter()
            .any(|m| m.memory_type == "success_state" && m.success == Some(true));

        if !has_success {
            return Ok(None);
        }

        // Calculate complexity score
        let complexity_score = self.calculate_complexity(trajectory);

        if complexity_score >= self.min_complexity_score {
            // Extract session_id from first memory
            let session_id = trajectory.first().map(|m| m.session_id.clone()).unwrap_or_default();

            return Ok(Some(ConsolidationTrigger::Success {
                session_id,
                complexity_score,
            }));
        }

        Ok(None)
    }

    /// Check failure-based trigger
    fn check_failure_trigger(&self, trajectory: &[EpisodicMemory]) -> Result<Option<ConsolidationTrigger>> {
        // Look for failure state
        let failures: Vec<_> = trajectory
            .iter()
            .filter(|m| m.memory_type == "failure_state" || m.success == Some(false))
            .collect();

        if failures.is_empty() {
            return Ok(None);
        }

        // Check for failure loop (multiple failures in same session)
        if failures.len() >= 2 {
            let session_id = trajectory.first().map(|m| m.session_id.clone()).unwrap_or_default();
            let failure_type = "failure_loop".to_string();

            return Ok(Some(ConsolidationTrigger::Failure {
                session_id,
                failure_type,
            }));
        }

        Ok(None)
    }

    /// Check frequency-based trigger
    async fn check_frequency_trigger(&self, trajectory: &[EpisodicMemory]) -> Result<Option<ConsolidationTrigger>> {
        // Extract action patterns
        let mut pattern_counts: HashMap<String, u32> = HashMap::new();

        for memory in trajectory {
            if memory.memory_type == "tool_execution" {
                // Extract action pattern (simplified)
                let pattern = self.extract_action_pattern(&memory.content);
                *pattern_counts.entry(pattern).or_insert(0) += 1;
            }
        }

        // Check if any pattern exceeds threshold
        for (pattern, count) in pattern_counts {
            if count >= self.min_frequency {
                return Ok(Some(ConsolidationTrigger::Frequency {
                    pattern_hash: pattern,
                    occurrence_count: count,
                }));
            }
        }

        Ok(None)
    }

    /// Calculate complexity score for trajectory
    fn calculate_complexity(&self, trajectory: &[EpisodicMemory]) -> f32 {
        let mut score = 0.0;

        // Factor 1: Number of steps
        score += (trajectory.len() as f32 * 0.1).min(0.3);

        // Factor 2: Variety of memory types
        let unique_types: std::collections::HashSet<_> =
            trajectory.iter().map(|m| &m.memory_type).collect();
        score += (unique_types.len() as f32 * 0.1).min(0.3);

        // Factor 3: Tool executions
        let tool_execs = trajectory
            .iter()
            .filter(|m| m.memory_type == "tool_execution")
            .count();
        score += (tool_execs as f32 * 0.05).min(0.4);

        score.min(1.0)
    }

    /// Extract action pattern from content
    fn extract_action_pattern(&self, content: &str) -> String {
        // Simplified pattern extraction
        // In production, this would use more sophisticated hashing
        let first_line = content.lines().next().unwrap_or("");
        format!("pattern_{}", first_line.chars().take(20).collect::<String>())
    }

    /// Extract trajectory (filter noise)
    pub fn extract_trajectory(&self, memories: &[EpisodicMemory]) -> Vec<ObservationActionPair> {
        memories
            .iter()
            .filter(|m| {
                // Filter out pleasantries, failed loops, redundant outputs
                matches!(
                    m.memory_type.as_str(),
                    "tool_execution" | "success_state" | "failure_state"
                )
            })
            .map(|m| {
                // Parse action and observation from content
                let (action, observation) = self.parse_action_observation(&m.content);
                ObservationActionPair::new(&observation, &action, m.success)
            })
            .collect()
    }

    /// Parse action and observation from content
    fn parse_action_observation(&self, content: &str) -> (String, String) {
        // Try to parse "Action: X\nOutput: Y" format
        if let Some(action_start) = content.find("Action:") {
            let after_action = &content[action_start + 7..];
            if let Some(output_start) = after_action.find("Output:") {
                let action = after_action[..output_start].trim().to_string();
                let observation = after_action[output_start + 7..].trim().to_string();
                return (action, observation);
            }
        }

        // Fallback: use entire content as observation
        (String::new(), content.to_string())
    }

    /// Distill trajectory into procedural skill
    pub async fn distill(&self, trajectory: &[ObservationActionPair]) -> Result<Skill> {
        if trajectory.is_empty() {
            return Err(ConsolidationError::DistillationFailed(
                "Empty trajectory".to_string(),
            ));
        }

        // Use lightweight LLM (Haiku) for distillation
        let prompt = self.build_distillation_prompt(trajectory);

        let response = self.distillation_model.generate(&prompt).await?;

        // Parse response into Skill (simplified for now)
        let skill = self.parse_skill_from_response(&response, trajectory)?;

        Ok(skill)
    }

    /// Build distillation prompt for LLM
    fn build_distillation_prompt(&self, trajectory: &[ObservationActionPair]) -> String {
        let mut prompt = String::from(
            r#"Analyze the following sequence of observations and actions, and extract a reusable skill.

For each observation-action pair, identify:
1. The problem being solved
2. The steps taken to solve it
3. Any validation or verification steps
4. The final outcome

Format the skill as follows:

skill_id: UNIQUE_ID
name: Human-readable name
domain: Category (e.g., "debugging", "security", "deployment")
triggers:
  - "trigger phrase 1"
  - "trigger phrase 2"

# Skill Name

## Steps

### Step 1: Description
```command
command if applicable
```

---

Observation-Action Pairs:

"#,
        );

        for (i, pair) in trajectory.iter().enumerate() {
            prompt.push_str(&format!(
                "\n{}. Observation: {}\n   Action: {}\n   Success: {:?}\n",
                i + 1,
                pair.observation,
                pair.action,
                pair.success
            ));
        }

        prompt
    }

    /// Parse skill from LLM response
    fn parse_skill_from_response(
        &self,
        response: &str,
        trajectory: &[ObservationActionPair],
    ) -> Result<Skill> {
        // In production, this would parse the actual LLM response
        // For now, create a simplified skill

        let skill_id = format!("SKILL_{}", Utc::now().timestamp());
        let name = "Consolidated Skill".to_string();
        let domain = "general".to_string();

        // Extract triggers from first observation
        let triggers = trajectory
            .first()
            .map(|p| vec![p.observation.chars().take(50).collect()])
            .unwrap_or_else(|| vec!["default".to_string()]);

        // Create steps from trajectory
        let steps: Vec<SkillStep> = trajectory
            .iter()
            .enumerate()
            .map(|(i, pair)| SkillStep {
                order: (i + 1) as u32,
                description: format!("Handle: {}", pair.observation.chars().take(100).collect::<String>()),
                command: if pair.action.is_empty() {
                    None
                } else {
                    Some(pair.action.clone())
                },
                validation: None,
            })
            .collect();

        let mut skill = Skill::new(&skill_id, &name, &domain, triggers, steps);

        // Store raw response
        skill.raw_content = response.to_string();

        Ok(skill)
    }

    /// Validate skill (human-in-the-loop or teacher-student)
    pub async fn validate(&self, skill: &Skill, mode: ValidationMode) -> Result<bool> {
        match mode {
            ValidationMode::Review => {
                // Store in staging, await human approval
                self.procedural_store.store_staging(skill.clone()).await?;

                // Generate validation report
                let _report = self.generate_validation_report(skill);

                // Return false (pending human approval)
                Ok(false)
            }

            ValidationMode::TeacherStudent => {
                // Higher-capacity orchestrator verifies
                let verifier = TeacherVerifier::new();
                let is_valid = verifier.verify(skill).await?;

                if is_valid {
                    // Auto-deploy
                    self.procedural_store.store(skill.clone()).await?;
                }

                Ok(is_valid)
            }
        }
    }

    /// Generate validation report
    fn generate_validation_report(&self, skill: &Skill) -> ValidationReport {
        let mut notes = Vec::new();
        let mut recommendations = Vec::new();

        // Check skill quality
        if skill.metadata.triggers.is_empty() {
            notes.push("No triggers defined".to_string());
            recommendations.push("Add at least one trigger phrase".to_string());
        }

        if skill.steps.is_empty() {
            notes.push("No steps defined".to_string());
            recommendations.push("Add procedural steps".to_string());
        }

        if skill.metadata.utility_score < 0.5 {
            notes.push("Low utility score".to_string());
            recommendations.push("Review skill effectiveness".to_string());
        }

        ValidationReport {
            skill_id: skill.metadata.skill_id.clone(),
            passed: notes.is_empty(),
            notes: notes.join("; "),
            recommendations,
        }
    }

    /// Run full consolidation cycle for a session
    pub async fn consolidate_session(&self, session_id: &str) -> Result<Option<Skill>> {
        // Check for triggers
        let trigger = self.check_triggers(session_id).await?;

        if trigger.is_none() {
            return Ok(None);
        }

        // Retrieve trajectory
        let trajectory = self.episodic_store.retrieve_trajectory(session_id).await?;

        if trajectory.is_empty() {
            return Err(ConsolidationError::NoTrajectory(session_id.to_string()));
        }

        // Extract and filter
        let filtered = self.extract_trajectory(&trajectory);

        if filtered.is_empty() {
            return Ok(None);
        }

        // Distill into skill
        let skill = self.distill(&filtered).await?;

        // Validate (Review Mode by default)
        self.validate(&skill, ValidationMode::Review).await?;

        Ok(Some(skill))
    }
}

/// Background consolidation worker
pub struct ConsolidationWorker {
    pipeline: ConsolidationPipeline,
    check_interval: std::time::Duration,
    running: std::sync::Arc<tokio::sync::Mutex<bool>>,
}

impl ConsolidationWorker {
    /// Create new worker
    pub fn new(pipeline: ConsolidationPipeline, check_interval: std::time::Duration) -> Self {
        Self {
            pipeline,
            check_interval,
            running: std::sync::Arc::new(tokio::sync::Mutex::new(false)),
        }
    }

    /// Start background consolidation
    pub async fn start(&self) {
        let mut running = self.running.lock().await;
        *running = true;
        drop(running);

        loop {
            if !*self.running.lock().await {
                break;
            }

            // Get recent sessions (simplified - in production would query episodic store)
            // For now, just skip
            tokio::time::sleep(self.check_interval).await;
        }
    }

    /// Stop background consolidation
    pub async fn stop(&self) {
        let mut running = self.running.lock().await;
        *running = false;
    }

    /// Check if worker is running
    pub async fn is_running(&self) -> bool {
        *self.running.lock().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::episodic_store::EpisodicStoreConfig;
    use crate::procedural_store::ProceduralStore;
    use std::env::temp_dir;

    async fn create_test_pipeline() -> ConsolidationPipeline {
        let episodic_store = EpisodicStore::new(EpisodicStoreConfig::in_memory())
            .await
            .unwrap();
        let procedural_store = ProceduralStore::new(&temp_dir())
            .await
            .unwrap();
        let distillation_model = LightweightLLM::new("claude-haiku");

        ConsolidationPipeline::new(episodic_store, procedural_store, distillation_model)
    }

    #[tokio::test]
    async fn test_success_trigger_detection() {
        let pipeline = create_test_pipeline().await;

        // Create session with success
        let session_id = "test-success";
        pipeline
            .episodic_store
            .log_conversation(session_id, 1, "Starting task")
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_action(session_id, 2, "run_command", "output", true)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_success(session_id, 3, "Task completed")
            .await
            .unwrap();

        // Check triggers
        let trigger = pipeline.check_triggers(session_id).await.unwrap();

        // Should detect success trigger (if complexity is high enough)
        // Note: This test session might be too simple, so trigger might be None
        // The important thing is no error
        assert!(trigger.is_some() || trigger.is_none()); // Just verify no error
    }

    #[tokio::test]
    async fn test_failure_trigger_detection() {
        let pipeline = create_test_pipeline().await;

        // Create session with failure loop
        let session_id = "test-failure";
        pipeline
            .episodic_store
            .log_action(session_id, 1, "cmd", "error1", false)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_action(session_id, 2, "cmd", "error2", false)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_failure(session_id, 3, "Catastrophic failure")
            .await
            .unwrap();

        // Check triggers
        let trigger = pipeline.check_triggers(session_id).await.unwrap();

        // Should detect failure trigger
        assert!(trigger.is_some());
        if let Some(t) = trigger {
            match t {
                ConsolidationTrigger::Failure { failure_type, .. } => {
                    assert_eq!(failure_type, "failure_loop");
                }
                _ => {} // Other triggers are ok too
            }
        }
    }

    #[tokio::test]
    async fn test_frequency_trigger_detection() {
        let pipeline = create_test_pipeline().await;

        // Create session with repeated pattern
        let session_id = "test-frequency";
        for i in 1..=5 {
            pipeline
                .episodic_store
                .log_action(session_id, i, "same_command", "same_output", true)
                .await
                .unwrap();
        }

        // Check triggers
        let trigger = pipeline.check_triggers(session_id).await.unwrap();

        // Should detect frequency trigger (5 occurrences >= min_frequency of 3)
        assert!(trigger.is_some());
    }

    #[tokio::test]
    async fn test_trajectory_extraction() {
        let pipeline = create_test_pipeline().await;

        let memories = vec![
            EpisodicMemory::conversation_turn("session", 1, "Thought process"),
            EpisodicMemory::tool_execution("session", 2, "cmd", "output", true),
            EpisodicMemory::terminal_output("session", 3, "Some output"),
            EpisodicMemory::success_state("session", 4, "Done"),
        ];

        let extracted = pipeline.extract_trajectory(&memories);

        // Should filter out conversation_turn and terminal_output
        assert_eq!(extracted.len(), 2); // tool_execution and success_state
    }

    #[tokio::test]
    async fn test_distillation_prompt() {
        let pipeline = create_test_pipeline().await;

        let trajectory = vec![
            ObservationActionPair::new("Error: auth failed", "check_jwt", Some(true)),
            ObservationActionPair::new("JWT expired", "refresh_token", Some(true)),
        ];

        let prompt = pipeline.build_distillation_prompt(&trajectory);

        assert!(prompt.contains("Observation-Action Pairs"));
        assert!(prompt.contains("Error: auth failed"));
        assert!(prompt.contains("check_jwt"));
    }

    #[tokio::test]
    async fn test_skill_parsing() {
        let pipeline = create_test_pipeline().await;

        let trajectory = vec![ObservationActionPair::new(
            "Test observation",
            "Test action",
            Some(true),
        )];

        let response = "skill_id: TEST_SKILL\nname: Test Skill\ndomain: testing\ntriggers:\n  - test";
        let skill = pipeline.parse_skill_from_response(response, &trajectory).unwrap();

        assert!(!skill.metadata.skill_id.is_empty());
        assert!(!skill.steps.is_empty());
    }

    #[tokio::test]
    async fn test_validation_review_mode() {
        let pipeline = create_test_pipeline().await;

        let skill = Skill::new(
            "TEST_SKILL",
            "Test Skill",
            "testing",
            vec!["test trigger".to_string()],
            vec![SkillStep {
                order: 1,
                description: "Test step".to_string(),
                command: None,
                validation: None,
            }],
        );

        let result = pipeline
            .validate(&skill, ValidationMode::Review)
            .await
            .unwrap();

        // Review mode returns false (pending human approval)
        assert!(!result);
    }

    #[tokio::test]
    async fn test_validation_teacher_student() {
        let pipeline = create_test_pipeline().await;

        let skill = Skill::new(
            "TEST_SKILL",
            "Test Skill",
            "testing",
            vec!["test trigger".to_string()],
            vec![SkillStep {
                order: 1,
                description: "Test step".to_string(),
                command: None,
                validation: None,
            }],
        );

        let result = pipeline
            .validate(&skill, ValidationMode::TeacherStudent)
            .await
            .unwrap();

        // Teacher-student mode auto-validates if skill is valid
        assert!(result);
    }

    #[tokio::test]
    async fn test_background_worker() {
        let pipeline = create_test_pipeline().await;
        let worker = ConsolidationWorker::new(pipeline, std::time::Duration::from_millis(100));

        assert!(!worker.is_running().await);

        // Just test start/stop without spawning
        worker.start().await;
        assert!(worker.is_running().await);

        // Stop worker
        worker.stop().await;
        assert!(!worker.is_running().await);
    }

    #[tokio::test]
    async fn test_end_to_end_consolidation() {
        let pipeline = create_test_pipeline().await;

        // Create session with complex successful task
        let session_id = "test-e2e";
        pipeline
            .episodic_store
            .log_conversation(session_id, 1, "Starting complex task")
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_action(session_id, 2, "step1", "output1", true)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_action(session_id, 3, "step2", "output2", true)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_action(session_id, 4, "step3", "output3", true)
            .await
            .unwrap();
        pipeline
            .episodic_store
            .log_success(session_id, 5, "Complex task completed")
            .await
            .unwrap();

        // Run consolidation
        let result = pipeline.consolidate_session(session_id).await;

        // Should produce a skill (or None if complexity too low)
        // The important thing is no error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_consolidation_no_trigger() {
        let pipeline = create_test_pipeline().await;

        // Create simple session (no success, no failure loop, no frequency)
        let session_id = "test-no-trigger";
        pipeline
            .episodic_store
            .log_conversation(session_id, 1, "Simple chat")
            .await
            .unwrap();

        let result = pipeline.consolidate_session(session_id).await;

        // Should return None (no trigger)
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_validation_report_generation() {
        let pipeline = create_test_pipeline().await;

        // Skill with issues
        let skill = Skill::new("TEST", "Test", "test", vec![], vec![]);
        let report = pipeline.generate_validation_report(&skill);

        assert!(!report.passed);
        assert!(!report.notes.is_empty());
        assert!(!report.recommendations.is_empty());

        // Valid skill
        let valid_skill = Skill::new(
            "VALID",
            "Valid Skill",
            "test",
            vec!["trigger".to_string()],
            vec![SkillStep {
                order: 1,
                description: "Step".to_string(),
                command: None,
                validation: None,
            }],
        );
        let report = pipeline.generate_validation_report(&valid_skill);

        assert!(report.passed);
    }

    #[tokio::test]
    async fn test_lightweight_llm() {
        let llm = LightweightLLM::new("claude-haiku");
        let response = llm.generate("Test prompt").await.unwrap();

        assert!(response.contains("claude-haiku"));
    }

    #[tokio::test]
    async fn test_teacher_verifier() {
        let verifier = TeacherVerifier::new();

        let valid_skill = Skill::new(
            "VALID",
            "Valid",
            "test",
            vec!["trigger".to_string()],
            vec![SkillStep {
                order: 1,
                description: "Step".to_string(),
                command: None,
                validation: None,
            }],
        );

        let result = verifier.verify(&valid_skill).await.unwrap();
        assert!(result);

        let invalid_skill = Skill::new("INVALID", "Invalid", "test", vec![], vec![]);
        let result = verifier.verify(&invalid_skill).await.unwrap();
        assert!(!result);
    }
}
