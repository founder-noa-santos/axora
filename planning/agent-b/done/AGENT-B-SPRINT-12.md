# Agent B — Sprint 12: Snapshot Blackboard Implementation

**Phase:** 2  
**Sprint:** 12 (Implementation)  
**File:** `crates/openakta-cache/src/blackboard.rs`  
**Priority:** HIGH (blocks Agent C's ReAct implementation)  
**Estimated Tokens:** ~100K output  

---

## 🎯 Task

Implement **Snapshot-Based Blackboard** with TOCTOU prevention (from concurrent task decomposition research).

### Context

Research validates our Blackboard pattern and adds CRITICAL safety features:
- **Immutable Snapshots** — Agents read from versioned snapshots (no stale reads)
- **Optimistic Concurrency** — Version check on writes (no TOCTOU)
- **Atomic Tool Operations** — Locks for shared resources

**Your job:** Implement this blackboard so Agent C can use it for ReAct loops.

---

## 📋 Deliverables

### 1. Create blackboard.rs

**File:** `crates/openakta-cache/src/blackboard.rs`

**Core Structure:**
```rust
//! Snapshot-Based Blackboard
//!
//! This module implements a concurrency-safe shared memory substrate:
//! - Immutable snapshots (agents read from versioned state)
//! - Optimistic concurrency (version check on writes)
//! - TOCTOU prevention (no stale data)

use dashmap::DashMap;
use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Deserialize, Serialize};

/// Blackboard snapshot (immutable, versioned)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardSnapshot {
    pub version: u64,
    pub timestamp: u64,
    pub state: HashMap<String, serde_json::Value>,
}

/// Blackboard with snapshot-based merging
pub struct Blackboard {
    // Immutable snapshots (agents read from these)
    snapshots: RwLock<HashMap<u64, BlackboardSnapshot>>,
    
    // Current version (for writes)
    current_version: AtomicU64,
    
    // Per-key locks (for atomic updates)
    locks: DashMap<String, Arc<RwLock<serde_json::Value>>>,
    
    // Metadata
    created_at: u64,
    last_updated: AtomicU64,
}

impl Blackboard {
    /// Create new blackboard
    pub fn new() -> Self;
    
    /// Get snapshot at version (immutable read)
    pub fn get_snapshot(&self, version: u64) -> Option<BlackboardSnapshot>;
    
    /// Get latest version number (for optimistic concurrency)
    pub fn get_current_version(&self) -> u64;
    
    /// Update value with version check (TOCTOU prevention)
    pub fn update(
        &self,
        key: &str,
        value: &serde_json::Value,
        expected_version: u64,
    ) -> Result<u64, BlackboardError>;
    
    /// Create new snapshot (called after updates)
    fn create_snapshot(&self, version: u64) -> Result<BlackboardSnapshot>;
    
    /// Get value without snapshot (for quick reads)
    pub fn get(&self, key: &str) -> Option<serde_json::Value>;
}
```

---

### 2. Implement TOCTOU Prevention

**File:** `crates/openakta-cache/src/blackboard.rs` (add to existing)

```rust
/// TOCTOU-safe read-modify-write
impl Blackboard {
    /// Atomic update with version check
    pub fn update(
        &self,
        key: &str,
        value: &serde_json::Value,
        expected_version: u64,
    ) -> Result<u64, BlackboardError> {
        // Check version (Time of Check)
        let current_version = self.current_version.load(Ordering::SeqCst);
        if current_version != expected_version {
            return Err(BlackboardError::StaleVersion {
                expected: expected_version,
                actual: current_version,
            });
        }
        
        // Get lock for this key
        let lock = self.locks
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(serde_json::Value::Null)))
            .clone();
        
        // Acquire write lock
        let mut write_guard = lock.write().map_err(|_| BlackboardError::LockPoisoned)?;
        
        // Re-check version (still valid?)
        let new_current = self.current_version.load(Ordering::SeqCst);
        if new_current != expected_version {
            return Err(BlackboardError::StaleVersion {
                expected: expected_version,
                actual: new_current,
            });
        }
        
        // Update value
        *write_guard = value.clone();
        drop(write_guard); // Release lock
        
        // Increment version (Time of Use)
        let new_version = self.current_version.fetch_add(1, Ordering::SeqCst) + 1;
        self.last_updated.store(new_version, Ordering::SeqCst);
        
        // Create new snapshot
        self.create_snapshot(new_version)?;
        
        Ok(new_version)
    }
}

/// Blackboard errors
#[derive(Error, Debug)]
pub enum BlackboardError {
    #[error("stale version: expected {expected}, actual {actual}")]
    StaleVersion { expected: u64, actual: u64 },
    
    #[error("lock poisoned")]
    LockPoisoned,
    
    #[error("snapshot creation failed: {0}")]
    SnapshotFailed(String),
}
```

---

### 3. Add Snapshot-Based Merging

**File:** `crates/openakta-cache/src/blackboard.rs` (add to existing)

```rust
impl Blackboard {
    /// Agent operates on snapshot (no TOCTOU during reasoning)
    pub fn operate_on_snapshot<F, T>(&self, version: u64, operation: F) -> Result<T, BlackboardError>
    where
        F: FnOnce(&BlackboardSnapshot) -> Result<T, BlackboardError>,
    {
        // Get immutable snapshot
        let snapshot = self.get_snapshot(version)
            .ok_or_else(|| BlackboardError::StaleVersion {
                expected: version,
                actual: self.current_version.load(Ordering::SeqCst),
            })?;
        
        // Agent operates on snapshot (read-only, no TOCTOU)
        let result = operation(&snapshot)?;
        
        Ok(result)
    }
    
    /// Merge agent result with current state (with version check)
    pub fn merge_result(
        &self,
        updates: HashMap<String, serde_json::Value>,
        base_version: u64,
    ) -> Result<u64, BlackboardError> {
        // Apply all updates atomically (with version check)
        let mut new_version = base_version;
        
        for (key, value) in updates {
            new_version = self.update(&key, &value, new_version)?;
        }
        
        Ok(new_version)
    }
}
```

---

### 4. Add Reflection Phase (For Stale Detection)

**File:** `crates/openakta-cache/src/blackboard.rs` (add to existing)

```rust
/// Reflection phase (when state changed during agent reasoning)
pub struct ReflectionPhase {
    pub original_snapshot: BlackboardSnapshot,
    pub current_snapshot: BlackboardSnapshot,
    pub pending_action: serde_json::Value,
}

impl Blackboard {
    /// Detect if state changed during agent reasoning
    pub fn detect_state_change(&self, agent_base_version: u64) -> bool {
        let current = self.current_version.load(Ordering::SeqCst);
        current != agent_base_version
    }
    
    /// Force agent into reflection phase (merge new context)
    pub fn create_reflection_phase(
        &self,
        agent_base_version: u64,
        pending_action: serde_json::Value,
    ) -> Result<ReflectionPhase, BlackboardError> {
        let original = self.get_snapshot(agent_base_version)
            .ok_or_else(|| BlackboardError::StaleVersion {
                expected: agent_base_version,
                actual: self.current_version.load(Ordering::SeqCst),
            })?;
        
        let current_version = self.current_version.load(Ordering::SeqCst);
        let current = self.get_snapshot(current_version).unwrap();
        
        Ok(ReflectionPhase {
            original_snapshot: original,
            current_snapshot: current,
            pending_action,
        })
    }
}

impl ReflectionPhase {
    /// Merge new context with pending action
    pub fn merge(&self) -> Result<serde_json::Value, BlackboardError> {
        // Agent must re-evaluate pending action in light of new state
        // This is where the LLM "reflects" on what changed
        
        // For now, just return pending action (LLM will re-evaluate)
        Ok(self.pending_action.clone())
    }
}
```

---

### 5. Add Strict JSON Schema Validation

**File:** `crates/openakta-cache/src/blackboard.rs` (add to existing)

```rust
/// Schema-validated post (prevents logical corruption)
pub struct BlackboardSchema {
    schemas: HashMap<String, serde_json::Value>, // JSON Schema per key
}

impl BlackboardSchema {
    pub fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), BlackboardError> {
        let schema = self.schemas.get(key)
            .ok_or_else(|| BlackboardError::UnknownKey(key.to_string()))?;
        
        // Validate against JSON Schema
        let is_valid = jsonschema::is_valid(schema, value);
        
        if !is_valid {
            return Err(BlackboardError::SchemaValidationFailed {
                key: key.to_string(),
                errors: jsonschema::validate(schema, value)
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect(),
            });
        }
        
        Ok(())
    }
}

impl Blackboard {
    /// Post with schema validation
    pub fn post_validated(
        &self,
        key: &str,
        value: &serde_json::Value,
        schema: &BlackboardSchema,
        expected_version: u64,
    ) -> Result<u64, BlackboardError> {
        // Validate against schema FIRST
        schema.validate(key, value)?;
        
        // Then update (with version check)
        self.update(key, value, expected_version)
    }
}
```

---

## 📁 File Boundaries

**Create:**
- `crates/openakta-cache/src/blackboard.rs` (NEW)

**Update:**
- `crates/openakta-cache/src/lib.rs` (add module export)

**DO NOT Edit:**
- `crates/openakta-agents/` (Agent C's domain)
- `crates/openakta-docs/` (Agent A's domain)

---

## 🧪 Tests Required

```rust
#[test]
fn test_snapshot_creation() { }

#[test]
fn test_snapshot_immutability() { }

#[test]
fn test_optimistic_concurrency() { }

#[test]
fn test_toctou_prevention() { }

#[test]
fn test_reflection_phase() { }

#[test]
fn test_schema_validation() { }

#[test]
fn test_concurrent_writes() { }

#[test]
fn test_snapshot_merge() { }
```

---

## ✅ Success Criteria

- [ ] `blackboard.rs` created (immutable snapshots)
- [ ] Optimistic concurrency implemented (version check)
- [ ] TOCTOU prevention works (stale version detection)
- [ ] Reflection phase implemented (merge new context)
- [ ] JSON schema validation works
- [ ] 8+ tests passing
- [ ] Concurrent writes are safe (no race conditions)

---

## 🔗 References

- [`PHASE-2-INTEGRATION-REACT-PATTERNS.md`](../shared/PHASE-2-INTEGRATION-REACT-PATTERNS.md) — Integration doc
- [`PHASE-2-PIVOT-GRAPH-WORKFLOW.md`](../shared/PHASE-2-PIVOT-GRAPH-WORKFLOW.md) — Graph pivot
- Research document — Snapshot-based merging spec

---

**Start NOW. Agent C needs this for ReAct loops.**

**Priority: HIGH — this is on the critical path.**
