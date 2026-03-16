//! Context builder with reordering

use crate::retriever::RetrievalResult;
use crate::Result;

/// Context builder
pub struct ContextBuilder {
    max_tokens: usize,
}

impl ContextBuilder {
    /// Create new context builder
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens }
    }

    /// Build context from retrieval results
    pub fn build(&self, results: Vec<RetrievalResult>, query: &str) -> Result<String> {
        // TODO: Implement "Lost in the Middle" reordering
        // For now, just concatenate

        let mut context = String::new();
        let mut token_count = 0;

        for result in results {
            let chunk_tokens = result.content.len() / 4; // Rough estimate

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
