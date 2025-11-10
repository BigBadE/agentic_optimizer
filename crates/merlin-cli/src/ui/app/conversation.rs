//! Conversation history management for TUI
//!
//! Handles conversation history extraction from tasks in a thread,
//! formatting for context building, and thread context management.

use merlin_routing::TaskId;

use crate::ui::task_manager::TaskManager;

/// Gets conversation history from a specific task
///
/// In the new thread-based system, tasks are flat and belong to threads.
/// This function now just returns the single task's conversation.
pub fn get_conversation_history_from_task(
    task_id: TaskId,
    task_manager: &TaskManager,
) -> Vec<(String, String)> {
    let mut history = Vec::new();

    // Get the single task
    if let Some(task) = task_manager.get_task(task_id) {
        // Add user message (task description)
        if !task.description.is_empty()
            && !task.description.starts_with("Saving task")
            && !task.description.starts_with("Loading task")
        {
            history.push(("user".to_string(), task.description.clone()));
        }

        // Add assistant response from output
        if !task.output.is_empty()
            && !task.output.contains("Saving task")
            && !task.output.contains("Loading task")
        {
            history.push(("assistant".to_string(), task.output.clone()));
        }
    }

    tracing::info!(
        "get_conversation_history_from_task() returning {} messages from task",
        history.len()
    );
    history
}

/// Returns the task ID itself (no hierarchy in new system)
///
/// Previously this would find the root conversation in a parent-child hierarchy.
/// Now tasks are flat and there is no hierarchy, so this just returns the `task_id`.
pub fn find_root_conversation(task_id: TaskId, _task_manager: &TaskManager) -> TaskId {
    // In the new flat system, each task is its own "root"
    // This function is kept for API compatibility but is now a no-op
    task_id
}
