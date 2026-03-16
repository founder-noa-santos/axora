//! MemGAS Retrieval
//!
//! This module implements multi-granularity retrieval:
//! - **GMM clustering** (Accept Set vs Reject Set)
//! - **Entropy-based routing** (select optimal granularity)
//! - **Association graph** for semantic edges
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                  MemGAS Retriever                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │  GMM Clustering             │  Entropy Router               │
//! │  - Accept Set (relevant)    │  - Auto granularity select    │
//! │  - Reject Set (noise)       │  - Turn vs Summary            │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │              Association Graph                              │
//! │  - Semantic edges between memories                          │
//! │  - Keyword clusters                                         │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// MemGAS retrieval errors
#[derive(Error, Debug)]
pub enum MemGASError {
    /// Feature extraction failed
    #[error("feature extraction failed: {0}")]
    FeatureExtraction(String),

    /// Clustering failed
    #[error("clustering failed: {0}")]
    ClusteringFailed(String),

    /// Memory not found
    #[error("memory not found: {0}")]
    NotFound(String),

    /// Invalid granularity
    #[error("invalid granularity: {0}")]
    InvalidGranularity(String),
}

/// Result type for MemGAS operations
pub type Result<T> = std::result::Result<T, MemGASError>;

/// Granularity level for retrieval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Granularity {
    /// Auto-select based on entropy
    Auto,
    /// Individual turn-level memories
    TurnLevel,
    /// Session-level summaries
    SessionSummary,
    /// Keyword-based clusters
    KeywordCluster,
}

/// Memory entity for retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    /// Unique identifier
    pub id: String,
    /// Session identifier
    pub session_id: String,
    /// Turn number (for turn-level)
    pub turn_number: Option<i32>,
    /// Content
    pub content: String,
    /// Keywords/Tags
    pub keywords: Vec<String>,
    /// Embedding vector (for similarity)
    pub embedding: Vec<f32>,
    /// Relevance score
    pub relevance_score: f32,
}

impl Memory {
    /// Create new memory
    pub fn new(
        id: &str,
        session_id: &str,
        content: &str,
        keywords: Vec<String>,
        embedding: Vec<f32>,
    ) -> Self {
        Self {
            id: id.to_string(),
            session_id: session_id.to_string(),
            turn_number: None,
            content: content.to_string(),
            keywords,
            embedding,
            relevance_score: 0.5,
        }
    }

    /// Create turn-level memory
    pub fn turn_level(id: &str, session_id: &str, turn: i32, content: &str) -> Self {
        Self {
            id: id.to_string(),
            session_id: session_id.to_string(),
            turn_number: Some(turn),
            content: content.to_string(),
            keywords: Vec::new(),
            embedding: Vec::new(),
            relevance_score: 0.5,
        }
    }

    /// Create session summary memory
    pub fn session_summary(id: &str, session_id: &str, summary: &str) -> Self {
        Self {
            id: id.to_string(),
            session_id: session_id.to_string(),
            turn_number: None,
            content: summary.to_string(),
            keywords: Vec::new(),
            embedding: Vec::new(),
            relevance_score: 0.5,
        }
    }
}

/// GMM clustering result
#[derive(Debug, Clone)]
pub struct GMMClustering {
    /// Accept Set (relevant memories)
    pub accept_set: Vec<Memory>,
    /// Reject Set (noise memories)
    pub reject_set: Vec<Memory>,
}

/// Context payload for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPayload {
    /// Retrieved memories
    pub memories: Vec<Memory>,
    /// Granularity used
    pub granularity: Granularity,
    /// Entropy score
    pub entropy: f32,
    /// Total tokens (estimated)
    pub estimated_tokens: usize,
}

impl ContextPayload {
    /// Create new context payload
    pub fn new(memories: Vec<Memory>, granularity: Granularity, entropy: f32) -> Self {
        let estimated_tokens = memories.iter().map(|m| m.content.len() / 4).sum();
        Self {
            memories,
            granularity,
            entropy,
            estimated_tokens,
        }
    }
}

/// Memory association graph
#[derive(Debug, Clone)]
pub struct MemoryAssociationGraph {
    /// Nodes (memory IDs)
    nodes: HashSet<String>,
    /// Edges (memory_id -> related_memory_ids)
    edges: HashMap<String, HashSet<String>>,
    /// Keyword index (keyword -> memory_ids)
    keyword_index: HashMap<String, HashSet<String>>,
}

impl MemoryAssociationGraph {
    /// Create new association graph
    pub fn new() -> Self {
        Self {
            nodes: HashSet::new(),
            edges: HashMap::new(),
            keyword_index: HashMap::new(),
        }
    }

    /// Add memory to graph
    pub fn add_memory(&mut self, memory: &Memory) {
        self.nodes.insert(memory.id.clone());

        // Index by keywords
        for keyword in &memory.keywords {
            self.keyword_index
                .entry(keyword.clone())
                .or_default()
                .insert(memory.id.clone());
        }
    }

    /// Add semantic edge between memories
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.entry(from.to_string()).or_default().insert(to.to_string());
        self.edges.entry(to.to_string()).or_default().insert(from.to_string());
    }

    /// Get related memories by keyword
    pub fn get_by_keyword(&self, keyword: &str) -> Vec<&String> {
        self.keyword_index
            .get(keyword)
            .map(|ids| ids.iter().collect())
            .unwrap_or_default()
    }

    /// Get related memories by edge
    pub fn get_related(&self, memory_id: &str) -> Vec<&String> {
        self.edges
            .get(memory_id)
            .map(|ids| ids.iter().collect())
            .unwrap_or_default()
    }

    /// Get all nodes
    pub fn all_nodes(&self) -> Vec<&String> {
        self.nodes.iter().collect()
    }

    /// Get graph statistics
    pub fn get_stats(&self) -> GraphStats {
        GraphStats {
            node_count: self.nodes.len(),
            edge_count: self.edges.values().map(|s| s.len()).sum::<usize>() / 2,
            keyword_count: self.keyword_index.len(),
        }
    }
}

impl Default for MemoryAssociationGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub keyword_count: usize,
}

/// High entropy threshold (for routing)
pub const HIGH_ENTROPY_THRESHOLD: f32 = 0.7;

/// MemGAS retriever
pub struct MemGASRetriever {
    memories: Vec<Memory>,
    association_graph: MemoryAssociationGraph,
    /// Number of GMM components
    n_components: usize,
}

impl MemGASRetriever {
    /// Create new retriever
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
            association_graph: MemoryAssociationGraph::new(),
            n_components: 2, // Accept vs Reject
        }
    }

    /// Create retriever with custom GMM components
    pub fn with_components(n_components: usize) -> Self {
        Self {
            memories: Vec::new(),
            association_graph: MemoryAssociationGraph::new(),
            n_components,
        }
    }

    /// Add memory to retriever
    pub fn add_memory(&mut self, memory: Memory) {
        self.association_graph.add_memory(&memory);
        self.memories.push(memory);
    }

    /// Add memories in batch
    pub fn add_memories(&mut self, memories: Vec<Memory>) {
        for memory in &memories {
            self.association_graph.add_memory(memory);
        }
        self.memories.extend(memories);
    }

    /// Add semantic edge between memories
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.association_graph.add_edge(from, to);
    }

    /// Convert memories to feature vectors for GMM
    fn memories_to_features(&self, memories: &[Memory]) -> Result<Array2<f32>> {
        if memories.is_empty() {
            return Err(MemGASError::FeatureExtraction(
                "No memories to convert".to_string(),
            ));
        }

        // Feature dimensions: relevance_score, content_length (normalized), keyword_count
        let n_features = 3;
        let mut features = Array2::zeros((memories.len(), n_features));

        // Calculate max content length for normalization
        let max_len = memories
            .iter()
            .map(|m| m.content.len() as f32)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(1.0)
            .max(1.0);

        for (i, memory) in memories.iter().enumerate() {
            features[[i, 0]] = memory.relevance_score;
            features[[i, 1]] = (memory.content.len() as f32) / max_len;
            features[[i, 2]] = (memory.keywords.len() as f32) / 10.0; // Normalize by 10
        }

        Ok(features)
    }

    /// Cluster memories using GMM (simplified implementation)
    pub fn cluster_memories(&self, memories: &[Memory]) -> Result<GMMClustering> {
        if memories.is_empty() {
            return Ok(GMMClustering {
                accept_set: Vec::new(),
                reject_set: Vec::new(),
            });
        }

        // Convert to features
        let features = self.memories_to_features(memories)?;

        // Simplified GMM: use relevance score threshold
        // In production, would use actual linfa GMM
        let threshold = self.calculate_gmm_threshold(&features);

        let mut accept_set = Vec::new();
        let mut reject_set = Vec::new();

        for (i, memory) in memories.iter().enumerate() {
            let relevance = features[[i, 0]];
            if relevance >= threshold {
                accept_set.push(memory.clone());
            } else {
                reject_set.push(memory.clone());
            }
        }

        Ok(GMMClustering {
            accept_set,
            reject_set,
        })
    }

    /// Calculate GMM threshold (simplified)
    fn calculate_gmm_threshold(&self, features: &Array2<f32>) -> f32 {
        // Calculate mean relevance score
        let n = features.nrows();
        let sum: f32 = (0..n).map(|i| features[[i, 0]]).sum();
        let mean = sum / n as f32;

        // Use mean as threshold (simplified GMM decision boundary)
        mean
    }

    /// Calculate entropy for query
    pub fn calculate_entropy(&self, query: &str) -> Result<f32> {
        if self.memories.is_empty() {
            return Ok(0.0);
        }

        // Calculate relevance distribution
        let query_lower = query.to_lowercase();
        let mut relevance_counts: HashMap<f32, usize> = HashMap::new();

        for memory in &self.memories {
            // Simple keyword matching for entropy calculation
            let matches = memory
                .keywords
                .iter()
                .any(|k| query_lower.contains(&k.to_lowercase()))
                || query_lower.contains(&memory.content.to_lowercase());

            let relevance = if matches {
                memory.relevance_score
            } else {
                0.0
            };

            // Bin relevance into buckets
            let bucket = (relevance * 10.0).round() / 10.0;
            *relevance_counts.entry(bucket).or_insert(0) += 1;
        }

        // Calculate Shannon entropy
        let total = self.memories.len() as f32;
        let entropy: f32 = relevance_counts
            .values()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f32 / total;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();

        // Normalize entropy (max entropy for 10 buckets is ~3.32)
        Ok((entropy / 3.32).min(1.0))
    }

    /// Retrieve by turn level
    pub fn retrieve_by_turn(&self, query: &str, limit: usize) -> Result<ContextPayload> {
        let query_lower = query.to_lowercase();

        // Filter and sort by relevance
        let mut relevant: Vec<&Memory> = self
            .memories
            .iter()
            .filter(|m| {
                m.turn_number.is_some()
                    && (query_lower.contains(&m.content.to_lowercase())
                        || m.keywords.iter().any(|k| query_lower.contains(&k.to_lowercase())))
            })
            .collect();

        relevant.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap()
        });

        let memories: Vec<Memory> = relevant.into_iter().take(limit).cloned().collect();
        let entropy = self.calculate_entropy(query)?;

        Ok(ContextPayload::new(
            memories,
            Granularity::TurnLevel,
            entropy,
        ))
    }

    /// Retrieve by session summary
    pub fn retrieve_by_summary(&self, query: &str, limit: usize) -> Result<ContextPayload> {
        let query_lower = query.to_lowercase();

        // Filter summaries (no turn_number)
        let mut relevant: Vec<&Memory> = self
            .memories
            .iter()
            .filter(|m| {
                m.turn_number.is_none()
                    && (query_lower.contains(&m.content.to_lowercase())
                        || m.keywords.iter().any(|k| query_lower.contains(&k.to_lowercase())))
            })
            .collect();

        relevant.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap()
        });

        let memories: Vec<Memory> = relevant.into_iter().take(limit).cloned().collect();
        let entropy = self.calculate_entropy(query)?;

        Ok(ContextPayload::new(
            memories,
            Granularity::SessionSummary,
            entropy,
        ))
    }

    /// Retrieve by keyword cluster
    pub fn retrieve_by_keywords(&self, query: &str, limit: usize) -> Result<ContextPayload> {
        let query_lower = query.to_lowercase();

        // Extract keywords from query
        let query_keywords: Vec<String> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .map(|s| s.to_string())
            .collect();

        // Find memories matching keywords
        let mut scores: HashMap<String, f32> = HashMap::new();

        for memory in &self.memories {
            let mut score = 0.0;
            for keyword in &query_keywords {
                if memory.keywords.iter().any(|k| k.contains(keyword)) {
                    score += 1.0;
                }
                if memory.content.to_lowercase().contains(keyword) {
                    score += 0.5;
                }
            }
            if score > 0.0 {
                scores.insert(memory.id.clone(), score);
            }
        }

        // Sort by score
        let mut sorted: Vec<_> = scores.iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

        let memories: Vec<Memory> = sorted
            .into_iter()
            .take(limit)
            .filter_map(|(id, _)| self.memories.iter().find(|m| &m.id == id).cloned())
            .collect();

        let entropy = self.calculate_entropy(query)?;

        Ok(ContextPayload::new(
            memories,
            Granularity::KeywordCluster,
            entropy,
        ))
    }

    /// Entropy-based router selects optimal granularity
    pub fn retrieve(&self, query: &str, granularity: Granularity, limit: usize) -> Result<ContextPayload> {
        match granularity {
            Granularity::Auto => {
                let entropy = self.calculate_entropy(query)?;

                if entropy > HIGH_ENTROPY_THRESHOLD {
                    // High entropy → use summary (broader context)
                    self.retrieve_by_summary(query, limit)
                } else {
                    // Low entropy → use turn-level (specific context)
                    self.retrieve_by_turn(query, limit)
                }
            }
            Granularity::TurnLevel => self.retrieve_by_turn(query, limit),
            Granularity::SessionSummary => self.retrieve_by_summary(query, limit),
            Granularity::KeywordCluster => self.retrieve_by_keywords(query, limit),
        }
    }

    /// Get all memories
    pub fn all_memories(&self) -> &[Memory] {
        &self.memories
    }

    /// Get association graph
    pub fn association_graph(&self) -> &MemoryAssociationGraph {
        &self.association_graph
    }

    /// Get retriever statistics
    pub fn get_stats(&self) -> RetrieverStats {
        let clustering = self.cluster_memories(&self.memories).unwrap_or(GMMClustering {
            accept_set: Vec::new(),
            reject_set: Vec::new(),
        });

        RetrieverStats {
            total_memories: self.memories.len(),
            accept_set_size: clustering.accept_set.len(),
            reject_set_size: clustering.reject_set.len(),
            graph_stats: self.association_graph.get_stats(),
        }
    }
}

impl Default for MemGASRetriever {
    fn default() -> Self {
        Self::new()
    }
}

/// Retriever statistics
#[derive(Debug, Clone)]
pub struct RetrieverStats {
    pub total_memories: usize,
    pub accept_set_size: usize,
    pub reject_set_size: usize,
    pub graph_stats: GraphStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_memories() -> Vec<Memory> {
        vec![
            Memory::turn_level("m1", "s1", 1, "User asked about authentication"),
            Memory::turn_level("m2", "s1", 2, "Checked JWT token"),
            Memory::turn_level("m3", "s1", 3, "Token was expired"),
            Memory::turn_level("m4", "s1", 4, "Refreshed token"),
            Memory::turn_level("m5", "s1", 5, "Authentication successful"),
            Memory::session_summary("m6", "s1", "Session: User authenticated with JWT"),
        ]
    }

    #[test]
    fn test_gmm_clustering() {
        let mut retriever = MemGASRetriever::new();
        let memories = create_test_memories();

        // Set different relevance scores for clustering
        for (i, memory) in memories.iter().enumerate() {
            let mut m = memory.clone();
            m.relevance_score = if i < 3 { 0.8 } else { 0.3 };
            retriever.add_memory(m);
        }

        let clustering = retriever.cluster_memories(retriever.all_memories()).unwrap();

        // Should separate into accept/reject sets
        assert!(!clustering.accept_set.is_empty());
        assert!(!clustering.reject_set.is_empty());
    }

    #[test]
    fn test_accept_reject_sets() {
        let mut retriever = MemGASRetriever::new();

        // Add high-relevance memories
        for i in 0..5 {
            let mut m = Memory::turn_level(&format!("high_{}", i), "s1", i as i32, "Important content");
            m.relevance_score = 0.9;
            retriever.add_memory(m);
        }

        // Add low-relevance memories
        for i in 0..5 {
            let mut m = Memory::turn_level(&format!("low_{}", i), "s1", i as i32, "Noise content");
            m.relevance_score = 0.1;
            retriever.add_memory(m);
        }

        let clustering = retriever.cluster_memories(retriever.all_memories()).unwrap();

        // High relevance should be in accept set
        assert!(clustering.accept_set.iter().any(|m| m.relevance_score > 0.5));
        // Low relevance should be in reject set
        assert!(clustering.reject_set.iter().any(|m| m.relevance_score < 0.5));
    }

    #[test]
    fn test_entropy_calculation() {
        let mut retriever = MemGASRetriever::new();

        // Add memories with varied relevance
        for i in 0..10 {
            let mut m = Memory::turn_level(&format!("m{}", i), "s1", i as i32, &format!("Content {}", i));
            m.relevance_score = (i as f32) / 10.0;
            retriever.add_memory(m);
        }

        let entropy = retriever.calculate_entropy("test query").unwrap();

        // Entropy should be between 0 and 1
        assert!(entropy >= 0.0);
        assert!(entropy <= 1.0);
    }

    #[test]
    fn test_entropy_based_routing() {
        let mut retriever = MemGASRetriever::new();

        // Add turn-level memories
        for i in 0..5 {
            let mut m = Memory::turn_level(&format!("m{}", i), "s1", i as i32, "Specific turn content");
            m.relevance_score = 0.8;
            retriever.add_memory(m);
        }

        // Add session summary
        let mut summary = Memory::session_summary("summary", "s1", "Session summary with broad context");
        summary.relevance_score = 0.7;
        retriever.add_memory(summary);

        // Test auto routing
        let result = retriever.retrieve("authentication", Granularity::Auto, 5).unwrap();

        // Should return some results
        assert!(!result.memories.is_empty());
    }

    #[test]
    fn test_turn_level_retrieval() {
        let mut retriever = MemGASRetriever::new();
        let memories = create_test_memories();
        retriever.add_memories(memories);

        let result = retriever.retrieve_by_turn("authentication", 3).unwrap();

        assert_eq!(result.granularity, Granularity::TurnLevel);
        assert!(result.memories.len() <= 3);
    }

    #[test]
    fn test_summary_retrieval() {
        let mut retriever = MemGASRetriever::new();
        let memories = create_test_memories();
        retriever.add_memories(memories);

        let result = retriever.retrieve_by_summary("session", 2).unwrap();

        assert_eq!(result.granularity, Granularity::SessionSummary);
        assert!(result.memories.len() <= 2);
    }

    #[test]
    fn test_keyword_cluster_retrieval() {
        let mut retriever = MemGASRetriever::new();

        // Add memories with keywords
        let mut m1 = Memory::turn_level("m1", "s1", 1, "JWT authentication");
        m1.keywords = vec!["auth".to_string(), "jwt".to_string(), "security".to_string()];
        retriever.add_memory(m1);

        let mut m2 = Memory::turn_level("m2", "s1", 2, "Token refresh");
        m2.keywords = vec!["token".to_string(), "refresh".to_string()];
        retriever.add_memory(m2);

        let result = retriever.retrieve_by_keywords("auth jwt", 5).unwrap();

        assert_eq!(result.granularity, Granularity::KeywordCluster);
        assert!(!result.memories.is_empty());
    }

    #[test]
    fn test_association_graph() {
        let mut retriever = MemGASRetriever::new();

        let mut m1 = Memory::turn_level("m1", "s1", 1, "Content 1");
        m1.keywords = vec!["auth".to_string()];
        retriever.add_memory(m1);

        let mut m2 = Memory::turn_level("m2", "s1", 2, "Content 2");
        m2.keywords = vec!["auth".to_string()];
        retriever.add_memory(m2);

        // Add edge
        retriever.add_edge("m1", "m2");

        // Check graph
        let graph = retriever.association_graph();
        let related = graph.get_related("m1");
        assert!(related.contains(&&"m2".to_string()));
    }

    #[test]
    fn test_retriever_stats() {
        let mut retriever = MemGASRetriever::new();

        for i in 0..10 {
            let mut m = Memory::turn_level(&format!("m{}", i), "s1", i as i32, &format!("Content {}", i));
            m.relevance_score = (i as f32) / 10.0;
            retriever.add_memory(m);
        }

        let stats = retriever.get_stats();

        assert_eq!(stats.total_memories, 10);
        assert!(stats.accept_set_size > 0);
        assert!(stats.reject_set_size > 0);
    }

    #[test]
    fn test_context_payload() {
        let memories = vec![
            Memory::turn_level("m1", "s1", 1, "Content 1"),
            Memory::turn_level("m2", "s1", 2, "Content 2"),
        ];

        let payload = ContextPayload::new(memories.clone(), Granularity::TurnLevel, 0.5);

        assert_eq!(payload.memories.len(), 2);
        assert_eq!(payload.granularity, Granularity::TurnLevel);
        assert!((payload.entropy - 0.5).abs() < 0.01);
        assert!(payload.estimated_tokens > 0);
    }

    #[test]
    fn test_memory_creation() {
        let memory = Memory::new("id1", "session1", "Test content", vec!["tag1".to_string()], vec![0.1; 384]);

        assert_eq!(memory.id, "id1");
        assert_eq!(memory.session_id, "session1");
        assert_eq!(memory.content, "Test content");
        assert_eq!(memory.keywords.len(), 1);
        assert_eq!(memory.embedding.len(), 384);
    }

    #[test]
    fn test_granularity_serialization() {
        let granularities = vec![
            Granularity::Auto,
            Granularity::TurnLevel,
            Granularity::SessionSummary,
            Granularity::KeywordCluster,
        ];

        for g in granularities {
            let json = serde_json::to_string(&g).unwrap();
            let deserialized: Granularity = serde_json::from_str(&json).unwrap();
            assert_eq!(g, deserialized);
        }
    }
}
