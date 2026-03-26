//! Runtime blackboard adapter backed by OPENAKTA Cache Blackboard v2.

use openakta_cache::{BlackboardV2 as SharedStateBlackboard, BlackboardV2Error};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::watch;

/// Blackboard entry published by runtime components.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlackboardEntry {
    /// Stable entry id and storage key.
    pub id: String,
    /// Optional typed namespace for the entry.
    #[serde(default)]
    pub namespace: Option<String>,
    /// Optional schema **label** (versioned shape name), e.g. `hitl_answer.v1`.
    /// Not a cryptographic hash and not validated against `content` — see
    /// `REPORT_AI_WORK_MANAGEMENT_ORCHESTRATION.md` §5.5 (MOL-A12).
    #[serde(default)]
    pub schema_hash: Option<String>,
    /// Serialized content payload.
    pub content: String,
}

/// Runtime blackboard facade with access control and version subscriptions.
pub struct RuntimeBlackboard {
    state: SharedStateBlackboard,
    access_control: HashMap<String, Vec<String>>,
    version_tx: watch::Sender<u64>,
}

impl RuntimeBlackboard {
    /// Create a new runtime blackboard.
    pub fn new() -> Self {
        let state = SharedStateBlackboard::new();
        let (version_tx, _) = watch::channel(state.current_version());
        Self {
            state,
            access_control: HashMap::new(),
            version_tx,
        }
    }

    /// Publish a new entry and grant access to the listed agents.
    pub fn publish(
        &mut self,
        entry: BlackboardEntry,
        accessible_by: Vec<String>,
    ) -> Result<u64, BlackboardV2Error> {
        self.publish_typed(entry, accessible_by)
    }

    /// Publish a typed entry with namespace and schema metadata embedded.
    pub fn publish_typed(
        &mut self,
        entry: BlackboardEntry,
        accessible_by: Vec<String>,
    ) -> Result<u64, BlackboardV2Error> {
        let id = entry.id.clone();
        let version = self.state.publish(
            &id,
            json!({
                "id": entry.id,
                "namespace": entry.namespace,
                "schema_hash": entry.schema_hash,
                "content": entry.content,
            }),
        )?;
        for agent_id in accessible_by {
            let accessible = self.access_control.entry(agent_id).or_default();
            if !accessible.iter().any(|existing| existing == &id) {
                accessible.push(id.clone());
            }
        }
        let _ = self.version_tx.send(version);
        Ok(version)
    }

    /// Read an accessible entry by id.
    pub fn read(&self, agent_id: &str, entry_id: &str) -> Option<BlackboardEntry> {
        let accessible = self.access_control.get(agent_id)?;
        if !accessible.iter().any(|id| id == entry_id) {
            return None;
        }
        self.state.get(entry_id).and_then(decode_entry)
    }

    /// Get all entries visible to an agent.
    pub fn get_accessible(&self, agent_id: &str) -> Vec<BlackboardEntry> {
        self.access_control
            .get(agent_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.state.get(id).and_then(decode_entry))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Current snapshot version.
    pub fn version(&self) -> u64 {
        self.state.current_version()
    }

    /// Subscribe to version changes for planner-side interrupts.
    pub fn subscribe_version(&self) -> watch::Receiver<u64> {
        self.version_tx.subscribe()
    }

    /// Summarize the visible snapshot for planning (`id`, namespace, content only;
    /// `schema_hash` is omitted). Only entries listed for `agent_id` in
    /// [`Self::publish`] `accessible_by` are visible — callers must match that id.
    pub fn snapshot_summary(&self, agent_id: &str) -> String {
        let accessible = self.get_accessible(agent_id);
        let mut summary = format!("version={} entries={}", self.version(), accessible.len());
        for entry in accessible.into_iter().take(3) {
            let namespace = entry.namespace.unwrap_or_else(|| "untyped".to_string());
            summary.push_str(&format!(
                "\n- [{}] ({namespace}) {}",
                entry.id, entry.content
            ));
        }
        summary
    }
}

impl Default for RuntimeBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

fn decode_entry(value: serde_json::Value) -> Option<BlackboardEntry> {
    serde_json::from_value(value).ok()
}
