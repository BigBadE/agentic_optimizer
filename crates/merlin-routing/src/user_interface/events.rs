use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::{TaskId, TaskResult};

/// UI event that tasks send to update display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiEvent {
    TaskStarted {
        task_id: TaskId,
        description: String,
        parent_id: Option<TaskId>,
    },
    TaskProgress {
        task_id: TaskId,
        progress: TaskProgress,
    },
    TaskOutput {
        task_id: TaskId,
        output: String,
    },
    TaskCompleted {
        task_id: TaskId,
        result: TaskResult,
    },
    TaskFailed {
        task_id: TaskId,
        error: String,
    },
    SystemMessage {
        level: MessageLevel,
        message: String,
    },
    // New streaming events
    TaskStepStarted {
        task_id: TaskId,
        step_id: String,
        step_type: String,
        content: String,
    },
    TaskStepCompleted {
        task_id: TaskId,
        step_id: String,
    },
    ToolCallStarted {
        task_id: TaskId,
        tool: String,
        args: Value,
    },
    ToolCallCompleted {
        task_id: TaskId,
        tool: String,
        result: Value,
    },
    ThinkingUpdate {
        task_id: TaskId,
        content: String,
    },
    SubtaskSpawned {
        parent_id: TaskId,
        child_id: TaskId,
        description: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgress {
    pub stage: String,
    pub current: u64,
    pub total: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageLevel {
    Info,
    Warning,
    Error,
    Success,
}
