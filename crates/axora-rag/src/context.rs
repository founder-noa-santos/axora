//! Prompt-context helper for ranked retrieval payloads.

use crate::Result;

/// Minimal chunk used for context assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextChunk {
    /// Stable identifier.
    pub chunk_id: String,
    /// Rendered content.
    pub content: String,
}

/// Context builder.
pub struct ContextBuilder {
    max_tokens: usize,
}

impl ContextBuilder {
    /// Create new context builder.
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Build context from ordered chunks.
    pub fn build(&self, results: Vec<ContextChunk>, _query: &str) -> Result<String> {
        let mut context = String::new();
        let mut token_count = 0usize;

        for result in results {
            let chunk_tokens = result.content.len() / 4;
            if token_count + chunk_tokens > self.max_tokens {
                break;
            }
            context.push_str(&format!(
                "\n\n=== Chunk {} ===\n{}\n",
                result.chunk_id, result.content
            ));
            token_count += chunk_tokens;
        }

        Ok(context)
    }
}

impl Default for ContextBuilder {
    fn default() -> Self {
        Self::new(8192)
    }
}
