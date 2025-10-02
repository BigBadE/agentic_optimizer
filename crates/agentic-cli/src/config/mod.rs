use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use agentic_core::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub providers: ProvidersConfig,
    pub context: ContextConfig,
}

impl Config {
    pub fn default() -> Self {
        Self {
            providers: ProvidersConfig::default(),
            context: ContextConfig::default(),
        }
    }

    #[allow(dead_code)]
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_env() -> Self {
        Self::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub anthropic_api_key: Option<String>,
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    pub max_files: usize,
    pub max_file_size: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_files: 50,
            max_file_size: 100_000,
        }
    }
}
