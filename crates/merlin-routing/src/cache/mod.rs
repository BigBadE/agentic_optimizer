//! Response caching with semantic similarity matching.
//!
//! This module provides caching infrastructure for LLM responses to reduce
//! API costs and improve response times for similar queries.

/// Cache storage implementation
pub mod storage;

pub use storage::{CachedResponse, ResponseCache};
