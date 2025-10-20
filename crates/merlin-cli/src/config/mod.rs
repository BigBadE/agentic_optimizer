use serde::{Deserialize, Serialize};
use std::env;
use std::fs::read_to_string;
use std::path::Path;
use toml::from_str;
use tracing::warn;

use merlin_core::{Result, RoutingError};

const ENV_OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";

/// Main configuration for the agentic optimizer
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Provider configuration (API keys and models)
    pub providers: ProvidersConfig,
}

impl Config {
    /// Load configuration from a TOML file.
    ///
    /// # Errors
    /// Returns an error if reading the file or parsing TOML fails.
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = read_to_string(path)?;
        let config: Self = from_str(&content)
            .map_err(|err| RoutingError::InvalidTask(format!("Failed to parse config: {err}")))?;
        Ok(config)
    }

    /// Create config from environment variables
    pub fn from_env() -> Self {
        Self::default()
    }

    /// Load config from project directory
    pub fn load_from_project(project_root: &Path) -> Self {
        let config_path = project_root.join("config.toml");
        if config_path.exists() {
            Self::from_file(&config_path).unwrap_or_else(|error| {
                warn!("Warning: Failed to load config.toml: {error}");
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
            openrouter_key: env::var(ENV_OPENROUTER_API_KEY).ok(),
            high_model: Some("anthropic/claude-sonnet-4-20250514".to_owned()),
            medium_model: Some("anthropic/claude-3.5-sonnet".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    use toml::to_string_pretty;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.providers.high_model.is_some());
        assert!(config.providers.medium_model.is_some());
    }

    #[test]
    fn test_config_from_env() {
        let config = Config::from_env();
        assert!(config.providers.high_model.is_some());
        assert_eq!(
            config.providers.high_model.as_deref(),
            Some("anthropic/claude-sonnet-4-20250514")
        );
    }

    #[test]
    fn test_config_from_valid_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let config_file = temp.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[providers]
openrouter_key = "test-key-123"
high_model = "test/high-model"
medium_model = "test/medium-model"
"#,
        )
        .expect("Failed to write config file");

        let config = Config::from_file(&config_file).expect("Failed to load config");
        assert_eq!(
            config.providers.openrouter_key.as_deref(),
            Some("test-key-123")
        );
        assert_eq!(
            config.providers.high_model.as_deref(),
            Some("test/high-model")
        );
        assert_eq!(
            config.providers.medium_model.as_deref(),
            Some("test/medium-model")
        );
    }

    #[test]
    fn test_config_from_invalid_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let config_file = temp.path().join("invalid.toml");

        fs::write(&config_file, "invalid toml content {{").expect("Failed to write file");

        let result = Config::from_file(&config_file);
        assert!(result.is_err(), "Should fail on invalid TOML");
    }

    #[test]
    fn test_config_from_nonexistent_file() {
        let result = Config::from_file(Path::new("/nonexistent/path/config.toml"));
        assert!(result.is_err(), "Should fail on nonexistent file");
    }

    #[test]
    fn test_load_from_project_with_config() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let config_file = temp.path().join("config.toml");

        fs::write(
            &config_file,
            r#"
[providers]
high_model = "custom/model"
"#,
        )
        .expect("Failed to write config file");

        let config = Config::load_from_project(temp.path());
        assert_eq!(config.providers.high_model.as_deref(), Some("custom/model"));
    }

    #[test]
    fn test_load_from_project_without_config() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let config = Config::load_from_project(temp.path());

        assert_eq!(
            config.providers.high_model.as_deref(),
            Some("anthropic/claude-sonnet-4-20250514")
        );
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
