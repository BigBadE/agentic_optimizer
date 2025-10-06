//! User interface (TUI) subsystem for Merlin routing.
//! Provides event handling, state management, rendering, and persistence.
use tokio::sync::mpsc;
use crate::{TaskId, TaskResult};

// Public modules
pub mod events;
mod text_width;
mod output_tree;

// New refactored modules
mod task_manager;
mod state;
mod persistence;
mod input;
mod event_handler;
mod renderer;
/// Theme configuration and persistence helpers
mod theme;
/// TUI application and main event loop
mod app;

// Re-exports
pub use events::{MessageLevel, TaskProgress, UiEvent};
pub use text_width::{EmojiMode, calculate_width, strip_emojis, truncate_to_width, wrap_text};
pub use app::TuiApp;

/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    /// Sender used to deliver `UiEvent`s to the TUI thread
    sender: mpsc::UnboundedSender<UiEvent>,
}

impl UiChannel {
    /// Sends a UI event
    pub fn send(&self, event: UiEvent) {
        drop(self.sender.send(event));
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
        self.send(UiEvent::TaskCompleted { task_id, result });
    }

    /// Sends task failed event
    pub fn failed(&self, task_id: TaskId, error: String) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}
