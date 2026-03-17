//! Hybrid retriever combining vector + BM25 + symbol search

use crate::error::RagError;
use crate::Result;
use axora_indexing::vector_store::{SearchResult as VectorSearchResult, VectorStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::debug;

/// Retrieval result with source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    /// Chunk ID
    pub chunk_id: String,
    /// Score
    pub score: f32,
    /// Source of retrieval
    pub source: RetrievalSource,
    /// Content
    pub content: String,
    /// File path
    pub file_path: String,
    /// Line range
    pub line_range: (usize, usize),
    /// Metadata
    pub metadata: serde_json::Value,
}

/// Source of retrieval
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RetrievalSource {
    /// Vector search (semantic)
    Vector,
    /// BM25 lexical search
    BM25,
    /// Symbol exact match
    Symbol,
}

/// Hybrid retriever combining vector + BM25 + symbol
pub struct HybridRetriever {
    vector_store: Option<Arc<VectorStore>>,
    symbol_index: HashMap<String, Vec<String>>, // symbol -> chunk_ids
}

impl HybridRetriever {
    /// Create new hybrid retriever
    pub fn new() -> Result<Self> {
        Ok(Self {
            vector_store: None,
            symbol_index: HashMap::new(),
        })
    }

    /// Initialize with vector store
    pub fn with_vector_store(mut self, vector_store: VectorStore) -> Self {
        self.vector_store = Some(Arc::new(vector_store));
        self
    }

    /// Add symbol to symbol index
    pub fn add_symbol(&mut self, symbol: &str, chunk_id: &str) {
        self.symbol_index
            .entry(symbol.to_lowercase())
            .or_default()
            .push(chunk_id.to_string());
    }

    /// Retrieve relevant chunks
    pub async fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<RetrievalResult>> {
        debug!("Hybrid retrieval for: {}", query);

        let mut all_results: Vec<RetrievalResult> = Vec::new();

        // 1. Vector search (semantic)
        if let Some(vector_store) = &self.vector_store {
            // Generate query embedding (placeholder - would use embedder in production)
            let query_embedding = vec![0.0f32; 768];

            let vector_results = vector_store
                .search(&query_embedding, limit / 3)
                .await
                .map_err(|e| RagError::Retrieval(e.to_string()))?;

            for result in vector_results {
                all_results.push(RetrievalResult {
                    chunk_id: result.id,
                    score: result.score,
                    source: RetrievalSource::Vector,
                    content: String::new(),
                    file_path: String::new(),
                    line_range: (0, 0),
                    metadata: result.payload,
                });
            }
        }

        // 2. BM25 search (lexical) - placeholder for now
        // Full implementation requires tantivy document indexing

        // 3. Symbol search (exact match)
        let query_lower = query.to_lowercase();
        if let Some(chunk_ids) = self.symbol_index.get(&query_lower) {
            for chunk_id in chunk_ids.iter().take(limit / 3) {
                all_results.push(RetrievalResult {
                    chunk_id: chunk_id.clone(),
                    score: 1.0, // Exact match
                    source: RetrievalSource::Symbol,
                    content: String::new(),
                    file_path: String::new(),
                    line_range: (0, 0),
                    metadata: serde_json::Value::Null,
                });
            }
        }

        // Apply Reciprocal Rank Fusion
        let fused_results = self.reciprocal_rank_fusion(all_results);

        // Take top results
        Ok(fused_results.into_iter().take(limit).collect())
    }

    /// Reciprocal Rank Fusion
    fn reciprocal_rank_fusion(&self, results: Vec<RetrievalResult>) -> Vec<RetrievalResult> {
        const K: f32 = 60.0;

        // Group by source
        let mut by_source: HashMap<RetrievalSource, Vec<&RetrievalResult>> = HashMap::new();
        for result in &results {
            by_source
                .entry(result.source.clone())
                .or_default()
                .push(result);
        }

        // Calculate RRF scores
        let mut rrf_scores: HashMap<String, f32> = HashMap::new();
        for (_source, source_results) in by_source {
            for (rank, result) in source_results.iter().enumerate() {
                let score = 1.0 / (K + rank as f32);
                *rrf_scores.entry(result.chunk_id.clone()).or_insert(0.0) += score;
            }
        }

        // Sort by RRF score
        let mut sorted: Vec<_> = rrf_scores.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Reconstruct results
        let result_map: HashMap<_, _> = results
            .into_iter()
            .map(|r| (r.chunk_id.clone(), r))
            .collect();

        sorted
            .into_iter()
            .filter_map(|(id, _)| result_map.get(&id).cloned())
            .collect()
    }

    /// Get symbol index size
    pub fn symbol_count(&self) -> usize {
        self.symbol_index.len()
    }
}

impl Default for HybridRetriever {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retriever_creation() {
        let retriever = HybridRetriever::new().unwrap();
        assert_eq!(retriever.symbol_count(), 0);
    }

    #[tokio::test]
    async fn test_symbol_search() {
        let mut retriever = HybridRetriever::new().unwrap();

        // Add symbols
        retriever.add_symbol("MyClass", "chunk1");
        retriever.add_symbol("MyClass", "chunk2");
        retriever.add_symbol("my_function", "chunk3");

        let results = retriever.retrieve("MyClass", 10).await.unwrap();

        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| matches!(r.source, RetrievalSource::Symbol)));
    }

    #[test]
    fn test_rrf_fusion() {
        let retriever = HybridRetriever::new().unwrap();

        let results = vec![
            RetrievalResult {
                chunk_id: "a".to_string(),
                score: 0.9,
                source: RetrievalSource::Vector,
                content: String::new(),
                file_path: String::new(),
                line_range: (0, 0),
                metadata: serde_json::Value::Null,
            },
            RetrievalResult {
                chunk_id: "b".to_string(),
                score: 0.8,
                source: RetrievalSource::BM25,
                content: String::new(),
                file_path: String::new(),
                line_range: (0, 0),
                metadata: serde_json::Value::Null,
            },
        ];

        let fused = retriever.reciprocal_rank_fusion(results);
        assert_eq!(fused.len(), 2);
    }
}
