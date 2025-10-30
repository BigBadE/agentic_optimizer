//! Configuration types for routing, validation, execution, and workspace settings.

use crate::routing_error::{Result, RoutingError};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Complete routing configuration (global, stored in `~/.merlin/config.toml`).
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Model tier configuration
    #[serde(default)]
    pub tiers: TierConfig,
    /// API keys for model providers
    #[serde(default)]
    pub api_keys: ApiKeys,
}

/// API keys for model providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// Groq API key for Groq models
    pub groq_api_key: Option<String>,
    /// `OpenRouter` API key for various models (including Claude via anthropic/* routes)
    pub openrouter_api_key: Option<String>,
}

/// Model tier configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    /// Whether local model tier is enabled
    pub local_enabled: bool,
    /// Default local model name
    pub local_model: String,
    /// Whether Groq tier is enabled
    pub groq_enabled: bool,
    /// Default Groq model name
    pub groq_model: String,
    /// Whether premium tier is enabled
    pub premium_enabled: bool,
    /// Maximum retry attempts per task
    pub max_retries: usize,
    /// Timeout in seconds for model requests
    pub timeout_seconds: u64,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            local_enabled: true,
            local_model: "qwen2.5-coder:7b".to_owned(),
            groq_enabled: true,
            groq_model: "llama-3.1-70b-versatile".to_owned(),
            premium_enabled: true,
            max_retries: 3,
            timeout_seconds: 300,
        }
    }
}

/// Types of validation checks that can be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValidationCheckType {
    /// Syntax validation
    Syntax,
    /// Build validation
    Build,
    /// Test validation
    Test,
    /// Lint validation
    Lint,
}

/// Validation checks to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationChecks {
    /// Set of checks to perform
    pub enabled_checks: Vec<ValidationCheckType>,
}

impl ValidationChecks {
    /// Check if a specific validation type is enabled.
    pub fn is_enabled(&self, check_type: ValidationCheckType) -> bool {
        self.enabled_checks.contains(&check_type)
    }

    /// Enable all validation checks.
    pub fn all() -> Self {
        Self {
            enabled_checks: vec![
                ValidationCheckType::Syntax,
                ValidationCheckType::Build,
                ValidationCheckType::Test,
                ValidationCheckType::Lint,
            ],
        }
    }

    /// Disable all validation checks.
    pub fn none() -> Self {
        Self {
            enabled_checks: vec![],
        }
    }
}

impl Default for ValidationChecks {
    fn default() -> Self {
        Self::all()
    }
}

/// Validation configuration (always enabled, never early-exit).
/// Timeouts are now per-project in `ProjectConfig`.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Checks to perform during validation
    #[serde(default)]
    pub checks: ValidationChecks,
}

/// Per-project configuration (stored in `<project>/.merlin/config.toml`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    /// Checks to perform during validation
    #[serde(default)]
    pub validation_checks: ValidationChecks,
    /// Timeout in seconds for build operations (since last output line, capped at 1000 lines)
    #[serde(default = "default_build_timeout")]
    pub build_timeout_seconds: u64,
    /// Timeout in seconds for test operations (since last output line, capped at 1000 lines)
    #[serde(default = "default_test_timeout")]
    pub test_timeout_seconds: u64,
    /// Whether the workspace is read-only (prevents file modifications)
    #[serde(default)]
    pub read_only: bool,
}

const fn default_build_timeout() -> u64 {
    60
}

const fn default_test_timeout() -> u64 {
    300
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            validation_checks: ValidationChecks::default(),
            build_timeout_seconds: default_build_timeout(),
            test_timeout_seconds: default_test_timeout(),
            read_only: false,
        }
    }
}

impl ProjectConfig {
    /// Load project config from `.merlin/config.toml` in the given directory.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed
    pub fn load_from_dir(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join(".merlin").join("config.toml");
        if !config_path.exists() {
            return Ok(Self::default());
        }
        Self::load_from_file(&config_path)
    }

    /// Load project config from a specific file.
    ///
    /// # Errors
    /// Returns an error if the config file cannot be read or parsed
    pub fn load_from_file(path: &Path) -> Result<Self> {
        use merlin_deps::toml::from_str;

        let contents = fs::read_to_string(path)
            .map_err(|err| RoutingError::Other(format!("Failed to read config: {err}")))?;
        from_str(&contents)
            .map_err(|err| RoutingError::Other(format!("Failed to parse config: {err}")))
    }
}

impl RoutingConfig {
    /// Get the default config directory path (`~/.merlin`)
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined
    pub fn config_dir() -> Result<PathBuf> {
        use merlin_deps::dirs::home_dir;
        let home = home_dir()
            .ok_or_else(|| RoutingError::Other("Could not determine home directory".to_owned()))?;
        Ok(home.join(".merlin"))
    }

    /// Get the default config file path (`~/.merlin/config.toml`)
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined
    pub fn config_path() -> Result<PathBuf> {
        Ok(Self::config_dir()?.join("config.toml"))
    }

    /// Load config from the default location (`~/.merlin/config.toml`)
    /// If the config doesn't exist, creates it with default values
    ///
    /// # Errors
    /// Returns an error if the config cannot be read or created
    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::config_path()?;

        if config_path.exists() {
            Self::load_from_file(&config_path)
        } else {
            let config = Self::default();
            config.save_to_file(&config_path)?;
            Ok(config)
        }
    }

    /// Load config from a specific file
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed
    pub fn load_from_file(path: &Path) -> Result<Self> {
        use merlin_deps::toml::from_str;
        let contents = fs::read_to_string(path)
            .map_err(|error| RoutingError::Other(format!("Failed to read config: {error}")))?;
        let config: Self = from_str(&contents)
            .map_err(|error| RoutingError::Other(format!("Failed to parse config: {error}")))?;

        merlin_deps::tracing::debug!(
            "Loaded config from {:?}: groq_api_key={}, openrouter_api_key={}",
            path,
            if config.api_keys.groq_api_key.is_some() {
                "present"
            } else {
                "missing"
            },
            if config.api_keys.openrouter_api_key.is_some() {
                "present"
            } else {
                "missing"
            }
        );

        Ok(config)
    }

    /// Save config to a specific file
    ///
    /// # Errors
    /// Returns an error if the file cannot be written
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        use merlin_deps::toml::to_string_pretty;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                RoutingError::Other(format!("Failed to create config directory: {error}"))
            })?;
        }

        let contents = to_string_pretty(self)
            .map_err(|error| RoutingError::Other(format!("Failed to serialize config: {error}")))?;

        let header = "# Merlin Configuration File\n\
                      # This file is automatically generated on first run\n\
                      # Edit this file to customize your settings\n\n";

        fs::write(path, format!("{header}{contents}"))
            .map_err(|error| RoutingError::Other(format!("Failed to write config: {error}")))?;

        Ok(())
    }

    /// Get API key for a provider, checking config first, then environment variables
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        match provider {
            "groq" => self
                .api_keys
                .groq_api_key
                .clone()
                .or_else(|| env::var("GROQ_API_KEY").ok()),
            "openrouter" => self
                .api_keys
                .openrouter_api_key
                .clone()
                .or_else(|| env::var("OPENROUTER_API_KEY").ok()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use merlin_deps::anyhow::Result;

    #[test]
    fn test_api_key_loading_from_toml() -> Result<()> {
        use merlin_deps::tempfile::NamedTempFile;
        use std::io::Write as _;

        // Create a temporary config file with API keys
        let toml_content = r#"
[tiers]
local_enabled = true
local_model = "qwen2.5-coder:7b"
groq_enabled = true
groq_model = "llama-3.1-70b-versatile"
premium_enabled = true
max_retries = 3
timeout_seconds = 300

[api_keys]
groq_api_key = "test_groq_key_123"
openrouter_api_key = "test_openrouter_key_456"
"#;

        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(toml_content.as_bytes())?;

        // Load config from the temp file
        let config = RoutingConfig::load_from_file(temp_file.path())?;

        // Verify API keys were loaded
        assert_eq!(
            config.api_keys.groq_api_key,
            Some("test_groq_key_123".to_owned())
        );
        assert_eq!(
            config.api_keys.openrouter_api_key,
            Some("test_openrouter_key_456".to_owned())
        );

        // Verify get_api_key method works
        assert_eq!(
            config.get_api_key("groq"),
            Some("test_groq_key_123".to_owned())
        );
        assert_eq!(
            config.get_api_key("openrouter"),
            Some("test_openrouter_key_456".to_owned())
        );
        Ok(())
    }

    #[test]
    fn test_load_actual_config_if_exists() -> Result<()> {
        // This test checks if the actual ~/.merlin/config.toml can be loaded
        // It's optional - passes if the file doesn't exist
        if let Ok(config_path) = RoutingConfig::config_path()
            && config_path.exists()
        {
            let config = RoutingConfig::load_from_file(&config_path)?;

            // Just verify it loaded without crashing
            merlin_deps::tracing::debug!("Loaded config from {config_path:?}");
            merlin_deps::tracing::debug!(
                "  groq_api_key present: {}",
                config.api_keys.groq_api_key.is_some()
            );
            merlin_deps::tracing::debug!(
                "  openrouter_api_key present: {}",
                config.api_keys.openrouter_api_key.is_some()
            );

            // Verify get_api_key returns the keys
            if config.api_keys.groq_api_key.is_some() {
                assert!(
                    config.get_api_key("groq").is_some(),
                    "groq_api_key is set in file but get_api_key returns None"
                );
            }
        }
        Ok(())
    }
}
