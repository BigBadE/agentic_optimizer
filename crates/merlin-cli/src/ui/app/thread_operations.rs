//! Thread management operations for TUI

use super::tui_app::TuiApp;
use merlin_deps::ratatui::backend::Backend;

impl<B: Backend> TuiApp<B> {
    /// Navigates up in the thread list
    pub(super) fn navigate_threads_up(&mut self) {
        let Ok(store) = self.runtime_state.thread_store.lock() else {
            return;
        };
        let active_threads = store.active_threads();
        if active_threads.is_empty() {
            return;
        }

        if let Some(current_id) = self.ui_components.state.active_thread_id {
            // Find current thread index
            if let Some(current_index) = active_threads
                .iter()
                .position(|thread| thread.id == current_id)
                && current_index > 0
            {
                self.ui_components.state.active_thread_id =
                    Some(active_threads[current_index - 1].id);
            }
        } else {
            // No selection, select last thread
            self.ui_components.state.active_thread_id =
                active_threads.last().map(|thread| thread.id);
        }
    }

    /// Navigates down in the thread list
    pub(super) fn navigate_threads_down(&mut self) {
        let Ok(store) = self.runtime_state.thread_store.lock() else {
            return;
        };
        let active_threads = store.active_threads();
        if active_threads.is_empty() {
            return;
        }

        if let Some(current_id) = self.ui_components.state.active_thread_id {
            // Find current thread index
            if let Some(current_index) = active_threads
                .iter()
                .position(|thread| thread.id == current_id)
                && current_index < active_threads.len() - 1
            {
                self.ui_components.state.active_thread_id =
                    Some(active_threads[current_index + 1].id);
            }
        } else {
            // No selection, select first thread
            self.ui_components.state.active_thread_id =
                active_threads.first().map(|thread| thread.id);
        }
    }

    /// Creates a new thread
    pub(super) fn create_new_thread(&mut self) {
        let Ok(mut store) = self.runtime_state.thread_store.lock() else {
            return;
        };

        // Get user input for thread name
        // For now, use a default name based on count
        let count = store.total_count() + 1;
        let thread = store.create_thread(format!("Thread {count}"));
        let thread_id = thread.id;

        // Save the thread
        if let Err(err) = store.save_thread(&thread) {
            merlin_deps::tracing::error!("Failed to save new thread: {err}");
            return;
        }

        // Select the new thread
        self.ui_components.state.active_thread_id = Some(thread_id);
        merlin_deps::tracing::info!("Created new thread {thread_id}");
    }

    /// Branches from the current message
    pub(super) fn branch_from_current(&mut self) {
        // Get the current thread and message
        let Some(thread_id) = self.ui_components.state.active_thread_id else {
            merlin_deps::tracing::warn!("No thread selected for branching");
            return;
        };

        let Ok(mut store) = self.runtime_state.thread_store.lock() else {
            return;
        };

        let Some(thread) = store.get_thread(thread_id) else {
            merlin_deps::tracing::warn!("Selected thread {thread_id} not found");
            return;
        };

        // Get the last message from the thread
        let Some(last_message) = thread.messages.last() else {
            merlin_deps::tracing::warn!("Thread {thread_id} has no messages to branch from");
            return;
        };

        let message_id = last_message.id;

        // Create branch
        let count = store.total_count() + 1;
        match store.create_branch(format!("Branch {count}"), thread_id, message_id) {
            Ok(branch) => {
                let branch_id = branch.id;

                // Save the branch
                if let Err(err) = store.save_thread(&branch) {
                    merlin_deps::tracing::error!("Failed to save branch: {err}");
                    return;
                }

                // Select the new branch
                self.ui_components.state.active_thread_id = Some(branch_id);
                merlin_deps::tracing::info!("Created branch {branch_id} from thread {thread_id}");
            }
            Err(err) => {
                merlin_deps::tracing::error!("Failed to create branch: {err}");
            }
        }
    }

    /// Archives the currently selected thread
    pub(super) fn archive_selected_thread(&mut self) {
        let Some(thread_id) = self.ui_components.state.active_thread_id else {
            return;
        };

        let Ok(mut store) = self.runtime_state.thread_store.lock() else {
            return;
        };

        if let Err(err) = store.archive_thread(thread_id) {
            merlin_deps::tracing::error!("Failed to archive thread {thread_id}: {err}");
            return;
        }

        // Clear selection and move to next thread
        self.ui_components.state.active_thread_id = None;
        drop(store); // Release lock before navigating
        self.navigate_threads_down();

        merlin_deps::tracing::info!("Archived thread {thread_id}");
    }
}
