//! Documentation index and retrieval system.
//!
//! This module provides indexing capabilities for documentation,
//! allowing agents to search and retrieve relevant documents.

use crate::schema::{DocId, Document};
use std::collections::HashMap;
use thiserror::Error;

/// Index errors
#[derive(Error, Debug)]
pub enum IndexError {
    /// Document not found
    #[error("document not found: {0}")]
    NotFound(String),

    /// Duplicate document ID
    #[error("duplicate document ID: {0}")]
    Duplicate(String),

    /// Index corruption or inconsistency
    #[error("index error: {0}")]
    Corrupted(String),
}

/// Result type for index operations
pub type Result<T> = std::result::Result<T, IndexError>;

/// Query for document retrieval
#[derive(Debug, Clone)]
pub struct DocQuery {
    /// Search terms (keywords)
    pub keywords: Vec<String>,
    /// Filter by module
    pub module: Option<String>,
    /// Filter by maintainer
    pub maintainer: Option<String>,
    /// Maximum number of results
    pub limit: usize,
    /// Minimum relevance score (0.0 to 1.0)
    pub min_score: f32,
}

impl DocQuery {
    /// Create a new query with search keywords
    pub fn new(keywords: &[&str]) -> Self {
        Self {
            keywords: keywords.iter().map(|s| s.to_string()).collect(),
            module: None,
            maintainer: None,
            limit: 10,
            min_score: 0.1,
        }
    }

    /// Set module filter
    pub fn with_module(mut self, module: &str) -> Self {
        self.module = Some(module.to_string());
        self
    }

    /// Set maintainer filter
    pub fn with_maintainer(mut self, maintainer: &str) -> Self {
        self.maintainer = Some(maintainer.to_string());
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set minimum relevance score
    pub fn with_min_score(mut self, score: f32) -> Self {
        self.min_score = score;
        self
    }
}

impl Default for DocQuery {
    fn default() -> Self {
        Self {
            keywords: Vec::new(),
            module: None,
            maintainer: None,
            limit: 10,
            min_score: 0.1,
        }
    }
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Document ID
    pub doc_id: DocId,
    /// Relevance score (0.0 to 1.0)
    pub score: f32,
    /// Matched keywords
    pub matched_keywords: Vec<String>,
}

/// Documentation index for storing and retrieving documents
pub struct DocIndex {
    /// Map of document ID to document
    docs: HashMap<DocId, Document>,
    /// Inverted index for keyword search: keyword -> doc IDs
    keyword_index: HashMap<String, Vec<DocId>>,
    /// Module index: module name -> doc IDs
    module_index: HashMap<String, Vec<DocId>>,
}

impl DocIndex {
    /// Create a new empty document index
    pub fn new() -> Self {
        Self {
            docs: HashMap::new(),
            keyword_index: HashMap::new(),
            module_index: HashMap::new(),
        }
    }

    /// Add a document to the index
    pub fn add(&mut self, doc: Document) -> Result<()> {
        if self.docs.contains_key(&doc.id) {
            return Err(IndexError::Duplicate(doc.id.clone()));
        }

        // Index keywords from content
        let keywords = self.extract_keywords(&doc.content);
        for keyword in &keywords {
            self.keyword_index
                .entry(keyword.clone())
                .or_insert_with(Vec::new)
                .push(doc.id.clone());
        }

        // Index by module
        self.module_index
            .entry(doc.schema.module.clone())
            .or_insert_with(Vec::new)
            .push(doc.id.clone());

        self.docs.insert(doc.id.clone(), doc);

        Ok(())
    }

    /// Update an existing document
    pub fn update(&mut self, doc: Document) -> Result<()> {
        if !self.docs.contains_key(&doc.id) {
            return Err(IndexError::NotFound(doc.id.clone()));
        }

        // Remove old keyword references
        self.remove_from_keyword_index(&doc.id);

        // Index new keywords
        let keywords = self.extract_keywords(&doc.content);
        for keyword in &keywords {
            self.keyword_index
                .entry(keyword.clone())
                .or_insert_with(Vec::new)
                .push(doc.id.clone());
        }

        self.docs.insert(doc.id.clone(), doc);

        Ok(())
    }

    /// Get a document by ID
    pub fn get(&self, doc_id: &str) -> Option<&Document> {
        self.docs.get(doc_id)
    }

    /// Get a mutable reference to a document by ID
    pub fn get_mut(&mut self, doc_id: &str) -> Option<&mut Document> {
        self.docs.get_mut(doc_id)
    }

    /// Search for documents matching a query
    pub fn retrieve(&self, query: &DocQuery) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = Vec::new();

        // Collect candidate documents
        let mut candidate_scores: HashMap<DocId, (f32, Vec<String>)> = HashMap::new();

        for keyword in &query.keywords {
            let keyword_lower = keyword.to_lowercase();

            // Find documents containing this keyword
            if let Some(doc_ids) = self.keyword_index.get(&keyword_lower) {
                for doc_id in doc_ids {
                    let (score, matched) = candidate_scores
                        .entry(doc_id.clone())
                        .or_insert_with(|| (0.0, Vec::new()));

                    *score += 1.0;
                    if !matched.contains(keyword) {
                        matched.push(keyword.clone());
                    }
                }
            }
        }

        // Apply filters and build results
        for (doc_id, (score, matched_keywords)) in candidate_scores {
            if let Some(doc) = self.docs.get(&doc_id) {
                // Apply module filter
                if let Some(ref module) = query.module {
                    if &doc.schema.module != module {
                        continue;
                    }
                }

                // Apply maintainer filter
                if let Some(ref maintainer) = query.maintainer {
                    if &doc.schema.maintainer != maintainer {
                        continue;
                    }
                }

                // Normalize score
                let normalized_score = score / query.keywords.len() as f32;

                // Apply minimum score filter
                if normalized_score >= query.min_score {
                    results.push(SearchResult {
                        doc_id,
                        score: normalized_score,
                        matched_keywords,
                    });
                }
            }
        }

        // Sort by score (descending)
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        results.truncate(query.limit);

        results
    }

    /// Simple keyword search (shorthand for retrieve)
    pub fn search(&self, keywords: &[&str]) -> Vec<SearchResult> {
        let query = DocQuery::new(keywords);
        self.retrieve(&query)
    }

    /// Find stale documents (not updated within max_age_days)
    pub fn find_stale(&self, max_age_days: u64) -> Vec<&Document> {
        let max_age_seconds = max_age_days * 86400;
        self.docs
            .values()
            .filter(|doc| doc.is_stale(max_age_seconds))
            .collect()
    }

    /// Get all documents in the index
    pub fn all(&self) -> Vec<&Document> {
        self.docs.values().collect()
    }

    /// Get document count
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Remove a document from the index
    pub fn remove(&mut self, doc_id: &str) -> Option<Document> {
        if let Some(doc) = self.docs.remove(doc_id) {
            self.remove_from_keyword_index(doc_id);

            // Remove from module index
            if let Some(doc_ids) = self.module_index.get_mut(&doc.schema.module) {
                doc_ids.retain(|id| id != doc_id);
            }

            Some(doc)
        } else {
            None
        }
    }

    /// Get documents by module
    pub fn by_module(&self, module: &str) -> Vec<&Document> {
        if let Some(doc_ids) = self.module_index.get(module) {
            doc_ids.iter().filter_map(|id| self.docs.get(id)).collect()
        } else {
            Vec::new()
        }
    }

    // === Internal Implementation ===

    /// Extract keywords from content for indexing
    fn extract_keywords(&self, content: &str) -> Vec<String> {
        let mut keywords: Vec<String> = Vec::new();

        // Simple keyword extraction: split on whitespace and punctuation
        for word in content.split(|c: char| c.is_whitespace() || c == '#' || c == '-' || c == '_') {
            let word = word.trim().to_lowercase();

            // Filter out very short words and common stop words
            if word.len() >= 3 && !is_stop_word(&word) {
                keywords.push(word);
            }
        }

        // Deduplicate
        keywords.sort();
        keywords.dedup();

        keywords
    }

    /// Remove document from keyword index
    fn remove_from_keyword_index(&mut self, doc_id: &str) {
        // Find and remove from all keyword lists
        let keywords_to_remove: Vec<String> = self
            .keyword_index
            .iter()
            .filter(|(_, doc_ids)| doc_ids.contains(&doc_id.to_string()))
            .map(|(k, _)| k.clone())
            .collect();

        for keyword in keywords_to_remove {
            if let Some(doc_ids) = self.keyword_index.get_mut(&keyword) {
                doc_ids.retain(|id| id != doc_id);
            }
        }
    }
}

impl Default for DocIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a word is a common stop word
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "is"
            | "are"
            | "was"
            | "were"
            | "be"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "do"
            | "does"
            | "did"
            | "will"
            | "would"
            | "could"
            | "should"
            | "may"
            | "might"
            | "must"
            | "this"
            | "that"
            | "these"
            | "those"
            | "it"
            | "its"
            | "as"
            | "if"
            | "then"
            | "than"
            | "so"
            | "such"
            | "not"
            | "no"
            | "yes"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::DocSchema;

    fn create_test_doc(id: &str, module: &str, content: &str) -> Document {
        let schema = DocSchema::new(module, "1.0", "test-agent");
        Document::new(id, schema, content.to_string(), "1.0.0")
    }

    #[test]
    fn test_doc_index_add_and_retrieve() {
        let mut index = DocIndex::new();

        let doc = create_test_doc(
            "test-doc",
            "auth",
            "Authentication API documentation with JWT support",
        );

        index.add(doc).expect("Failed to add document");

        assert_eq!(index.len(), 1);

        let retrieved = index.get("test-doc");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-doc");
    }

    #[test]
    fn test_doc_index_duplicate_prevention() {
        let mut index = DocIndex::new();

        let doc = create_test_doc("test-doc", "auth", "content");
        index.add(doc).expect("Failed to add document");

        let doc2 = create_test_doc("test-doc", "auth", "content 2");
        let result = index.add(doc2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IndexError::Duplicate(_)));
    }

    #[test]
    fn test_doc_semantic_search() {
        let mut index = DocIndex::new();

        index
            .add(create_test_doc(
                "auth-api",
                "auth",
                "JWT authentication API for user login and token refresh",
            ))
            .expect("Failed to add");

        index
            .add(create_test_doc(
                "cache-api",
                "cache",
                "Redis cache API for storing and retrieving data",
            ))
            .expect("Failed to add");

        index
            .add(create_test_doc(
                "user-api",
                "auth",
                "User management API for CRUD operations",
            ))
            .expect("Failed to add");

        // Search for authentication-related docs
        let results = index.search(&["authentication", "api"]);

        assert!(!results.is_empty());
        assert_eq!(results[0].doc_id, "auth-api");
        assert!(results[0].score > 0.0);
    }

    #[test]
    fn test_doc_index_search_with_filters() {
        let mut index = DocIndex::new();

        index
            .add(create_test_doc("doc1", "auth", "authentication token jwt"))
            .expect("Failed");
        index
            .add(create_test_doc("doc2", "auth", "user login session"))
            .expect("Failed");
        index
            .add(create_test_doc("doc3", "cache", "redis cache storage"))
            .expect("Failed");

        // Search with module filter - use keyword that exists in content
        let query = DocQuery::new(&["authentication"])
            .with_module("auth")
            .with_limit(10);

        let results = index.retrieve(&query);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].doc_id, "doc1");
        assert!(results
            .iter()
            .all(|r| { index.get(&r.doc_id).unwrap().schema.module == "auth" }));
    }

    #[test]
    fn test_doc_index_update() {
        let mut index = DocIndex::new();

        let doc = create_test_doc("test-doc", "auth", "original content");
        index.add(doc).expect("Failed to add");

        // Update with new content
        let mut updated_doc =
            create_test_doc("test-doc", "auth", "updated content with new keywords");
        updated_doc.version = "1.1.0".to_string();

        index.update(updated_doc).expect("Failed to update");

        let retrieved = index.get("test-doc").unwrap();
        assert_eq!(retrieved.content, "updated content with new keywords");
        assert_eq!(retrieved.version, "1.1.0");
    }

    #[test]
    fn test_doc_index_remove() {
        let mut index = DocIndex::new();

        index
            .add(create_test_doc("doc1", "auth", "content 1"))
            .expect("Failed");
        index
            .add(create_test_doc("doc2", "cache", "content 2"))
            .expect("Failed");

        let removed = index.remove("doc1");

        assert!(removed.is_some());
        assert_eq!(index.len(), 1);
        assert!(index.get("doc1").is_none());
    }

    #[test]
    fn test_doc_index_by_module() {
        let mut index = DocIndex::new();

        index
            .add(create_test_doc("auth1", "auth", "auth content"))
            .expect("Failed");
        index
            .add(create_test_doc("auth2", "auth", "more auth"))
            .expect("Failed");
        index
            .add(create_test_doc("cache1", "cache", "cache content"))
            .expect("Failed");

        let auth_docs = index.by_module("auth");

        assert_eq!(auth_docs.len(), 2);
        assert!(auth_docs.iter().all(|d| d.schema.module == "auth"));
    }

    #[test]
    fn test_doc_staleness_detection() {
        let mut index = DocIndex::new();

        // Add a document
        index
            .add(create_test_doc("fresh-doc", "auth", "fresh content"))
            .expect("Failed");

        // Fresh document should not appear in stale results with 30 day threshold
        let stale = index.find_stale(30);
        assert!(stale.is_empty());

        // With 0 day threshold, freshly created document is NOT stale (age is 0)
        let stale = index.find_stale(0);
        assert!(stale.is_empty());
    }

    #[test]
    fn test_doc_search_relevance_scoring() {
        let mut index = DocIndex::new();

        // Document with more keyword matches should score higher
        index
            .add(create_test_doc(
                "doc1",
                "auth",
                "authentication authentication authentication",
            ))
            .expect("Failed");

        index
            .add(create_test_doc("doc2", "auth", "authentication"))
            .expect("Failed");

        let results = index.search(&["authentication"]);

        // Both should match, doc1 might have higher score due to frequency
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.score > 0.0));
    }

    #[test]
    fn test_doc_index_empty_queries() {
        let mut index = DocIndex::new();

        index
            .add(create_test_doc("doc1", "auth", "content"))
            .expect("Failed");

        // Empty query should return no results
        let query = DocQuery::default();
        let results = index.retrieve(&query);

        assert!(results.is_empty());
    }

    #[test]
    fn test_doc_index_not_found() {
        let index = DocIndex::new();

        let result = index.get("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_doc_index_update_nonexistent() {
        let mut index = DocIndex::new();

        let doc = create_test_doc("nonexistent", "auth", "content");
        let result = index.update(doc);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IndexError::NotFound(_)));
    }
}
