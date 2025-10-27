#![allow(dead_code, reason = "Work in progress")]
//! Configuration management for Merlin CLI
//!
//! Handles loading configuration from files and environment variables.

use serde::{Deserialize, Serialize};
use std::env;

const ENV_OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";

/// Main configuration for the agentic optimizer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Provider configuration (API keys and models)
    pub providers: ProvidersConfig,
}

/// Configuration for remote model providers
///
/// This is where you configure which models to use for different task complexities.
/// Add or modify models here to customize the AI behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// `OpenRouter` API key for accessing remote models
    pub openrouter_key: Option<String>,
    /// High-complexity model for demanding tasks (default: anthropic/claude-sonnet-4-20250514)
    pub high_model: Option<String>,
    /// Medium-complexity model for balanced performance (default: anthropic/claude-3.5-sonnet)
    pub medium_model: Option<String>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            openrouter_key: env::var(ENV_OPENROUTER_API_KEY).ok(),
            high_model: Some("anthropic/claude-sonnet-4-20250514".to_owned()),
            medium_model: Some("anthropic/claude-3.5-sonnet".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toml::to_string_pretty;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.high_model.is_some());
        assert!(config.providers.medium_model.is_some());
    }

    #[test]
    fn test_providers_config_default() {
        let config = ProvidersConfig::default();
        assert!(config.high_model.is_some());
        assert!(config.medium_model.is_some());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = to_string_pretty(&config).expect("Failed to serialize");
        assert!(toml_str.contains("providers"));
    }
}
