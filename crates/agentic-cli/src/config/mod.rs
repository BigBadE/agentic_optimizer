use serde::{Deserialize, Serialize};
use std::env;
use std::fs::read_to_string;
use std::path::PathBuf;

use agentic_core::Result;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
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
                eprintln!("Warning: Failed to load config.toml: {}", error);
                Self::from_env()
            })
        } else {
            Self::from_env()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub openrouter_key: Option<String>,
    pub high_model: Option<String>,
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

