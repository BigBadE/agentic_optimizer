//! Streaming events and channels for real-time task execution updates.
//!
//! This module provides infrastructure for streaming task execution progress,
//! including steps, tool calls, and thinking updates.

use crate::task::TaskId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;
use uuid::Uuid;

/// Channel for streaming events
pub mod channel;

pub use channel::StreamingChannel;

/// Unique identifier for a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(Uuid);

impl Default for StepId {
    /// Creates a new unique step identifier.
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

/// Individual step in task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    /// Unique identifier for this step
    pub id: StepId,
    /// ID of the task this step belongs to
    pub task_id: TaskId,
    /// Type of step (thinking, tool call, etc.)
    pub step_type: StepType,
    /// When this step started
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
    /// Content/output of this step
    pub content: String,
}

impl TaskStep {
    /// Creates a new task step with generated ID and current timestamp.
    pub fn new(task_id: TaskId, step_type: StepType, content: String) -> Self {
        Self {
            id: StepId::default(),
            task_id,
            step_type,
            timestamp: Instant::now(),
            content,
        }
    }
}

/// Type of execution step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Agent is thinking/reasoning
    Thinking,
    /// Agent is calling a tool
    ToolCall {
        /// Name of the tool being called
        tool: String,
        /// Arguments passed to the tool
        args: Value,
    },
    /// Tool call result
    ToolResult {
        /// Name of the tool that was called
        tool: String,
        /// Result returned by the tool
        result: Value,
    },
    /// Final output
    Output,
    /// Spawned a subtask
    SubtaskSpawned {
        /// ID of the spawned child task
        child_id: TaskId,
    },
}

/// Streaming event for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingEvent {
    /// Task step started
    StepStarted {
        /// ID of the task
        task_id: TaskId,
        /// The step that started
        step: TaskStep,
    },
    /// Task step completed
    StepCompleted {
        /// ID of the task
        task_id: TaskId,
        /// The step that completed
        step: TaskStep,
    },
    /// Tool call started
    ToolCallStarted {
        /// ID of the task
        task_id: TaskId,
        /// Name of the tool
        tool: String,
        /// Arguments passed to the tool
        args: Value,
    },
    /// Tool call completed
    ToolCallCompleted {
        /// ID of the task
        task_id: TaskId,
        /// Name of the tool
        tool: String,
        /// Result from the tool
        result: Value,
    },
    /// Thinking update
    ThinkingUpdate {
        /// ID of the task
        task_id: TaskId,
        /// Thinking content
        content: String,
    },
    /// Subtask spawned
    SubtaskSpawned {
        /// ID of the parent task
        parent_id: TaskId,
        /// ID of the child task
        child_id: TaskId,
        /// Description of the subtask
        description: String,
    },
}
