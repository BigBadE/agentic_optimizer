//! Thread management operations for TUI

use super::tui_app::TuiApp;
use ratatui::backend::Backend;

impl<B: Backend> TuiApp<B> {
    /// Navigates up in the thread list
    pub(super) fn navigate_threads_up(&mut self) {
        let active_threads = self.thread_store.active_threads();
        if active_threads.is_empty() {
            return;
        }

        if let Some(current_id) = self.state.active_thread_id {
            // Find current thread index
            if let Some(current_index) = active_threads
                .iter()
                .position(|thread| thread.id == current_id)
                && current_index > 0
            {
                self.state.active_thread_id = Some(active_threads[current_index - 1].id);
            }
        } else {
            // No selection, select last thread
            self.state.active_thread_id = active_threads.last().map(|thread| thread.id);
        }
    }

    /// Navigates down in the thread list
    pub(super) fn navigate_threads_down(&mut self) {
        let active_threads = self.thread_store.active_threads();
        if active_threads.is_empty() {
            return;
        }

        if let Some(current_id) = self.state.active_thread_id {
            // Find current thread index
            if let Some(current_index) = active_threads
                .iter()
                .position(|thread| thread.id == current_id)
                && current_index < active_threads.len() - 1
            {
                self.state.active_thread_id = Some(active_threads[current_index + 1].id);
            }
        } else {
            // No selection, select first thread
            self.state.active_thread_id = active_threads.first().map(|thread| thread.id);
        }
    }

    /// Creates a new thread
    pub(super) fn create_new_thread(&mut self) {
        // Get user input for thread name
        // For now, use a default name based on count
        let count = self.thread_store.total_count() + 1;
        let thread = self.thread_store.create_thread(format!("Thread {count}"));
        let thread_id = thread.id;

        // Save the thread
        if let Err(err) = self.thread_store.save_thread(&thread) {
            tracing::error!("Failed to save new thread: {err}");
            return;
        }

        // Select the new thread
        self.state.active_thread_id = Some(thread_id);
        tracing::info!("Created new thread {thread_id}");
    }

    /// Branches from the current message
    pub(super) fn branch_from_current(&mut self) {
        // Get the current thread and message
        let Some(thread_id) = self.state.active_thread_id else {
            tracing::warn!("No thread selected for branching");
            return;
        };

        let Some(thread) = self.thread_store.get_thread(thread_id) else {
            tracing::warn!("Selected thread {thread_id} not found");
            return;
        };

        // Get the last message from the thread
        let Some(last_message) = thread.messages.last() else {
            tracing::warn!("Thread {thread_id} has no messages to branch from");
            return;
        };

        let message_id = last_message.id;

        // Create branch
        let count = self.thread_store.total_count() + 1;
        match self
            .thread_store
            .create_branch(format!("Branch {count}"), thread_id, message_id)
        {
            Ok(branch) => {
                let branch_id = branch.id;

                // Save the branch
                if let Err(err) = self.thread_store.save_thread(&branch) {
                    tracing::error!("Failed to save branch: {err}");
                    return;
                }

                // Select the new branch
                self.state.active_thread_id = Some(branch_id);
                tracing::info!("Created branch {branch_id} from thread {thread_id}");
            }
            Err(err) => {
                tracing::error!("Failed to create branch: {err}");
            }
        }
    }

    /// Archives the currently selected thread
    pub(super) fn archive_selected_thread(&mut self) {
        let Some(thread_id) = self.state.active_thread_id else {
            return;
        };

        if let Err(err) = self.thread_store.archive_thread(thread_id) {
            tracing::error!("Failed to archive thread {thread_id}: {err}");
            return;
        }

        // Clear selection and move to next thread
        self.state.active_thread_id = None;
        self.navigate_threads_down();

        tracing::info!("Archived thread {thread_id}");
    }
}
