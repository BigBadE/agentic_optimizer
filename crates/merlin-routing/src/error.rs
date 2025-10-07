use crate::types::{TaskId, ValidationResult};
use merlin_core::Error as CoreError;
use serde_json;
use serde_json::Error as JsonError;
use std::path::PathBuf;
use std::result::Result as StdResult;
use std::{fmt, io};
use thiserror::Error;

pub type Result<T> = StdResult<T, RoutingError>;

#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Format error: {0}")]
    Format(#[from] fmt::Error),

    #[error("JSON error: {0}")]
    Json(#[from] JsonError),

    #[error("Provider temporarily unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Cyclic dependency detected in task graph")]
    CyclicDependency,

    #[error("Invalid task configuration: {0}")]
    InvalidTask(String),

    #[error("No available tier for task")]
    NoAvailableTier,

    #[error("Max retries exceeded for task {task_id:?}")]
    MaxRetriesExceeded {
        task_id: TaskId,
        validation: ValidationResult,
    },

    #[error("No higher tier available for escalation")]
    NoHigherTierAvailable,

    #[error("File locked by task {holder:?}: {file}")]
    FileLockedByTask { file: PathBuf, holder: TaskId },

    #[error("File has {readers} active readers: {file}")]
    FileHasActiveReaders { file: PathBuf, readers: usize },

    #[error("Conflict detected: {0:?}")]
    ConflictDetected(ConflictReport),

    #[error("Max conflict retries exceeded for task {task_id:?}")]
    MaxConflictRetries {
        task_id: TaskId,
        report: ConflictReport,
    },

    #[error("Validation failed: {0:?}")]
    ValidationFailed(ValidationResult),

    #[error("Task execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),

    #[error("{0}")]
    Other(String),
}

impl RoutingError {
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::ProviderUnavailable(_) | Self::RateLimitExceeded(_) | Self::Timeout(_)
        )
    }

    #[must_use]
    pub fn can_escalate(&self) -> bool {
        matches!(self, Self::MaxRetriesExceeded { .. })
    }
}

#[derive(Debug, Clone)]
pub struct ConflictReport {
    pub conflicts: Vec<FileConflict>,
}

#[derive(Debug, Clone)]
pub struct FileConflict {
    pub path: PathBuf,
    pub base_hash: u64,
    pub current_hash: u64,
}
