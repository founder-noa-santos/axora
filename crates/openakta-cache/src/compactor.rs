//! Context compaction for long-running agent conversations and execution logs.
//!
//! This module combines rolling summaries, hierarchical memory, and importance
//! scoring to keep critical context while reducing token usage.

pub mod hierarchical_memory;
pub mod importance_scorer;
pub mod rolling_summary;

use hierarchical_memory::HierarchicalMemory;
use importance_scorer::{ImportanceScorer, ItemKind, ScoredItem};
use rolling_summary::{RollingSummary, Turn};
use thiserror::Error;

/// Result type for context compaction.
pub type Result<T> = std::result::Result<T, CompactorError>;

/// Errors produced by the context compactor.
#[derive(Debug, Error)]
pub enum CompactorError {
    /// The compactor could not produce output within the configured limits.
    #[error("unable to compact context within token budget")]
    BudgetExceeded,
}

/// A single unit of source context.
#[derive(Debug, Clone, PartialEq)]
pub struct ContextEntry {
    /// Stable identifier for the entry.
    pub id: String,
    /// Source role for the entry, such as `user`, `assistant`, or `system`.
    pub role: String,
    /// Full entry content.
    pub content: String,
    /// Semantic kind used by the importance scorer.
    pub kind: ItemKind,
    /// Priority hint in the 0.0-1.0 range.
    pub priority: f32,
}

impl ContextEntry {
    /// Creates a new conversational turn entry.
    pub fn new(id: impl Into<String>, role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            role: role.into(),
            content: content.into(),
            kind: ItemKind::Turn,
            priority: 0.5,
        }
    }

    /// Applies a semantic kind used during scoring.
    pub fn with_kind(mut self, kind: ItemKind) -> Self {
        self.kind = kind;
        self
    }

    /// Applies a priority hint used during scoring.
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority.clamp(0.0, 1.0);
        self
    }

    /// Estimates token count for this entry.
    pub fn token_count(&self) -> usize {
        estimate_tokens(&self.content)
    }
}

/// Source context to compact.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    /// Ordered source entries.
    pub entries: Vec<ContextEntry>,
}

impl Context {
    /// Creates an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a context from prebuilt entries.
    pub fn from_entries(entries: Vec<ContextEntry>) -> Self {
        Self { entries }
    }

    /// Appends an entry.
    pub fn add_entry(&mut self, entry: ContextEntry) {
        self.entries.push(entry);
    }

    /// Returns true when the context has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Estimates total tokens in the source context.
    pub fn token_count(&self) -> usize {
        self.entries.iter().map(ContextEntry::token_count).sum()
    }
}

/// Configuration for [`ContextCompactor`].
#[derive(Debug, Clone, PartialEq)]
pub struct CompactorConfig {
    /// Number of newest turns to preserve in full.
    pub recent_turns_full: usize,
    /// Number of preceding turns to keep as summarized mid-context.
    pub mid_turns_summarized: usize,
    /// Number of older turns to treat as archived history.
    pub old_turns_archived: usize,
    /// Minimum importance score required to keep an item.
    pub importance_threshold: f32,
    /// Maximum token budget for compacted output.
    pub max_tokens: usize,
}

impl Default for CompactorConfig {
    fn default() -> Self {
        Self {
            recent_turns_full: 10,
            mid_turns_summarized: 40,
            old_turns_archived: 50,
            importance_threshold: 0.3,
            max_tokens: 20_000,
        }
    }
}

/// Compacted context and metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct CompactContext {
    /// Rendered compacted content.
    pub content: String,
    /// Estimated source token count.
    pub original_tokens: usize,
    /// Estimated compacted token count.
    pub compacted_tokens: usize,
    /// Compression ratio expressed as saved-token fraction in the 0.0-1.0 range.
    pub compression_ratio: f32,
}

/// Unified context compactor.
#[derive(Debug, Clone)]
pub struct ContextCompactor {
    /// Rolling summary of raw turns.
    pub rolling_summary: RollingSummary,
    /// Hierarchical memory over the same context.
    pub hierarchical: HierarchicalMemory,
    /// Importance scorer used for pruning.
    pub scorer: ImportanceScorer,
    /// Compaction configuration.
    pub config: CompactorConfig,
}

#[derive(Debug, Clone)]
struct Fragment {
    id: String,
    order: usize,
    kind: ItemKind,
    content: String,
    priority: f32,
}

impl ContextCompactor {
    /// Creates a new compactor.
    pub fn new(config: CompactorConfig) -> Self {
        let rolling_summary = RollingSummary::new(config.recent_turns_full);
        let mid_limit = config.recent_turns_full + config.mid_turns_summarized;
        let hierarchical = HierarchicalMemory::with_limits(config.recent_turns_full, mid_limit);

        Self {
            rolling_summary,
            hierarchical,
            scorer: ImportanceScorer::new(),
            config,
        }
    }

    /// Compacts a full context into a token-bounded summary.
    pub fn compact(&self, context: &Context) -> Result<CompactContext> {
        let original_tokens = context.token_count();

        if context.is_empty() {
            return Ok(CompactContext {
                content: String::new(),
                original_tokens: 0,
                compacted_tokens: 0,
                compression_ratio: 0.0,
            });
        }

        let mut rolling = RollingSummary::new(self.config.recent_turns_full);
        let mut hierarchical = HierarchicalMemory::with_limits(
            self.config.recent_turns_full,
            self.config.recent_turns_full + self.config.mid_turns_summarized,
        );

        for entry in &context.entries {
            rolling.add_turn(Turn::new(entry.role.clone(), entry.content.clone()));
            hierarchical.add_entry(entry.role.clone(), entry.content.clone());
        }

        let fragments = self.build_fragments(context, &rolling, &hierarchical);
        let scored = self.to_scored_items(&fragments);
        let compacted = self.prune_to_budget(&fragments, &scored)?;
        let compacted_tokens = estimate_tokens(&compacted);

        Ok(CompactContext {
            content: compacted,
            original_tokens,
            compacted_tokens,
            compression_ratio: self.compression_ratio(original_tokens, compacted_tokens),
        })
    }

    /// Returns the saved-token fraction for a compaction operation.
    pub fn compression_ratio(&self, original: usize, compacted: usize) -> f32 {
        if original == 0 {
            0.0
        } else {
            (original.saturating_sub(compacted) as f32) / original as f32
        }
    }

    fn build_fragments(
        &self,
        context: &Context,
        rolling: &RollingSummary,
        hierarchical: &HierarchicalMemory,
    ) -> Vec<Fragment> {
        let mut fragments = Vec::new();
        if let Some(history_summary) = historical_portion(&rolling.summarize()) {
            fragments.push(Fragment {
                id: "rolling-summary".to_string(),
                order: 0,
                kind: ItemKind::Document,
                content: format!("Rolling summary\n{history_summary}"),
                priority: 0.9,
            });
        }

        let hierarchy = hierarchical.get_context();

        if let Some(old) = hierarchy.old {
            fragments.push(Fragment {
                id: "hierarchy-old".to_string(),
                order: 1,
                kind: ItemKind::Document,
                content: format!(
                    "Archived history [{}-{}] ({} entries)\n{}",
                    old.start_index, old.end_index, old.entry_count, old.summary
                ),
                priority: 0.75,
            });
        }

        for (idx, summary) in hierarchy.mid.iter().enumerate() {
            fragments.push(Fragment {
                id: format!("hierarchy-mid-{idx}"),
                order: 10 + idx,
                kind: ItemKind::Document,
                content: format!(
                    "Mid history [{}-{}] ({} entries)\n{}",
                    summary.start_index, summary.end_index, summary.entry_count, summary.summary
                ),
                priority: 0.6,
            });
        }

        let recent_offset = 100;
        let total = context.entries.len().max(1);
        let recent_start = context
            .entries
            .len()
            .saturating_sub(self.config.recent_turns_full);
        for (idx, entry) in context.entries.iter().enumerate().skip(recent_start) {
            let recency = (idx + 1) as f32 / total as f32;
            let priority = ((entry.priority * 0.7) + (recency * 0.3)).clamp(0.0, 1.0);

            fragments.push(Fragment {
                id: entry.id.clone(),
                order: recent_offset + idx,
                kind: entry.kind,
                content: format!("[{}] {}", entry.role, entry.content),
                priority,
            });
        }

        fragments
    }

    fn to_scored_items(&self, fragments: &[Fragment]) -> Vec<ScoredItem> {
        fragments
            .iter()
            .map(|fragment| {
                ScoredItem::new(fragment.id.clone(), fragment.kind, fragment.content.clone())
                    .with_priority(fragment.priority)
            })
            .collect()
    }

    fn prune_to_budget(&self, fragments: &[Fragment], scored: &[ScoredItem]) -> Result<String> {
        let mut best_render = None::<String>;
        let mut best_tokens = usize::MAX;
        let mut threshold = self.config.importance_threshold.clamp(0.0, 1.0);

        while threshold <= 1.0 {
            let kept_ids = self
                .scorer
                .prune_below(scored, threshold)
                .into_iter()
                .map(|item| item.id)
                .collect::<Vec<_>>();

            let mut kept = fragments
                .iter()
                .filter(|fragment| kept_ids.contains(&fragment.id))
                .cloned()
                .collect::<Vec<_>>();

            if kept.is_empty() {
                if let Some(top) = self.highest_scoring_fragment(fragments, scored) {
                    kept.push(top.clone());
                }
            }

            let render = self.render_fragments(&kept);
            let tokens = estimate_tokens(&render);

            if tokens < best_tokens {
                best_tokens = tokens;
                best_render = Some(render.clone());
            }

            if tokens <= self.config.max_tokens {
                return Ok(render);
            }

            let tight_render = self.fit_highest_scoring(fragments, scored);
            let tight_tokens = estimate_tokens(&tight_render);
            if tight_tokens <= self.config.max_tokens {
                return Ok(tight_render);
            }

            if tight_tokens < best_tokens {
                best_tokens = tight_tokens;
                best_render = Some(tight_render);
            }

            threshold += 0.1;
        }

        best_render
            .filter(|render| !render.is_empty())
            .ok_or(CompactorError::BudgetExceeded)
    }

    fn highest_scoring_fragment<'a>(
        &self,
        fragments: &'a [Fragment],
        scored: &[ScoredItem],
    ) -> Option<&'a Fragment> {
        fragments
            .iter()
            .zip(scored.iter())
            .max_by(|(_, left), (_, right)| {
                self.scorer.score(left).total_cmp(&self.scorer.score(right))
            })
            .map(|(fragment, _)| fragment)
    }

    fn fit_highest_scoring(&self, fragments: &[Fragment], scored: &[ScoredItem]) -> String {
        let mut scored_fragments = fragments
            .iter()
            .zip(scored.iter())
            .map(|(fragment, scored)| (fragment.clone(), self.scorer.score(scored)))
            .collect::<Vec<_>>();

        scored_fragments.sort_by(
            |(left_fragment, left_score), (right_fragment, right_score)| {
                right_score
                    .total_cmp(left_score)
                    .then(left_fragment.order.cmp(&right_fragment.order))
            },
        );

        let mut selected = Vec::new();
        for (fragment, _) in scored_fragments {
            let mut candidate = selected.clone();
            candidate.push(fragment.clone());
            let render = self.render_fragments(&candidate);
            if estimate_tokens(&render) <= self.config.max_tokens || selected.is_empty() {
                selected.push(fragment);
            }
        }

        self.render_fragments(&selected)
    }

    fn render_fragments(&self, fragments: &[Fragment]) -> String {
        let mut ordered = fragments.to_vec();
        ordered.sort_by_key(|fragment| fragment.order);

        ordered
            .into_iter()
            .map(|fragment| fragment.content)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

fn estimate_tokens(content: &str) -> usize {
    content.chars().count().div_ceil(4).max(1)
}

fn historical_portion(summary: &str) -> Option<String> {
    let trimmed = summary.trim();
    if trimmed.is_empty()
        || trimmed == "No conversation history."
        || trimmed.starts_with("Recent turns:\n")
    {
        return None;
    }

    let historical = summary
        .split("\n\nRecent turns:\n")
        .next()
        .map(str::trim)
        .unwrap_or_default();

    if historical.is_empty() || historical == "No conversation history." {
        None
    } else {
        Some(historical.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{CompactorConfig, Context, ContextCompactor, ContextEntry};
    use crate::compactor::importance_scorer::ItemKind;

    #[test]
    fn compression_ratio_reports_saved_fraction() {
        let compactor = ContextCompactor::new(CompactorConfig::default());
        let ratio = compactor.compression_ratio(1_000, 250);

        assert!((ratio - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn compact_empty_context_returns_zero_metrics() {
        let context = Context::new();
        let compacted = ContextCompactor::new(CompactorConfig::default())
            .compact(&context)
            .expect("empty context should compact");

        assert_eq!(compacted.content, "");
        assert_eq!(compacted.original_tokens, 0);
        assert_eq!(compacted.compacted_tokens, 0);
    }

    #[test]
    fn compact_keeps_high_signal_content() {
        let mut context = Context::new();
        context.add_entry(ContextEntry::new("1", "user", "ok thanks").with_kind(ItemKind::Note));
        context.add_entry(
            ContextEntry::new(
                "2",
                "assistant",
                "Architecture decision: must preserve the database migration plan.",
            )
            .with_kind(ItemKind::Decision)
            .with_priority(1.0),
        );

        let compacted = ContextCompactor::new(CompactorConfig {
            max_tokens: 80,
            ..CompactorConfig::default()
        })
        .compact(&context)
        .expect("context should compact");

        assert!(compacted.content.contains("Architecture decision"));
    }
}
