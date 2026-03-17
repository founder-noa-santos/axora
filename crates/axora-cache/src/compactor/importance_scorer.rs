//! Importance scoring for context compaction.
//!
//! The scorer assigns deterministic 0.0-1.0 scores so callers can keep
//! critical items and prune low-signal chatter.

/// Category used to seed a deterministic importance score.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    /// A conversational turn.
    Turn,
    /// A design or execution decision that should usually be preserved.
    Decision,
    /// A code-related artifact or excerpt.
    Code,
    /// A documentation artifact or specification.
    Document,
    /// A short note or status update.
    Note,
}

/// A scored item that can be pruned during compaction.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredItem {
    /// Stable identifier for the item.
    pub id: String,
    /// Item category.
    pub kind: ItemKind,
    /// Item content used for lexical scoring.
    pub content: String,
    /// Caller-supplied priority hint in the 0.0-1.0 range.
    pub priority: f32,
}

impl ScoredItem {
    /// Creates a new scored item with a neutral priority hint.
    pub fn new(id: impl Into<String>, kind: ItemKind, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind,
            content: content.into(),
            priority: 0.5,
        }
    }

    /// Applies a caller-provided priority hint.
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority.clamp(0.0, 1.0);
        self
    }
}

/// Deterministic importance scorer for pruning context.
#[derive(Debug, Clone)]
pub struct ImportanceScorer {
    keyword_weight: f32,
    priority_weight: f32,
    brevity_weight: f32,
}

impl ImportanceScorer {
    /// Creates a scorer with conservative defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns an importance score in the 0.0-1.0 range.
    pub fn score(&self, item: &ScoredItem) -> f32 {
        let base = self.base_score(item.kind);
        let keywords = self.keyword_signal(&item.content) * self.keyword_weight;
        let priority = item.priority * self.priority_weight;
        let brevity = self.brevity_signal(&item.content) * self.brevity_weight;
        let filler_penalty = self.filler_penalty(&item.content);

        (base + keywords + priority + brevity - filler_penalty).clamp(0.0, 1.0)
    }

    /// Prunes items whose score is strictly below `threshold`.
    ///
    /// The original order is preserved.
    pub fn prune_below(&self, items: &[ScoredItem], threshold: f32) -> Vec<ScoredItem> {
        let threshold = threshold.clamp(0.0, 1.0);

        items
            .iter()
            .filter(|item| self.score(item) >= threshold)
            .cloned()
            .collect()
    }

    fn base_score(&self, kind: ItemKind) -> f32 {
        match kind {
            ItemKind::Decision => 0.55,
            ItemKind::Code => 0.42,
            ItemKind::Document => 0.40,
            ItemKind::Turn => 0.32,
            ItemKind::Note => 0.22,
        }
    }

    fn keyword_signal(&self, content: &str) -> f32 {
        let normalized = content.to_ascii_lowercase();
        let important_keywords = [
            "must",
            "required",
            "critical",
            "blocker",
            "security",
            "error",
            "incident",
            "decision",
            "architecture",
            "api",
            "database",
            "migration",
            "deadline",
            "regression",
            "failing",
        ];

        let matches = important_keywords
            .iter()
            .filter(|keyword| normalized.contains(**keyword))
            .count();

        (matches as f32 * 0.12).min(0.30)
    }

    fn brevity_signal(&self, content: &str) -> f32 {
        let words = content.split_whitespace().count();

        match words {
            0..=2 => 0.0,
            3..=12 => 0.12,
            13..=40 => 0.08,
            41..=80 => 0.03,
            _ => 0.0,
        }
    }

    fn filler_penalty(&self, content: &str) -> f32 {
        let normalized = content.to_ascii_lowercase();
        let filler_markers = [
            "thanks",
            "thank you",
            "sounds good",
            "okay",
            "ok",
            "noted",
            "ack",
            "acknowledged",
            "sgtm",
            "done",
        ];

        let filler_hits = filler_markers
            .iter()
            .filter(|marker| normalized.contains(**marker))
            .count();

        let short_chatter_penalty = if normalized.split_whitespace().count() <= 6 {
            0.10
        } else {
            0.0
        };

        (filler_hits as f32 * 0.08 + short_chatter_penalty).min(0.35)
    }
}

impl Default for ImportanceScorer {
    fn default() -> Self {
        Self {
            keyword_weight: 1.0,
            priority_weight: 0.25,
            brevity_weight: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ImportanceScorer, ItemKind, ScoredItem};

    #[test]
    fn decision_items_score_high() {
        let scorer = ImportanceScorer::new();
        let item = ScoredItem::new(
            "decision-1",
            ItemKind::Decision,
            "Architecture decision: must keep the security migration plan.",
        );

        assert!(scorer.score(&item) >= 0.9);
    }

    #[test]
    fn filler_notes_score_low() {
        let scorer = ImportanceScorer::new();
        let item = ScoredItem::new("note-1", ItemKind::Note, "ok thanks");

        assert!(scorer.score(&item) < 0.2);
    }

    #[test]
    fn priority_hint_increases_score() {
        let scorer = ImportanceScorer::new();
        let low = ScoredItem::new(
            "turn-low",
            ItemKind::Turn,
            "Investigate the failing API behavior.",
        )
        .with_priority(0.1);
        let high = ScoredItem::new(
            "turn-high",
            ItemKind::Turn,
            "Investigate the failing API behavior.",
        )
        .with_priority(1.0);

        assert!(scorer.score(&high) > scorer.score(&low));
    }

    #[test]
    fn keywords_raise_importance() {
        let scorer = ImportanceScorer::new();
        let plain = ScoredItem::new("doc-1", ItemKind::Document, "Update the notes later.");
        let critical = ScoredItem::new(
            "doc-2",
            ItemKind::Document,
            "Critical security regression: API migration is required before release.",
        );

        assert!(scorer.score(&critical) > scorer.score(&plain));
    }

    #[test]
    fn prune_below_preserves_order_of_kept_items() {
        let scorer = ImportanceScorer::new();
        let items = vec![
            ScoredItem::new(
                "keep-1",
                ItemKind::Decision,
                "Decision: must preserve the database rollback procedure.",
            ),
            ScoredItem::new("drop-1", ItemKind::Note, "sounds good"),
            ScoredItem::new(
                "keep-2",
                ItemKind::Code,
                "Fix failing API migration in src/compactor.rs",
            ),
        ];

        let kept = scorer.prune_below(&items, 0.45);
        let kept_ids = kept.iter().map(|item| item.id.as_str()).collect::<Vec<_>>();

        assert_eq!(kept_ids, vec!["keep-1", "keep-2"]);
    }

    #[test]
    fn scores_are_clamped_to_unit_interval() {
        let scorer = ImportanceScorer::new();
        let item = ScoredItem::new(
            "maxed",
            ItemKind::Decision,
            "Critical required security decision for failing API migration and database blocker.",
        )
        .with_priority(5.0);

        let score = scorer.score(&item);

        assert!((0.0..=1.0).contains(&score));
    }
}
