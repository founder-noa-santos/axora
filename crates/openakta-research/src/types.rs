//! Standardized search query and result types.

use serde::{Deserialize, Serialize};

/// Single web search query (token-efficient surface for agents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search terms.
    pub q: String,
}

/// Normalized hit for LLM context (minimal fields).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

/// Caps payload size before it reaches the LLM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchOptions {
    /// Maximum number of hits to return after normalization.
    pub max_results: usize,
    /// Unicode-safe max snippet length.
    pub max_snippet_chars: usize,
    /// Unicode-safe max title length.
    pub max_title_chars: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            max_results: 5,
            max_snippet_chars: 280,
            max_title_chars: 120,
        }
    }
}
