use serde::{Deserialize, Serialize};
use std::env;
use std::fs::read_to_string;
use std::path::PathBuf;

use merlin_core::Result;

/// Main configuration for the agentic optimizer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Provider configuration (API keys and models)
    pub providers: ProvidersConfig,
}

impl Config {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_env() -> Self {
        Self::default()
    }

    pub fn load_from_project(project_root: &PathBuf) -> Self {
        let config_path = project_root.join("config.toml");
        if config_path.exists() {
            Self::from_file(&config_path).unwrap_or_else(|error| {
                eprintln!("Warning: Failed to load config.toml: {error}");
                Self::from_env()
            })
        } else {
            Self::from_env()
        }
    }
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
            openrouter_key: env::var("OPENROUTER_API_KEY").ok(),
            high_model: Some("anthropic/claude-sonnet-4-20250514".to_owned()),
            medium_model: Some("anthropic/claude-3.5-sonnet".to_owned()),
        }
    }
}


