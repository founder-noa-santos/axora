//! Prefix caching for token optimization
//!
//! Caches static prompt prefixes to avoid re-computation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

/// Get current timestamp in milliseconds
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Cached prefix entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPrefix {
    /// Unique identifier
    pub id: String,
    /// Prefix content
    pub content: String,
    /// Cache key (hash of content)
    pub cache_key: String,
    /// Access count
    pub access_count: usize,
    /// Created at (timestamp in milliseconds)
    pub created_at: u64,
    /// Last accessed at (timestamp in milliseconds)
    pub last_accessed: u64,
    /// Size in tokens
    pub token_count: usize,
}

/// Prefix cache manager
pub struct PrefixCache {
    /// Cached prefixes
    cache: HashMap<String, CachedPrefix>,
    /// Maximum cache size (number of entries)
    max_entries: usize,
    /// Token savings counter
    total_tokens_saved: usize,
}

impl PrefixCache {
    /// Create new prefix cache
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_entries,
            total_tokens_saved: 0,
        }
    }

    /// Create cache with default settings
    pub fn default() -> Self {
        Self::new(100)
    }

    /// Add a prefix to cache
    pub fn add(&mut self, id: &str, content: &str, token_count: usize) -> String {
        let cache_key = self.compute_cache_key(content);
        let now = now_millis();

        // Check if already cached
        if let Some(existing) = self.cache.get(&cache_key) {
            debug!("Prefix already cached: {}", existing.id);
            return existing.id.clone();
        }

        // Enforce capacity
        self.enforce_capacity();

        // Create new entry
        let entry = CachedPrefix {
            id: id.to_string(),
            content: content.to_string(),
            cache_key: cache_key.clone(),
            access_count: 0,
            created_at: now,
            last_accessed: now,
            token_count,
        };

        self.cache.insert(cache_key, entry);
        info!("Cached prefix: {} ({} tokens)", id, token_count);

        id.to_string()
    }

    /// Get a cached prefix
    pub fn get(&mut self, cache_key: &str) -> Option<&CachedPrefix> {
        if let Some(entry) = self.cache.get_mut(cache_key) {
            entry.access_count += 1;
            entry.last_accessed = now_millis();
            self.total_tokens_saved += entry.token_count;
            debug!("Cache hit: {} (saved {} tokens)", entry.id, entry.token_count);
            Some(entry)
        } else {
            debug!("Cache miss: {}", cache_key);
            None
        }
    }

    /// Check if prefix is cached
    pub fn contains(&self, content: &str) -> bool {
        let cache_key = self.compute_cache_key(content);
        self.cache.contains_key(&cache_key)
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_entries = self.cache.len();
        let total_tokens_cached: usize = self.cache.values().map(|e| e.token_count).sum();
        let avg_access_count = if total_entries > 0 {
            self.cache.values().map(|e| e.access_count).sum::<usize>() / total_entries
        } else {
            0
        };

        CacheStats {
            total_entries,
            total_tokens_cached,
            total_tokens_saved: self.total_tokens_saved,
            avg_access_count,
            hit_rate: self.calculate_hit_rate(),
        }
    }

    /// Remove a prefix from cache
    pub fn remove(&mut self, cache_key: &str) -> bool {
        if self.cache.remove(cache_key).is_some() {
            debug!("Removed cached prefix: {}", cache_key);
            true
        } else {
            false
        }
    }

    /// Clear all cached prefixes
    pub fn clear(&mut self) {
        let count = self.cache.len();
        self.cache.clear();
        info!("Cleared {} cached prefixes", count);
    }

    /// Enforce cache capacity
    fn enforce_capacity(&mut self) {
        if self.cache.len() >= self.max_entries {
            // Remove least recently accessed
            let lru_key = self.cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone());

            if let Some(key) = lru_key {
                self.cache.remove(&key);
                debug!("Evicted LRU prefix: {}", key);
            }
        }
    }

    /// Compute cache key (simple hash)
    fn compute_cache_key(&self, content: &str) -> String {
        // Simple hash for demonstration
        // In production, use a proper hash function like BLAKE3
        format!("prefix_{:x}", md5::compute(content.as_bytes()))
    }

    /// Calculate hit rate
    fn calculate_hit_rate(&self) -> f32 {
        let total_accesses: usize = self.cache.values().map(|e| e.access_count).sum();
        if total_accesses == 0 {
            0.0
        } else {
            self.total_tokens_saved as f32 / total_accesses as f32
        }
    }

    /// Set max entries
    pub fn with_max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }
}

impl Default for PrefixCache {
    fn default() -> Self {
        Self::default()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total cached entries
    pub total_entries: usize,
    /// Total tokens cached
    pub total_tokens_cached: usize,
    /// Total tokens saved (from cache hits)
    pub total_tokens_saved: usize,
    /// Average access count per entry
    pub avg_access_count: usize,
    /// Cache hit rate (tokens saved per access)
    pub hit_rate: f32,
}

/// Prompt builder with prefix caching
pub struct CachedPromptBuilder {
    /// Prefix cache
    cache: PrefixCache,
    /// Current prompt parts
    parts: Vec<String>,
}

impl CachedPromptBuilder {
    /// Create new prompt builder
    pub fn new() -> Self {
        Self {
            cache: PrefixCache::default(),
            parts: Vec::new(),
        }
    }

    /// Create builder with custom cache
    pub fn with_cache(cache: PrefixCache) -> Self {
        Self {
            cache,
            parts: Vec::new(),
        }
    }

    /// Add cached system prompt
    pub fn add_system_prompt(&mut self, content: &str) -> &mut Self {
        let token_count = self.estimate_tokens(content);
        self.cache.add("system", content, token_count);
        self.parts.push(content.to_string());
        self
    }

    /// Add cached prefix
    pub fn add_cached_prefix(&mut self, id: &str, content: &str) -> &mut Self {
        let token_count = self.estimate_tokens(content);
        self.cache.add(id, content, token_count);
        self.parts.push(content.to_string());
        self
    }

    /// Add dynamic content (not cached)
    pub fn add_dynamic(&mut self, content: &str) -> &mut Self {
        self.parts.push(content.to_string());
        self
    }

    /// Build the final prompt
    pub fn build(&self) -> String {
        self.parts.join("\n\n")
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        self.cache.stats()
    }

    /// Estimate token count (rough estimate: 1 token ≈ 4 characters)
    fn estimate_tokens(&self, content: &str) -> usize {
        content.len() / 4
    }

    /// Clear builder (but keep cache)
    pub fn clear(&mut self) {
        self.parts.clear();
    }
}

impl Default for CachedPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_cache_creation() {
        let cache = PrefixCache::new(50);
        assert_eq!(cache.max_entries, 50);
        assert_eq!(cache.stats().total_entries, 0);
    }

    #[test]
    fn test_prefix_cache_add() {
        let mut cache = PrefixCache::default();
        
        let id = cache.add("test", "This is a test prefix", 5);
        
        assert_eq!(id, "test");
        assert_eq!(cache.stats().total_entries, 1);
    }

    #[test]
    fn test_prefix_cache_hit() {
        let mut cache = PrefixCache::default();
        
        let content = "This is a test prefix";
        cache.add("test", content, 5);
        
        let cache_key = cache.compute_cache_key(content);
        let entry = cache.get(&cache_key);
        
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().access_count, 1);
    }

    #[test]
    fn test_prefix_cache_miss() {
        let mut cache = PrefixCache::default();
        
        let entry = cache.get("nonexistent");
        assert!(entry.is_none());
    }

    #[test]
    fn test_cache_capacity_enforcement() {
        let mut cache = PrefixCache::new(3);
        
        cache.add("1", "Content 1", 10);
        cache.add("2", "Content 2", 10);
        cache.add("3", "Content 3", 10);
        cache.add("4", "Content 4", 10); // Should evict LRU
        
        assert_eq!(cache.stats().total_entries, 3);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = PrefixCache::default();
        
        cache.add("test", "Test content", 10);
        let cache_key = cache.compute_cache_key("Test content");
        cache.get(&cache_key); // Access once
        cache.get(&cache_key); // Access twice
        
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.total_tokens_cached, 10);
        assert_eq!(stats.avg_access_count, 2);
    }

    #[test]
    fn test_prompt_builder() {
        let mut builder = CachedPromptBuilder::new();
        
        builder
            .add_system_prompt("You are a helpful assistant.")
            .add_dynamic("User question: What is Rust?");
        
        let prompt = builder.build();
        assert!(prompt.contains("You are a helpful assistant."));
        assert!(prompt.contains("User question: What is Rust?"));
    }

    #[test]
    fn test_prompt_builder_cache_stats() {
        let mut builder = CachedPromptBuilder::new();
        
        builder.add_system_prompt("System prompt here");
        
        let stats = builder.cache_stats();
        assert!(stats.total_entries > 0);
    }
}
