//! Experimental latent context side-channel storage.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Stored latent context record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatentContextRecord {
    /// Opaque handle used across orchestration messages.
    pub handle: String,
    /// Model family the latent payload is compatible with.
    pub model_family: String,
    /// Binary latent payload.
    pub payload: Vec<u8>,
    /// Optional audit correlation identifier.
    pub audit_correlation_id: Option<String>,
    /// Creation timestamp in unix seconds.
    pub created_at: u64,
}

/// In-process latent context cache keyed by opaque handle.
#[derive(Clone, Default)]
pub struct LatentContextStore {
    records: Arc<DashMap<String, LatentContextRecord>>,
}

impl LatentContextStore {
    /// Create a new latent context store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Store a latent payload and return its opaque handle.
    pub fn put(
        &self,
        model_family: impl Into<String>,
        payload: Vec<u8>,
        audit_correlation_id: Option<String>,
    ) -> String {
        let handle = Uuid::new_v4().to_string();
        let record = LatentContextRecord {
            handle: handle.clone(),
            model_family: model_family.into(),
            payload,
            audit_correlation_id,
            created_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        self.records.insert(handle.clone(), record);
        handle
    }

    /// Load a latent payload by handle.
    pub fn get(&self, handle: &str) -> Option<LatentContextRecord> {
        self.records.get(handle).map(|entry| entry.clone())
    }

    /// Remove a latent payload by handle.
    pub fn remove(&self, handle: &str) -> Option<LatentContextRecord> {
        self.records.remove(handle).map(|(_, value)| value)
    }

    /// Return the number of active latent payloads.
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true when the store is empty.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::LatentContextStore;

    #[test]
    fn stores_and_retrieves_latent_context() {
        let store = LatentContextStore::new();
        let handle = store.put("claude", vec![1, 2, 3], Some("audit-1".to_string()));

        let record = store.get(&handle).expect("record should exist");
        assert_eq!(record.model_family, "claude");
        assert_eq!(record.payload, vec![1, 2, 3]);
        assert_eq!(record.audit_correlation_id.as_deref(), Some("audit-1"));
    }
}
