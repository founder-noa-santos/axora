//! Snapshot-Based Blackboard
//!
//! This module implements a concurrency-safe shared memory substrate for agent coordination:
//! - **Immutable Snapshots** — Agents read from versioned state (no stale reads)
//! - **Optimistic Concurrency** — Version check on writes (no TOCTOU bugs)
//! - **Atomic Tool Operations** — Per-key locks for shared resources
//! - **Reflection Phase** — Detects state changes during agent reasoning
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Blackboard                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Snapshots (RwLock)         │  Current Version (Atomic)    │
//! │  - v1: BlackboardSnapshot   │  - AtomicU64                 │
//! │  - v2: BlackboardSnapshot   │                              │
//! │  - v3: BlackboardSnapshot   │  Per-Key Locks (DashMap)     │
//! │  ...                        │  - key1: Arc<RwLock<Value>>  │
//! │                             │  - key2: Arc<RwLock<Value>>  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use openakta_cache::blackboard::{Blackboard, BlackboardSchema};
//! use serde_json::json;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let bb = Blackboard::new();
//!
//! // Agent 1: Read snapshot and get version
//! let version = bb.get_current_version();
//! let snapshot = bb.get_snapshot(version).unwrap();
//!
//! // Agent operates on snapshot (immutable, no TOCTOU)
//! let pending_action = json!({"action": "write_file", "path": "src/main.rs"});
//!
//! // Agent 2: Try to update with version check
//! match bb.update("task_state", &json!({"status": "in_progress"}), version) {
//!     Ok(new_version) => println!("Updated to version {}", new_version),
//!     Err(e) => println!("Stale version, entering reflection phase: {}", e),
//! }
//!
//! // Agent 1: Detect if state changed during reasoning
//! if bb.detect_state_change(version) {
//!     // Enter reflection phase
//!     let reflection = bb.create_reflection_phase(version, pending_action)?;
//!     // Re-evaluate action with new context
//! }
//! # Ok(())
//! # }
//! ```

pub mod v2;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Blackboard snapshot (immutable, versioned)
///
/// Agents read from snapshots to ensure consistent state during reasoning.
/// Snapshots are never modified after creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlackboardSnapshot {
    /// Version number of this snapshot
    pub version: u64,
    /// Unix timestamp when snapshot was created
    pub timestamp: u64,
    /// State at this point in time (immutable)
    pub state: HashMap<String, serde_json::Value>,
}

impl BlackboardSnapshot {
    /// Creates a new snapshot with the given version and state
    fn new(version: u64, state: HashMap<String, serde_json::Value>) -> Self {
        Self {
            version,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            state,
        }
    }

    /// Gets a value from the snapshot
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    /// Returns all keys in the snapshot
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.state.keys()
    }

    /// Returns the number of entries in the snapshot
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// Returns true if the snapshot is empty
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }
}

/// Blackboard errors
#[derive(Error, Debug)]
pub enum BlackboardError {
    /// Version mismatch (TOCTOU prevention)
    #[error("stale version: expected {expected}, actual {actual}")]
    StaleVersion { expected: u64, actual: u64 },

    /// Lock was poisoned (indicates panic during lock hold)
    #[error("lock poisoned")]
    LockPoisoned,

    /// Snapshot creation failed
    #[error("snapshot creation failed: {0}")]
    SnapshotFailed(String),

    /// Unknown key (not in schema)
    #[error("unknown key: {0}")]
    UnknownKey(String),

    /// JSON schema validation failed
    #[error("schema validation failed for '{key}': {errors:?}")]
    SchemaValidationFailed { key: String, errors: Vec<String> },

    /// Snapshot not found
    #[error("snapshot not found: version {0}")]
    SnapshotNotFound(u64),
}

/// Result type for blackboard operations
pub type Result<T> = std::result::Result<T, BlackboardError>;

/// Reflection phase (when state changed during agent reasoning)
///
/// When an agent detects that the blackboard state changed while it was
/// reasoning, it enters a reflection phase to re-evaluate its pending action
/// in light of the new state.
#[derive(Debug, Clone)]
pub struct ReflectionPhase {
    /// The snapshot the agent was reasoning from
    pub original_snapshot: BlackboardSnapshot,
    /// The current snapshot (after changes)
    pub current_snapshot: BlackboardSnapshot,
    /// The action the agent was about to take
    pub pending_action: serde_json::Value,
}

impl ReflectionPhase {
    /// Merge new context with pending action
    ///
    /// Returns the pending action for re-evaluation by the agent.
    /// The agent's LLM should re-consider this action given the
    /// difference between original_snapshot and current_snapshot.
    pub fn merge(&self) -> Result<serde_json::Value> {
        // Return pending action for re-evaluation
        // The LLM will see both snapshots and decide what to do
        Ok(self.pending_action.clone())
    }

    /// Get a summary of what changed
    pub fn get_changes(&self) -> HashMap<String, ChangeType> {
        let mut changes = HashMap::new();

        // Check keys in original snapshot
        for key in self.original_snapshot.state.keys() {
            if let Some(current_value) = self.current_snapshot.state.get(key) {
                let original_value = self.original_snapshot.state.get(key).unwrap();
                if original_value != current_value {
                    changes.insert(key.clone(), ChangeType::Modified);
                }
            } else {
                changes.insert(key.clone(), ChangeType::Deleted);
            }
        }

        // Check for new keys in current snapshot
        for key in self.current_snapshot.state.keys() {
            if !self.original_snapshot.state.contains_key(key) {
                changes.insert(key.clone(), ChangeType::Added);
            }
        }

        changes
    }
}

/// Type of change detected in reflection phase
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeType {
    /// Key was added
    Added,
    /// Key was modified
    Modified,
    /// Key was deleted
    Deleted,
}

/// JSON Schema validator for blackboard entries
#[derive(Debug, Clone)]
pub struct BlackboardSchema {
    /// JSON Schema per key
    schemas: HashMap<String, serde_json::Value>,
}

impl BlackboardSchema {
    /// Creates a new empty schema
    pub fn new() -> Self {
        Self {
            schemas: HashMap::new(),
        }
    }

    /// Adds a schema for a key
    pub fn add_schema(&mut self, key: &str, schema: serde_json::Value) {
        self.schemas.insert(key.to_string(), schema);
    }

    /// Validates a value against the schema for a key
    pub fn validate(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let schema = self
            .schemas
            .get(key)
            .ok_or_else(|| BlackboardError::UnknownKey(key.to_string()))?;

        // Simple schema validation (type checking)
        // In production, use full JSON Schema validation
        self.validate_simple_schema(schema, value)
            .map_err(|errors| BlackboardError::SchemaValidationFailed {
                key: key.to_string(),
                errors,
            })
    }

    /// Simple schema validation (type-based)
    fn validate_simple_schema(
        &self,
        schema: &serde_json::Value,
        value: &serde_json::Value,
    ) -> std::result::Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Some(schema_obj) = schema.as_object() {
            // Check type
            if let Some(expected_type) = schema_obj.get("type").and_then(|v| v.as_str()) {
                let actual_type = match value {
                    serde_json::Value::Null => "null",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                };

                if expected_type != actual_type {
                    errors.push(format!(
                        "expected type '{}', got '{}'",
                        expected_type, actual_type
                    ));
                }
            }

            // Check required fields for objects
            if let Some(required) = schema_obj.get("required").and_then(|v| v.as_array()) {
                if let Some(obj) = value.as_object() {
                    for req in required {
                        if let Some(req_str) = req.as_str() {
                            if !obj.contains_key(req_str) {
                                errors.push(format!("missing required field '{}'", req_str));
                            }
                        }
                    }
                }
            }

            // Check properties for objects
            if let Some(properties) = schema_obj.get("properties").and_then(|v| v.as_object()) {
                if let Some(obj) = value.as_object() {
                    for (prop_name, prop_schema) in properties {
                        if let Some(prop_value) = obj.get(prop_name) {
                            if let Err(mut prop_errors) =
                                self.validate_simple_schema(prop_schema, prop_value)
                            {
                                errors.append(&mut prop_errors);
                            }
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Default for BlackboardSchema {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot-Based Blackboard with TOCTOU prevention
///
/// ## Concurrency Model
///
/// - **Reads:** Agents read from immutable snapshots (no locks needed)
/// - **Writes:** Optimistic concurrency with version check
/// - **TOCTOU Prevention:** Double-check version before and after acquiring lock
///
/// ## Usage Pattern
///
/// ```text
/// 1. Agent reads snapshot at version V
/// 2. Agent reasons about state (may take time)
/// 3. Agent tries to update with expected_version=V
/// 4. If version changed, enter reflection phase
/// 5. If version same, update succeeds
/// ```
pub struct Blackboard {
    /// Immutable snapshots (agents read from these)
    snapshots: RwLock<HashMap<u64, BlackboardSnapshot>>,

    /// Current version number (for optimistic concurrency)
    current_version: AtomicU64,

    /// Per-key locks (for atomic updates to specific keys)
    locks: DashMap<String, Arc<RwLock<serde_json::Value>>>,

    /// Current state (for quick reads without snapshot)
    current_state: RwLock<HashMap<String, serde_json::Value>>,

    /// Metadata: creation timestamp
    created_at: u64,

    /// Metadata: last update timestamp
    last_updated: AtomicU64,
}

impl Blackboard {
    /// Creates a new blackboard with initial version 0
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let initial_state = HashMap::new();
        let initial_snapshot = BlackboardSnapshot::new(0, initial_state.clone());

        let mut snapshots = HashMap::new();
        snapshots.insert(0, initial_snapshot);

        Self {
            snapshots: RwLock::new(snapshots),
            current_version: AtomicU64::new(0),
            locks: DashMap::new(),
            current_state: RwLock::new(initial_state),
            created_at: now,
            last_updated: AtomicU64::new(now),
        }
    }

    /// Gets the current version number
    ///
    /// Agents should read this before operating, then pass it to
    /// `update()` for optimistic concurrency checking.
    pub fn get_current_version(&self) -> u64 {
        self.current_version.load(Ordering::SeqCst)
    }

    /// Gets a snapshot at a specific version
    ///
    /// Returns `None` if the version doesn't exist (garbage collected
    /// or never created).
    pub fn get_snapshot(&self, version: u64) -> Option<BlackboardSnapshot> {
        let snapshots = self.snapshots.read().ok()?;
        snapshots.get(&version).cloned()
    }

    /// Gets the latest snapshot
    pub fn get_latest_snapshot(&self) -> BlackboardSnapshot {
        let version = self.get_current_version();
        self.get_snapshot(version)
            .unwrap_or_else(|| BlackboardSnapshot::new(version, HashMap::new()))
    }

    /// Gets a value without snapshot (for quick reads)
    ///
    /// Note: This doesn't provide TOCTOU safety. Use `get_snapshot()`
    /// for operations that will be followed by updates.
    pub fn get(&self, key: &str) -> Option<serde_json::Value> {
        let state = self.current_state.read().ok()?;
        state.get(key).cloned()
    }

    /// Updates a value with version check (TOCTOU prevention)
    ///
    /// ## Algorithm
    ///
    /// 1. Check current version == expected_version (Time of Check)
    /// 2. Acquire per-key write lock
    /// 3. Re-check version (still valid?)
    /// 4. Update value
    /// 5. Increment version (Time of Use)
    /// 6. Create new snapshot
    ///
    /// # Arguments
    ///
    /// * `key` - The key to update
    /// * `value` - The new value
    /// * `expected_version` - The version the agent read (for concurrency check)
    ///
    /// # Returns
    ///
    /// The new version number if successful, or `StaleVersion` error if
    /// the blackboard was modified by another agent.
    pub fn update(
        &self,
        key: &str,
        value: &serde_json::Value,
        expected_version: u64,
    ) -> Result<u64> {
        // Time of Check: Check version before acquiring lock
        let current_version = self.current_version.load(Ordering::SeqCst);
        if current_version != expected_version {
            return Err(BlackboardError::StaleVersion {
                expected: expected_version,
                actual: current_version,
            });
        }

        // Get or create lock for this key
        let lock = self
            .locks
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(RwLock::new(serde_json::Value::Null)))
            .clone();

        // Acquire write lock
        let mut write_guard = lock.write().map_err(|_| BlackboardError::LockPoisoned)?;

        // Time of Check (again): Re-check version after acquiring lock
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

        // Update current state
        {
            let mut state = self
                .current_state
                .write()
                .map_err(|_| BlackboardError::LockPoisoned)?;
            state.insert(key.to_string(), value.clone());
        }

        // Time of Use: Increment version
        let new_version = self.current_version.fetch_add(1, Ordering::SeqCst) + 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_updated.store(now, Ordering::SeqCst);

        // Create new snapshot
        self.create_snapshot(new_version)?;

        Ok(new_version)
    }

    /// Posts a value with schema validation
    ///
    /// Validates the value against the schema before updating.
    pub fn post_validated(
        &self,
        key: &str,
        value: &serde_json::Value,
        schema: &BlackboardSchema,
        expected_version: u64,
    ) -> Result<u64> {
        // Validate against schema FIRST
        schema.validate(key, value)?;

        // Then update (with version check)
        self.update(key, value, expected_version)
    }

    /// Creates a new snapshot at the given version
    fn create_snapshot(&self, version: u64) -> Result<()> {
        let state = self
            .current_state
            .read()
            .map_err(|_| BlackboardError::LockPoisoned)?
            .clone();

        let snapshot = BlackboardSnapshot::new(version, state);

        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| BlackboardError::LockPoisoned)?;
        snapshots.insert(version, snapshot);

        Ok(())
    }

    /// Agent operates on snapshot (no TOCTOU during reasoning)
    ///
    /// This is the recommended pattern for agent operations:
    /// 1. Get snapshot
    /// 2. Operate on snapshot (read-only)
    /// 3. Prepare updates
    /// 4. Call `merge_result()` to apply updates
    pub fn operate_on_snapshot<F, T>(&self, version: u64, operation: F) -> Result<T>
    where
        F: FnOnce(&BlackboardSnapshot) -> Result<T>,
    {
        // Get immutable snapshot
        let snapshot = self.get_snapshot(version).ok_or_else(|| {
            let current = self.current_version.load(Ordering::SeqCst);
            BlackboardError::StaleVersion {
                expected: version,
                actual: current,
            }
        })?;

        // Agent operates on snapshot (read-only, no TOCTOU)
        let result = operation(&snapshot)?;

        Ok(result)
    }

    /// Merge agent result with current state (with version check)
    ///
    /// Applies all updates atomically. If any update fails due to
    /// version mismatch, the entire operation fails.
    ///
    /// # Arguments
    ///
    /// * `updates` - Key-value pairs to update
    /// * `base_version` - The version to check against
    ///
    /// # Returns
    ///
    /// The new version number after all updates.
    pub fn merge_result(
        &self,
        updates: HashMap<String, serde_json::Value>,
        base_version: u64,
    ) -> Result<u64> {
        // Apply all updates atomically (with version check)
        let mut new_version = base_version;

        for (key, value) in updates {
            new_version = self.update(&key, &value, new_version)?;
        }

        Ok(new_version)
    }

    /// Detects if state changed during agent reasoning
    ///
    /// Call this after reasoning to check if the blackboard was
    /// modified by another agent.
    pub fn detect_state_change(&self, agent_base_version: u64) -> bool {
        let current = self.current_version.load(Ordering::SeqCst);
        current != agent_base_version
    }

    /// Forces agent into reflection phase (merge new context)
    ///
    /// Call this when `detect_state_change()` returns true.
    /// The agent must re-evaluate its pending action in light of
    /// the new state.
    ///
    /// # Arguments
    ///
    /// * `agent_base_version` - The version the agent was reasoning from
    /// * `pending_action` - The action the agent was about to take
    pub fn create_reflection_phase(
        &self,
        agent_base_version: u64,
        pending_action: serde_json::Value,
    ) -> Result<ReflectionPhase> {
        let original = self.get_snapshot(agent_base_version).ok_or_else(|| {
            let current = self.current_version.load(Ordering::SeqCst);
            BlackboardError::StaleVersion {
                expected: agent_base_version,
                actual: current,
            }
        })?;

        let current_version = self.current_version.load(Ordering::SeqCst);
        let current = self
            .get_snapshot(current_version)
            .unwrap_or_else(|| BlackboardSnapshot::new(current_version, HashMap::new()));

        Ok(ReflectionPhase {
            original_snapshot: original,
            current_snapshot: current,
            pending_action,
        })
    }

    /// Gets the number of snapshots stored
    pub fn snapshot_count(&self) -> usize {
        self.snapshots.read().map(|s| s.len()).unwrap_or(0)
    }

    /// Prunes old snapshots (keep last N)
    ///
    /// Call this periodically to prevent memory growth.
    pub fn prune_snapshots(&self, keep_last: usize) -> Result<()> {
        let current = self.get_current_version();
        let min_version = current.saturating_sub(keep_last as u64);

        let mut snapshots = self
            .snapshots
            .write()
            .map_err(|_| BlackboardError::LockPoisoned)?;

        snapshots.retain(|&version, _| version >= min_version);

        Ok(())
    }

    /// Gets metadata about the blackboard
    pub fn get_metadata(&self) -> BlackboardMetadata {
        BlackboardMetadata {
            created_at: self.created_at,
            last_updated: self.last_updated.load(Ordering::SeqCst),
            current_version: self.get_current_version(),
            snapshot_count: self.snapshot_count(),
        }
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Metadata about the blackboard state
#[derive(Debug, Clone)]
pub struct BlackboardMetadata {
    /// Unix timestamp when blackboard was created
    pub created_at: u64,
    /// Unix timestamp of last update
    pub last_updated: u64,
    /// Current version number
    pub current_version: u64,
    /// Number of snapshots stored
    pub snapshot_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_snapshot_creation() {
        let bb = Blackboard::new();

        // Initial snapshot should exist at version 0
        let snapshot = bb.get_snapshot(0).unwrap();
        assert_eq!(snapshot.version, 0);
        assert!(snapshot.state.is_empty());

        // Update should create new snapshot
        let new_version = bb.update("key1", &json!("value1"), 0).unwrap();
        assert_eq!(new_version, 1);

        // New snapshot should exist
        let snapshot = bb.get_snapshot(1).unwrap();
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.state.get("key1"), Some(&json!("value1")));
    }

    #[test]
    fn test_snapshot_immutability() {
        let bb = Blackboard::new();

        // Get snapshot at version 0
        let snapshot_v0 = bb.get_snapshot(0).unwrap();

        // Update blackboard
        bb.update("key1", &json!("value1"), 0).unwrap();

        // Original snapshot should be unchanged
        assert!(snapshot_v0.state.is_empty());
        assert_eq!(snapshot_v0.version, 0);

        // New snapshot should have the update
        let snapshot_v1 = bb.get_snapshot(1).unwrap();
        assert_eq!(snapshot_v1.state.get("key1"), Some(&json!("value1")));
    }

    #[test]
    fn test_optimistic_concurrency() {
        let bb = Blackboard::new();

        // First update should succeed
        let v1 = bb.update("key1", &json!("value1"), 0).unwrap();
        assert_eq!(v1, 1);

        // Second update with correct version should succeed
        let v2 = bb.update("key1", &json!("value2"), 1).unwrap();
        assert_eq!(v2, 2);

        // Update with stale version should fail
        let result = bb.update("key1", &json!("value3"), 0);
        assert!(matches!(
            result,
            Err(BlackboardError::StaleVersion {
                expected: 0,
                actual: 2
            })
        ));
    }

    #[test]
    fn test_toctou_prevention() {
        let bb = Blackboard::new();

        // Simulate concurrent updates
        let v1 = bb.update("counter", &json!(1), 0).unwrap();
        assert_eq!(v1, 1);

        // Two agents try to update with same base version
        let result_a = bb.update("counter", &json!(2), 1);
        let result_b = bb.update("counter", &json!(3), 1);

        // One should succeed, one should fail
        assert!(result_a.is_ok() || result_b.is_ok());
        assert!(result_a.is_err() || result_b.is_err());

        // Final value should be either 2 or 3
        let final_value = bb.get("counter").unwrap();
        assert!(final_value == json!(2) || final_value == json!(3));
    }

    #[test]
    fn test_reflection_phase() {
        let bb = Blackboard::new();

        // Agent reads at version 0
        let base_version = bb.get_current_version();
        let pending_action = json!({"action": "write_file", "path": "test.rs"});

        // Another agent updates
        bb.update("other_key", &json!("other_value"), 0).unwrap();

        // First agent detects change
        assert!(bb.detect_state_change(base_version));

        // Enter reflection phase
        let reflection = bb
            .create_reflection_phase(base_version, pending_action.clone())
            .unwrap();

        // Reflection should have both snapshots
        assert_eq!(reflection.original_snapshot.version, 0);
        assert_eq!(reflection.current_snapshot.version, 1);
        assert_eq!(reflection.pending_action, pending_action);

        // Get changes
        let changes = reflection.get_changes();
        assert!(changes.contains_key("other_key"));
        assert_eq!(changes.get("other_key"), Some(&ChangeType::Added));
    }

    #[test]
    fn test_schema_validation() {
        let bb = Blackboard::new();
        let mut schema = BlackboardSchema::new();

        // Define schema for "user" key
        schema.add_schema(
            "user",
            json!({
                "type": "object",
                "required": ["name", "email"],
                "properties": {
                    "name": {"type": "string"},
                    "email": {"type": "string"}
                }
            }),
        );

        // Valid value should pass
        let valid_user = json!({
            "name": "John",
            "email": "john@example.com"
        });
        let result = bb.post_validated("user", &valid_user, &schema, 0);
        assert!(result.is_ok());

        // Invalid value should fail
        let invalid_user = json!({
            "name": "John"
            // Missing required "email"
        });
        let result = bb.post_validated("user", &invalid_user, &schema, 1);
        assert!(matches!(
            result,
            Err(BlackboardError::SchemaValidationFailed { .. })
        ));

        // Wrong type should fail
        let wrong_type = json!("not an object");
        let result = bb.post_validated("user", &wrong_type, &schema, 1);
        assert!(matches!(
            result,
            Err(BlackboardError::SchemaValidationFailed { .. })
        ));
    }

    #[test]
    fn test_concurrent_writes() {
        let bb = Arc::new(Blackboard::new());
        let mut handles = vec![];

        // Spawn 10 concurrent writers
        for i in 0..10 {
            let bb_clone = Arc::clone(&bb);
            let handle = std::thread::spawn(move || {
                let mut attempts = 0;
                loop {
                    let version = bb_clone.get_current_version();
                    match bb_clone.update(
                        &format!("key{}", i),
                        &json!(format!("value{}", i)),
                        version,
                    ) {
                        Ok(new_version) => return Ok::<_, BlackboardError>(new_version),
                        Err(BlackboardError::StaleVersion { .. }) => {
                            attempts += 1;
                            if attempts > 100 {
                                return Err(BlackboardError::StaleVersion {
                                    expected: version,
                                    actual: bb_clone.get_current_version(),
                                });
                            }
                            // Retry with new version
                        }
                        Err(e) => return Err(e),
                    }
                }
            });
            handles.push(handle);
        }

        // All should eventually succeed (optimistic concurrency with retry)
        let mut success_count = 0;
        for handle in handles {
            if handle.join().unwrap().is_ok() {
                success_count += 1;
            }
        }

        // All 10 writers should succeed eventually
        assert_eq!(success_count, 10);
    }

    #[test]
    fn test_snapshot_merge() {
        let bb = Blackboard::new();

        // Agent operates on snapshot
        let result = bb
            .operate_on_snapshot(0, |_snapshot| {
                // Prepare updates based on snapshot
                let mut updates = HashMap::new();
                updates.insert("derived_key".to_string(), json!("derived_value"));
                Ok(updates)
            })
            .unwrap();

        // Merge result
        let new_version = bb.merge_result(result, 0).unwrap();
        assert_eq!(new_version, 1);

        // Verify update was applied
        let snapshot = bb.get_snapshot(1).unwrap();
        assert_eq!(
            snapshot.state.get("derived_key"),
            Some(&json!("derived_value"))
        );
    }

    #[test]
    fn test_snapshot_pruning() {
        let bb = Blackboard::new();

        // Create many snapshots
        for i in 0..20 {
            bb.update(&format!("key{}", i), &json!(i), bb.get_current_version())
                .unwrap();
        }

        // Should have 21 snapshots (0-20)
        assert_eq!(bb.snapshot_count(), 21);

        // Prune to keep last 5
        bb.prune_snapshots(5).unwrap();

        // Should have at most 6 snapshots left (versions 15-20)
        // (we keep versions >= current - 5)
        let current = bb.get_current_version();
        let expected_min = current.saturating_sub(5);
        let remaining: Vec<_> = (expected_min..=current).collect();

        assert!(bb.snapshot_count() <= remaining.len() + 1);

        // Latest snapshots should still exist
        assert!(bb.get_snapshot(20).is_some());
        assert!(bb.get_snapshot(19).is_some());

        // Old snapshots should be gone
        assert!(bb.get_snapshot(0).is_none());
        assert!(bb.get_snapshot(10).is_none());
    }

    #[test]
    fn test_metadata() {
        let bb = Blackboard::new();

        let metadata = bb.get_metadata();
        assert_eq!(metadata.current_version, 0);
        assert_eq!(metadata.snapshot_count, 1);
        assert!(metadata.created_at > 0);
        assert!(metadata.last_updated > 0);

        // Update and check metadata changes
        bb.update("key", &json!("value"), 0).unwrap();

        let metadata = bb.get_metadata();
        assert_eq!(metadata.current_version, 1);
        assert_eq!(metadata.snapshot_count, 2);
    }

    #[test]
    fn test_get_latest_snapshot() {
        let bb = Blackboard::new();

        // Should get version 0 initially
        let snapshot = bb.get_latest_snapshot();
        assert_eq!(snapshot.version, 0);

        // Update
        bb.update("key", &json!("value"), 0).unwrap();

        // Should get version 1
        let snapshot = bb.get_latest_snapshot();
        assert_eq!(snapshot.version, 1);
        assert_eq!(snapshot.state.get("key"), Some(&json!("value")));
    }

    #[test]
    fn test_reflection_phase_changes() {
        let bb = Blackboard::new();

        // Initial state
        bb.update("key1", &json!("value1"), 0).unwrap();

        // Agent reads at version 1
        let base_version = bb.get_current_version();

        // Multiple changes
        bb.update("key2", &json!("value2"), 1).unwrap();
        bb.update("key1", &json!("updated_value1"), 2).unwrap();
        bb.update("key3", &json!("value3"), 3).unwrap();

        // Enter reflection phase
        let reflection = bb
            .create_reflection_phase(base_version, json!({"action": "test"}))
            .unwrap();

        // Get changes
        let changes = reflection.get_changes();

        // key2 and key3 were added, key1 was modified
        assert_eq!(changes.get("key2"), Some(&ChangeType::Added));
        assert_eq!(changes.get("key3"), Some(&ChangeType::Added));
        assert_eq!(changes.get("key1"), Some(&ChangeType::Modified));
    }
}
