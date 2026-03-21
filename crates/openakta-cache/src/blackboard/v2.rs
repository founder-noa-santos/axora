//! Blackboard v2 with versioned shared state and diff-based notifications.
//!
//! ```rust,no_run
//! use openakta_cache::blackboard::v2::BlackboardV2;
//! use serde_json::json;
//!
//! let blackboard = BlackboardV2::new();
//! let (_subscription_id, mut receiver) = blackboard.subscribe_channel("task");
//!
//! let version = blackboard
//!     .set("task", json!({"status": "running", "owner": "agent-a"}))
//!     .unwrap();
//!
//! let update = receiver.try_recv().unwrap();
//! assert_eq!(version, update.version);
//! assert!(update.size_reduction() >= 0.0);
//! ```

#[path = "v2_pubsub.rs"]
pub mod v2_pubsub;
#[path = "v2_versioning.rs"]
pub mod v2_versioning;

use self::v2_pubsub::{PubSubHub, Subscriber, SubscriptionId};
use self::v2_versioning::{
    Result as VersionedResult, Update, VersionedContext, VersionedContextError, VersionedValue,
};
use serde_json::Value;
use thiserror::Error;
use tokio::sync::mpsc::UnboundedReceiver;

/// Result type for Blackboard v2 operations.
pub type Result<T> = std::result::Result<T, BlackboardV2Error>;

/// Blackboard v2 operation errors.
#[derive(Debug, Error)]
pub enum BlackboardV2Error {
    /// Underlying versioned state rejected the operation.
    #[error(transparent)]
    Versioning(#[from] VersionedContextError),

    /// No changes are available for a key since the requested version.
    #[error("no changes for key '{key}' since version {since_version}")]
    NoChangesSinceVersion { key: String, since_version: u64 },
}

/// Versioned, diff-publishing blackboard for concurrent agents.
#[derive(Default)]
pub struct BlackboardV2 {
    context: VersionedContext,
    subscribers: PubSubHub,
}

impl BlackboardV2 {
    /// Creates an empty Blackboard v2 instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reads the latest value for a key.
    pub fn get(&self, key: &str) -> Option<Value> {
        self.context.get(key)
    }

    /// Reads the full versioned value for a key.
    pub fn get_versioned(&self, key: &str) -> Option<VersionedValue> {
        self.context.get_versioned(key)
    }

    /// Returns the newest update for a key after the requested version.
    pub fn get_with_version(&self, key: &str, since_version: u64) -> Result<Update> {
        self.get_since_version(since_version)?
            .into_iter()
            .rfind(|update| update.key == key)
            .ok_or_else(|| BlackboardV2Error::NoChangesSinceVersion {
                key: key.to_string(),
                since_version,
            })
    }

    /// Returns all updates after the provided version.
    pub fn get_since_version(&self, since_version: u64) -> Result<Vec<Update>> {
        let updates: VersionedResult<Vec<Update>> = self.context.get_since_version(since_version);
        Ok(updates?)
    }

    /// Stores a value, retrying on optimistic version conflicts.
    pub fn set(&self, key: &str, value: Value) -> Result<u64> {
        loop {
            let expected_version = self.current_version();
            match self
                .context
                .update_with_version(key, expected_version, value.clone())
            {
                Ok(update) => {
                    self.subscribers.publish(key, update.clone());
                    return Ok(update.version);
                }
                Err(VersionedContextError::StaleVersion { .. }) => continue,
                Err(error) => return Err(error.into()),
            }
        }
    }

    /// Stores a value and publishes its diff to subscribers.
    pub fn publish(&self, key: &str, value: Value) -> Result<u64> {
        self.set(key, value)
    }

    /// Registers a trait-based subscriber for a key.
    pub fn subscribe<S>(&self, key: &str, subscriber: S) -> SubscriptionId
    where
        S: Subscriber + 'static,
    {
        self.subscribers.subscribe(key, subscriber)
    }

    /// Registers a channel subscriber for a key.
    pub fn subscribe_channel(&self, key: &str) -> (SubscriptionId, UnboundedReceiver<Update>) {
        self.subscribers.subscribe_channel(key)
    }

    /// Removes a subscription.
    pub fn unsubscribe(&self, id: SubscriptionId) -> bool {
        self.subscribers.unsubscribe(id)
    }

    /// Returns the current global version.
    pub fn current_version(&self) -> u64 {
        self.context.version()
    }
}

#[cfg(test)]
mod tests {
    use super::BlackboardV2;
    use serde_json::json;

    #[test]
    fn set_updates_current_version() {
        let blackboard = BlackboardV2::new();

        let version = blackboard
            .set("task", json!({"status": "running"}))
            .unwrap();

        assert_eq!(version, 1);
        assert_eq!(blackboard.current_version(), 1);
    }

    #[test]
    fn get_with_version_returns_latest_matching_change() {
        let blackboard = BlackboardV2::new();
        blackboard.set("task", json!({"status": "draft"})).unwrap();
        blackboard.set("task", json!({"status": "done"})).unwrap();

        let update = blackboard.get_with_version("task", 1).unwrap();

        assert_eq!(update.version, 2);
        assert_eq!(update.new_value, json!({"status": "done"}));
    }

    #[test]
    fn channel_subscription_receives_diff_payload() {
        let blackboard = BlackboardV2::new();
        blackboard
            .set(
                "doc",
                json!({"title": "Spec", "body": "x".repeat(1024), "status": "draft"}),
            )
            .unwrap();
        let (_id, mut receiver) = blackboard.subscribe_channel("doc");

        blackboard
            .set(
                "doc",
                json!({"title": "Spec", "body": "x".repeat(1024), "status": "done"}),
            )
            .unwrap();

        let update = receiver.try_recv().unwrap();
        assert_eq!(update.diff, json!({"status": {"$replace": "done"}}));
        assert!(update.size_reduction() > 0.8);
    }
}
