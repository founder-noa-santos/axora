# Agent C — Sprint 29: Consolidation Pipeline (Episodic → Procedural)

**Phase:** 2  
**Sprint:** 29 (Memory Architecture)  
**File:** `crates/openakta-memory/src/consolidation.rs`  
**Priority:** CRITICAL (enables learning from experience)  
**Estimated Tokens:** ~120K output  

---

## 🎯 Task

Implement **Consolidation Pipeline** for converting episodic memories into procedural skills.

### Context

R-14 research provides CRITICAL implementation details:
- **Consolidation** — Episodic → Procedural (experience replay)
- **Trigger Conditions** — Success-based, failure-based, frequency-based
- **Multi-Faceted Distillation** — Lightweight LLM (Haiku) analyzes patterns
- **Human-in-the-Loop Validation** — Review Mode before deployment

**Your job:** Implement consolidation pipeline (enables compounding expertise).

---

## 📋 Deliverables

### 1. Create consolidation.rs

**File:** `crates/openakta-memory/src/consolidation.rs`

**Core Structure:**
```rust
//! Consolidation Pipeline
//!
//! This module implements episodic → procedural consolidation:
//! - Trigger conditions (success, failure, frequency)
//! - Trajectory extraction
//! - Multi-faceted distillation (Haiku)
//! - Human-in-the-loop validation

use crate::episodic_store::{EpisodicStore, EpisodicMemory};
use crate::procedural_store::{ProceduralStore, Skill, SkillMetadata, SkillStep};

/// Consolidation pipeline
pub struct ConsolidationPipeline {
    episodic_store: EpisodicStore,
    procedural_store: ProceduralStore,
    distillation_model: LightweightLLM, // Claude Haiku
}

/// Consolidation trigger
#[derive(Debug, Clone)]
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

impl ConsolidationPipeline {
    /// Create new consolidation pipeline
    pub fn new(
        episodic_store: EpisodicStore,
        procedural_store: ProceduralStore,
        distillation_model: LightweightLLM,
    ) -> Self {
        Self {
            episodic_store,
            procedural_store,
            distillation_model,
        }
    }
    
    /// Check for consolidation triggers
    pub async fn check_triggers(&self, session_id: &str) -> Result<Option<ConsolidationTrigger>> {
        // Retrieve session trajectory
        let trajectory = self.episodic_store.retrieve_trajectory(session_id).await?;
        
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
    
    /// Extract trajectory (filter noise)
    pub fn extract_trajectory(&self, memories: &[EpisodicMemory]) -> Vec<ObservationActionPair> {
        memories
            .iter()
            .filter(|m| {
                // Filter out pleasantries, failed loops, redundant outputs
                matches!(m.memory_type, MemoryType::ToolExecution | MemoryType::SuccessState | MemoryType::FailureState)
            })
            .map(|m| ObservationActionPair {
                observation: m.content.clone(),
                success: m.success,
            })
            .collect()
    }
    
    /// Distill trajectory into procedural skill
    pub async fn distill(&self, trajectory: &[ObservationActionPair]) -> Result<Skill> {
        // Use lightweight LLM (Haiku) for distillation
        let prompt = self.build_distillation_prompt(trajectory);
        
        let response = self.distillation_model.generate(&prompt).await?;
        
        // Parse response into Skill
        let skill = self.parse_skill_from_response(&response)?;
        
        Ok(skill)
    }
    
    /// Validate skill (human-in-the-loop or teacher-student)
    pub async fn validate(&self, skill: &Skill, mode: ValidationMode) -> Result<bool> {
        match mode {
            ValidationMode::Review => {
                // Store in staging, await human approval
                self.procedural_store.store_staging(skill.clone()).await?;
                
                // Generate validation report
                let report = self.generate_validation_report(skill);
                
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
}

/// Validation mode
#[derive(Debug, Clone)]
pub enum ValidationMode {
    Review, // Human-in-the-loop
    TeacherStudent, // Automated verification
}
```

---

### 2. Background Consolidation Worker

**File:** `crates/openakta-memory/src/consolidation.rs` (add to existing)

```rust
/// Background consolidation worker
pub struct ConsolidationWorker {
    pipeline: ConsolidationPipeline,
    check_interval: Duration,
}

impl ConsolidationWorker {
    /// Create new worker
    pub fn new(pipeline: ConsolidationPipeline, check_interval: Duration) -> Self {
        Self {
            pipeline,
            check_interval,
        }
    }
    
    /// Run background consolidation (async loop)
    pub async fn run(&self) -> Result<()> {
        loop {
            // Get recent sessions
            let recent_sessions = self.get_recent_sessions().await?;
            
            for session_id in recent_sessions {
                // Check for triggers
                if let Some(trigger) = self.pipeline.check_triggers(&session_id).await? {
                    // Extract trajectory
                    let trajectory = self.episodic_store.retrieve_trajectory(&session_id).await?;
                    let filtered = self.pipeline.extract_trajectory(&trajectory);
                    
                    // Distill into skill
                    let skill = self.pipeline.distill(&filtered).await?;
                    
                    // Validate (Review Mode by default)
                    self.pipeline.validate(&skill, ValidationMode::Review).await?;
                }
            }
            
            // Wait for next check
            tokio::time::sleep(self.check_interval).await;
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-memory/src/consolidation.rs` (NEW)

**Update:**
- None (new module)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-indexing/` (Agent B's domain)
- `crates/openakta-docs/` (Agent A's other work)

---

## 🧪 Tests Required

```rust
#[test]
fn test_success_trigger_detection() { }

#[test]
fn test_failure_trigger_detection() { }

#[test]
fn test_frequency_trigger_detection() { }

#[test]
fn test_trajectory_extraction() { }

#[test]
fn test_distillation_prompt() { }

#[test]
fn test_skill_parsing() { }

#[test]
fn test_validation_review_mode() { }

#[test]
fn test_validation_teacher_student() { }

#[test]
fn test_background_worker() { }

#[test]
fn test_end_to_end_consolidation() { }
```

---

## ✅ Success Criteria

- [ ] `consolidation.rs` created (episodic → procedural)
- [ ] Trigger detection works (success, failure, frequency)
- [ ] Trajectory extraction works (filter noise)
- [ ] Distillation prompt works (Haiku integration)
- [ ] Skill parsing works
- [ ] Validation modes work (Review + Teacher-Student)
- [ ] Background worker works
- [ ] 10+ tests passing
- [ ] Performance: <5s for full consolidation cycle

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md) — Memory architecture
- Research document — R-14 Consolidation Algorithm spec

---

**⚠️ DEPENDENCIES:** This sprint requires **A-27 (Episodic Memory)** and **A-28 (Procedural Memory)** to be complete first.

**Start AFTER Agent A completes Sprints 27 and 28.**

**Priority: CRITICAL — enables true learning from experience.**

**Dependencies:**
- A-27 (Episodic Memory Store) — must be complete
- A-28 (Procedural Memory Store) — must be complete

**Blocks:**
- C-30 (MemGAS Retrieval — needs consolidation for utility tracking)
