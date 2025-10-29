//! Task execution context and file operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::conversation::WorkUnit;
use crate::{Response, TokenUsage};

use super::core::TaskId;
use super::validation::ValidationResult;

/// Result of executing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// ID of the task that was executed
    pub task_id: TaskId,
    /// Response from the model
    pub response: Response,
    /// Name of the model tier that was used
    pub tier_used: String,
    /// Token usage statistics
    pub tokens_used: TokenUsage,
    /// Validation results
    pub validation: ValidationResult,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Optional `WorkUnit` containing the work performed for this task
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_unit: Option<WorkUnit>,
}

/// File change operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileChange {
    /// Create a new file
    Create {
        /// Path to create
        path: PathBuf,
        /// Initial content
        content: String,
    },
    /// Modify an existing file
    Modify {
        /// Path to modify
        path: PathBuf,
        /// New content
        content: String,
    },
    /// Delete a file
    Delete {
        /// Path to delete
        path: PathBuf,
    },
}

impl FileChange {
    /// Gets the path affected by this file change.
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Create { path, .. } | Self::Modify { path, .. } | Self::Delete { path } => path,
        }
    }
}

/// Execution context accumulated across tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Original user request
    pub original_request: String,
    /// Files that have been read during execution
    pub files_read: HashMap<PathBuf, String>,
    /// Files that have been written during execution
    pub files_written: HashMap<PathBuf, String>,
    /// Commands that have been executed
    pub commands_run: Vec<CommandExecution>,
    /// Key findings discovered during execution
    pub findings: Vec<String>,
    /// Errors encountered during execution
    pub errors: Vec<String>,
}

impl ExecutionContext {
    /// Creates a new execution context for the given request.
    pub fn new(original_request: String) -> Self {
        Self {
            original_request,
            files_read: HashMap::default(),
            files_written: HashMap::default(),
            commands_run: Vec::default(),
            findings: Vec::default(),
            errors: Vec::default(),
        }
    }
}

/// Record of a command execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    /// Command that was run
    pub command: String,
    /// Output from the command
    pub output: String,
    /// Exit code returned by the command
    pub exit_code: i32,
    /// When the command was executed
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
}
