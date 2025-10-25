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

#[cfg(test)]
#[allow(
    unsafe_code,
    clippy::undocumented_unsafe_blocks,
    reason = "Test module needs to manipulate environment variables"
)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Env var tests can't run in parallel - run with --ignored --test-threads=1"]
    fn test_from_env_uses_defaults() {
        // Save current values
        let old_small = env::var("LOCAL_SMALL_MODEL").ok();
        let old_medium = env::var("LOCAL_MEDIUM_MODEL").ok();
        let old_large = env::var("LARGE_MODEL").ok();
        let old_embed = env::var("EMBEDDING_MODEL").ok();

        // Unset any environment variables that might interfere
        unsafe {
            env::remove_var("LOCAL_SMALL_MODEL");
            env::remove_var("LOCAL_MEDIUM_MODEL");
            env::remove_var("LARGE_MODEL");
            env::remove_var("EMBEDDING_MODEL");
        }

        let config = ModelConfig::from_env();

        // Restore original values
        unsafe {
            if let Some(val) = old_small {
                env::set_var("LOCAL_SMALL_MODEL", val);
            }
            if let Some(val) = old_medium {
                env::set_var("LOCAL_MEDIUM_MODEL", val);
            }
            if let Some(val) = old_large {
                env::set_var("LARGE_MODEL", val);
            }
            if let Some(val) = old_embed {
                env::set_var("EMBEDDING_MODEL", val);
            }
        }

        assert_eq!(config.small, "qwen2.5-coder:1.5b-instruct-q4_K_M");
        assert_eq!(config.medium, "qwen2.5-coder:7b-instruct-q4_K_M");
        assert_eq!(config.large, "qwen2.5-coder:32b");
        assert_eq!(config.embedding, "nomic-embed-text");
    }

    #[test]
    #[ignore = "Env var tests can't run in parallel - run with --ignored --test-threads=1"]
    fn test_from_env_uses_custom_values() {
        // Save current values
        let old_small = env::var("LOCAL_SMALL_MODEL").ok();
        let old_medium = env::var("LOCAL_MEDIUM_MODEL").ok();
        let old_large = env::var("LARGE_MODEL").ok();
        let old_embed = env::var("EMBEDDING_MODEL").ok();

        // Set custom values
        unsafe {
            env::set_var("LOCAL_SMALL_MODEL", "custom-small");
            env::set_var("LOCAL_MEDIUM_MODEL", "custom-medium");
            env::set_var("LARGE_MODEL", "custom-large");
            env::set_var("EMBEDDING_MODEL", "custom-embed");
        }

        // Get config AFTER setting the env vars
        let config = ModelConfig::from_env();

        // Restore original values
        unsafe {
            if let Some(val) = old_small {
                env::set_var("LOCAL_SMALL_MODEL", val);
            } else {
                env::remove_var("LOCAL_SMALL_MODEL");
            }
            if let Some(val) = old_medium {
                env::set_var("LOCAL_MEDIUM_MODEL", val);
            } else {
                env::remove_var("LOCAL_MEDIUM_MODEL");
            }
            if let Some(val) = old_large {
                env::set_var("LARGE_MODEL", val);
            } else {
                env::remove_var("LARGE_MODEL");
            }
            if let Some(val) = old_embed {
                env::set_var("EMBEDDING_MODEL", val);
            } else {
                env::remove_var("EMBEDDING_MODEL");
            }
        }

        // Verify
        assert_eq!(config.small, "custom-small");
        assert_eq!(config.medium, "custom-medium");
        assert_eq!(config.large, "custom-large");
        assert_eq!(config.embedding, "custom-embed");
    }
}
