//! Versioned state management for Blackboard v2.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Result type for Blackboard v2 versioning operations.
pub type Result<T> = std::result::Result<T, VersionedContextError>;

/// A value stored with version metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VersionedValue {
    /// Stored JSON payload.
    pub value: Value,
    /// Global blackboard version at which this value was last updated.
    pub version: u64,
    /// Unix timestamp for the last write.
    pub timestamp: u64,
}

/// A single versioned change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Update {
    /// Key that changed.
    pub key: String,
    /// Prior value, if one existed.
    pub old_value: Option<Value>,
    /// New full value after the change.
    pub new_value: Value,
    /// Compact diff payload for subscribers.
    pub diff: Value,
    /// Global version assigned to this change.
    pub version: u64,
    /// Unix timestamp for the change.
    pub timestamp: u64,
    /// Serialized size of the full value.
    pub full_size_bytes: usize,
    /// Serialized size of the compact diff payload.
    pub diff_size_bytes: usize,
}

impl Update {
    /// Returns the fraction of bytes saved by sending the diff instead of the full value.
    pub fn size_reduction(&self) -> f32 {
        if self.full_size_bytes == 0 {
            0.0
        } else {
            (self.full_size_bytes.saturating_sub(self.diff_size_bytes) as f32)
                / self.full_size_bytes as f32
        }
    }
}

/// Errors returned by [`VersionedContext`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VersionedContextError {
    /// The caller tried to update from a stale base version.
    #[error("stale version: expected {expected}, actual {actual}")]
    StaleVersion { expected: u64, actual: u64 },

    /// Internal history lock could not be acquired.
    #[error("history lock poisoned")]
    HistoryLockPoisoned,

    /// Internal update lock could not be acquired.
    #[error("update lock poisoned")]
    UpdateLockPoisoned,
}

/// Version-tracked concurrent state for Blackboard v2.
#[derive(Debug, Default)]
pub struct VersionedContext {
    state: DashMap<String, VersionedValue>,
    current_version: AtomicU64,
    history: RwLock<Vec<Update>>,
    update_lock: Mutex<()>,
}

impl VersionedContext {
    /// Creates an empty versioned context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the current global version.
    pub fn version(&self) -> u64 {
        self.current_version.load(Ordering::SeqCst)
    }

    /// Reads a value by key.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.state.get(key).map(|entry| entry.value.clone())
    }

    /// Reads the full versioned value for a key.
    pub fn get_versioned(&self, key: &str) -> Option<VersionedValue> {
        self.state.get(key).map(|entry| entry.clone())
    }

    /// Performs an optimistic update guarded by the expected version.
    ///
    /// Callers read [`Self::version`] first, compute a new value, and then apply
    /// it with that version. If another writer updated the context in between,
    /// this returns [`VersionedContextError::StaleVersion`].
    pub fn update_with_version(
        &self,
        key: &str,
        expected_version: u64,
        new_value: Value,
    ) -> Result<Update> {
        let _guard = self
            .update_lock
            .lock()
            .map_err(|_| VersionedContextError::UpdateLockPoisoned)?;

        let actual_version = self.version();
        if actual_version != expected_version {
            return Err(VersionedContextError::StaleVersion {
                expected: expected_version,
                actual: actual_version,
            });
        }

        let old_value = self.state.get(key).map(|entry| entry.value.clone());
        let diff = compute_diff(old_value.as_ref(), &new_value);
        let next_version = expected_version + 1;
        let timestamp = unix_timestamp();
        let full_size_bytes = serialized_size(&new_value);
        let diff_size_bytes = serialized_size(&diff);

        self.state.insert(
            key.to_string(),
            VersionedValue {
                value: new_value.clone(),
                version: next_version,
                timestamp,
            },
        );

        let update = Update {
            key: key.to_string(),
            old_value,
            new_value,
            diff,
            version: next_version,
            timestamp,
            full_size_bytes,
            diff_size_bytes,
        };

        self.history
            .write()
            .map_err(|_| VersionedContextError::HistoryLockPoisoned)?
            .push(update.clone());
        self.current_version.store(next_version, Ordering::SeqCst);

        Ok(update)
    }

    /// Returns all updates after the provided version.
    pub fn get_since_version(&self, since_version: u64) -> Result<Vec<Update>> {
        let history = self
            .history
            .read()
            .map_err(|_| VersionedContextError::HistoryLockPoisoned)?;

        Ok(history
            .iter()
            .filter(|update| update.version > since_version)
            .cloned()
            .collect())
    }
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn serialized_size(value: &Value) -> usize {
    serde_json::to_vec(value)
        .map(|bytes| bytes.len())
        .unwrap_or(0)
}

fn compute_diff(old_value: Option<&Value>, new_value: &Value) -> Value {
    match old_value {
        None => new_value.clone(),
        Some(old_value) if old_value == new_value => Value::Object(Map::new()),
        Some(Value::Object(old_map)) if new_value.is_object() => {
            let new_map = new_value.as_object().expect("checked is_object");
            let mut diff = Map::new();

            for (key, next_value) in new_map {
                match old_map.get(key) {
                    Some(previous_value) => {
                        let nested_diff = compute_diff(Some(previous_value), next_value);
                        if !is_empty_diff(&nested_diff) {
                            diff.insert(key.clone(), nested_diff);
                        }
                    }
                    None => {
                        diff.insert(key.clone(), next_value.clone());
                    }
                }
            }

            let removed: Vec<Value> = old_map
                .keys()
                .filter(|key| !new_map.contains_key(*key))
                .map(|key| Value::String(key.clone()))
                .collect();

            if !removed.is_empty() {
                diff.insert("$removed".to_string(), Value::Array(removed));
            }

            Value::Object(diff)
        }
        Some(Value::Array(old_items)) if new_value.is_array() => {
            let new_items = new_value.as_array().expect("checked is_array");
            if old_items == new_items {
                Value::Object(Map::new())
            } else {
                let mut diff = Map::new();
                diff.insert("$replace".to_string(), new_value.clone());
                Value::Object(diff)
            }
        }
        Some(_) => {
            let mut diff = Map::new();
            diff.insert("$replace".to_string(), new_value.clone());
            Value::Object(diff)
        }
    }
}

fn is_empty_diff(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{VersionedContext, VersionedContextError};
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn version_starts_at_zero() {
        let context = VersionedContext::new();
        assert_eq!(context.version(), 0);
        assert!(context.get_since_version(0).unwrap().is_empty());
    }

    #[test]
    fn update_increments_version_and_stores_value() {
        let context = VersionedContext::new();

        let update = context
            .update_with_version("task", 0, json!({"status": "running"}))
            .unwrap();

        assert_eq!(update.version, 1);
        assert_eq!(context.version(), 1);
        assert_eq!(context.get("task"), Some(json!({"status": "running"})));
    }

    #[test]
    fn stale_version_is_rejected() {
        let context = VersionedContext::new();
        context
            .update_with_version("task", 0, json!({"status": "running"}))
            .unwrap();

        let error = context
            .update_with_version("task", 0, json!({"status": "done"}))
            .unwrap_err();

        assert_eq!(
            error,
            VersionedContextError::StaleVersion {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn get_since_version_returns_incremental_history() {
        let context = VersionedContext::new();
        context
            .update_with_version("task", 0, json!({"status": "running"}))
            .unwrap();
        context
            .update_with_version("task", 1, json!({"status": "done"}))
            .unwrap();

        let updates = context.get_since_version(1).unwrap();

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].version, 2);
        assert_eq!(updates[0].new_value, json!({"status": "done"}));
    }

    #[test]
    fn diff_payload_contains_only_changed_fields_for_objects() {
        let context = VersionedContext::new();
        context
            .update_with_version(
                "doc",
                0,
                json!({
                    "title": "Architecture",
                    "body": "a".repeat(2_000),
                    "status": "draft"
                }),
            )
            .unwrap();

        let update = context
            .update_with_version(
                "doc",
                1,
                json!({
                    "title": "Architecture",
                    "body": "a".repeat(2_000),
                    "status": "published"
                }),
            )
            .unwrap();

        assert_eq!(update.diff, json!({"status": {"$replace": "published"}}));
        assert!(update.size_reduction() > 0.8);
    }

    #[test]
    fn concurrent_writers_only_one_succeeds_per_version() {
        let context = Arc::new(VersionedContext::new());
        let left = Arc::clone(&context);
        let right = Arc::clone(&context);

        let first = std::thread::spawn(move || {
            left.update_with_version("key", 0, json!({"writer": "left"}))
        });
        let second = std::thread::spawn(move || {
            right.update_with_version("key", 0, json!({"writer": "right"}))
        });

        let first = first.join().unwrap();
        let second = second.join().unwrap();

        assert!(first.is_ok() ^ second.is_ok());
        assert_eq!(context.version(), 1);
        assert_eq!(context.get_since_version(0).unwrap().len(), 1);
    }
}
