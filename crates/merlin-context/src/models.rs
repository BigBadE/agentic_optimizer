//! Model configuration for various AI tasks.

use std::env;

/// Model configuration for different task sizes
pub struct ModelConfig {
    /// Small, fast model for simple tasks
    pub small: String,
    /// Medium model for balanced performance
    pub medium: String,
    /// Large model for complex reasoning
    pub large: String,
    /// Embedding model for semantic search
    pub embedding: String,
}

impl ModelConfig {
    /// Get model configuration from environment variables with fallback defaults
    pub fn from_env() -> Self {
        Self {
            small: env::var("LOCAL_SMALL_MODEL")
                .unwrap_or_else(|_| "qwen2.5-coder:1.5b-instruct-q4_K_M".to_string()),
            medium: env::var("LOCAL_MEDIUM_MODEL")
                .unwrap_or_else(|_| "qwen2.5-coder:7b-instruct-q4_K_M".to_string()),
            large: env::var("LARGE_MODEL").unwrap_or_else(|_| "qwen2.5-coder:32b".to_string()),
            embedding: env::var("EMBEDDING_MODEL")
                .unwrap_or_else(|_| "nomic-embed-text".to_string()),
        }
    }
}
