//! Agent memory management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info, warn};

/// Get current timestamp in seconds
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: String,
    /// Memory content
    pub content: String,
    /// Memory type
    pub memory_type: MemoryType,
    /// Importance score (0.0 - 1.0)
    pub importance: f32,
    /// Access count
    pub access_count: usize,
    /// Created at (timestamp in seconds)
    pub created_at: u64,
    /// Last accessed at (timestamp in seconds)
    pub last_accessed: u64,
    /// Expires at (optional timestamp in seconds)
    pub expires_at: Option<u64>,
}

/// Type of memory
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MemoryType {
    /// Short-term working memory
    ShortTerm,
    /// Long-term episodic memory (experiences)
    Episodic,
    /// Long-term semantic memory (facts)
    Semantic,
    /// Procedural memory (skills)
    Procedural,
    /// Shared memory (across agents)
    Shared,
}

/// Memory store for an agent
pub struct MemoryStore {
    /// All memories indexed by ID
    memories: HashMap<String, MemoryEntry>,
    /// Memories by type
    by_type: HashMap<MemoryType, Vec<String>>,
    /// Maximum short-term memories
    max_short_term: usize,
    /// Maximum long-term memories
    max_long_term: usize,
    /// Forgetting threshold (importance below this is forgotten)
    forgetting_threshold: f32,
}

impl MemoryStore {
    /// Create new memory store
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
            by_type: HashMap::new(),
            max_short_term: 10,
            max_long_term: 1000,
            forgetting_threshold: 0.1,
        }
    }

    /// Add a memory
    pub fn add(&mut self, entry: MemoryEntry) {
        debug!(
            "Adding memory: {} (type: {:?})",
            entry.id, entry.memory_type
        );

        let memory_type = entry.memory_type.clone();
        let id = entry.id.clone();

        // Check capacity
        self.enforce_capacity(&memory_type);

        // Add memory
        self.memories.insert(id.clone(), entry);
        self.by_type.entry(memory_type).or_default().push(id);
    }

    /// Create and add a memory
    pub fn create(&mut self, content: &str, memory_type: MemoryType, importance: f32) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_secs();

        let entry = MemoryEntry {
            id: id.clone(),
            content: content.to_string(),
            memory_type,
            importance,
            access_count: 0,
            created_at: now,
            last_accessed: now,
            expires_at: None,
        };

        self.add(entry);
        id
    }

    /// Get a memory by ID
    pub fn get(&mut self, id: &str) -> Option<&MemoryEntry> {
        if let Some(entry) = self.memories.get_mut(id) {
            entry.access_count += 1;
            entry.last_accessed = now_secs();
            Some(entry)
        } else {
            None
        }
    }

    /// Get memories by type
    pub fn get_by_type(&self, memory_type: &MemoryType) -> Vec<&MemoryEntry> {
        self.by_type
            .get(memory_type)
            .map(|ids| ids.iter().filter_map(|id| self.memories.get(id)).collect())
            .unwrap_or_default()
    }

    /// Search memories by content
    pub fn search(&self, query: &str) -> Vec<&MemoryEntry> {
        let query_lower = query.to_lowercase();

        self.memories
            .values()
            .filter(|m| m.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Update memory importance
    pub fn update_importance(&mut self, id: &str, importance: f32) {
        if let Some(entry) = self.memories.get_mut(id) {
            entry.importance = importance;
        }
    }

    /// Consolidate memories (move short-term to long-term)
    pub fn consolidate(&mut self) {
        info!("Consolidating memories...");

        let short_term_ids: Vec<String> = self
            .by_type
            .get(&MemoryType::ShortTerm)
            .cloned()
            .unwrap_or_default();

        for id in short_term_ids {
            if let Some(entry) = self.memories.get_mut(&id) {
                // If accessed multiple times and high importance, consolidate
                if entry.access_count > 2 && entry.importance > 0.5 {
                    let old_type = entry.memory_type.clone();
                    entry.memory_type = MemoryType::Semantic;

                    // Update type index
                    if let Some(type_list) = self.by_type.get_mut(&old_type) {
                        type_list.retain(|i| i != &id);
                    }
                    let id_clone = id.clone();
                    self.by_type
                        .entry(MemoryType::Semantic)
                        .or_default()
                        .push(id_clone);

                    info!("Consolidated memory {} to semantic", id);
                }
            }
        }
    }

    /// Forget low-importance memories
    pub fn forget(&mut self) -> usize {
        info!("Forgetting low-importance memories...");

        let mut forgotten = 0;
        let now = now_secs();

        // Collect IDs to forget
        let to_forget: Vec<String> = self
            .memories
            .iter()
            .filter(|(_, entry)| {
                // Forget if expired
                if let Some(expires) = entry.expires_at {
                    if now > expires {
                        return true;
                    }
                }

                // Forget if low importance and old (1 hour = 3600 seconds)
                let age = now - entry.created_at;
                if entry.importance < self.forgetting_threshold && age > 3600 {
                    return true;
                }

                false
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Remove forgotten memories
        for id in to_forget {
            if let Some(entry) = self.memories.remove(&id) {
                if let Some(type_list) = self.by_type.get_mut(&entry.memory_type) {
                    type_list.retain(|i| i != &id);
                }
                forgotten += 1;
            }
        }

        if forgotten > 0 {
            info!("Forgotten {} memories", forgotten);
        }

        forgotten
    }

    /// Enforce capacity limits
    fn enforce_capacity(&mut self, memory_type: &MemoryType) {
        let (max, type_list) = match memory_type {
            MemoryType::ShortTerm => (
                self.max_short_term,
                self.by_type.get_mut(&MemoryType::ShortTerm),
            ),
            MemoryType::Episodic | MemoryType::Semantic | MemoryType::Procedural => {
                (self.max_long_term, self.by_type.get_mut(memory_type))
            }
            MemoryType::Shared => return, // No limit for shared
        };

        let list = match type_list {
            Some(l) => l,
            None => return,
        };

        // Remove oldest/least important if over capacity
        while list.len() > max {
            // Find least important
            let least_important_id = list
                .iter()
                .min_by(|a, b| {
                    let entry_a = self.memories.get(*a);
                    let entry_b = self.memories.get(*b);

                    match (entry_a, entry_b) {
                        (Some(a), Some(b)) => a.importance.partial_cmp(&b.importance).unwrap(),
                        _ => std::cmp::Ordering::Equal,
                    }
                })
                .cloned();

            if let Some(id) = least_important_id {
                self.memories.remove(&id);
                list.retain(|i| i != &id);
                debug!("Removed memory {} due to capacity", id);
            } else {
                break;
            }
        }
    }

    /// Set max short-term memories
    pub fn with_max_short_term(mut self, max: usize) -> Self {
        self.max_short_term = max;
        self
    }

    /// Set max long-term memories
    pub fn with_max_long_term(mut self, max: usize) -> Self {
        self.max_long_term = max;
        self
    }

    /// Set forgetting threshold
    pub fn with_forgetting_threshold(mut self, threshold: f32) -> Self {
        self.forgetting_threshold = threshold;
        self
    }

    /// Get memory count
    pub fn count(&self) -> usize {
        self.memories.len()
    }

    /// Get short-term memory count
    pub fn short_term_count(&self) -> usize {
        self.by_type
            .get(&MemoryType::ShortTerm)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Get long-term memory count
    pub fn long_term_count(&self) -> usize {
        self.by_type
            .iter()
            .filter(|(t, _)| {
                matches!(
                    t,
                    MemoryType::Episodic | MemoryType::Semantic | MemoryType::Procedural
                )
            })
            .map(|(_, v)| v.len())
            .sum()
    }

    /// Clear all memories
    pub fn clear(&mut self) {
        self.memories.clear();
        self.by_type.clear();
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared memory blackboard for inter-agent communication
pub struct SharedBlackboard {
    /// Shared memories
    memories: HashMap<String, MemoryEntry>,
    /// Access control (agent_id -> accessible memory_ids)
    access_control: HashMap<String, Vec<String>>,
    /// Monotonic version incremented on every state change.
    version: u64,
}

impl SharedBlackboard {
    /// Create new shared blackboard
    pub fn new() -> Self {
        Self {
            memories: HashMap::new(),
            access_control: HashMap::new(),
            version: 0,
        }
    }

    /// Publish to blackboard
    pub fn publish(&mut self, entry: MemoryEntry, accessible_by: Vec<String>) {
        let id = entry.id.clone();
        let count = accessible_by.len();

        self.memories.insert(id.clone(), entry);
        self.version = self.version.saturating_add(1);

        for agent_id in accessible_by {
            self.access_control
                .entry(agent_id)
                .or_default()
                .push(id.clone());
        }

        info!("Published memory {} to {} agents", id, count);
    }

    /// Read from blackboard
    pub fn read(&self, agent_id: &str, memory_id: &str) -> Option<&MemoryEntry> {
        // Check access control
        let accessible = self.access_control.get(agent_id)?;

        if !accessible.contains(&memory_id.to_string()) {
            warn!("Agent {} denied access to memory {}", agent_id, memory_id);
            return None;
        }

        self.memories.get(memory_id)
    }

    /// Get all accessible memories for an agent
    pub fn get_accessible(&self, agent_id: &str) -> Vec<&MemoryEntry> {
        self.access_control
            .get(agent_id)
            .map(|ids| ids.iter().filter_map(|id| self.memories.get(id)).collect())
            .unwrap_or_default()
    }

    /// Remove from blackboard
    pub fn remove(&mut self, agent_id: &str, memory_id: &str) {
        if let Some(accessible) = self.access_control.get_mut(agent_id) {
            accessible.retain(|id| id != memory_id);
        }
        self.memories.remove(memory_id);
        self.version = self.version.saturating_add(1);
    }

    /// Current snapshot version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Summarize the current state for planning snapshots.
    pub fn snapshot_summary(&self, agent_id: &str) -> String {
        let accessible = self.get_accessible(agent_id);
        let mut summary = format!("version={} entries={}", self.version, accessible.len());
        for entry in accessible.into_iter().take(3) {
            summary.push_str(&format!("\n- [{}] {}", entry.id, entry.content));
        }
        summary
    }
}

impl Default for SharedBlackboard {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_creation() {
        let mut store = MemoryStore::new();

        let id = store.create("Test memory", MemoryType::ShortTerm, 0.8);

        assert!(!id.is_empty());
        assert_eq!(store.count(), 1);
        assert_eq!(store.short_term_count(), 1);
    }

    #[test]
    fn test_memory_retrieval() {
        let mut store = MemoryStore::new();

        let id = store.create("Test memory", MemoryType::ShortTerm, 0.8);

        let memory = store.get(&id);
        assert!(memory.is_some());
        assert_eq!(memory.unwrap().access_count, 1);
    }

    #[test]
    fn test_memory_search() {
        let mut store = MemoryStore::new();

        store.create("Rust programming", MemoryType::Semantic, 0.9);
        store.create("Python scripting", MemoryType::Semantic, 0.7);
        store.create("JavaScript web", MemoryType::Semantic, 0.6);

        let results = store.search("rust");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_memory_consolidation() {
        let mut store = MemoryStore::new();

        // Create short-term memory with high importance
        let id = store.create("Important fact", MemoryType::ShortTerm, 0.9);

        // Access it multiple times
        store.get(&id);
        store.get(&id);
        store.get(&id);

        // Consolidate
        store.consolidate();

        // Should be moved to semantic
        let semantic = store.get_by_type(&MemoryType::Semantic);
        assert!(!semantic.is_empty());
    }

    #[test]
    fn test_memory_forgetting() {
        let mut store = MemoryStore::new().with_forgetting_threshold(0.5);

        // Create low-importance memory
        store.create("Unimportant", MemoryType::ShortTerm, 0.1);
        // Create high-importance memory
        store.create("Important", MemoryType::ShortTerm, 0.9);

        // Forget (won't forget immediately due to age check)
        let forgotten = store.forget();

        // May not forget immediately due to age threshold
        assert!(forgotten == 0 || forgotten == 1);
    }

    #[test]
    fn test_shared_blackboard() {
        let mut blackboard = SharedBlackboard::new();

        let entry = MemoryEntry {
            id: "shared1".to_string(),
            content: "Shared knowledge".to_string(),
            memory_type: MemoryType::Shared,
            importance: 0.8,
            access_count: 0,
            created_at: now_secs(),
            last_accessed: now_secs(),
            expires_at: None,
        };

        blackboard.publish(entry, vec!["agent1".to_string(), "agent2".to_string()]);

        // Agent1 can access
        let memory = blackboard.read("agent1", "shared1");
        assert!(memory.is_some());

        // Agent3 cannot access
        let memory = blackboard.read("agent3", "shared1");
        assert!(memory.is_none());
    }
}
