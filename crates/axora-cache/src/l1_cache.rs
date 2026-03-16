//! L1 in-memory cache using DashMap

use crate::CacheError;
use crate::Result;
use dashmap::DashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get current timestamp in milliseconds
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// Cache entry with TTL
pub struct CacheEntry<T> {
    value: T,
    expires_at: u64,
}

/// L1 in-memory cache
pub struct L1Cache<T> {
    cache: DashMap<String, CacheEntry<T>>,
    default_ttl_secs: u64,
}

impl<T> L1Cache<T>
where
    T: Clone + Send + Sync,
{
    /// Create new L1 cache
    pub fn new(default_ttl_secs: u64) -> Self {
        Self {
            cache: DashMap::new(),
            default_ttl_secs,
        }
    }

    /// Get value from cache
    pub fn get(&self, key: &str) -> Result<T>
    where
        T: Clone,
    {
        if let Some(entry) = self.cache.get(key) {
            let now = now_millis();
            if now < entry.expires_at {
                return Ok(entry.value.clone());
            }
        }
        Err(CacheError::Miss)
    }

    /// Set value in cache
    pub fn set(&self, key: &str, value: T) {
        let now = now_millis();
        let entry = CacheEntry {
            value,
            expires_at: now + (self.default_ttl_secs * 1000),
        };
        self.cache.insert(key.to_string(), entry);
    }

    /// Remove value from cache
    pub fn remove(&self, key: &str) {
        self.cache.remove(key);
    }

    /// Clear all expired entries
    pub fn cleanup(&self) {
        let now = now_millis();
        self.cache.retain(|_, entry| now < entry.expires_at);
    }
}

impl<T> Default for L1Cache<T>
where
    T: Clone + Send + Sync,
{
    fn default() -> Self {
        Self::new(3600) // 1 hour default TTL
    }
}
