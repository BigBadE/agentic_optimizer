use serde::{Deserialize, Serialize};
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
