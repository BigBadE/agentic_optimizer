//! User interface event system for Merlin.
//! Provides channel and event types for UI updates.

use crate::task::{TaskId, TaskResult};
use tokio::sync::mpsc;
use tracing::warn;

/// Event types for UI updates
pub mod events;

// Re-exports
pub use events::{MessageLevel, TaskProgress, UiEvent};

/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    /// Sender used to deliver `UiEvent`s to the UI thread
    sender: mpsc::UnboundedSender<UiEvent>,
}

impl UiChannel {
    /// Creates a UI channel from an existing sender (for testing)
    pub fn from_sender(sender: mpsc::UnboundedSender<UiEvent>) -> Self {
        Self { sender }
    }

    /// Sends a UI event
    pub fn send(&self, event: UiEvent) {
        if let Err(error) = self.sender.send(event) {
            warn!("Failed to send UI event: {}", error);
        }
    }

    /// Sends a task started event
    pub fn task_started(&self, task_id: TaskId, description: String) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id: None,
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

    /// Sends task failed event
    pub fn failed(&self, task_id: TaskId, error: String) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}
