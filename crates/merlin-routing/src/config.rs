//! Configuration types for routing, validation, execution, and workspace settings.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete routing configuration.
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Model tier configuration
    pub tiers: TierConfig,
    /// Validation configuration
    pub validation: ValidationConfig,
    /// Execution configuration
    pub execution: ExecutionConfig,
    /// Workspace configuration
    pub workspace: WorkspaceConfig,
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

/// Validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Configuration struct can have more bools"
)]
pub struct ValidationConfig {
    /// Whether validation is enabled
    pub enabled: bool,
    /// Whether to stop on first validation failure
    pub early_exit: bool,
    /// Whether to check syntax
    pub syntax_check: bool,
    /// Whether to check build
    pub build_check: bool,
    /// Whether to run tests
    pub test_check: bool,
    /// Whether to run linting
    pub lint_check: bool,
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
            syntax_check: true,
            build_check: true,
            test_check: true,
            lint_check: true,
            build_timeout_seconds: 60,
            test_timeout_seconds: 300,
        }
    }
}

/// Execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum number of tasks to execute concurrently
    pub max_concurrent_tasks: usize,
    /// Whether parallel execution is enabled
    pub enable_parallel: bool,
    /// Whether conflict detection is enabled
    pub enable_conflict_detection: bool,
    /// Whether file locking is enabled
    pub enable_file_locking: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            enable_parallel: true,
            enable_conflict_detection: true,
            enable_file_locking: true,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    /// # Panics
    /// Panics if default config does not meet baseline expectations.
    fn test_default_config() {
        let config = RoutingConfig::default();
        assert!(config.tiers.local_enabled);
        assert!(config.validation.enabled);
        assert_eq!(config.execution.max_concurrent_tasks, 4);
    }

    #[test]
    /// # Panics
    /// Panics if serialization or deserialization fails.
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
