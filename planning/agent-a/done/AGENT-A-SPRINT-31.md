# Agent A — Sprint 31: Memory Lifecycle (Decay + Pruning)

**Phase:** 2  
**Sprint:** 31 (Memory Architecture)  
**File:** `crates/openakta-memory/src/lifecycle.rs`  
**Priority:** HIGH (prevents memory bloat, context rot)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Memory Lifecycle Management** with Ebbinghaus decay and utility-based pruning.

### Context

R-14 research provides CRITICAL implementation details:
- **Ebbinghaus Forgetting Curve** — Exponential decay over time
- **Utility-Based Refinement** — Track success/failure rates
- **Automatic Pruning** — Delete memories below thresholds
- **Conflict Resolution** — Time-decay weighting for contradictions

**Your job:** Implement memory lifecycle (prevents context rot, memory bloat).

---

## 📋 Deliverables

### 1. Create lifecycle.rs

**File:** `crates/openakta-memory/src/lifecycle.rs`

**Core Structure:**
```rust
//! Memory Lifecycle Management
//!
//! This module implements memory decay and pruning:
//! - Ebbinghaus Forgetting Curve (exponential decay)
//! - Utility-based refinement (success/failure tracking)
//! - Automatic pruning (delete below thresholds)
//! - Conflict resolution (time-decay weighting)

use crate::semantic_store::SemanticStore;
use crate::episodic_store::EpisodicStore;
use crate::procedural_store::ProceduralStore;

/// Memory lifecycle manager
pub struct MemoryLifecycle {
    decay_model: EbbinghausDecay,
    utility_tracker: UtilityTracker,
    config: LifecycleConfig,
}

/// Ebbinghaus decay model
pub struct EbbinghausDecay {
    half_life_days: f32, // Default: 30 days
}

/// Utility tracker
pub struct UtilityTracker {
    db: SqlitePool,
}

/// Lifecycle configuration
#[derive(Debug, Clone)]
pub struct LifecycleConfig {
    pub strength_threshold: f32, // Delete if below this
    pub utility_threshold: f32, // Delete if below this
    pub pruning_interval: Duration, // How often to prune
}

impl MemoryLifecycle {
    /// Create new lifecycle manager
    pub fn new(config: LifecycleConfig) -> Result<Self> {
        let decay_model = EbbinghausDecay::new(30.0); // 30-day half-life
        let utility_tracker = UtilityTracker::new().await?;
        
        Ok(Self {
            decay_model,
            utility_tracker,
            config,
        })
    }
    
    /// Calculate memory strength (Ebbinghaus Forgetting Curve)
    pub fn calculate_strength(&self, memory: &Memory) -> f32 {
        let time_decay = self.decay_model.exponential_decay(memory.created_at);
        let importance_boost = memory.importance_scalar;
        let retrieval_reinforcement = (memory.retrieval_count as f32 + 1.0).log10();
        
        time_decay * importance_boost * retrieval_reinforcement
    }
    
    /// Update utility score (success/failure tracking)
    pub async fn update_utility(
        &self,
        skill_id: &str,
        outcome: SkillOutcome,
    ) -> Result<()> {
        self.utility_tracker.update(skill_id, outcome).await
    }
    
    /// Get utility score for skill
    pub async fn get_utility(&self, skill_id: &str) -> Result<f32> {
        self.utility_tracker.get_score(skill_id).await
    }
    
    /// Prune memories below thresholds
    pub async fn prune(
        &self,
        semantic_store: &SemanticStore,
        episodic_store: &EpisodicStore,
        procedural_store: &ProceduralStore,
    ) -> Result<PruningReport> {
        let mut report = PruningReport::default();
        
        // Prune episodic memories (time-decay only)
        let episodic_pruned = self.prune_episodic(episodic_store).await?;
        report.episodic_pruned = episodic_pruned;
        
        // Prune procedural skills (utility + decay)
        let procedural_pruned = self.prune_procedural(procedural_store).await?;
        report.procedural_pruned = procedural_pruned;
        
        // Semantic memories are not pruned (living docs source of truth)
        report.semantic_pruned = 0;
        
        Ok(report)
    }
    
    /// Prune episodic memories (time-decay)
    async fn prune_episodic(&self, store: &EpisodicStore) -> Result<usize> {
        // Get all episodic memories
        let memories = store.get_all_memories().await?;
        
        let mut pruned_count = 0;
        
        for memory in memories {
            let strength = self.calculate_strength(&memory);
            
            if strength < self.config.strength_threshold {
                store.delete(&memory.id).await?;
                pruned_count += 1;
            }
        }
        
        Ok(pruned_count)
    }
    
    /// Prune procedural skills (utility + decay)
    async fn prune_procedural(&self, store: &ProceduralStore) -> Result<usize> {
        // Get all skills
        let skills = store.get_all_skills().await?;
        
        let mut pruned_count = 0;
        
        for skill in skills {
            let utility = self.get_utility(&skill.metadata.skill_id).await?;
            let strength = self.calculate_strength(&skill.into());
            
            // Prune if below combined threshold
            if utility < self.config.utility_threshold || strength < self.config.strength_threshold {
                store.delete(&skill.metadata.skill_id).await?;
                pruned_count += 1;
            }
        }
        
        Ok(pruned_count)
    }
    
    /// Resolve conflicts (time-decay weighting)
    pub async fn resolve_conflicts(
        &self,
        semantic_store: &SemanticStore,
    ) -> Result<ConflictResolutionReport> {
        // Scan for contradictory memories
        let conflicts = self.detect_conflicts(semantic_store).await?;
        
        let mut report = ConflictResolutionReport::default();
        
        for conflict in conflicts {
            // Newer memories have higher temporal confidence
            let winner = conflict.memories
                .iter()
                .max_by(|a, b| {
                    let a_score = self.calculate_strength(a);
                    let b_score = self.calculate_strength(b);
                    a_score.partial_cmp(&b_score).unwrap()
                })
                .unwrap();
            
            // Delete losers
            for memory in &conflict.memories {
                if memory.id != winner.id {
                    semantic_store.delete(&memory.id).await?;
                    report.resolved_count += 1;
                }
            }
        }
        
        Ok(report)
    }
}

/// Pruning report
#[derive(Debug, Default)]
pub struct PruningReport {
    pub semantic_pruned: usize,
    pub episodic_pruned: usize,
    pub procedural_pruned: usize,
}

/// Conflict resolution report
#[derive(Debug, Default)]
pub struct ConflictResolutionReport {
    pub resolved_count: usize,
}
```

---

### 2. Background Pruning Worker

**File:** `crates/openakta-memory/src/lifecycle.rs` (add to existing)

```rust
/// Background pruning worker
pub struct PruningWorker {
    lifecycle: MemoryLifecycle,
    check_interval: Duration,
}

impl PruningWorker {
    /// Create new pruning worker
    pub fn new(lifecycle: MemoryLifecycle, check_interval: Duration) -> Self {
        Self {
            lifecycle,
            check_interval,
        }
    }
    
    /// Run background pruning (async loop)
    pub async fn run(
        &self,
        semantic_store: SemanticStore,
        episodic_store: EpisodicStore,
        procedural_store: ProceduralStore,
    ) -> Result<()> {
        loop {
            // Prune memories
            let report = self.lifecycle.prune(
                &semantic_store,
                &episodic_store,
                &procedural_store,
            ).await?;
            
            // Resolve conflicts
            let conflict_report = self.lifecycle.resolve_conflicts(&semantic_store).await?;
            
            // Log report
            tracing::info!(
                "Pruning complete: {} episodic, {} procedural, {} conflicts resolved",
                report.episodic_pruned,
                report.procedural_pruned,
                conflict_report.resolved_count
            );
            
            // Wait for next check
            tokio::time::sleep(self.check_interval).await;
        }
    }
}
```

---

### 3. Ebbinghaus Decay Implementation

**File:** `crates/openakta-memory/src/lifecycle.rs` (add to existing)

```rust
impl EbbinghausDecay {
    /// Create new decay model
    pub fn new(half_life_days: f32) -> Self {
        Self { half_life_days }
    }
    
    /// Calculate exponential decay
    pub fn exponential_decay(&self, created_at: u64) -> f32 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_secs = now - created_at;
        let age_days = age_secs as f32 / (24.0 * 60.0 * 60.0);
        
        // Ebbinghaus Forgetting Curve: S = exp(-t / half_life)
        (-age_days / self.half_life_days).exp()
    }
    
    /// Calculate retrieval reinforcement
    pub fn retrieval_reinforcement(&self, retrieval_count: u32) -> f32 {
        // Logarithmic reinforcement (diminishing returns)
        (retrieval_count as f32 + 1.0).log10()
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-memory/src/lifecycle.rs` (NEW)

**Update:**
- None (new module)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-indexing/` (Agent B's domain)
- `crates/openakta-agents/` (Agent C's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_ebbinghaus_decay_calculation() { }

#[test]
fn test_memory_strength_calculation() { }

#[test]
fn test_utility_update() { }

#[test]
fn test_episodic_pruning() { }

#[test]
fn test_procedural_pruning() { }

#[test]
fn test_combined_threshold_pruning() { }

#[test]
fn test_conflict_resolution() { }

#[test]
fn test_time_decay_weighting() { }

#[test]
fn test_background_pruning_worker() { }

#[test]
fn test_end_to_end_lifecycle() { }
```

---

## ✅ Success Criteria

- [ ] `lifecycle.rs` created (decay + pruning)
- [ ] Ebbinghaus decay calculation works
- [ ] Memory strength calculation works
- [ ] Utility tracking works
- [ ] Episodic pruning works
- [ ] Procedural pruning works
- [ ] Conflict resolution works
- [ ] Background worker works
- [ ] 10+ tests passing
- [ ] Performance: <1s for full pruning cycle

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md) — Memory architecture
- Research document — R-14 Memory Lifecycle spec

---

**Start AFTER Sprint 30 (MemGAS Retrieval) is complete.**

**Priority: HIGH — prevents memory bloat, context rot.**

**Dependencies:**
- Sprint 26 (Semantic Memory) — must be complete
- Sprint 27 (Episodic Memory) — must be complete
- Sprint 28 (Procedural Memory) — must be complete

**Blocks:**
- None (lifecycle management, no downstream dependencies)

---

**This is the FINAL sprint for Memory Architecture (Phase 2 complete after this).**
