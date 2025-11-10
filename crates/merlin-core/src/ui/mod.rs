//! User interface event system for Merlin.
//! Provides channel and event types for UI updates.

use crate::conversation::{ThreadId, WorkUnit};
use crate::task::{TaskId, TaskResult};
use merlin_tooling::ToolError;
use std::sync::Arc;
use tokio::spawn;
use tokio::sync::{Mutex, mpsc};
use tracing::warn;

/// Event types for UI updates
pub mod events;

// Re-exports
pub use events::{MessageLevel, TaskProgress, UiEvent};

/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    /// Sender used to deliver `UiEvent`s to the UI thread (bounded for backpressure)
    sender: mpsc::Sender<UiEvent>,
}

impl UiChannel {
    /// Creates a UI channel from an existing bounded sender
    pub fn from_sender(sender: mpsc::Sender<UiEvent>) -> Self {
        Self { sender }
    }

    /// Sends a UI event (spawns a task to avoid blocking the caller)
    pub fn send(&self, event: UiEvent) {
        let sender = self.sender.clone();
        spawn(async move {
            if let Err(error) = sender.send(event).await {
                warn!("Failed to send UI event: {}", error);
            }
        });
    }

    /// Sends a task started event
    pub fn task_started(&self, task_id: TaskId, description: String) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id: None,
            thread_id: None,
        });
    }

    /// Sends a task started event with parent
    pub fn task_started_with_parent(
        &self,
        task_id: TaskId,
        description: String,
        parent_id: Option<TaskId>,
    ) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id,
            thread_id: None,
        });
    }

    /// Sends a task started event with parent and thread ID
    pub fn task_started_with_thread(
        &self,
        task_id: TaskId,
        description: String,
        parent_id: Option<TaskId>,
        thread_id: Option<ThreadId>,
    ) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id,
            thread_id,
        });
    }

    /// Sends a progress update
    pub fn progress(&self, task_id: TaskId, stage: String, message: String) {
        self.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage,
                current: 0,
                total: None,
                message,
            },
        });
    }

    /// Sends `WorkUnit` started event with live reference
    pub fn work_unit_started(&self, task_id: TaskId, work_unit: Arc<Mutex<WorkUnit>>) {
        self.send(UiEvent::WorkUnitStarted { task_id, work_unit });
    }

    /// Sends `WorkUnit` progress update
    pub fn work_unit_progress(
        &self,
        task_id: TaskId,
        progress_percentage: u8,
        completed_subtasks: usize,
        total_subtasks: usize,
    ) {
        self.send(UiEvent::WorkUnitProgress {
            task_id,
            progress_percentage,
            completed_subtasks,
            total_subtasks,
        });
    }

    /// Sends task output
    pub fn output(&self, task_id: TaskId, output: String) {
        self.send(UiEvent::TaskOutput { task_id, output });
    }

    /// Sends task completed event
    pub fn completed(&self, task_id: TaskId, result: TaskResult) {
        self.send(UiEvent::TaskCompleted {
            task_id,
            result: Box::new(result),
        });
    }

    /// Sends task failed event from a `ToolError`
    pub fn failed(&self, task_id: TaskId, error: ToolError) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}
