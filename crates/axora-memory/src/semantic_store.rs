//! Semantic Memory Store
//!
//! This module implements semantic memory storage for AXORA agents:
//! - Vector Database (Qdrant) for high-dimensional embeddings
//! - Top-K similarity retrieval
//! - Integration with Living Docs
//!
//! # Note
//!
//! This implementation provides the data structures and traits for semantic memory.
//! Full Qdrant integration requires the builder pattern API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Semantic memory errors
#[derive(Error, Debug)]
pub enum SemanticError {
    /// Invalid embedding dimension
    #[error("invalid embedding dimension: expected {expected}, got {actual}")]
    InvalidDimension { expected: usize, actual: usize },

    /// Memory not found
    #[error("memory not found: {0}")]
    NotFound(String),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Storage error
    #[error("storage error: {0}")]
    Storage(String),
}

/// Result type for semantic memory operations
pub type Result<T> = std::result::Result<T, SemanticError>;

/// Document type for semantic memory
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocType {
    /// API contract documentation
    ApiContract,
    /// Database schema documentation
    DatabaseSchema,
    /// Architectural documentation
    ArchitecturalDoc,
    /// Coding conventions and standards
    CodingConvention,
    /// Business rules
    BusinessRule,
    /// Test documentation
    TestDoc,
    /// User guide
    UserGuide,
    /// Other documentation type
    Other,
}

impl DocType {
    /// Convert string to DocType
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "api_contract" | "apicontract" => DocType::ApiContract,
            "database_schema" | "databaseschema" => DocType::DatabaseSchema,
            "architectural_doc" | "architecturaldoc" | "architecture" => {
                DocType::ArchitecturalDoc
            }
            "coding_convention" | "codingconvention" | "convention" => DocType::CodingConvention,
            "business_rule" | "businessrule" | "business" => DocType::BusinessRule,
            "test_doc" | "testdoc" | "test" => DocType::TestDoc,
            "user_guide" | "userguide" | "guide" => DocType::UserGuide,
            _ => DocType::Other,
        }
    }

    /// Convert DocType to string
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::ApiContract => "api_contract",
            DocType::DatabaseSchema => "database_schema",
            DocType::ArchitecturalDoc => "architectural_doc",
            DocType::CodingConvention => "coding_convention",
            DocType::BusinessRule => "business_rule",
            DocType::TestDoc => "test_doc",
            DocType::UserGuide => "user_guide",
            DocType::Other => "other",
        }
    }
}

/// Semantic metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMetadata {
    /// Source of the memory (e.g., "living_docs", "manual_import")
    pub source: String,
    /// Document type
    pub doc_type: DocType,
    /// Unix timestamp when created
    pub created_at: u64,
    /// Unix timestamp when last updated
    pub updated_at: u64,
    /// Optional tags for categorization
    pub tags: Vec<String>,
    /// Optional reference to related memories
    pub related_ids: Vec<String>,
}

impl SemanticMetadata {
    /// Create new metadata
    pub fn new(source: &str, doc_type: DocType) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            source: source.to_string(),
            doc_type,
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            related_ids: Vec::new(),
        }
    }

    /// Create metadata with custom timestamps
    pub fn with_timestamps(
        source: &str,
        doc_type: DocType,
        created_at: u64,
        updated_at: u64,
    ) -> Self {
        Self {
            source: source.to_string(),
            doc_type,
            created_at,
            updated_at,
            tags: Vec::new(),
            related_ids: Vec::new(),
        }
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Add related memory ID
    pub fn with_related(mut self, id: &str) -> Self {
        self.related_ids.push(id.to_string());
        self
    }
}

/// Semantic memory entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    /// Unique identifier
    pub id: String,
    /// Text content
    pub content: String,
    /// Embedding vector (384 dimensions for all-MiniLM-L6-v2)
    pub embedding: Vec<f32>,
    /// Metadata
    pub metadata: SemanticMetadata,
}

impl SemanticMemory {
    /// Create new semantic memory
    pub fn new(id: &str, content: &str, embedding: Vec<f32>, metadata: SemanticMetadata) -> Self {
        Self {
            id: id.to_string(),
            content: content.to_string(),
            embedding,
            metadata,
        }
    }

    /// Create semantic memory with auto-generated ID
    pub fn from_content(content: &str, embedding: Vec<f32>, metadata: SemanticMetadata) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self::new(&id, content, embedding, metadata)
    }

    /// Get embedding dimension
    pub fn dimension(&self) -> usize {
        self.embedding.len()
    }

    /// Convert to payload for storage
    pub fn to_payload(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut payload = HashMap::new();
        payload.insert("id".to_string(), serde_json::Value::String(self.id.clone()));
        payload.insert("content".to_string(), serde_json::Value::String(self.content.clone()));
        payload.insert(
            "embedding".to_string(),
            serde_json::to_value(&self.embedding)?,
        );
        payload.insert(
            "metadata".to_string(),
            serde_json::to_value(&self.metadata)?,
        );
        Ok(payload)
    }
}

/// Search result with score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Memory ID
    pub id: String,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Memory content
    pub content: String,
    /// Memory metadata
    pub metadata: SemanticMetadata,
}

/// Collection statistics
#[derive(Debug, Clone)]
pub struct CollectionStats {
    /// Total number of points
    pub point_count: u64,
    /// Total number of vectors
    pub vectors_count: u64,
    /// Number of indexed vectors
    pub indexed_vectors_count: u64,
}

/// In-memory semantic store for testing
pub struct InMemorySemanticStore {
    memories: dashmap::DashMap<String, SemanticMemory>,
    embedding_dim: usize,
}

impl InMemorySemanticStore {
    /// Create new in-memory store
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            memories: dashmap::DashMap::new(),
            embedding_dim,
        }
    }

    /// Insert semantic memory
    pub fn insert(&self, memory: SemanticMemory) -> Result<()> {
        if memory.dimension() != self.embedding_dim {
            return Err(SemanticError::InvalidDimension {
                expected: self.embedding_dim,
                actual: memory.dimension(),
            });
        }
        self.memories.insert(memory.id.clone(), memory);
        Ok(())
    }

    /// Batch insert memories
    pub fn insert_batch(&self, memories: Vec<SemanticMemory>) -> Result<()> {
        for memory in memories {
            self.insert(memory)?;
        }
        Ok(())
    }

    /// Get memory by ID
    pub fn get(&self, id: &str) -> Option<SemanticMemory> {
        self.memories.get(id).map(|r| r.clone())
    }

    /// Retrieve top-K similar memories (simple cosine similarity)
    pub fn retrieve(&self, query_embedding: &[f32], k: usize) -> Result<Vec<SearchResult>> {
        if query_embedding.len() != self.embedding_dim {
            return Err(SemanticError::InvalidDimension {
                expected: self.embedding_dim,
                actual: query_embedding.len(),
            });
        }

        // Calculate cosine similarity for all memories
        let mut results: Vec<(String, f32)> = self
            .memories
            .iter()
            .map(|r| {
                let memory = r.value();
                let score = cosine_similarity(query_embedding, &memory.embedding);
                (r.key().clone(), score)
            })
            .collect();

        // Sort by score (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top-K
        results.truncate(k);

        // Convert to SearchResult
        let search_results = results
            .into_iter()
            .filter_map(|(id, score)| {
                self.memories.get(&id).map(|r| {
                    let memory = r.value();
                    SearchResult {
                        id: memory.id.clone(),
                        score,
                        content: memory.content.clone(),
                        metadata: memory.metadata.clone(),
                    }
                })
            })
            .collect();

        Ok(search_results)
    }

    /// Delete memory by ID
    pub fn delete(&self, id: &str) -> Option<SemanticMemory> {
        self.memories.remove(id).map(|(_, v)| v)
    }

    /// Get collection statistics
    pub fn stats(&self) -> CollectionStats {
        let count = self.memories.len() as u64;
        CollectionStats {
            point_count: count,
            vectors_count: count,
            indexed_vectors_count: count,
        }
    }

    /// Get all memories
    pub fn all(&self) -> Vec<SemanticMemory> {
        self.memories.iter().map(|r| r.value().clone()).collect()
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_type_from_str() {
        assert_eq!(DocType::from_str("api_contract"), DocType::ApiContract);
        assert_eq!(DocType::from_str("ApiContract"), DocType::ApiContract);
        assert_eq!(DocType::from_str("API_CONTRACT"), DocType::ApiContract);
        assert_eq!(DocType::from_str("architecture"), DocType::ArchitecturalDoc);
        assert_eq!(DocType::from_str("business"), DocType::BusinessRule);
        assert_eq!(DocType::from_str("unknown"), DocType::Other);
    }

    #[test]
    fn test_doc_type_as_str() {
        assert_eq!(DocType::ApiContract.as_str(), "api_contract");
        assert_eq!(DocType::ArchitecturalDoc.as_str(), "architectural_doc");
        assert_eq!(DocType::BusinessRule.as_str(), "business_rule");
        assert_eq!(DocType::Other.as_str(), "other");
    }

    #[test]
    fn test_semantic_metadata_creation() {
        let metadata = SemanticMetadata::new("living_docs", DocType::ArchitecturalDoc);

        assert_eq!(metadata.source, "living_docs");
        assert_eq!(metadata.doc_type, DocType::ArchitecturalDoc);
        assert!(metadata.created_at > 0);
        assert!(metadata.updated_at > 0);
        assert!(metadata.tags.is_empty());
        assert!(metadata.related_ids.is_empty());
    }

    #[test]
    fn test_semantic_metadata_with_tags() {
        let metadata = SemanticMetadata::new("living_docs", DocType::ApiContract)
            .with_tag("authentication")
            .with_tag("security")
            .with_related("AUTH-001");

        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.tags.contains(&"authentication".to_string()));
        assert!(metadata.tags.contains(&"security".to_string()));
        assert_eq!(metadata.related_ids.len(), 1);
        assert!(metadata.related_ids.contains(&"AUTH-001".to_string()));
    }

    #[test]
    fn test_semantic_memory_creation() {
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::new("test-001", "test content", vec![0.1; 384], metadata);

        assert_eq!(memory.id, "test-001");
        assert_eq!(memory.content, "test content");
        assert_eq!(memory.dimension(), 384);
    }

    #[test]
    fn test_semantic_memory_from_content() {
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::from_content("test content", vec![0.1; 384], metadata);

        assert!(!memory.id.is_empty());
        assert_eq!(memory.content, "test content");
        assert_eq!(memory.dimension(), 384);
    }

    #[test]
    fn test_semantic_memory_to_payload() {
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::new("test-001", "test content", vec![0.1; 384], metadata);

        let payload = memory.to_payload().unwrap();

        assert!(payload.contains_key("id"));
        assert!(payload.contains_key("content"));
        assert!(payload.contains_key("embedding"));
        assert!(payload.contains_key("metadata"));
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        // Same vectors should have similarity 1.0
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        // Orthogonal vectors should have similarity 0.0
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }

    #[test]
    fn test_in_memory_store_insert() {
        let store = InMemorySemanticStore::new(384);
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::new("test-001", "test content", vec![0.1; 384], metadata);

        store.insert(memory).unwrap();

        let retrieved = store.get("test-001");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "test content");
    }

    #[test]
    fn test_in_memory_store_retrieve() {
        let store = InMemorySemanticStore::new(3);

        // Insert memories with different embeddings
        let metadata = SemanticMetadata::new("test", DocType::Other);
        store
            .insert(SemanticMemory::new(
                "mem-1",
                "content 1",
                vec![1.0, 0.0, 0.0],
                metadata.clone(),
            ))
            .unwrap();
        store
            .insert(SemanticMemory::new(
                "mem-2",
                "content 2",
                vec![0.0, 1.0, 0.0],
                metadata.clone(),
            ))
            .unwrap();
        store
            .insert(SemanticMemory::new(
                "mem-3",
                "content 3",
                vec![0.0, 0.0, 1.0],
                metadata,
            ))
            .unwrap();

        // Query should return most similar memory first
        let results = store.retrieve(&[1.0, 0.0, 0.0], 2).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "mem-1"); // Most similar
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn test_in_memory_store_batch_insert() {
        let store = InMemorySemanticStore::new(384);

        let memories = vec![
            SemanticMemory::new(
                "mem-1",
                "content 1",
                vec![0.1; 384],
                SemanticMetadata::new("test", DocType::Other),
            ),
            SemanticMemory::new(
                "mem-2",
                "content 2",
                vec![0.2; 384],
                SemanticMetadata::new("test", DocType::Other),
            ),
        ];

        store.insert_batch(memories).unwrap();

        assert_eq!(store.stats().point_count, 2);
    }

    #[test]
    fn test_in_memory_store_delete() {
        let store = InMemorySemanticStore::new(384);
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::new("test-001", "test content", vec![0.1; 384], metadata);

        store.insert(memory).unwrap();
        assert!(store.get("test-001").is_some());

        store.delete("test-001");
        assert!(store.get("test-001").is_none());
    }

    #[test]
    fn test_in_memory_store_stats() {
        let store = InMemorySemanticStore::new(384);

        assert_eq!(store.stats().point_count, 0);

        let metadata = SemanticMetadata::new("test", DocType::Other);
        store
            .insert(SemanticMemory::new(
                "test-001",
                "test content",
                vec![0.1; 384],
                metadata,
            ))
            .unwrap();

        assert_eq!(store.stats().point_count, 1);
    }

    #[test]
    fn test_in_memory_store_invalid_dimension() {
        let store = InMemorySemanticStore::new(384);
        let metadata = SemanticMetadata::new("test", DocType::Other);
        let memory = SemanticMemory::new("test-001", "test content", vec![0.1; 100], metadata);

        let result = store.insert(memory);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SemanticError::InvalidDimension { .. }
        ));
    }

    #[test]
    fn test_collection_stats() {
        let stats = CollectionStats {
            point_count: 100,
            vectors_count: 100,
            indexed_vectors_count: 100,
        };

        assert_eq!(stats.point_count, 100);
        assert_eq!(stats.vectors_count, 100);
        assert_eq!(stats.indexed_vectors_count, 100);
    }
}
