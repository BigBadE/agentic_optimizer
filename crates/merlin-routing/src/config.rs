use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub tiers: TierConfig,
    pub validation: ValidationConfig,
    pub execution: ExecutionConfig,
    pub workspace: WorkspaceConfig,
}

impl RoutingConfig {
    pub fn default_config() -> Self {
        Self {
            tiers: TierConfig::default(),
            validation: ValidationConfig::default(),
            execution: ExecutionConfig::default(),
            workspace: WorkspaceConfig::default(),
        }
    }
}

impl Default for RoutingConfig {
    fn default() -> Self {
        Self::default_config()
    }
}

/// Model tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub local_enabled: bool,
    pub local_model: String,
    pub groq_enabled: bool,
    pub groq_model: String,
    pub premium_enabled: bool,
    pub max_retries: usize,
    pub timeout_seconds: u64,
}

impl Default for TierConfig {
    fn default() -> Self {
        Self {
            local_enabled: true,
            local_model: "qwen2.5-coder:7b".to_string(),
            groq_enabled: true,
            groq_model: "llama-3.1-70b-versatile".to_string(),
            premium_enabled: true,
            max_retries: 3,
            timeout_seconds: 300,
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub enabled: bool,
    pub early_exit: bool,
    pub syntax_check: bool,
    pub build_check: bool,
    pub test_check: bool,
    pub lint_check: bool,
    pub build_timeout_seconds: u64,
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

/// Execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub max_concurrent_tasks: usize,
    pub enable_parallel: bool,
    pub enable_conflict_detection: bool,
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

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root_path: PathBuf,
    pub enable_snapshots: bool,
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
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: RoutingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.tiers.local_model, deserialized.tiers.local_model);
    }
}
