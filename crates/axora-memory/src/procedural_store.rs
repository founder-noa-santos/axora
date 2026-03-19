//! Procedural Memory Store
//!
//! This module implements procedural memory storage for AXORA agents:
//! - File-system repository (SKILL.md format)
//! - Trigger-based retrieval
//! - Progressive disclosure (load only required skills)
//! - Staging area for skills awaiting validation
//!
//! # SKILL.md Format
//!
//! ```markdown
//! ---
//! skill_id: "DEBUG_AUTH_FAILURE"
//! name: "Debug Authentication Failure"
//! triggers:
//!   - "authentication failure"
//!   - "JWT validation failed"
//! domain: "security"
//! success_count: 15
//! failure_count: 2
//! utility_score: 0.88
//! ---
//!
//! # Debug Authentication Failure
//!
//! ## Steps
//!
//! ### Step 1: Extract JWT from Request
//! ```bash
//! curl -v https://api.example.com/secure/endpoint
//! ```
//! ```

use crate::lifecycle::MemoryTrait;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::fs;

/// Procedural memory errors
#[derive(Error, Debug)]
pub enum ProceduralError {
    /// IO error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_yaml::Error),

    /// Skill not found
    #[error("skill not found: {0}")]
    NotFound(String),

    /// Invalid skill format
    #[error("invalid skill format: {0}")]
    InvalidFormat(String),

    /// Trigger match error
    #[error("no matching trigger for: {0}")]
    NoTriggerMatch(String),
}

/// Result type for procedural memory operations
pub type Result<T> = std::result::Result<T, ProceduralError>;

/// Skill outcome for utility tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillOutcome {
    /// Skill executed successfully
    Success,
    /// Skill execution failed
    Failure,
}

/// Skill step in procedural workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    /// Step order (1-indexed)
    pub order: u32,
    /// Step description
    pub description: String,
    /// Optional terminal command
    pub command: Option<String>,
    /// Optional validation check
    pub validation: Option<String>,
}

/// Script attached to skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    /// Script language (bash, python, rust, etc.)
    pub language: String,
    /// Script code
    pub code: String,
}

/// Skill metadata (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    /// Unique skill identifier
    pub skill_id: String,
    /// Human-readable name
    pub name: String,
    /// Natural language triggers for retrieval
    pub triggers: Vec<String>,
    /// Domain category
    pub domain: String,
    /// Unix timestamp when created
    pub created_at: u64,
    /// Unix timestamp when last updated
    pub updated_at: u64,
    /// Number of successful executions
    pub success_count: u32,
    /// Number of failed executions
    pub failure_count: u32,
    /// Utility score (0.0 to 1.0)
    pub utility_score: f32,
}

impl SkillMetadata {
    /// Create new metadata
    pub fn new(skill_id: &str, name: &str, domain: &str, triggers: Vec<String>) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            skill_id: skill_id.to_string(),
            name: name.to_string(),
            triggers,
            domain: domain.to_string(),
            created_at: now,
            updated_at: now,
            success_count: 0,
            failure_count: 0,
            utility_score: 0.5, // Default score
        }
    }

    /// Get total execution count
    pub fn total_executions(&self) -> u32 {
        self.success_count + self.failure_count
    }

    /// Get success rate
    pub fn success_rate(&self) -> f32 {
        let total = self.total_executions();
        if total == 0 {
            0.5
        } else {
            self.success_count as f32 / total as f32
        }
    }
}

/// Skill entity (procedural memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill metadata
    pub metadata: SkillMetadata,
    /// Procedural steps (markdown content)
    pub steps: Vec<SkillStep>,
    /// Optional execution scripts
    pub scripts: Option<Vec<Script>>,
    /// Related skill IDs
    pub related_skills: Vec<String>,
    /// Full markdown content (for storage)
    pub raw_content: String,
}

impl Skill {
    /// Create new skill
    pub fn new(
        skill_id: &str,
        name: &str,
        domain: &str,
        triggers: Vec<String>,
        steps: Vec<SkillStep>,
    ) -> Self {
        Self {
            metadata: SkillMetadata::new(skill_id, name, domain, triggers),
            steps,
            scripts: None,
            related_skills: Vec::new(),
            raw_content: String::new(),
        }
    }

    /// Add a script to the skill
    pub fn with_script(mut self, language: &str, code: &str) -> Self {
        let script = Script {
            language: language.to_string(),
            code: code.to_string(),
        };

        match self.scripts {
            Some(ref mut scripts) => scripts.push(script),
            None => self.scripts = Some(vec![script]),
        }

        self
    }

    /// Add related skill
    pub fn with_related(mut self, skill_id: &str) -> Self {
        self.related_skills.push(skill_id.to_string());
        self
    }

    /// Check if any trigger matches the task context
    pub fn matches_trigger(&self, task_context: &str) -> bool {
        let task_lower = task_context.to_lowercase();
        self.metadata
            .triggers
            .iter()
            .any(|trigger| task_lower.contains(&trigger.to_lowercase()))
    }

    /// Serialize skill to SKILL.md format
    pub fn to_skill_md(&self) -> Result<String> {
        // Serialize YAML frontmatter
        let mut content = String::from("---\n");
        let yaml = serde_yaml::to_string(&self.metadata)?;
        content.push_str(&yaml);
        content.push_str("---\n\n");

        // Add title
        content.push_str(&format!("# {}\n\n", self.metadata.name));

        // Add steps
        for step in &self.steps {
            content.push_str(&format!("### Step {}: {}\n", step.order, step.description));
            if let Some(ref command) = step.command {
                content.push_str("```bash\n");
                content.push_str(command);
                content.push_str("\n```\n\n");
            }
            if let Some(ref validation) = step.validation {
                content.push_str(&format!("**Validation:** {}\n\n", validation));
            }
        }

        // Add scripts
        if let Some(ref scripts) = self.scripts {
            content.push_str("## Scripts\n\n");
            for script in scripts {
                content.push_str(&format!("```{}\n{}\n```\n\n", script.language, script.code));
            }
        }

        // Add related skills
        if !self.related_skills.is_empty() {
            content.push_str("## Related Skills\n\n");
            for skill_id in &self.related_skills {
                content.push_str(&format!("- {}\n", skill_id));
            }
        }

        Ok(content)
    }

    /// Deserialize skill from SKILL.md format
    pub fn from_skill_md(content: &str) -> Result<Self> {
        // Extract YAML frontmatter
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            return Err(ProceduralError::InvalidFormat(
                "Missing YAML frontmatter delimiters".to_string(),
            ));
        }

        let yaml_content = parts[1];
        let markdown_content = parts[2];

        // Parse metadata
        let metadata: SkillMetadata = serde_yaml::from_str(yaml_content)?;

        // Parse steps from markdown (simplified parsing)
        let steps = Self::parse_steps(markdown_content);

        Ok(Self {
            metadata,
            steps,
            scripts: None,
            related_skills: Vec::new(),
            raw_content: content.to_string(),
        })
    }

    /// Parse steps from markdown content
    fn parse_steps(content: &str) -> Vec<SkillStep> {
        let mut steps = Vec::new();
        let mut current_step: Option<SkillStep> = None;

        for line in content.lines() {
            // Check for step header
            if line.starts_with("### Step ") {
                // Save previous step
                if let Some(step) = current_step.take() {
                    steps.push(step);
                }

                // Parse step number and description
                let parts: Vec<&str> = line.splitn(2, ": ").collect();
                if parts.len() == 2 {
                    let order = parts[0]
                        .trim_start_matches("### Step ")
                        .parse()
                        .unwrap_or(0);
                    let description = parts[1].to_string();

                    current_step = Some(SkillStep {
                        order,
                        description,
                        command: None,
                        validation: None,
                    });
                }
            } else if let Some(ref mut step) = current_step {
                // Check for command block
                if line.trim() == "```bash" {
                    // Extract command (simplified)
                    // In production, would parse full code block
                }

                // Check for validation
                if line.starts_with("**Validation:**") {
                    step.validation = Some(
                        line.trim_start_matches("**Validation:**")
                            .trim()
                            .to_string(),
                    );
                }
            }
        }

        // Add last step
        if let Some(step) = current_step {
            steps.push(step);
        }

        steps
    }
}

impl MemoryTrait for Skill {
    fn id(&self) -> &str {
        &self.metadata.skill_id
    }

    fn created_at(&self) -> u64 {
        self.metadata.created_at
    }

    fn updated_at(&self) -> u64 {
        self.metadata.updated_at
    }

    fn retrieval_count(&self) -> u32 {
        self.metadata.total_executions()
    }

    fn importance(&self) -> f32 {
        self.metadata.utility_score
    }
}

/// Procedural memory store
pub struct ProceduralStore {
    skills_dir: PathBuf,
    staging_dir: PathBuf,
}

impl ProceduralStore {
    /// Create new procedural store
    pub async fn new(base_dir: &Path) -> Result<Self> {
        let skills_dir = base_dir.join("skills");
        let staging_dir = base_dir.join("staging");

        // Create directories
        fs::create_dir_all(&skills_dir).await?;
        fs::create_dir_all(&staging_dir).await?;

        Ok(Self {
            skills_dir,
            staging_dir,
        })
    }

    /// Create procedural store with custom directories
    pub fn with_dirs(skills_dir: PathBuf, staging_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            staging_dir,
        }
    }

    /// Store skill (after validation)
    pub async fn store(&self, skill: Skill) -> Result<()> {
        let file_path = self
            .skills_dir
            .join(format!("{}.md", skill.metadata.skill_id));

        // Serialize to SKILL.md format
        let content = skill.to_skill_md()?;

        // Write to file
        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// Store skill in staging (awaiting validation)
    pub async fn store_staging(&self, skill: Skill) -> Result<()> {
        let file_path = self
            .staging_dir
            .join(format!("{}.md", skill.metadata.skill_id));

        let content = skill.to_skill_md()?;
        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// Retrieve skill by trigger match
    pub async fn retrieve_by_trigger(&self, task_context: &str) -> Result<Option<Skill>> {
        // Scan skills directory for trigger matches
        let mut entries = fs::read_dir(&self.skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                let content = fs::read_to_string(&path).await?;
                let skill = Skill::from_skill_md(&content)?;

                // Check if any trigger matches task context
                if skill.matches_trigger(task_context) {
                    return Ok(Some(skill));
                }
            }
        }

        Ok(None) // No matching skill
    }

    /// Get skill by ID
    pub async fn get(&self, skill_id: &str) -> Result<Option<Skill>> {
        let file_path = self.skills_dir.join(format!("{}.md", skill_id));

        if !file_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&file_path).await?;
        let skill = Skill::from_skill_md(&content)?;

        Ok(Some(skill))
    }

    /// Get all skills
    pub async fn get_all(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();
        let mut entries = fs::read_dir(&self.skills_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                let content = fs::read_to_string(&path).await?;
                if let Ok(skill) = Skill::from_skill_md(&content) {
                    skills.push(skill);
                }
            }
        }

        Ok(skills)
    }

    /// Promote skill from staging to active
    pub async fn promote_from_staging(&self, skill_id: &str) -> Result<()> {
        let staging_path = self.staging_dir.join(format!("{}.md", skill_id));
        let active_path = self.skills_dir.join(format!("{}.md", skill_id));

        if !staging_path.exists() {
            return Err(ProceduralError::NotFound(format!(
                "Staging skill not found: {}",
                skill_id
            )));
        }

        fs::rename(&staging_path, &active_path).await?;

        Ok(())
    }

    /// Delete skill
    pub async fn delete(&self, skill_id: &str) -> Result<()> {
        let file_path = self.skills_dir.join(format!("{}.md", skill_id));

        if file_path.exists() {
            fs::remove_file(&file_path).await?;
        }

        Ok(())
    }

    /// Update skill utility score
    pub async fn update_utility(&self, skill_id: &str, outcome: SkillOutcome) -> Result<()> {
        // Load skill
        let skill_path = self.skills_dir.join(format!("{}.md", skill_id));

        if !skill_path.exists() {
            return Err(ProceduralError::NotFound(format!(
                "Skill not found: {}",
                skill_id
            )));
        }

        let content = fs::read_to_string(&skill_path).await?;
        let mut skill = Skill::from_skill_md(&content)?;

        // Update counters
        match outcome {
            SkillOutcome::Success => skill.metadata.success_count += 1,
            SkillOutcome::Failure => skill.metadata.failure_count += 1,
        }

        // Recalculate utility score
        skill.metadata.utility_score = self.calculate_utility(&skill.metadata);
        skill.metadata.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Save updated skill
        self.store(skill).await
    }

    /// Calculate utility score (success rate with time decay)
    fn calculate_utility(&self, metadata: &SkillMetadata) -> f32 {
        let total = metadata.total_executions();
        if total == 0 {
            return 0.5; // Default score
        }

        let success_rate = metadata.success_rate();

        // Apply time decay (30-day half-life)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age_secs = now.saturating_sub(metadata.updated_at);
        let decay_factor = (-(age_secs as i64) as f32 / (30.0 * 24.0 * 60.0 * 60.0)).exp();

        success_rate * decay_factor
    }

    /// Get staging skills
    pub async fn get_staging(&self) -> Result<Vec<Skill>> {
        let mut skills = Vec::new();
        let mut entries = fs::read_dir(&self.staging_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "md") {
                let content = fs::read_to_string(&path).await?;
                if let Ok(skill) = Skill::from_skill_md(&content) {
                    skills.push(skill);
                }
            }
        }

        Ok(skills)
    }

    /// Get skills directory path
    pub fn skills_dir(&self) -> &Path {
        &self.skills_dir
    }

    /// Get staging directory path
    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }
}

/// Skill repository for batch operations
pub struct SkillRepository {
    store: ProceduralStore,
}

impl SkillRepository {
    /// Create new skill repository
    pub async fn new(base_dir: &Path) -> Result<Self> {
        let store = ProceduralStore::new(base_dir).await?;
        Ok(Self { store })
    }

    /// Find best matching skill for task
    pub async fn find_best_skill(&self, task_context: &str) -> Result<Option<Skill>> {
        let skills = self.store.get_all().await?;

        // Find best matching skill by trigger score
        let best_skill = skills
            .into_iter()
            .filter(|skill| skill.matches_trigger(task_context))
            .max_by(|a, b| {
                b.metadata
                    .utility_score
                    .partial_cmp(&a.metadata.utility_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        Ok(best_skill)
    }

    /// Get skills by domain
    pub async fn get_by_domain(&self, domain: &str) -> Result<Vec<Skill>> {
        let skills = self.store.get_all().await?;
        Ok(skills
            .into_iter()
            .filter(|skill| skill.metadata.domain == domain)
            .collect())
    }

    /// Get top N skills by utility score
    pub async fn get_top_skills(&self, n: usize) -> Result<Vec<Skill>> {
        let mut skills = self.store.get_all().await?;

        skills.sort_by(|a, b| {
            b.metadata
                .utility_score
                .partial_cmp(&a.metadata.utility_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        skills.truncate(n);
        Ok(skills)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_skill() -> Skill {
        Skill::new(
            "TEST_SKILL",
            "Test Skill",
            "testing",
            vec!["test trigger".to_string(), "example".to_string()],
            vec![
                SkillStep {
                    order: 1,
                    description: "First step".to_string(),
                    command: Some("echo step 1".to_string()),
                    validation: Some("Check output".to_string()),
                },
                SkillStep {
                    order: 2,
                    description: "Second step".to_string(),
                    command: None,
                    validation: None,
                },
            ],
        )
    }

    #[test]
    fn test_skill_metadata_creation() {
        let metadata = SkillMetadata::new("test-001", "Test", "testing", vec![]);

        assert_eq!(metadata.skill_id, "test-001");
        assert_eq!(metadata.name, "Test");
        assert_eq!(metadata.domain, "testing");
        assert_eq!(metadata.success_count, 0);
        assert_eq!(metadata.failure_count, 0);
        assert!(metadata.created_at > 0);
        assert!(metadata.updated_at > 0);
    }

    #[test]
    fn test_skill_metadata_success_rate() {
        let mut metadata = SkillMetadata::new("test-001", "Test", "testing", vec![]);
        metadata.success_count = 8;
        metadata.failure_count = 2;

        assert_eq!(metadata.total_executions(), 10);
        assert_eq!(metadata.success_rate(), 0.8);
    }

    #[test]
    fn test_skill_trigger_matching() {
        let skill = create_test_skill();

        assert!(skill.matches_trigger("I need to test trigger something"));
        assert!(skill.matches_trigger("This is an example"));
        assert!(!skill.matches_trigger("unrelated task"));
    }

    #[test]
    fn test_skill_serialization() {
        let skill = create_test_skill();
        let content = skill.to_skill_md().unwrap();

        assert!(content.contains("---"));
        assert!(content.contains("skill_id: TEST_SKILL"));
        assert!(content.contains("Test Skill"));
        assert!(content.contains("### Step 1: First step"));
    }

    #[test]
    fn test_skill_deserialization() {
        let skill = create_test_skill();
        let content = skill.to_skill_md().unwrap();
        let deserialized = Skill::from_skill_md(&content).unwrap();

        assert_eq!(deserialized.metadata.skill_id, skill.metadata.skill_id);
        assert_eq!(deserialized.metadata.name, skill.metadata.name);
        assert_eq!(deserialized.steps.len(), skill.steps.len());
    }

    #[test]
    fn test_skill_with_script() {
        let skill = create_test_skill().with_script("python", "print('hello')");

        assert!(skill.scripts.is_some());
        assert_eq!(skill.scripts.unwrap().len(), 1);
    }

    #[test]
    fn test_skill_with_related() {
        let skill = create_test_skill()
            .with_related("SKILL_001")
            .with_related("SKILL_002");

        assert_eq!(skill.related_skills.len(), 2);
        assert!(skill.related_skills.contains(&"SKILL_001".to_string()));
    }

    #[tokio::test]
    async fn test_procedural_store_creation() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();

        assert!(store.skills_dir.exists());
        assert!(store.staging_dir.exists());
    }

    #[tokio::test]
    async fn test_procedural_store_storage() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();
        let skill = create_test_skill();

        store.store(skill.clone()).await.unwrap();

        let file_path = store.skills_dir.join("TEST_SKILL.md");
        assert!(file_path.exists());
    }

    #[tokio::test]
    async fn test_procedural_store_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();
        let skill = create_test_skill();

        store.store(skill.clone()).await.unwrap();

        let retrieved = store
            .retrieve_by_trigger("I need to test trigger something")
            .await
            .unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().metadata.skill_id, "TEST_SKILL");
    }

    #[tokio::test]
    async fn test_procedural_store_staging() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();
        let skill = create_test_skill();

        store.store_staging(skill.clone()).await.unwrap();

        let staging_path = store.staging_dir.join("TEST_SKILL.md");
        assert!(staging_path.exists());

        store.promote_from_staging("TEST_SKILL").await.unwrap();

        assert!(!staging_path.exists());
        assert!(store.skills_dir.join("TEST_SKILL.md").exists());
    }

    #[tokio::test]
    async fn test_utility_update() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();
        let skill = create_test_skill();

        store.store(skill.clone()).await.unwrap();

        store
            .update_utility("TEST_SKILL", SkillOutcome::Success)
            .await
            .unwrap();

        let retrieved = store.get("TEST_SKILL").await.unwrap().unwrap();
        assert_eq!(retrieved.metadata.success_count, 1);
        assert_eq!(retrieved.metadata.failure_count, 0);
    }

    #[tokio::test]
    async fn test_utility_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let store = ProceduralStore::new(temp_dir.path()).await.unwrap();

        let mut metadata = SkillMetadata::new("test", "Test", "test", vec![]);
        metadata.success_count = 8;
        metadata.failure_count = 2;

        let utility = store.calculate_utility(&metadata);

        // Should be close to success rate (0.8) with minimal decay
        assert!(utility > 0.7);
        assert!(utility <= 0.8);
    }

    #[tokio::test]
    async fn test_skill_repository_find_best() {
        let temp_dir = TempDir::new().unwrap();
        let repo = SkillRepository::new(temp_dir.path()).await.unwrap();

        // Add skills with different success counts (which determines utility)
        let mut skill1 = create_test_skill();
        skill1.metadata.skill_id = "SKILL_1".to_string();
        skill1.metadata.success_count = 3;
        skill1.metadata.failure_count = 7; // 30% success rate
        repo.store.store(skill1).await.unwrap();

        let mut skill2 = create_test_skill();
        skill2.metadata.skill_id = "SKILL_2".to_string();
        skill2.metadata.success_count = 9;
        skill2.metadata.failure_count = 1; // 90% success rate
        repo.store.store(skill2).await.unwrap();

        let best = repo.find_best_skill("test trigger").await.unwrap().unwrap();

        // Best skill should be SKILL_2 (higher success rate)
        assert_eq!(best.metadata.skill_id, "SKILL_2");
    }

    #[tokio::test]
    async fn test_skill_repository_by_domain() {
        let temp_dir = TempDir::new().unwrap();
        let repo = SkillRepository::new(temp_dir.path()).await.unwrap();

        let mut skill1 = create_test_skill();
        skill1.metadata.skill_id = "SEC_SKILL".to_string();
        skill1.metadata.domain = "security".to_string();
        repo.store.store(skill1).await.unwrap();

        let mut skill2 = create_test_skill();
        skill2.metadata.skill_id = "TEST_SKILL_2".to_string();
        skill2.metadata.domain = "testing".to_string();
        repo.store.store(skill2).await.unwrap();

        let security_skills = repo.get_by_domain("security").await.unwrap();
        assert!(security_skills.len() >= 1);
        assert!(security_skills
            .iter()
            .any(|s| s.metadata.domain == "security"));
    }
}
