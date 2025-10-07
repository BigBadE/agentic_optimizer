use crate::TaskId;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Instant;
use uuid::Uuid;

pub mod channel;

pub use channel::StreamingChannel;

/// Unique identifier for a step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StepId(Uuid);

impl StepId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StepId {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual step in task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub id: StepId,
    pub task_id: TaskId,
    pub step_type: StepType,
    #[serde(skip, default = "Instant::now")]
    pub timestamp: Instant,
    pub content: String,
}

impl TaskStep {
    #[must_use]
    pub fn new(task_id: TaskId, step_type: StepType, content: String) -> Self {
        Self {
            id: StepId::new(),
            task_id,
            step_type,
            timestamp: Instant::now(),
            content,
        }
    }
}

/// Type of execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Agent is thinking/reasoning
    Thinking,
    /// Agent is calling a tool
    ToolCall { tool: String, args: Value },
    /// Tool call result
    ToolResult { tool: String, result: Value },
    /// Final output
    Output,
    /// Spawned a subtask
    SubtaskSpawned { child_id: TaskId },
}

/// Streaming event for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingEvent {
    /// Task step started
    StepStarted { task_id: TaskId, step: TaskStep },
    /// Task step completed
    StepCompleted { task_id: TaskId, step: TaskStep },
    /// Tool call started
    ToolCallStarted {
        task_id: TaskId,
        tool: String,
        args: Value,
    },
    /// Tool call completed
    ToolCallCompleted {
        task_id: TaskId,
        tool: String,
        result: Value,
    },
    /// Thinking update
    ThinkingUpdate { task_id: TaskId, content: String },
    /// Subtask spawned
    SubtaskSpawned {
        parent_id: TaskId,
        child_id: TaskId,
        description: String,
    },
}
