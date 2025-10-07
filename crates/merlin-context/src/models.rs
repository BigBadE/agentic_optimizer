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
    #[must_use]
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

    /// Get default configuration without env vars
    #[must_use]
    pub fn default_config() -> Self {
        Self {
            small: "qwen2.5-coder:1.5b-instruct-q4_K_M".to_string(),
            medium: "qwen2.5-coder:7b-instruct-q4_K_M".to_string(),
            large: "qwen2.5-coder:32b".to_string(),
            embedding: "nomic-embed-text".to_string(),
        }
    }

    /// Select appropriate model based on task complexity
    #[must_use]
    pub fn select_for_task(&self, complexity: TaskComplexity) -> &str {
        match complexity {
            TaskComplexity::Simple => &self.small,
            TaskComplexity::Medium => &self.medium,
            TaskComplexity::Complex => &self.large,
        }
    }
}

/// Task complexity for model selection
#[derive(Debug, Clone, Copy)]
pub enum TaskComplexity {
    /// Simple tasks (quick classifications, basic parsing)
    Simple,
    /// Medium tasks (context planning, code analysis)
    Medium,
    /// Complex tasks (architecture decisions, deep reasoning)
    Complex,
}
