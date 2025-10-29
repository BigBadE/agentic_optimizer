use crate::task::{TaskId, TaskResult};
use merlin_deps::serde_json::Value;
use merlin_tooling::ToolError;
use serde::{Deserialize, Serialize};

/// UI event that tasks send to update display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiEvent {
    /// Task has started execution
    TaskStarted {
        /// ID of the task
        task_id: TaskId,
        /// Description of what the task will do
        description: String,
        /// Optional parent task ID if this is a subtask
        parent_id: Option<TaskId>,
    },
    /// Task progress update
    TaskProgress {
        /// ID of the task
        task_id: TaskId,
        /// Progress information
        progress: TaskProgress,
    },
    /// Task produced output
    TaskOutput {
        /// ID of the task
        task_id: TaskId,
        /// Output text
        output: String,
    },
    /// Task completed successfully
    TaskCompleted {
        /// ID of the task
        task_id: TaskId,
        /// Final result
        result: Box<TaskResult>,
    },
    /// Task failed with an error
    TaskFailed {
        /// ID of the task
        task_id: TaskId,
        /// Error object with full type information
        error: ToolError,
    },
    /// Task is retrying after a failure
    TaskRetrying {
        /// ID of the task
        task_id: TaskId,
        /// Current retry count (1 = first retry, 2 = second retry, etc.)
        retry_count: u32,
        /// Error from the failed attempt
        error: ToolError,
    },
    /// System-level message
    SystemMessage {
        /// Message severity level
        level: MessageLevel,
        /// Message text
        message: String,
    },
    /// Task step has started (streaming event)
    TaskStepStarted {
        /// ID of the task
        task_id: TaskId,
        /// ID of the step
        step_id: String,
        /// Type of step (e.g., `thinking`, `tool_call`)
        step_type: String,
        /// Step content
        content: String,
    },
    /// Task step has completed (streaming event)
    TaskStepCompleted {
        /// ID of the task
        task_id: TaskId,
        /// ID of the step
        step_id: String,
    },
    /// Task step has failed (streaming event)
    TaskStepFailed {
        /// ID of the task
        task_id: TaskId,
        /// ID of the step
        step_id: String,
        /// Error message
        error: String,
    },
    /// Tool call started
    ToolCallStarted {
        /// ID of the task making the tool call
        task_id: TaskId,
        /// Name of the tool being called
        tool: String,
        /// Arguments passed to the tool
        args: Value,
    },
    /// Tool call completed
    ToolCallCompleted {
        /// ID of the task that made the tool call
        task_id: TaskId,
        /// Name of the tool that was called
        tool: String,
        /// Result returned by the tool
        result: Value,
    },
    /// Agent thinking/reasoning update
    ThinkingUpdate {
        /// ID of the task
        task_id: TaskId,
        /// Thinking content
        content: String,
    },
    /// Subtask was spawned
    SubtaskSpawned {
        /// ID of the parent task
        parent_id: TaskId,
        /// ID of the newly spawned child task
        child_id: TaskId,
        /// Description of the subtask
        description: String,
    },
    /// Embedding indexing progress update
    EmbeddingProgress {
        /// Current progress
        current: u64,
        /// Total items
        total: u64,
        /// Stage description
        stage: String,
    },
}

/// Progress information for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    /// Current stage name (e.g., "analyzing", "executing", "validating")
    pub stage: String,
    /// Current progress value
    pub current: u64,
    /// Total expected value (if known)
    pub total: Option<u64>,
    /// Human-readable progress message
    pub message: String,
}

/// Message severity level for system messages.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageLevel {
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
    /// Success message
    Success,
}
