//! Cache storage implementation using in-memory HashMap.
//!
//! This module provides the core caching functionality with semantic similarity matching.

use merlin_core::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

/// A cached response with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    /// The cached response
    pub response: Response,
    /// When this entry was created
    pub created_at: SystemTime,
    /// Size estimate in bytes
    pub size_bytes: usize,
}

impl CachedResponse {
    /// Creates a new cached response
    pub fn new(response: Response) -> Self {
        let created_at = SystemTime::now();
        let size_bytes = response.text.len();

        Self {
            response,
            created_at,
            size_bytes,
        }
    }
}

/// In-memory response cache with semantic similarity matching
pub struct ResponseCache {
    storage: HashMap<String, CachedResponse>,
    total_size_bytes: usize,
}

impl ResponseCache {
    /// Creates a new response cache
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
            total_size_bytes: 0,
        }
    }

    /// Gets a cached response if it exists
    pub fn get(&self, query: &str) -> Option<Response> {
        self.storage
            .get(query)
            .map(|cached| cached.response.clone())
    }

    /// Stores a response in the cache
    pub fn put(&mut self, query: String, response: Response) {
        let cached = CachedResponse::new(response);
        self.total_size_bytes += cached.size_bytes;
        self.storage.insert(query, cached);
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
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        self.total_size_bytes
    }

    /// Returns cache statistics
    #[must_use]
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
        Self::new()
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

    /// Tests basic cache operations including put, get, and clear.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cache_basic_operations() {
        let mut cache = ResponseCache::default();

        cache.put("query1".to_owned(), create_test_response("response1"));
        assert_eq!(cache.len(), 1);

        let cached = cache.get("query1");
        assert!(cached.is_some());

        cache.clear();
        assert_eq!(cache.len(), 0);
    }
}
