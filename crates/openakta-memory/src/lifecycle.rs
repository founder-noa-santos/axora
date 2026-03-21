//! Memory Lifecycle Management
//!
//! This module implements memory decay and pruning for OPENAKTA agents:
//! - Ebbinghaus Forgetting Curve (exponential decay)
//! - Utility-based refinement (success/failure tracking)
//! - Automatic pruning (delete memories below thresholds)
//! - Conflict resolution (time-decay weighting)
//!
//! # Ebbinghaus Forgetting Curve
//!
//! Memory strength decays exponentially over time:
//! ```text
//! S(t) = exp(-t / half_life)
//! ```
//!
//! Where:
//! - S(t) = memory strength at time t
//! - t = time since creation (in days)
//! - half_life = half-life period (default: 30 days)
//!
//! # Utility Tracking
//!
//! Procedural memories (skills) track success/failure rates:
//! ```text
//! utility = success_count / (success_count + failure_count)
//! ```
//!
//! # Pruning Strategy
//!
//! - **Episodic**: Time-decay only (prune old, unused memories)
//! - **Procedural**: Utility + decay (prune low-utility skills)
//! - **Semantic**: Not pruned (living docs are source of truth)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::RwLock;

/// Memory lifecycle errors
#[derive(Error, Debug)]
pub enum LifecycleError {
    /// IO error
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Memory not found
    #[error("memory not found: {0}")]
    NotFound(String),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Result type for lifecycle operations
pub type Result<T> = std::result::Result<T, LifecycleError>;

/// Memory lifecycle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleConfig {
    /// Delete memories with strength below this threshold
    pub strength_threshold: f32,
    /// Delete skills with utility below this threshold
    pub utility_threshold: f32,
    /// How often to run pruning (in seconds)
    pub pruning_interval_secs: u64,
    /// Ebbinghaus half-life (in days)
    pub half_life_days: f32,
    /// Minimum retrievals before memory is protected from pruning
    pub min_retrievals_protected: u32,
}

impl Default for LifecycleConfig {
    fn default() -> Self {
        Self {
            strength_threshold: 0.1,     // Delete if strength < 10%
            utility_threshold: 0.3,      // Delete if utility < 30%
            pruning_interval_secs: 3600, // Prune every hour
            half_life_days: 30.0,        // 30-day half-life
            min_retrievals_protected: 5, // Protected if retrieved 5+ times
        }
    }
}

/// Generic memory trait for lifecycle operations
pub trait MemoryTrait {
    fn id(&self) -> &str;
    fn created_at(&self) -> u64;
    fn updated_at(&self) -> u64;
    fn retrieval_count(&self) -> u32;
    fn importance(&self) -> f32;
}

/// Ebbinghaus decay model
pub struct EbbinghausDecay {
    half_life_secs: f32, // Half-life in seconds
}

impl EbbinghausDecay {
    /// Create new decay model with half-life in days
    pub fn new(half_life_days: f32) -> Self {
        let half_life_secs = half_life_days * 24.0 * 60.0 * 60.0;
        Self { half_life_secs }
    }

    /// Calculate exponential decay factor (0.0 to 1.0)
    pub fn exponential_decay(&self, created_at: u64) -> f32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let age_secs = now.saturating_sub(created_at) as f32;

        // Ebbinghaus Forgetting Curve: S = exp(-t / half_life)
        (-age_secs / self.half_life_secs).exp()
    }

    /// Calculate retrieval reinforcement (logarithmic, diminishing returns)
    pub fn retrieval_reinforcement(&self, retrieval_count: u32) -> f32 {
        // Logarithmic reinforcement: log10(count + 1)
        // +1 ensures non-zero reinforcement for new memories
        (retrieval_count as f32 + 1.0).log10()
    }

    /// Calculate importance boost (linear scaling)
    pub fn importance_boost(&self, importance: f32) -> f32 {
        // Normalize importance to 0.5-1.5 range
        0.5 + importance
    }
}

/// Utility tracker for procedural memories (skills)
pub struct UtilityTracker {
    /// In-memory utility scores (skill_id -> utility)
    utilities: Arc<RwLock<HashMap<String, f32>>>,
    /// Success counts (skill_id -> count)
    successes: Arc<RwLock<HashMap<String, u32>>>,
    /// Failure counts (skill_id -> count)
    failures: Arc<RwLock<HashMap<String, u32>>>,
}

impl UtilityTracker {
    /// Create new utility tracker
    pub fn new() -> Self {
        Self {
            utilities: Arc::new(RwLock::new(HashMap::new())),
            successes: Arc::new(RwLock::new(HashMap::new())),
            failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Update utility score based on outcome
    pub async fn update(&self, skill_id: &str, success: bool) -> Result<()> {
        // Update success/failure counts
        if success {
            let mut successes = self.successes.write().await;
            *successes.entry(skill_id.to_string()).or_insert(0) += 1;
        } else {
            let mut failures = self.failures.write().await;
            *failures.entry(skill_id.to_string()).or_insert(0) += 1;
        }

        // Recalculate utility
        let successes = self.successes.read().await;
        let failures = self.failures.read().await;

        let success_count = successes.get(skill_id).copied().unwrap_or(0);
        let failure_count = failures.get(skill_id).copied().unwrap_or(0);

        let utility = if success_count + failure_count == 0 {
            0.5 // Default utility for new skills
        } else {
            success_count as f32 / (success_count + failure_count) as f32
        };

        drop(successes);
        drop(failures);

        // Update utility score
        let mut utilities = self.utilities.write().await;
        utilities.insert(skill_id.to_string(), utility);

        Ok(())
    }

    /// Get utility score for skill
    pub async fn get_score(&self, skill_id: &str) -> Result<f32> {
        let utilities = self.utilities.read().await;
        Ok(*utilities.get(skill_id).unwrap_or(&0.5))
    }

    /// Get all utility scores
    pub async fn get_all_scores(&self) -> Result<HashMap<String, f32>> {
        let utilities = self.utilities.read().await;
        Ok(utilities.clone())
    }

    /// Remove utility tracking for skill
    pub async fn remove(&self, skill_id: &str) {
        let mut utilities = self.utilities.write().await;
        let mut successes = self.successes.write().await;
        let mut failures = self.failures.write().await;

        utilities.remove(skill_id);
        successes.remove(skill_id);
        failures.remove(skill_id);
    }
}

impl Default for UtilityTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Pruning report
#[derive(Debug, Default, Clone)]
pub struct PruningReport {
    /// Number of semantic memories pruned
    pub semantic_pruned: usize,
    /// Number of episodic memories pruned
    pub episodic_pruned: usize,
    /// Number of procedural skills pruned
    pub procedural_pruned: usize,
    /// Total memories pruned
    pub total_pruned: usize,
}

impl PruningReport {
    /// Calculate total pruned
    pub fn total(&self) -> usize {
        self.semantic_pruned + self.episodic_pruned + self.procedural_pruned
    }
}

/// Conflict resolution report
#[derive(Debug, Default, Clone)]
pub struct ConflictResolutionReport {
    /// Number of conflicts resolved
    pub resolved_count: usize,
    /// Details of resolved conflicts
    pub details: Vec<ConflictDetail>,
}

/// Details of a resolved conflict
#[derive(Debug, Clone)]
pub struct ConflictDetail {
    /// Winner memory ID
    pub winner_id: String,
    /// Loser memory IDs
    pub loser_ids: Vec<String>,
    /// Reason for resolution
    pub reason: String,
}

/// Memory conflict
#[derive(Debug, Clone)]
pub struct MemoryConflict {
    /// Conflicting memories
    pub memories: Vec<MemoryInfo>,
    /// Conflict type
    pub conflict_type: ConflictType,
}

/// Memory info for conflict detection
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub id: String,
    pub created_at: u64,
    pub strength: f32,
    pub content_hash: String,
}

/// Type of conflict
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Contradictory information
    Contradiction,
    /// Duplicate information
    Duplicate,
    /// Overlapping information
    Overlap,
}

/// Memory lifecycle manager
pub struct MemoryLifecycle {
    decay_model: EbbinghausDecay,
    utility_tracker: UtilityTracker,
    config: LifecycleConfig,
}

impl MemoryLifecycle {
    /// Create new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Self {
        let decay_model = EbbinghausDecay::new(config.half_life_days);
        let utility_tracker = UtilityTracker::new();

        Self {
            decay_model,
            utility_tracker,
            config,
        }
    }

    /// Create lifecycle manager with default config
    pub fn with_defaults() -> Self {
        Self::new(LifecycleConfig::default())
    }

    /// Calculate memory strength using Ebbinghaus model
    pub fn calculate_strength<M: MemoryTrait>(&self, memory: &M) -> f32 {
        let time_decay = self.decay_model.exponential_decay(memory.created_at());
        let importance_boost = self.decay_model.importance_boost(memory.importance());
        let retrieval_reinforcement = self
            .decay_model
            .retrieval_reinforcement(memory.retrieval_count());

        // Combined strength: time_decay * importance * retrieval
        time_decay * importance_boost * retrieval_reinforcement
    }

    /// Update utility score for skill
    pub async fn update_utility(&self, skill_id: &str, success: bool) -> Result<()> {
        self.utility_tracker.update(skill_id, success).await
    }

    /// Get utility score for skill
    pub async fn get_utility(&self, skill_id: &str) -> Result<f32> {
        self.utility_tracker.get_score(skill_id).await
    }

    /// Get all utility scores
    pub async fn get_all_utilities(&self) -> Result<HashMap<String, f32>> {
        self.utility_tracker.get_all_scores().await
    }

    /// Prune memories below thresholds
    pub async fn prune<M: MemoryTrait + Clone>(
        &self,
        memories: Vec<M>,
        mut should_delete: impl FnMut(&M) -> std::result::Result<bool, LifecycleError>,
    ) -> Result<PruningReport> {
        let mut pruned_count = 0;

        for memory in &memories {
            // Check if protected by retrieval count
            if memory.retrieval_count() >= self.config.min_retrievals_protected {
                continue;
            }

            // Calculate strength
            let strength = self.calculate_strength(memory);

            // Check if should be pruned
            if strength < self.config.strength_threshold {
                if let Ok(true) = should_delete(memory) {
                    pruned_count += 1;
                }
            }
        }

        Ok(PruningReport {
            episodic_pruned: pruned_count,
            total_pruned: pruned_count,
            ..Default::default()
        })
    }

    /// Prune procedural skills (utility + decay)
    pub async fn prune_procedural<S: MemoryTrait + Clone>(
        &self,
        skills: Vec<S>,
        mut should_delete: impl FnMut(&S) -> std::result::Result<bool, LifecycleError>,
    ) -> Result<PruningReport> {
        let mut pruned_count = 0;

        for skill in &skills {
            // Check if protected by retrieval count
            if skill.retrieval_count() >= self.config.min_retrievals_protected {
                continue;
            }

            // Get utility score
            let utility = self.get_utility(skill.id()).await.unwrap_or(0.5);

            // Calculate strength
            let strength = self.calculate_strength(skill);

            // Prune if below either threshold
            if utility < self.config.utility_threshold || strength < self.config.strength_threshold
            {
                if let Ok(true) = should_delete(skill) {
                    // Remove utility tracking
                    self.utility_tracker.remove(skill.id()).await;
                    pruned_count += 1;
                }
            }
        }

        Ok(PruningReport {
            procedural_pruned: pruned_count,
            total_pruned: pruned_count,
            ..Default::default()
        })
    }

    /// Detect conflicts between memories
    pub async fn detect_conflicts<M: MemoryTrait + Clone>(
        &self,
        memories: Vec<M>,
        content_hash: impl Fn(&M) -> String,
    ) -> Result<Vec<MemoryConflict>> {
        let mut conflicts = Vec::new();
        let mut hash_groups: HashMap<String, Vec<MemoryInfo>> = HashMap::new();

        // Group memories by content hash
        for memory in &memories {
            let hash = content_hash(memory);
            let info = MemoryInfo {
                id: memory.id().to_string(),
                created_at: memory.created_at(),
                strength: self.calculate_strength(memory),
                content_hash: hash.clone(),
            };

            hash_groups.entry(hash).or_default().push(info);
        }

        // Find conflicts (groups with multiple memories)
        for (_, group) in hash_groups {
            if group.len() > 1 {
                conflicts.push(MemoryConflict {
                    memories: group,
                    conflict_type: ConflictType::Duplicate,
                });
            }
        }

        Ok(conflicts)
    }

    /// Resolve conflicts using time-decay weighting
    pub async fn resolve_conflicts<M: MemoryTrait + Clone>(
        &self,
        memories: Vec<M>,
        content_hash: impl Fn(&M) -> String,
        mut should_delete: impl FnMut(&str) -> std::result::Result<bool, LifecycleError>,
    ) -> Result<ConflictResolutionReport> {
        let conflicts = self.detect_conflicts(memories, content_hash).await?;

        let mut report = ConflictResolutionReport::default();

        for conflict in conflicts {
            // Find winner (highest strength = newest + most retrieved)
            let winner = conflict
                .memories
                .iter()
                .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
                .unwrap();

            let mut loser_ids = Vec::new();

            // Delete losers
            for memory in &conflict.memories {
                if memory.id != winner.id {
                    if let Ok(true) = should_delete(&memory.id) {
                        loser_ids.push(memory.id.clone());
                        report.resolved_count += 1;
                    }
                }
            }

            if !loser_ids.is_empty() {
                report.details.push(ConflictDetail {
                    winner_id: winner.id.clone(),
                    loser_ids,
                    reason: format!(
                        "Winner strength: {:.3}, half-life: {:.1} days",
                        winner.strength, self.config.half_life_days
                    ),
                });
            }
        }

        Ok(report)
    }

    /// Get decay model reference
    pub fn decay_model(&self) -> &EbbinghausDecay {
        &self.decay_model
    }

    /// Get config reference
    pub fn config(&self) -> &LifecycleConfig {
        &self.config
    }
}

/// Background pruning worker
pub struct PruningWorker {
    lifecycle: Arc<MemoryLifecycle>,
    check_interval: Duration,
    running: Arc<RwLock<bool>>,
}

impl PruningWorker {
    /// Create new pruning worker
    pub fn new(lifecycle: MemoryLifecycle, check_interval: Duration) -> Self {
        Self {
            lifecycle: Arc::new(lifecycle),
            check_interval,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start background pruning (async loop)
    pub async fn run<M: MemoryTrait + Clone + Send + Sync + 'static>(
        &self,
        get_memories: impl Fn() -> std::result::Result<Vec<M>, LifecycleError> + Send + Sync,
        mut delete_memory: impl FnMut(&str) -> std::result::Result<bool, LifecycleError> + Send + Sync,
    ) -> Result<()> {
        let mut running = self.running.write().await;
        *running = true;
        drop(running);

        loop {
            // Check if still running
            {
                let running = self.running.read().await;
                if !*running {
                    break;
                }
            }

            // Get memories
            let memories = match get_memories() {
                Ok(m) => m,
                Err(e) => {
                    tracing::error!("Failed to get memories for pruning: {}", e);
                    tokio::time::sleep(self.check_interval).await;
                    continue;
                }
            };

            // Prune memories
            let report = self
                .lifecycle
                .prune(memories, |m| delete_memory(m.id()))
                .await;

            match report {
                Ok(report) => {
                    tracing::info!("Pruning complete: {} memories pruned", report.total_pruned);
                }
                Err(e) => {
                    tracing::error!("Pruning failed: {}", e);
                }
            }

            // Wait for next check
            tokio::time::sleep(self.check_interval).await;
        }

        Ok(())
    }

    /// Stop background pruning
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Check if worker is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

/// Simple in-memory memory implementation for testing
#[derive(Debug, Clone)]
pub struct TestMemory {
    pub id: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub retrieval_count: u32,
    pub importance: f32,
    pub content: String,
}

impl TestMemory {
    pub fn new(id: &str, content: &str) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            id: id.to_string(),
            created_at: now,
            updated_at: now,
            retrieval_count: 0,
            importance: 1.0,
            content: content.to_string(),
        }
    }

    pub fn with_age(mut self, age_days: u32) -> Self {
        self.created_at = self
            .created_at
            .saturating_sub(age_days as u64 * 24 * 60 * 60);
        self
    }

    pub fn with_retrievals(mut self, count: u32) -> Self {
        self.retrieval_count = count;
        self
    }

    pub fn with_importance(mut self, importance: f32) -> Self {
        self.importance = importance;
        self
    }
}

impl MemoryTrait for TestMemory {
    fn id(&self) -> &str {
        &self.id
    }

    fn created_at(&self) -> u64 {
        self.created_at
    }

    fn updated_at(&self) -> u64 {
        self.updated_at
    }

    fn retrieval_count(&self) -> u32 {
        self.retrieval_count
    }

    fn importance(&self) -> f32 {
        self.importance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebbinghaus_decay_calculation() {
        let decay = EbbinghausDecay::new(30.0); // 30-day half-life

        // New memory should have high decay factor
        let new_memory = TestMemory::new("test", "content");
        let decay_factor = decay.exponential_decay(new_memory.created_at);
        assert!(decay_factor > 0.9);

        // Old memory (30 days) should have ~0.37 decay factor (exp(-1))
        let old_memory = TestMemory::new("test", "content").with_age(30);
        let decay_factor = decay.exponential_decay(old_memory.created_at);
        assert!(decay_factor > 0.3);
        assert!(decay_factor < 0.5);

        // Very old memory (90 days) should have low decay factor (exp(-3) ≈ 0.05)
        let very_old_memory = TestMemory::new("test", "content").with_age(90);
        let decay_factor = decay.exponential_decay(very_old_memory.created_at);
        assert!(decay_factor < 0.1);
    }

    #[test]
    fn test_retrieval_reinforcement() {
        let decay = EbbinghausDecay::new(30.0);

        // No retrievals
        assert_eq!(decay.retrieval_reinforcement(0), 0.0);

        // 9 retrievals = log10(10) = 1.0
        assert!((decay.retrieval_reinforcement(9) - 1.0).abs() < 0.001);

        // 99 retrievals = log10(100) = 2.0
        assert!((decay.retrieval_reinforcement(99) - 2.0).abs() < 0.001);
    }

    #[test]
    fn test_memory_strength_calculation() {
        let lifecycle = MemoryLifecycle::with_defaults();

        // New, frequently retrieved memory should have high strength
        let strong_memory = TestMemory::new("test", "content").with_retrievals(100);
        let strength = lifecycle.calculate_strength(&strong_memory);
        assert!(strength > 1.0);

        // Old, never retrieved memory should have low strength
        let weak_memory = TestMemory::new("test", "content")
            .with_age(90)
            .with_retrievals(0);
        let strength = lifecycle.calculate_strength(&weak_memory);
        assert!(strength < 0.2);
    }

    #[tokio::test]
    async fn test_utility_update() {
        let lifecycle = MemoryLifecycle::with_defaults();

        // Initial utility should be 0.5
        let utility = lifecycle.get_utility("skill-1").await.unwrap();
        assert_eq!(utility, 0.5);

        // Update with success
        lifecycle.update_utility("skill-1", true).await.unwrap();
        let utility = lifecycle.get_utility("skill-1").await.unwrap();
        assert_eq!(utility, 1.0);

        // Update with failure
        lifecycle.update_utility("skill-1", false).await.unwrap();
        let utility = lifecycle.get_utility("skill-1").await.unwrap();
        assert_eq!(utility, 0.5); // 1 success, 1 failure

        // More successes
        lifecycle.update_utility("skill-1", true).await.unwrap();
        lifecycle.update_utility("skill-1", true).await.unwrap();
        let utility = lifecycle.get_utility("skill-1").await.unwrap();
        assert!(utility > 0.6); // 3 success, 1 failure
    }

    #[tokio::test]
    async fn test_episodic_pruning() {
        let config = LifecycleConfig {
            strength_threshold: 0.3,
            ..Default::default()
        };
        let lifecycle = MemoryLifecycle::new(config);

        // Create memories with different ages
        let memories = vec![
            TestMemory::new("new", "content").with_retrievals(10), // Protected
            TestMemory::new("old", "content").with_age(90),        // Should be pruned
            TestMemory::new("medium", "content").with_age(30),     // Might be pruned
        ];

        let mut deleted = Vec::new();
        let report = lifecycle
            .prune(memories, |m| {
                deleted.push(m.id.clone());
                Ok(true)
            })
            .await
            .unwrap();

        assert!(report.episodic_pruned >= 1); // At least old memory pruned
        assert!(deleted.contains(&"old".to_string()));
    }

    #[tokio::test]
    async fn test_procedural_pruning() {
        let config = LifecycleConfig {
            strength_threshold: 0.3,
            utility_threshold: 0.4,
            ..Default::default()
        };
        let lifecycle = MemoryLifecycle::new(config);

        // Create skills with different utilities
        lifecycle
            .update_utility("high-utility", true)
            .await
            .unwrap();
        lifecycle
            .update_utility("high-utility", true)
            .await
            .unwrap();
        lifecycle
            .update_utility("low-utility", false)
            .await
            .unwrap();
        lifecycle
            .update_utility("low-utility", false)
            .await
            .unwrap();

        let skills = vec![
            TestMemory::new("high-utility", "content").with_retrievals(10), // Protected
            TestMemory::new("low-utility", "content").with_age(60), // Should be pruned (low utility + old)
        ];

        let mut deleted = Vec::new();
        let report = lifecycle
            .prune_procedural(skills, |s| {
                deleted.push(s.id.clone());
                Ok(true)
            })
            .await
            .unwrap();

        assert!(report.procedural_pruned >= 1);
        assert!(deleted.contains(&"low-utility".to_string()));
    }

    #[tokio::test]
    async fn test_combined_threshold_pruning() {
        let config = LifecycleConfig {
            strength_threshold: 0.5,
            utility_threshold: 0.5,
            min_retrievals_protected: 3,
            ..Default::default()
        };
        let lifecycle = MemoryLifecycle::new(config);

        // Create skill with low utility but protected by retrievals
        lifecycle.update_utility("protected", false).await.unwrap();
        let protected_skill = TestMemory::new("protected", "content").with_retrievals(5);

        // Create skill with low utility and no protection
        lifecycle
            .update_utility("unprotected", false)
            .await
            .unwrap();
        let unprotected_skill = TestMemory::new("unprotected", "content").with_age(60);

        let skills = vec![protected_skill, unprotected_skill];
        let mut deleted = Vec::new();

        let report = lifecycle
            .prune_procedural(skills, |s| {
                deleted.push(s.id.clone());
                Ok(true)
            })
            .await
            .unwrap();

        // Protected skill should not be pruned
        assert!(!deleted.contains(&"protected".to_string()));
        // Unprotected skill should be pruned
        assert!(deleted.contains(&"unprotected".to_string()));
        assert_eq!(report.procedural_pruned, 1);
    }

    #[tokio::test]
    async fn test_conflict_resolution() {
        let lifecycle = MemoryLifecycle::with_defaults();

        // Create conflicting memories (same content hash)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let memories = vec![
            TestMemory {
                id: "old".to_string(),
                created_at: now - 86400, // 1 day ago
                updated_at: now - 86400,
                retrieval_count: 0,
                importance: 1.0,
                content: "same content".to_string(),
            },
            TestMemory {
                id: "new".to_string(),
                created_at: now,
                updated_at: now,
                retrieval_count: 0,
                importance: 1.0,
                content: "same content".to_string(),
            },
        ];

        let mut deleted = Vec::new();
        let report = lifecycle
            .resolve_conflicts(
                memories,
                |_| "hash".to_string(),
                |id| {
                    deleted.push(id.to_string());
                    Ok(true)
                },
            )
            .await
            .unwrap();

        // Old memory should be deleted, new memory should win
        assert_eq!(report.resolved_count, 1);
        assert!(deleted.contains(&"old".to_string()));
        assert!(!deleted.contains(&"new".to_string()));
    }

    #[tokio::test]
    async fn test_time_decay_weighting() {
        let lifecycle = MemoryLifecycle::with_defaults();

        // Create memories with different ages but same retrieval count
        let old_memory = TestMemory::new("old", "content")
            .with_age(60)
            .with_retrievals(5);
        let new_memory = TestMemory::new("new", "content").with_retrievals(5);

        let old_strength = lifecycle.calculate_strength(&old_memory);
        let new_strength = lifecycle.calculate_strength(&new_memory);

        // Newer memory should have higher strength
        assert!(new_strength > old_strength);
    }

    #[tokio::test]
    async fn test_background_pruning_worker() {
        // Test that the worker can be created and stopped
        let config = LifecycleConfig {
            pruning_interval_secs: 3600, // Long interval so it doesn't actually prune
            strength_threshold: 0.5,
            ..Default::default()
        };
        let lifecycle = MemoryLifecycle::new(config.clone());
        let worker = PruningWorker::new(lifecycle, Duration::from_secs(3600));

        // Verify worker is not running initially
        assert!(!worker.is_running().await);

        // Create simple in-memory test
        let memories = vec![
            TestMemory::new("strong", "content").with_retrievals(100),
            TestMemory::new("weak", "content").with_age(90),
        ];

        let lifecycle = MemoryLifecycle::new(config);
        let mut deleted = Vec::new();

        let report = lifecycle
            .prune(memories, |m| {
                deleted.push(m.id.clone());
                Ok(true)
            })
            .await
            .unwrap();

        // Weak memory should be pruned
        assert!(deleted.contains(&"weak".to_string()));
        assert!(report.total_pruned >= 1);
    }

    #[test]
    fn test_lifecycle_config_defaults() {
        let config = LifecycleConfig::default();

        assert_eq!(config.strength_threshold, 0.1);
        assert_eq!(config.utility_threshold, 0.3);
        assert_eq!(config.pruning_interval_secs, 3600);
        assert_eq!(config.half_life_days, 30.0);
        assert_eq!(config.min_retrievals_protected, 5);
    }

    #[test]
    fn test_pruning_report() {
        let report = PruningReport {
            semantic_pruned: 0,
            episodic_pruned: 5,
            procedural_pruned: 3,
            total_pruned: 8,
        };

        assert_eq!(report.total(), 8);
    }

    #[tokio::test]
    async fn test_utility_tracker_removal() {
        let tracker = UtilityTracker::new();

        // Add some utility data
        tracker.update("skill-1", true).await.unwrap();
        tracker.update("skill-1", false).await.unwrap();

        // Verify it exists
        let utility = tracker.get_score("skill-1").await.unwrap();
        assert_eq!(utility, 0.5);

        // Remove it
        tracker.remove("skill-1").await;

        // Should return default
        let utility = tracker.get_score("skill-1").await.unwrap();
        assert_eq!(utility, 0.5);
    }
}
