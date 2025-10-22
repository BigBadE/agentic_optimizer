//! Conversation history management for TUI
//!
//! Handles conversation history extraction from tasks and their ancestors,
//! formatting for context building, and thread context management.

use merlin_routing::TaskId;
use std::time::Instant;

use crate::ui::task_manager::TaskManager;

/// Gets conversation history from a specific task and its ancestors
pub fn get_conversation_history_from_task(
    task_id: TaskId,
    task_manager: &TaskManager,
) -> Vec<(String, String)> {
    let mut history = Vec::new();

    // Find the root task
    let mut current_id = task_id;
    let root_id = loop {
        if let Some(task) = task_manager.get_task(current_id) {
            if let Some(parent_id) = task.parent_id {
                current_id = parent_id;
            } else {
                break current_id;
            }
        } else {
            break task_id;
        }
    };

    // Collect all tasks in the conversation chain (root and its children)
    let mut conversation_tasks = vec![root_id];
    for (id, task) in task_manager.iter_tasks() {
        if task.parent_id == Some(root_id) {
            conversation_tasks.push(id);
        }
    }

    // Sort by timestamp to maintain chronological order
    conversation_tasks.sort_by(|task_a, task_b| {
        let time_a = task_manager
            .get_task(*task_a)
            .map_or_else(Instant::now, |task| task.timestamp);
        let time_b = task_manager
            .get_task(*task_b)
            .map_or_else(Instant::now, |task| task.timestamp);
        time_a.cmp(&time_b)
    });

    // Extract conversation from each task's description and output
    for id in conversation_tasks {
        if let Some(task) = task_manager.get_task(id) {
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
    }

    tracing::info!(
        "get_conversation_history_from_task() returning {} messages from task chain",
        history.len()
    );
    history
}

/// Gets thread context for a specific task (parent and siblings)
///
/// Returns tuples of `(task_id, description, output)` for the parent task and all siblings.
///
/// TODO: This is for conversation threading features - allows displaying related tasks
/// in a conversation view.
#[cfg(any(test, feature = "test-util"))]
#[allow(
    dead_code,
    reason = "Test utility for future conversation threading feature"
)]
pub fn get_thread_context(
    selected_task_id: Option<TaskId>,
    task_manager: &TaskManager,
) -> Vec<(TaskId, String, String)> {
    let parent_id = selected_task_id
        .and_then(|task_id| task_manager.get_task(task_id)?.parent_id)
        .or(selected_task_id);

    let Some(parent_id) = parent_id else {
        return Vec::default();
    };

    let mut context = Vec::default();

    if let Some(parent_task) = task_manager.get_task(parent_id) {
        context.push((
            parent_id,
            parent_task.description.clone(),
            parent_task.output.clone(),
        ));
    }

    for &task_id in task_manager.task_order() {
        if task_id == parent_id {
            continue;
        }

        if let Some(task) = task_manager.get_task(task_id)
            && task.parent_id == Some(parent_id)
        {
            context.push((task_id, task.description.clone(), task.output.clone()));
        }
    }

    context
}

/// Finds the root conversation for a given task ID
pub fn find_root_conversation(task_id: TaskId, task_manager: &TaskManager) -> TaskId {
    let mut current_id = task_id;
    while let Some(task) = task_manager.get_task(current_id) {
        if let Some(parent_id) = task.parent_id {
            current_id = parent_id;
        } else {
            return current_id;
        }
    }
    task_id
}
