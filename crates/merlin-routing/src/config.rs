//! Configuration types for routing, validation, execution, and workspace settings.

use crate::error::{Result, RoutingError};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Complete routing configuration.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Model tier configuration
    pub tiers: TierConfig,
    /// API keys for model providers
    pub api_keys: ApiKeys,
    /// Validation configuration
    pub validation: ValidationConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Workspace configuration
    pub workspace: WorkspaceConfig,
    /// Cache configuration
    pub cache: CacheConfig,
}

/// API keys for model providers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiKeys {
    /// Anthropic API key for Claude models
    pub anthropic_api_key: Option<String>,
    /// Groq API key for Groq models
    pub groq_api_key: Option<String>,
    /// `OpenRouter` API key for various models
    pub openrouter_api_key: Option<String>,
}

/// Cache configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Whether caching is enabled
    pub enabled: bool,
    /// Time-to-live for cache entries in hours
    pub ttl_hours: u64,
    /// Maximum cache size in megabytes
    pub max_size_mb: usize,
    /// Similarity threshold for semantic matching (0.0-1.0)
    pub similarity_threshold: f32,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl_hours: 24,
            max_size_mb: 100,
            similarity_threshold: 0.95,
        }
    }
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

/// Validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Whether validation is enabled
    pub enabled: bool,
    /// Whether to stop on first validation failure
    pub early_exit: bool,
    /// Checks to perform during validation
    pub checks: ValidationChecks,
    /// Timeout in seconds for build operations
    pub build_timeout_seconds: u64,
    /// Timeout in seconds for test operations
    pub test_timeout_seconds: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            early_exit: true,
            checks: ValidationChecks::default(),
            build_timeout_seconds: 60,
            test_timeout_seconds: 300,
        }
    }
}

/// Execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Configuration struct with multiple feature flags"
)]
pub struct ExecutionConfig {
    /// Maximum number of tasks to execute concurrently
    pub max_concurrent_tasks: usize,
    /// Whether parallel execution is enabled
    pub enable_parallel: bool,
    /// Whether conflict detection is enabled
    pub enable_conflict_detection: bool,
    /// Whether file locking is enabled
    pub enable_file_locking: bool,
    /// Dump full context to debug.log before each model call
    pub context_dump: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            enable_parallel: true,
            enable_conflict_detection: true,
            enable_file_locking: true,
            context_dump: false,
        }
    }
}

/// Workspace configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root path of the workspace
    pub root_path: PathBuf,
    /// Whether workspace snapshots are enabled
    pub enable_snapshots: bool,
    /// Whether transactional operations are enabled
    pub enable_transactions: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root_path: PathBuf::from("."),
            enable_snapshots: true,
            enable_transactions: true,
        }
    }
}

impl RoutingConfig {
    /// Get the default config directory path (`~/.merlin`)
    ///
    /// # Errors
    /// Returns an error if the home directory cannot be determined
    pub fn config_dir() -> Result<PathBuf> {
        use dirs::home_dir;
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
        use toml::from_str;
        let contents = fs::read_to_string(path)
            .map_err(|error| RoutingError::Other(format!("Failed to read config: {error}")))?;
        from_str(&contents)
            .map_err(|error| RoutingError::Other(format!("Failed to parse config: {error}")))
    }

    /// Save config to a specific file
    ///
    /// # Errors
    /// Returns an error if the file cannot be written
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        use toml::to_string_pretty;
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
            "anthropic" => self
                .api_keys
                .anthropic_api_key
                .clone()
                .or_else(|| env::var("ANTHROPIC_API_KEY").ok()),
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
    use serde_json::{from_str, to_string};

    #[test]
    fn test_default_config() {
        let config = RoutingConfig::default();
        assert!(config.tiers.local_enabled);
        assert!(config.validation.enabled);
        assert_eq!(config.execution.max_concurrent_tasks, 4);
    }

    #[test]
    fn test_serialization() {
        let config = RoutingConfig::default();
        let json = match to_string(&config) {
            Ok(serialized_json) => serialized_json,
            Err(error) => panic!("serialize failed: {error}"),
        };
        let deserialized: RoutingConfig = match from_str(&json) {
            Ok(value) => value,
            Err(error) => panic!("deserialize failed: {error}"),
        };
        assert_eq!(config.tiers.local_model, deserialized.tiers.local_model);
    }
}
