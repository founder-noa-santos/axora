//! Hierarchical memory for compacting long conversational or execution context.
//!
//! The memory model separates entries into three windows:
//! - Recent: newest 0-10 entries, kept in full
//! - Mid: entries 11-50, lightly summarized
//! - Old: entries 50+, heavily summarized

/// A single context entry tracked by hierarchical memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryEntry {
    /// Monotonic position in the source sequence.
    pub index: usize,
    /// Role or source of the entry (user, assistant, system, task, etc.).
    pub role: String,
    /// Full content for the entry.
    pub content: String,
}

impl MemoryEntry {
    /// Creates a new memory entry.
    pub fn new(index: usize, role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            index,
            role: role.into(),
            content: content.into(),
        }
    }
}

/// A summarized representation of older memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummary {
    /// Inclusive start index covered by this summary.
    pub start_index: usize,
    /// Inclusive end index covered by this summary.
    pub end_index: usize,
    /// Number of entries represented.
    pub entry_count: usize,
    /// Human-readable summary text.
    pub summary: String,
}

/// Context returned from [`HierarchicalMemory::get_context`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchicalContext {
    /// Newest entries preserved in full.
    pub recent: Vec<MemoryEntry>,
    /// Mid-range entries compressed into grouped summaries.
    pub mid: Vec<MemorySummary>,
    /// Oldest entries compressed into a single coarse summary.
    pub old: Option<MemorySummary>,
}

impl HierarchicalContext {
    /// Returns true when there is no represented context.
    pub fn is_empty(&self) -> bool {
        self.recent.is_empty() && self.mid.is_empty() && self.old.is_none()
    }
}

/// Three-level hierarchical memory optimized for context compaction.
#[derive(Debug, Clone)]
pub struct HierarchicalMemory {
    recent_limit: usize,
    mid_limit: usize,
    mid_chunk_size: usize,
    entries: Vec<MemoryEntry>,
}

impl HierarchicalMemory {
    /// Creates a new hierarchical memory with the default level boundaries.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates hierarchical memory with custom level boundaries.
    ///
    /// `recent_limit` is the number of newest entries to retain in full.
    /// `mid_limit` is the total number of newest entries covered by recent + mid.
    pub fn with_limits(recent_limit: usize, mid_limit: usize) -> Self {
        let normalized_recent = recent_limit.max(1);
        let normalized_mid = mid_limit.max(normalized_recent);
        Self {
            recent_limit: normalized_recent,
            mid_limit: normalized_mid,
            mid_chunk_size: 10,
            entries: Vec::new(),
        }
    }

    /// Adds a new entry to memory.
    pub fn add_entry(&mut self, role: impl Into<String>, content: impl Into<String>) {
        let index = self.entries.len();
        self.entries.push(MemoryEntry::new(index, role, content));
    }

    /// Returns all tracked entries.
    pub fn entries(&self) -> &[MemoryEntry] {
        &self.entries
    }

    /// Builds a level-aware context view over all stored entries.
    pub fn get_context(&self) -> HierarchicalContext {
        if self.entries.is_empty() {
            return HierarchicalContext {
                recent: Vec::new(),
                mid: Vec::new(),
                old: None,
            };
        }

        let total = self.entries.len();
        let recent_start = total.saturating_sub(self.recent_limit);
        let mid_start = total.saturating_sub(self.mid_limit);

        let recent = self.entries[recent_start..].to_vec();
        let mid = self.summarize_mid(mid_start, recent_start);
        let old = self.summarize_old(mid_start);

        HierarchicalContext { recent, mid, old }
    }

    fn summarize_mid(&self, mid_start: usize, mid_end: usize) -> Vec<MemorySummary> {
        if mid_start >= mid_end {
            return Vec::new();
        }

        self.entries[mid_start..mid_end]
            .chunks(self.mid_chunk_size)
            .map(Self::summarize_chunk)
            .collect()
    }

    fn summarize_old(&self, old_end: usize) -> Option<MemorySummary> {
        if old_end == 0 {
            return None;
        }

        Some(Self::summarize_chunk(&self.entries[..old_end]))
    }

    fn summarize_chunk(entries: &[MemoryEntry]) -> MemorySummary {
        debug_assert!(!entries.is_empty());

        let start_index = entries.first().map(|entry| entry.index).unwrap_or(0);
        let end_index = entries
            .last()
            .map(|entry| entry.index)
            .unwrap_or(start_index);
        let entry_count = entries.len();

        let mut preview = Vec::new();
        for entry in entries.iter().take(3) {
            let snippet = truncate_for_summary(&entry.content, 48);
            preview.push(format!("#{} {}: {}", entry.index, entry.role, snippet));
        }

        let summary = if entry_count <= 3 {
            preview.join(" | ")
        } else {
            format!(
                "{} | ... {} additional entr{}",
                preview.join(" | "),
                entry_count - 3,
                if entry_count - 3 == 1 { "y" } else { "ies" }
            )
        };

        MemorySummary {
            start_index,
            end_index,
            entry_count,
            summary,
        }
    }
}

impl Default for HierarchicalMemory {
    fn default() -> Self {
        Self {
            recent_limit: 10,
            mid_limit: 50,
            mid_chunk_size: 10,
            entries: Vec::new(),
        }
    }
}

fn truncate_for_summary(content: &str, max_chars: usize) -> String {
    let trimmed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if trimmed.chars().count() <= max_chars {
        return trimmed;
    }

    let truncated: String = trimmed.chars().take(max_chars.saturating_sub(3)).collect();
    format!("{truncated}...")
}

#[cfg(test)]
mod tests {
    use super::{HierarchicalMemory, MemoryEntry};

    fn make_memory(count: usize) -> HierarchicalMemory {
        let mut memory = HierarchicalMemory::new();
        for index in 0..count {
            memory.add_entry(
                if index % 2 == 0 { "user" } else { "assistant" },
                format!("message {index} with enough detail to summarize clearly"),
            );
        }
        memory
    }

    #[test]
    fn returns_empty_context_for_empty_memory() {
        let memory = HierarchicalMemory::new();

        let context = memory.get_context();

        assert!(context.is_empty());
    }

    #[test]
    fn keeps_all_entries_recent_when_within_recent_window() {
        let memory = make_memory(8);

        let context = memory.get_context();

        assert_eq!(context.recent.len(), 8);
        assert!(context.mid.is_empty());
        assert!(context.old.is_none());
        assert_eq!(context.recent[0].index, 0);
        assert_eq!(context.recent[7].index, 7);
    }

    #[test]
    fn summarizes_mid_window_after_recent_limit() {
        let memory = make_memory(20);

        let context = memory.get_context();

        assert_eq!(context.recent.len(), 10);
        assert_eq!(context.recent[0].index, 10);
        assert_eq!(context.recent[9].index, 19);
        assert_eq!(context.mid.len(), 1);
        assert_eq!(context.mid[0].start_index, 0);
        assert_eq!(context.mid[0].end_index, 9);
        assert_eq!(context.mid[0].entry_count, 10);
        assert!(context.old.is_none());
    }

    #[test]
    fn summarizes_old_window_beyond_mid_limit() {
        let memory = make_memory(75);

        let context = memory.get_context();

        assert_eq!(context.recent.len(), 10);
        assert_eq!(context.recent[0].index, 65);
        assert_eq!(context.mid.len(), 4);
        assert_eq!(context.mid[0].start_index, 25);
        assert_eq!(context.mid[3].end_index, 64);
        let old = context.old.expect("old summary should exist");
        assert_eq!(old.start_index, 0);
        assert_eq!(old.end_index, 24);
        assert_eq!(old.entry_count, 25);
    }

    #[test]
    fn custom_limits_are_applied() {
        let mut memory = HierarchicalMemory::with_limits(3, 6);
        for index in 0..9 {
            memory.add_entry("task", format!("task update {index}"));
        }

        let context = memory.get_context();

        assert_eq!(context.recent.len(), 3);
        assert_eq!(context.recent[0].index, 6);
        assert_eq!(context.mid.len(), 1);
        assert_eq!(context.mid[0].start_index, 3);
        assert_eq!(context.mid[0].end_index, 5);
        let old = context.old.expect("old summary should exist");
        assert_eq!(old.start_index, 0);
        assert_eq!(old.end_index, 2);
    }

    #[test]
    fn summaries_include_role_and_content_previews() {
        let memory = make_memory(55);

        let context = memory.get_context();

        assert!(!context.mid.is_empty());
        assert!(context.mid[0].summary.contains("user:"));
        assert!(context.mid[0].summary.contains("assistant:"));
        assert!(context
            .old
            .expect("old summary should exist")
            .summary
            .contains("message 0"));
    }

    #[test]
    fn entries_accessor_returns_inserted_entries() {
        let mut memory = HierarchicalMemory::new();
        memory.add_entry("system", "initialize");
        memory.add_entry("assistant", "ready");

        assert_eq!(
            memory.entries(),
            &[
                MemoryEntry::new(0, "system", "initialize"),
                MemoryEntry::new(1, "assistant", "ready"),
            ]
        );
    }
}
