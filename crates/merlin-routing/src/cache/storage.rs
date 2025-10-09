//! Cache storage implementation using in-memory HashMap.
//!
//! This module provides the core caching functionality with TTL-based expiration
//! and optional semantic similarity matching.

use crate::config::CacheConfig;
use merlin_core::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// A cached response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The cached response
    pub response: Response,
    /// When this entry was created
    pub created_at: SystemTime,
    /// When this entry expires
    pub expires_at: SystemTime,
    /// Size estimate in bytes
    pub size_bytes: usize,
}

impl CachedResponse {
    /// Creates a new cached response with the given TTL
    pub fn new(response: Response, ttl: Duration) -> Self {
        let created_at = SystemTime::now();
        let expires_at = created_at + ttl;
        let size_bytes = response.text.len();

        Self {
            response,
            created_at,
            expires_at,
            size_bytes,
        }
    }

    /// Checks if this cache entry has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
}

/// In-memory response cache with TTL-based expiration
pub struct ResponseCache {
    storage: HashMap<String, CachedResponse>,
    config: CacheConfig,
    total_size_bytes: usize,
}

impl ResponseCache {
    /// Creates a new response cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        Self {
            storage: HashMap::new(),
            config,
            total_size_bytes: 0,
        }
    }

    /// Gets a cached response if it exists and hasn't expired
    pub fn get(&mut self, query: &str) -> Option<Response> {
        if !self.config.enabled {
            return None;
        }

        // Check for exact match
        if let Some(cached) = self.storage.get(query) {
            if !cached.is_expired() {
                return Some(cached.response.clone());
            }
            // Remove expired entry
            self.remove(query);
        }

        None
    }

    /// Stores a response in the cache
    pub fn put(&mut self, query: String, response: Response) {
        if !self.config.enabled {
            return;
        }

        let ttl = Duration::from_secs(self.config.ttl_hours * 3600);
        let cached = CachedResponse::new(response, ttl);

        // Check if we need to evict entries to stay under size limit
        if self.config.max_size_mb > 0 {
            let max_bytes = self.config.max_size_mb * 1024 * 1024;
            while self.total_size_bytes + cached.size_bytes > max_bytes && !self.storage.is_empty()
            {
                self.evict_oldest();
            }
        }

        self.total_size_bytes += cached.size_bytes;
        self.storage.insert(query, cached);
    }

    /// Removes a specific entry from the cache
    fn remove(&mut self, query: &str) {
        if let Some(cached) = self.storage.remove(query) {
            self.total_size_bytes = self.total_size_bytes.saturating_sub(cached.size_bytes);
        }
    }

    /// Evicts the oldest entry from the cache
    fn evict_oldest(&mut self) {
        if let Some((oldest_key, _)) = self
            .storage
            .iter()
            .min_by_key(|(_, cached)| cached.created_at)
        {
            let key = oldest_key.clone();
            self.remove(&key);
        }
    }

    /// Clears all expired entries from the cache
    pub fn clear_expired(&mut self) {
        let expired_keys: Vec<String> = self
            .storage
            .iter()
            .filter(|(_, cached)| cached.is_expired())
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.remove(&key);
        }
    }

    /// Clears all entries from the cache
    pub fn clear(&mut self) {
        self.storage.clear();
        self.total_size_bytes = 0;
    }

    /// Returns the number of entries in the cache
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    /// Returns whether the cache is empty
    pub fn is_empty(&self) -> bool {
        self.storage.is_empty()
    }

    /// Returns the total size of cached data in bytes
    pub fn size_bytes(&self) -> usize {
        self.total_size_bytes
    }

    /// Returns cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.len(),
            size_bytes: self.size_bytes(),
            size_mb: self.size_bytes() as f64 / (1024.0 * 1024.0),
        }
    }
}

impl Default for ResponseCache {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in the cache
    pub entries: usize,
    /// Total size in bytes
    pub size_bytes: usize,
    /// Total size in megabytes
    pub size_mb: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_core::TokenUsage;

    fn create_test_response(text: &str) -> Response {
        Response {
            text: text.to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 100,
        }
    }

    #[test]
    fn test_cache_put_and_get() {
        let mut cache = ResponseCache::default();
        let query = "test query".to_owned();
        let response = create_test_response("test response");

        cache.put(query.clone(), response);

        let cached = cache.get(&query);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().text, "test response");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ResponseCache::default();
        let cached = cache.get("nonexistent");
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_expiration() {
        let config = CacheConfig {
            enabled: true,
            ttl_hours: 0, // Expire immediately
            max_size_mb: 100,
            similarity_threshold: 0.95,
        };
        let mut cache = ResponseCache::new(config);
        let query = "test query".to_owned();
        let response = create_test_response("test response");

        cache.put(query.clone(), response);

        // Entry should be expired and removed
        let cached = cache.get(&query);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_disabled() {
        let config = CacheConfig {
            enabled: false,
            ttl_hours: 24,
            max_size_mb: 100,
            similarity_threshold: 0.95,
        };
        let mut cache = ResponseCache::new(config);
        let query = "test query".to_owned();
        let response = create_test_response("test response");

        cache.put(query.clone(), response);

        // Cache is disabled, should not store
        assert_eq!(cache.len(), 0);
        let cached = cache.get(&query);
        assert!(cached.is_none());
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ResponseCache::default();
        cache.put("query1".to_owned(), create_test_response("response1"));
        cache.put("query2".to_owned(), create_test_response("response2"));

        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.size_bytes(), 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut cache = ResponseCache::default();
        cache.put("query1".to_owned(), create_test_response("response1"));
        cache.put("query2".to_owned(), create_test_response("response2"));

        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert!(stats.size_bytes > 0);
        assert!(stats.size_mb > 0.0);
    }
}
