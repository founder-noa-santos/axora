# Agent A — Sprint 28: Procedural Memory Store (SKILL.md)

**Phase:** 2  
**Sprint:** 28 (Memory Architecture)  
**File:** `crates/openakta-memory/src/procedural_store.rs`  
**Priority:** CRITICAL (learned workflows, compounding expertise)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Procedural Memory Store** using file-system with SKILL.md format.

### Context

R-14 research provides CRITICAL implementation details:
- **Procedural Memory** — Learned workflows, heuristics, diagnostic patterns
- **File-System Storage** — SKILL.md format with YAML frontmatter
- **Trigger-Based Retrieval** — Exact match of task context triggers
- **Progressive Disclosure** — Load only required skills (insulate context window)

**Your job:** Implement procedural memory store (compounding expertise).

---

## 📋 Deliverables

### 1. Create procedural_store.rs

**File:** `crates/openakta-memory/src/procedural_store.rs`

**Core Structure:**
```rust
//! Procedural Memory Store
//!
//! This module implements procedural memory storage:
//! - File-system repository (SKILL.md format)
//! - Trigger-based retrieval
//! - Progressive disclosure (load only required skills)

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tokio::fs;

/// Skill entity (procedural memory)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// YAML frontmatter
    pub metadata: SkillMetadata,
    
    /// Procedural steps (markdown)
    pub steps: Vec<SkillStep>,
    
    /// Optional execution scripts
    pub scripts: Option<Vec<Script>>,
}

/// Skill metadata (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub skill_id: String,
    pub name: String,
    pub triggers: Vec<String>, // Natural language triggers
    pub domain: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub success_count: u32,
    pub failure_count: u32,
    pub utility_score: f32,
}

/// Skill step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStep {
    pub order: u32,
    pub description: String,
    pub command: Option<String>, // Optional terminal command
    pub validation: Option<String>, // Optional validation check
}

/// Optional script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub language: String,
    pub code: String,
}

/// Procedural memory store
pub struct ProceduralStore {
    skills_dir: PathBuf,
    staging_dir: PathBuf, // For skills awaiting validation
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
    
    /// Store skill (after validation)
    pub async fn store(&self, skill: Skill) -> Result<()> {
        let file_path = self.skills_dir.join(format!("{}.md", skill.metadata.skill_id));
        
        // Serialize to SKILL.md format
        let content = self.serialize_skill(&skill)?;
        
        // Write to file
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
                let skill = self.deserialize_skill(&content)?;
                
                // Check if any trigger matches task context
                if skill.metadata.triggers.iter().any(|trigger| {
                    task_context.contains(trigger)
                }) {
                    return Ok(Some(skill));
                }
            }
        }
        
        Ok(None) // No matching skill
    }
    
    /// Store skill in staging (awaiting validation)
    pub async fn store_staging(&self, skill: Skill) -> Result<()> {
        let file_path = self.staging_dir.join(format!("{}.md", skill.metadata.skill_id));
        
        let content = self.serialize_skill(&skill)?;
        fs::write(&file_path, content).await?;
        
        Ok(())
    }
    
    /// Promote skill from staging to active
    pub async fn promote_from_staging(&self, skill_id: &str) -> Result<()> {
        let staging_path = self.staging_dir.join(format!("{}.md", skill_id));
        let active_path = self.skills_dir.join(format!("{}.md", skill_id));
        
        fs::rename(&staging_path, &active_path).await?;
        
        Ok(())
    }
    
    /// Update skill utility score
    pub async fn update_utility(&self, skill_id: &str, outcome: SkillOutcome) -> Result<()> {
        // Load skill
        let skill_path = self.skills_dir.join(format!("{}.md", skill_id));
        let content = fs::read_to_string(&skill_path).await?;
        let mut skill = self.deserialize_skill(&content)?;
        
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
    
    /// Calculate utility score (success rate with decay)
    fn calculate_utility(&self, metadata: &SkillMetadata) -> f32 {
        let total = metadata.success_count + metadata.failure_count;
        if total == 0 {
            return 0.5; // Default score
        }
        
        let success_rate = metadata.success_count as f32 / total as f32;
        
        // Apply time decay (optional)
        let age_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - metadata.updated_at;
        
        let decay_factor = (-age_secs as f32 / (30 * 24 * 60 * 60) as f32).exp(); // 30-day half-life
        
        success_rate * decay_factor
    }
}

/// Skill outcome
#[derive(Debug, Clone)]
pub enum SkillOutcome {
    Success,
    Failure,
}
```

---

### 2. SKILL.md Format Specification

**Example File:** `skills/DEBUG_AUTH_FAILURE.md`

```markdown
---
skill_id: "DEBUG_AUTH_FAILURE"
name: "Debug Authentication Failure"
triggers:
  - "authentication failure"
  - "JWT validation failed"
  - "401 unauthorized"
domain: "security"
created_at: 1710604800
updated_at: 1710604800
success_count: 15
failure_count: 2
utility_score: 0.88
---

# Debug Authentication Failure

## Prerequisites
- Access to application logs
- Access to JWT secret key
- Understanding of JWT structure

## Steps

### Step 1: Extract JWT from Request
```bash
# Extract Authorization header
curl -v https://api.example.com/secure/endpoint 2>&1 | grep "Authorization:"
```

### Step 2: Decode JWT Payload
```bash
# Use jq to decode JWT payload
echo $JWT_PAYLOAD | base64 -d | jq .
```

### Step 3: Check Expiration Claim
```bash
# Check 'exp' claim
echo $JWT_PAYLOAD | jq '.exp'
# Compare with current timestamp
date +%s
```

### Step 4: Validate Signature
```bash
# Verify JWT signature
jwt verify --secret $JWT_SECRET $JWT_TOKEN
```

### Step 5: Check Common Issues
- Token expired → Request new token
- Signature mismatch → Verify secret key
- Missing claims → Check token generation logic

## Validation
- [ ] JWT successfully decoded
- [ ] Expiration claim valid
- [ ] Signature verified
- [ ] All required claims present

## Related Skills
- REFRESH_TOKEN
- GENERATE_JWT
```

---

### 3. Integrate with Task Decomposition

**File:** `crates/openakta-agents/src/coordinator.rs` (UPDATE)

```rust
// Add to existing Coordinator
impl Coordinator {
    /// Execute task with procedural memory (if skill matches)
    pub async fn execute_with_procedural_memory(
        &self,
        task: &Task,
        procedural_store: &ProceduralStore,
    ) -> Result<TaskResult> {
        // Query procedural memory for matching skill
        if let Some(skill) = procedural_store.retrieve_by_trigger(&task.description).await? {
            // Execute with procedural heuristic (bypass reasoning)
            tracing::info!("Executing task with learned skill: {}", skill.metadata.name);
            
            let result = self.execute_with_skill(task, &skill).await?;
            
            // Update skill utility
            let outcome = if result.success {
                SkillOutcome::Success
            } else {
                SkillOutcome::Failure
            };
            
            procedural_store.update_utility(&skill.metadata.skill_id, outcome).await?;
            
            Ok(result)
        } else {
            // No matching skill → Execute with reasoning from first principles
            tracing::info!("No matching skill, executing with reasoning");
            
            let result = self.execute_with_reasoning(task).await?;
            
            Ok(result)
        }
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-memory/src/procedural_store.rs` (NEW)
- `crates/openakta-memory/skills/` directory (for SKILL.md files)

**Update:**
- `crates/openakta-agents/src/coordinator.rs` (integrate procedural memory)

**DO NOT Edit:**
- `crates/openakta-cache/` (Agent B's domain)
- `crates/openakta-indexing/` (Agent B's domain)
- `crates/openakta-docs/` (Agent A's other work)

---

## 🧪 Tests Required

```rust
#[test]
fn test_skill_storage() { }

#[test]
fn test_trigger_matching() { }

#[test]
fn test_staging_promotion() { }

#[test]
fn test_utility_update() { }

#[test]
fn test_utility_calculation() { }

#[test]
fn test_skill_serialization() { }

#[test]
fn test_coordinator_integration() { }

#[test]
fn test_progressive_disclosure() { }
```

---

## ✅ Success Criteria

- [ ] `procedural_store.rs` created (SKILL.md file-system)
- [ ] SKILL.md format implemented (YAML frontmatter + steps)
- [ ] Trigger-based retrieval works
- [ ] Staging/promotion workflow works (human-in-the-loop)
- [ ] Utility score calculation works
- [ ] Coordinator integration works
- [ ] 8+ tests passing
- [ ] Performance: <50ms for trigger matching

---

## 🔗 References

- [`PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md`](../shared/PHASE-2-INTEGRATION-MEMORY-ARCHITECTURE.md) — Memory architecture
- Research document — R-14 Procedural Memory spec

---

**Start AFTER Sprint 27 (Episodic Memory Store) is complete.**

**Priority: CRITICAL — compounding expertise, 90% token reduction for repetitive tasks.**

**Dependencies:**
- Sprint 27 (Episodic Memory) — recommended but not required

**Blocks:**
- Sprint 29 (Consolidation Pipeline — needs procedural store)
