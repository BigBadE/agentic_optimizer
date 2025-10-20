//! Error types for the routing system.

use crate::Error as CoreError;
use crate::task::{TaskId, ValidationResult};
use serde_json;
use serde_json::Error as JsonError;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::{fmt, io};
use thiserror::Error;

/// Result type alias using `RoutingError`.
pub type Result<T> = StdResult<T, RoutingError>;

/// Error types that can occur during task routing and execution.
#[derive(Debug, Error)]
pub enum RoutingError {
    /// Error from merlin-core
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    /// Formatting error
    #[error("Format error: {0}")]
    Format(#[from] fmt::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] JsonError),

    /// Provider is temporarily unavailable
    #[error("Provider temporarily unavailable: {0}")]
    ProviderUnavailable(String),

    /// Rate limit has been exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Operation timed out
    #[error("Timeout after {0}ms")]
    Timeout(u64),

    /// Cyclic dependency detected in task graph
    #[error("Cyclic dependency detected in task graph")]
    CyclicDependency,

    /// Invalid task configuration
    #[error("Invalid task configuration: {0}")]
    InvalidTask(String),

    /// No suitable tier available for routing
    #[error("No available tier for task")]
    NoAvailableTier,

    /// Maximum retry attempts exceeded
    #[error("Max retries exceeded for task {task_id:?}")]
    MaxRetriesExceeded {
        /// ID of the task that exceeded retries
        task_id: TaskId,
        /// Validation result from the last attempt
        validation: ValidationResult,
    },

    /// No higher tier available for escalation
    #[error("No higher tier available for escalation")]
    NoHigherTierAvailable,

    /// File is locked by another task
    #[error("File locked by task {holder:?}: {file}")]
    FileLockedByTask {
        /// Path to the locked file
        file: PathBuf,
        /// Task ID holding the lock
        holder: TaskId,
    },

    /// File has active readers preventing write access
    #[error("File has {readers} active readers: {file}")]
    FileHasActiveReaders {
        /// Path to the file
        file: PathBuf,
        /// Number of active readers
        readers: usize,
    },

    /// Conflict detected during execution
    #[error("Conflict detected: {0:?}")]
    ConflictDetected(ConflictReport),

    /// Maximum conflict resolution retries exceeded
    #[error("Max conflict retries exceeded for task {task_id:?}")]
    MaxConflictRetries {
        /// ID of the task that exceeded conflict retries
        task_id: TaskId,
        /// Conflict report
        report: ConflictReport,
    },

    /// Validation failed
    #[error("Validation failed: {0:?}")]
    ValidationFailed(ValidationResult),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    /// Analysis failed
    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    /// Other error
    #[error("{0}")]
    Other(String),
}

impl RoutingError {
    /// Checks if this error is retryable (transient failure).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ProviderUnavailable(_) | Self::RateLimitExceeded(_) | Self::Timeout(_)
        )
    }

    /// Checks if this error condition allows escalation to a higher tier.
    pub fn can_escalate(&self) -> bool {
        matches!(self, Self::MaxRetriesExceeded { .. })
    }
}

/// Report of file conflicts detected during execution.
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// List of conflicting files
    pub conflicts: Vec<FileConflict>,
}

/// Information about a file conflict.
#[derive(Debug, Clone)]
pub struct FileConflict {
    /// Path to the conflicting file
    pub path: PathBuf,
    /// Hash of the file when task started
    pub base_hash: u64,
    /// Current hash of the file
    pub current_hash: u64,
}
