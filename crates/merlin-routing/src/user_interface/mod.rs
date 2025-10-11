//! User interface (TUI) subsystem for Merlin routing.
//! Provides event handling, state management, rendering, and persistence.
use crate::{TaskId, TaskResult};
use tokio::sync::mpsc;
use tracing::warn;

// Public modules
/// Event types for UI updates
pub mod events;
mod text_width;

// Publicly exposed for testing
/// Input event source abstraction (public so tests can inject events)
pub mod event_source;
/// Input management
pub mod input;
/// Output tree structure for hierarchical display
pub mod output_tree;
/// Rendering components
pub mod renderer;
/// UI state management
pub mod state;
/// Task management
pub mod task_manager;
/// Theme definitions
pub mod theme;

// Private modules
/// TUI application and main event loop
mod app;
mod event_handler;
/// Task persistence (public for testing)
pub mod persistence;

// Re-exports
pub use app::TuiApp;
pub use events::{MessageLevel, TaskProgress, UiEvent};
pub use text_width::{EmojiMode, calculate_width, strip_emojis, truncate_to_width, wrap_text};

/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    /// Sender used to deliver `UiEvent`s to the TUI thread
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
        self.send(UiEvent::TaskCompleted { task_id, result });
    }

    /// Sends task failed event
    pub fn failed(&self, task_id: TaskId, error: String) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}
