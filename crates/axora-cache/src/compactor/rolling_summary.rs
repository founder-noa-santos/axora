//! Rolling conversation summaries for context compaction.
//!
//! The newest turns stay verbatim while older turns are collapsed into a
//! compact textual summary.

use std::collections::BTreeMap;

/// A single conversational turn tracked by the compactor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    /// Logical speaker label such as `user`, `assistant`, or `system`.
    pub role: String,
    /// Full turn content.
    pub content: String,
    /// Estimated token count for this turn.
    pub token_count: usize,
}

impl Turn {
    /// Creates a new turn with a simple token estimate.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_count = estimate_tokens(&content);

        Self {
            role: role.into(),
            content,
            token_count,
        }
    }
}

/// Maintains a rolling history where older turns are summarized.
#[derive(Debug, Clone)]
pub struct RollingSummary {
    keep_full: usize,
    turns: Vec<Turn>,
}

impl Default for RollingSummary {
    fn default() -> Self {
        Self::new(10)
    }
}

impl RollingSummary {
    /// Creates a rolling summary with a custom number of full turns to keep.
    pub fn new(keep_full: usize) -> Self {
        Self {
            keep_full,
            turns: Vec::new(),
        }
    }

    /// Appends a turn to the rolling history.
    pub fn add_turn(&mut self, turn: Turn) {
        self.turns.push(turn);
    }

    /// Returns all stored turns.
    pub fn turns(&self) -> &[Turn] {
        &self.turns
    }

    /// Builds a compact textual summary.
    ///
    /// The newest `keep_full` turns are preserved verbatim. Older turns are
    /// aggregated by role with short excerpts to retain salient context.
    pub fn summarize(&self) -> String {
        if self.turns.is_empty() {
            return "No conversation history.".to_string();
        }

        let split_index = self.turns.len().saturating_sub(self.keep_full);
        let (older, recent) = self.turns.split_at(split_index);
        let mut sections = Vec::new();

        if !older.is_empty() {
            sections.push(self.summarize_older_turns(older));
        }

        if !recent.is_empty() {
            let rendered_recent = recent
                .iter()
                .map(|turn| format!("[{}] {}", turn.role, turn.content))
                .collect::<Vec<_>>()
                .join("\n");
            sections.push(format!("Recent turns:\n{}", rendered_recent));
        }

        sections.join("\n\n")
    }

    fn summarize_older_turns(&self, older: &[Turn]) -> String {
        let mut grouped: BTreeMap<&str, Vec<&Turn>> = BTreeMap::new();
        let mut total_tokens = 0usize;

        for turn in older {
            grouped.entry(turn.role.as_str()).or_default().push(turn);
            total_tokens += turn.token_count;
        }

        let mut lines = vec![format!(
            "Historical summary: {} turns compacted (~{} tokens).",
            older.len(),
            total_tokens
        )];

        for (role, turns) in grouped {
            let excerpts = turns
                .iter()
                .take(3)
                .map(|turn| truncate(&turn.content, 48))
                .collect::<Vec<_>>()
                .join(" | ");
            lines.push(format!(
                "- {}: {} turns. Highlights: {}",
                role,
                turns.len(),
                excerpts
            ));
        }

        lines.join("\n")
    }
}

fn truncate(input: &str, max_chars: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;

    for ch in input.chars() {
        if count == max_chars {
            out.push_str("...");
            return out;
        }

        out.push(ch);
        count += 1;
    }

    out
}

fn estimate_tokens(content: &str) -> usize {
    let char_count = content.chars().count();
    char_count.div_ceil(4).max(1)
}

#[cfg(test)]
mod tests {
    use super::{RollingSummary, Turn};

    fn make_turn(index: usize) -> Turn {
        Turn::new(
            if index.is_multiple_of(2) {
                "user"
            } else {
                "assistant"
            },
            format!("turn {index} content with enough detail to be visible"),
        )
    }

    #[test]
    fn summarize_empty_history() {
        let summary = RollingSummary::default().summarize();
        assert_eq!(summary, "No conversation history.");
    }

    #[test]
    fn keep_all_turns_when_history_is_short() {
        let mut summary = RollingSummary::default();
        for index in 0..4 {
            summary.add_turn(make_turn(index));
        }

        let rendered = summary.summarize();

        assert!(rendered.contains("Recent turns:"));
        assert!(!rendered.contains("Historical summary:"));
        assert!(rendered.contains("[user] turn 0 content"));
        assert!(rendered.contains("[assistant] turn 1 content"));
    }

    #[test]
    fn summarize_older_turns_after_threshold() {
        let mut summary = RollingSummary::default();
        for index in 0..14 {
            summary.add_turn(make_turn(index));
        }

        let rendered = summary.summarize();

        assert!(rendered.contains("Historical summary: 4 turns compacted"));
        assert!(rendered.contains("Highlights: turn 0 content"));
        assert!(rendered.contains("[user] turn 4 content"));
        assert!(rendered.contains("[assistant] turn 13 content"));
        assert!(!rendered.contains("[user] turn 0 content"));
    }

    #[test]
    fn older_summary_groups_by_role() {
        let mut summary = RollingSummary::new(2);
        summary.add_turn(Turn::new("system", "initial policy and guardrails"));
        summary.add_turn(Turn::new("user", "first request"));
        summary.add_turn(Turn::new("assistant", "first answer"));
        summary.add_turn(Turn::new("user", "latest request"));

        let rendered = summary.summarize();

        assert!(rendered.contains("- system: 1 turns. Highlights: initial policy and guardrails"));
        assert!(rendered.contains("- user: 1 turns. Highlights: first request"));
        assert!(rendered.contains("[assistant] first answer"));
        assert!(rendered.contains("[user] latest request"));
    }

    #[test]
    fn add_turn_preserves_order() {
        let mut summary = RollingSummary::new(3);
        summary.add_turn(Turn::new("user", "one"));
        summary.add_turn(Turn::new("assistant", "two"));
        summary.add_turn(Turn::new("user", "three"));

        assert_eq!(summary.turns()[0].content, "one");
        assert_eq!(summary.turns()[1].content, "two");
        assert_eq!(summary.turns()[2].content, "three");
    }

    #[test]
    fn excerpts_are_truncated_for_long_older_turns() {
        let mut summary = RollingSummary::new(1);
        summary.add_turn(Turn::new(
            "user",
            "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789",
        ));
        summary.add_turn(Turn::new("assistant", "recent turn"));

        let rendered = summary.summarize();

        assert!(rendered.contains("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUV..."));
        assert!(rendered.contains("[assistant] recent turn"));
    }
}
