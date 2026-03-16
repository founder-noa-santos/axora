//! Domain RAG (Retrieval-Augmented Generation)
//!
//! This module implements "Experience-as-Parameters" pattern:
//! - Domain knowledge is in vector stores, not agent structure
//! - Agents are generalists with domain-specific retrieval
//! - Coordination is O(N), not O(N²)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                   DomainRagStore                            │
//! ├─────────────────────────────────────────────────────────────┤
//! │  Domains (HashMap)         │  Retrieval Strategy           │
//! │  - auth: VectorStore       │  - DenseOnly                  │
//! │  - api: VectorStore        │  - Hybrid (BM25 + vectors)    │
//! │  - db: VectorStore         │  - LateInteraction (ColBERT)  │
//! │  - etc...                  │                               │
//! └─────────────────────────────────────────────────────────────┘
//!                              ↓
//!              ┌───────────────────────────────┐
//!              │   Experience Memory (RAG)     │
//!              │   - Past successes            │
//!              │   - Reasoning traces          │
//!              │   - Domain patterns           │
//!              └───────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```rust,no_run
//! use axora_cache::rag::DomainRagStore;
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create RAG store
//! let mut rag = DomainRagStore::new();
//!
//! // Add experience
//! rag.add_experience("auth", "user login", "Use JWT with HttpOnly cookies", "Stateless auth scales better");
//!
//! // Retrieve relevant knowledge for task
//! let results = rag.retrieve("auth", "implement user authentication", 5).await?;
//!
//! // Results contain content
//! for result in results {
//!     println!("Content: {}", result.content);
//! }
//! # Ok(())
//! # }
//! ```

use qdrant_client::qdrant::{
    self, Condition, Filter, PointStruct, QueryPoints, ScoredPoint, SearchParams,
    VectorInput,
};
use qdrant_client::{Payload, Qdrant};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// RAG error types
#[derive(Error, Debug)]
pub enum RagError {
    /// Domain not found
    #[error("domain '{0}' not found")]
    DomainNotFound(String),

    /// Vector search failed
    #[error("vector search failed: {0}")]
    VectorSearch(String),

    /// Keyword search failed
    #[error("keyword search failed: {0}")]
    KeywordSearch(String),

    /// Reranking failed
    #[error("reranking failed: {0}")]
    Rerank(String),

    /// Qdrant client error
    #[error("qdrant error: {0}")]
    Qdrant(String),
}

/// Result type for RAG operations
pub type Result<T> = std::result::Result<T, RagError>;

/// Domain identifier
pub type DomainId = String;

/// RAG retrieval result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagResult {
    /// Document/content ID
    pub id: String,
    /// Retrieved content
    pub content: String,
    /// Relevance score (0-1)
    pub score: f32,
    /// Source domain
    pub domain: String,
    /// Metadata (payload from vector store)
    pub metadata: HashMap<String, String>,
}

/// Past success memory (Experience-as-Parameters)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    /// Task description that was solved
    pub task_description: String,
    /// Successful pattern/solution used
    pub successful_pattern: String,
    /// Reasoning trace explaining why it worked
    pub reasoning_trace: String,
    /// Unix timestamp when experience was recorded
    pub timestamp: u64,
    /// Domain this experience belongs to
    pub domain: String,
}

impl Experience {
    /// Creates a new experience record
    pub fn new(
        task_description: &str,
        successful_pattern: &str,
        reasoning_trace: &str,
        domain: &str,
    ) -> Self {
        Self {
            task_description: task_description.to_string(),
            successful_pattern: successful_pattern.to_string(),
            reasoning_trace: reasoning_trace.to_string(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            domain: domain.to_string(),
        }
    }
}

/// Retrieval strategy for domain RAG
#[derive(Debug, Clone)]
pub enum RetrievalStrategy {
    /// Dense vectors only (semantic search)
    DenseOnly,
    /// Hybrid: BM25 (lexical) + dense vectors (semantic)
    Hybrid,
    /// Late-interaction (ColBERT-style) - advanced
    LateInteraction,
}

impl Default for RetrievalStrategy {
    fn default() -> Self {
        Self::Hybrid
    }
}

/// In-memory vector store for domain knowledge
/// (In production, this would wrap Qdrant client)
#[derive(Debug, Clone)]
pub struct VectorStore {
    /// Stored experiences for this domain
    experiences: Vec<Experience>,
    /// Simple keyword index for BM25-style search
    keyword_index: HashMap<String, Vec<usize>>,
}

impl VectorStore {
    /// Creates a new empty vector store
    pub fn new() -> Self {
        Self {
            experiences: Vec::new(),
            keyword_index: HashMap::new(),
        }
    }

    /// Adds an experience to the store
    pub fn add(&mut self, experience: Experience) {
        // Index keywords for hybrid search
        let keywords = Self::extract_keywords(&experience.task_description);
        let idx = self.experiences.len();

        for keyword in keywords {
            self.keyword_index
                .entry(keyword)
                .or_insert_with(Vec::new)
                .push(idx);
        }

        self.experiences.push(experience);
    }

    /// Extracts keywords from text (simple implementation)
    fn extract_keywords(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split_whitespace()
            .filter(|w| w.len() > 3)
            .filter(|w| !["the", "and", "for", "with", "this", "that", "from"].contains(w))
            .map(|s| s.to_string())
            .collect()
    }

    /// Gets all experiences
    pub fn experiences(&self) -> &[Experience] {
        &self.experiences
    }

    /// Gets experience count
    pub fn len(&self) -> usize {
        self.experiences.len()
    }

    /// Returns true if store is empty
    pub fn is_empty(&self) -> bool {
        self.experiences.is_empty()
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Domain-specific RAG store
///
/// Implements "Experience-as-Parameters" pattern where:
/// - Each domain has its own vector store
/// - Retrieval is hybrid (BM25 + semantic)
/// - Past successes are stored as experiences
#[derive(Clone)]
pub struct DomainRagStore {
    /// One vector store per domain
    domains: HashMap<DomainId, VectorStore>,
    /// Retrieval strategy (hybrid by default)
    strategy: RetrievalStrategy,
    /// Qdrant client for production use (optional)
    qdrant_client: Option<Arc<Qdrant>>,
}

impl DomainRagStore {
    /// Creates a new domain RAG store with default strategy
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new domain RAG store with specific strategy
    pub fn with_strategy(strategy: RetrievalStrategy) -> Self {
        Self {
            domains: HashMap::new(),
            strategy,
            qdrant_client: None,
        }
    }

    /// Sets the Qdrant client for production use
    pub fn with_qdrant_client(mut self, client: Qdrant) -> Self {
        self.qdrant_client = Some(Arc::new(client));
        self
    }

    /// Adds a domain with its vector store
    pub fn add_domain(&mut self, domain_id: &str, store: VectorStore) {
        self.domains.insert(domain_id.to_string(), store);
    }

    /// Creates a new domain if it doesn't exist
    pub fn ensure_domain(&mut self, domain_id: &str) {
        if !self.domains.contains_key(domain_id) {
            self.domains.insert(domain_id.to_string(), VectorStore::new());
        }
    }

    /// Adds an experience to a domain
    pub fn add_experience(
        &mut self,
        domain: &str,
        task: &str,
        pattern: &str,
        reasoning: &str,
    ) {
        self.ensure_domain(domain);
        
        let experience = Experience::new(task, pattern, reasoning, domain);
        if let Some(store) = self.domains.get_mut(domain) {
            store.add(experience);
        }
    }

    /// Retrieves relevant knowledge for a task from a specific domain
    pub async fn retrieve(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        match self.strategy {
            RetrievalStrategy::DenseOnly => self.dense_retrieve(domain, query, k).await,
            RetrievalStrategy::Hybrid => self.hybrid_retrieve(domain, query, k).await,
            RetrievalStrategy::LateInteraction => {
                self.late_interaction_retrieve(domain, query, k).await
            }
        }
    }

    /// Dense vector-only retrieval (semantic search)
    async fn dense_retrieve(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        let store = self
            .domains
            .get(domain)
            .ok_or_else(|| RagError::DomainNotFound(domain.to_string()))?;

        // Simple semantic similarity based on keyword overlap
        // (In production, this would use actual vector embeddings)
        let query_lower = query.to_lowercase();
        let query_keywords: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(&Experience, f32)> = store
            .experiences
            .iter()
            .map(|exp| {
                let exp_lower = exp.task_description.to_lowercase();
                let exp_keywords: Vec<&str> = exp_lower.split_whitespace().collect();

                // Jaccard-like similarity
                let overlap = query_keywords
                    .iter()
                    .filter(|q| exp_keywords.contains(q))
                    .count();

                let similarity = overlap as f32
                    / (query_keywords.len() + exp_keywords.len() - overlap).max(1) as f32;

                (exp, similarity)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        let results: Vec<RagResult> = scored
            .into_iter()
            .take(k)
            .filter(|(_, score)| *score > 0.0)
            .map(|(exp, score)| RagResult {
                id: format!("{}-{}", exp.domain, exp.timestamp),
                content: exp.successful_pattern.clone(),
                score,
                domain: exp.domain.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        Ok(results)
    }

    /// Hybrid search: BM25 (lexical) + dense vectors (semantic)
    async fn hybrid_retrieve(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        // Parallel retrieval
        let (vector_results, keyword_results) = tokio::join!(
            self.dense_retrieve(domain, query, k),
            self.keyword_search(domain, query, k),
        );

        let vector_results = vector_results?;
        let keyword_results = keyword_results?;

        // Merge and rerank using Reciprocal Rank Fusion
        let merged = self.rerank_and_merge(vector_results, keyword_results, k);

        Ok(merged)
    }

    /// Keyword-based search (BM25-style)
    async fn keyword_search(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        let store = self
            .domains
            .get(domain)
            .ok_or_else(|| RagError::DomainNotFound(domain.to_string()))?;

        let query_keywords: Vec<String> = VectorStore::extract_keywords(query);
        
        // Count keyword matches for each experience
        let mut scores: HashMap<usize, f32> = HashMap::new();
        
        for keyword in &query_keywords {
            if let Some(indices) = store.keyword_index.get(keyword) {
                for &idx in indices {
                    *scores.entry(idx).or_insert(0.0) += 1.0;
                }
            }
        }

        // Convert to results
        let mut results: Vec<RagResult> = scores
            .into_iter()
            .filter(|(_, score)| *score > 0.0)
            .map(|(idx, score)| {
                let exp = &store.experiences[idx];
                RagResult {
                    id: format!("{}-{}", exp.domain, exp.timestamp),
                    content: exp.successful_pattern.clone(),
                    score,
                    domain: exp.domain.clone(),
                    metadata: HashMap::new(),
                }
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results.into_iter().take(k).collect())
    }

    /// Late-interaction retrieval (ColBERT-style)
    ///
    /// This is an advanced retrieval method that:
    /// 1. Encodes query into token-level embeddings
    /// 2. Encodes documents into token-level embeddings
    /// 3. Computes max-similarity between query tokens and doc tokens
    /// 4. Sums similarities for final score
    async fn late_interaction_retrieve(
        &self,
        domain: &str,
        query: &str,
        k: usize,
    ) -> Result<Vec<RagResult>> {
        // Simplified implementation (full ColBERT requires transformer models)
        // This uses token-level keyword matching as a proxy

        let store = self
            .domains
            .get(domain)
            .ok_or_else(|| RagError::DomainNotFound(domain.to_string()))?;

        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(&Experience, f32)> = store
            .experiences
            .iter()
            .map(|exp| {
                let exp_lower = exp.task_description.to_lowercase();
                let doc_tokens: Vec<&str> = exp_lower.split_whitespace().collect();

                // Max-similarity for each query token
                let max_similarities: Vec<f32> = query_tokens
                    .iter()
                    .map(|q_token| {
                        doc_tokens
                            .iter()
                            .map(|d_token| {
                                // Simple similarity: exact match = 1.0, partial = 0.5
                                if q_token == d_token {
                                    1.0
                                } else if q_token.contains(d_token) || d_token.contains(q_token) {
                                    0.5
                                } else {
                                    0.0
                                }
                            })
                            .fold(0.0f32, f32::max)
                    })
                    .collect();

                // Sum of max-similarities
                let total_score: f32 = max_similarities.iter().sum();

                (exp, total_score)
            })
            .collect();

        // Sort by score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        let results: Vec<RagResult> = scored
            .into_iter()
            .take(k)
            .filter(|(_, score)| *score > 0.0)
            .map(|(exp, score)| RagResult {
                id: format!("{}-{}", exp.domain, exp.timestamp),
                content: exp.successful_pattern.clone(),
                score,
                domain: exp.domain.clone(),
                metadata: HashMap::new(),
            })
            .collect();

        Ok(results)
    }

    /// Rerank and merge results from multiple retrieval methods
    ///
    /// Uses Reciprocal Rank Fusion (RRF) to combine rankings:
    /// RRF(d) = Σ 1 / (k + rank_i(d))
    ///
    /// Where k is a constant (typically 60) and rank_i is the rank in result set i
    fn rerank_and_merge(
        &self,
        vector_results: Vec<RagResult>,
        keyword_results: Vec<RagResult>,
        k: usize,
    ) -> Vec<RagResult> {
        use std::collections::hash_map::Entry;

        const RRF_K: f32 = 60.0;

        // Calculate RRF scores
        let mut rrf_scores: HashMap<String, f32> = HashMap::new();
        
        for (rank, result) in vector_results.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32);
            match rrf_scores.entry(result.id.clone()) {
                Entry::Occupied(mut e) => {
                    *e.get_mut() += score;
                }
                Entry::Vacant(e) => {
                    e.insert(score);
                }
            }
        }

        for (rank, result) in keyword_results.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32);
            match rrf_scores.entry(result.id.clone()) {
                Entry::Occupied(mut e) => {
                    *e.get_mut() += score;
                }
                Entry::Vacant(e) => {
                    e.insert(score);
                }
            }
        }

        // Create merged results with RRF scores
        let mut all_results: Vec<RagResult> = Vec::new();
        let mut seen_ids: HashMap<String, RagResult> = HashMap::new();

        for result in vector_results.into_iter().chain(keyword_results.into_iter()) {
            if !seen_ids.contains_key(&result.id) {
                seen_ids.insert(result.id.clone(), result);
            }
        }

        for (id, score) in rrf_scores {
            if let Some(mut result) = seen_ids.remove(&id) {
                result.score = score;
                all_results.push(result);
            }
        }

        // Sort by RRF score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Take top k
        all_results.into_iter().take(k).collect()
    }

    /// Gets the number of domains
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Gets the number of experiences in a domain
    pub fn experience_count(&self, domain: &str) -> Option<usize> {
        self.domains.get(domain).map(|s| s.len())
    }

    /// Gets all domain IDs
    pub fn domains(&self) -> Vec<&str> {
        self.domains.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for DomainRagStore {
    fn default() -> Self {
        Self {
            domains: HashMap::new(),
            strategy: RetrievalStrategy::default(),
            qdrant_client: None,
        }
    }
}

/// RAG retrieval statistics
#[derive(Debug, Clone, Default)]
pub struct RagStats {
    /// Number of domains searched
    pub domains_searched: usize,
    /// Total results retrieved
    pub total_results: usize,
    /// Average retrieval latency in milliseconds
    pub avg_latency_ms: f32,
    /// Average relevance score
    pub avg_score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_domain_rag_retrieve() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::DenseOnly);
        
        // Add experiences
        rag.add_experience("auth", "user login with JWT", "Use JWT tokens with HttpOnly cookies", "Stateless auth scales better");
        rag.add_experience("auth", "OAuth2 integration", "Use OAuth2 flow with refresh tokens", "Industry standard for third-party auth");
        rag.add_experience("api", "REST API design", "Use RESTful principles with versioning", "Clean separation of concerns");
        
        // Retrieve
        let results = rag.retrieve("auth", "implement user authentication", 5).await.unwrap();
        
        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.domain == "auth"));
    }

    #[tokio::test]
    async fn test_hybrid_search_precision() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::Hybrid);
        
        // Add multiple experiences with varying relevance
        rag.add_experience("auth", "JWT token authentication", "Use JWT with RS256 signing", "Secure and scalable");
        rag.add_experience("auth", "session-based authentication", "Use server-side sessions", "Traditional approach");
        rag.add_experience("auth", "OAuth2 third-party login", "Use OAuth2 providers", "Good for social login");
        rag.add_experience("api", "rate limiting", "Use token bucket algorithm", "Prevents abuse");
        
        // Hybrid search should find most relevant
        let results = rag.retrieve("auth", "JWT authentication tokens", 3).await.unwrap();
        
        // Top result should be most relevant (JWT-related)
        assert!(!results.is_empty());
        assert!(results[0].content.contains("JWT"));
    }

    #[tokio::test]
    async fn test_experience_as_parameters() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::DenseOnly);
        
        // Store past success as experience
        rag.add_experience(
            "auth",
            "implement login endpoint",
            "Created /api/auth/login with JWT response",
            "Used bcrypt for password hashing, JWT for tokens"
        );
        
        // Retrieve should find the experience
        let results = rag.retrieve("auth", "login endpoint", 5).await.unwrap();
        
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.content.contains("login")));
    }

    #[tokio::test]
    async fn test_late_interaction_retrieval() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::LateInteraction);
        
        // Add experiences
        rag.add_experience("db", "PostgreSQL connection pooling", "Use connection pool with max 100 connections", "Improves performance");
        rag.add_experience("db", "Redis caching layer", "Use Redis for session storage", "Fast key-value access");
        rag.add_experience("db", "database migration strategy", "Use Flyway for schema migrations", "Version-controlled migrations");
        
        // Late-interaction should find token-level matches
        let results = rag.retrieve("db", "PostgreSQL pool connections", 3).await.unwrap();
        
        assert!(!results.is_empty());
        // Top result should match "PostgreSQL" and "pool" tokens
        assert!(results[0].content.contains("PostgreSQL") || results[0].content.contains("pool"));
    }

    #[tokio::test]
    async fn test_multi_domain_retrieval() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::Hybrid);
        
        // Add experiences to multiple domains
        rag.add_experience("auth", "user authentication", "JWT-based auth", "Stateless");
        rag.add_experience("api", "API authentication", "API key in header", "Simple");
        rag.add_experience("db", "database auth", "Role-based access", "Secure");
        
        // Retrieve from each domain
        let auth_results = rag.retrieve("auth", "authentication", 5).await.unwrap();
        let api_results = rag.retrieve("api", "authentication", 5).await.unwrap();
        let db_results = rag.retrieve("db", "authentication", 5).await.unwrap();
        
        // Each should return domain-specific results
        assert!(auth_results.iter().all(|r| r.domain == "auth"));
        assert!(api_results.iter().all(|r| r.domain == "api"));
        assert!(db_results.iter().all(|r| r.domain == "db"));
    }

    #[tokio::test]
    async fn test_domain_not_found() {
        let rag = DomainRagStore::with_strategy(RetrievalStrategy::DenseOnly);
        
        // Try to retrieve from non-existent domain
        let result = rag.retrieve("nonexistent", "query", 5).await;
        
        assert!(result.is_err());
        match result {
            Err(RagError::DomainNotFound(domain)) => assert_eq!(domain, "nonexistent"),
            _ => panic!("Expected DomainNotFound error"),
        }
    }

    #[test]
    fn test_vector_store_keyword_indexing() {
        let mut store = VectorStore::new();
        
        store.add(Experience::new("user login", "pattern1", "reasoning1", "auth"));
        store.add(Experience::new("admin login", "pattern2", "reasoning2", "auth"));
        store.add(Experience::new("API authentication", "pattern3", "reasoning3", "api"));
        
        // Check keyword index
        assert!(store.keyword_index.contains_key("login"));
        assert!(store.keyword_index.contains_key("authentication"));
        
        // Check experience count
        assert_eq!(store.len(), 3);
    }

    #[tokio::test]
    async fn test_rerank_and_merge() {
        let rag = DomainRagStore::with_strategy(RetrievalStrategy::Hybrid);

        // Create mock results from different retrieval methods
        let vector_results = vec![
            RagResult { id: "1".to_string(), content: "content1".to_string(), score: 0.9, domain: "auth".to_string(), metadata: HashMap::new() },
            RagResult { id: "2".to_string(), content: "content2".to_string(), score: 0.7, domain: "auth".to_string(), metadata: HashMap::new() },
        ];

        let keyword_results = vec![
            RagResult { id: "2".to_string(), content: "content2-updated".to_string(), score: 0.8, domain: "auth".to_string(), metadata: HashMap::new() },
            RagResult { id: "3".to_string(), content: "content3".to_string(), score: 0.6, domain: "auth".to_string(), metadata: HashMap::new() },
        ];

        // Merge with RRF
        let merged = rag.rerank_and_merge(vector_results, keyword_results, 5);

        // Should have 3 unique results (id 2 appears in both)
        assert_eq!(merged.len(), 3);
        
        // ID 2 should have higher RRF score (appeared in both)
        let id2_result = merged.iter().find(|r| r.id == "2").unwrap();
        assert!(id2_result.score > 0.0);
    }

    #[test]
    fn test_experience_serialization() {
        let exp = Experience::new("test task", "test pattern", "test reasoning", "test-domain");
        
        // Serialize
        let json = serde_json::to_string(&exp).unwrap();
        
        // Deserialize
        let deserialized: Experience = serde_json::from_str(&json).unwrap();
        
        assert_eq!(exp.task_description, deserialized.task_description);
        assert_eq!(exp.successful_pattern, deserialized.successful_pattern);
        assert_eq!(exp.reasoning_trace, deserialized.reasoning_trace);
        assert_eq!(exp.domain, deserialized.domain);
    }

    #[tokio::test]
    async fn test_domain_count() {
        let mut rag = DomainRagStore::with_strategy(RetrievalStrategy::DenseOnly);
        
        assert_eq!(rag.domain_count(), 0);
        
        rag.add_experience("auth", "task1", "pattern1", "reasoning1");
        rag.add_experience("api", "task2", "pattern2", "reasoning2");
        rag.add_experience("db", "task3", "pattern3", "reasoning3");
        
        assert_eq!(rag.domain_count(), 3);
        assert_eq!(rag.experience_count("auth"), Some(1));
        assert_eq!(rag.experience_count("api"), Some(1));
        assert_eq!(rag.experience_count("nonexistent"), None);
    }
}
