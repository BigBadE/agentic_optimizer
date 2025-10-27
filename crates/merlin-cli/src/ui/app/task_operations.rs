//! Task operations, deletion, and scroll management

use merlin_deps::ratatui::backend::Backend;
use merlin_deps::tracing::warn;

use super::conversation;
use super::tui_app::TuiApp;
use crate::ui::renderer::Renderer;
use crate::ui::state::ConversationRole;
use merlin_routing::TaskId;

impl<B: Backend> TuiApp<B> {
    /// Takes the task ID to continue conversation from (clears it after taking)
    pub fn take_continuing_conversation_from(&mut self) -> Option<TaskId> {
        self.state.continuing_conversation_from.take()
    }

    /// Gets conversation history in (role, content) format for context building
    pub fn get_conversation_history(&self) -> Vec<(String, String)> {
        // If continuing a conversation, load history from that task
        if let Some(task_id) = self.state.continuing_conversation_from {
            merlin_deps::tracing::info!(
                "TuiApp::get_conversation_history() - continuing from task {:?}",
                task_id
            );
            return conversation::get_conversation_history_from_task(task_id, &self.task_manager);
        }

        // Otherwise, use current conversation history, filtering out system messages
        let history: Vec<(String, String)> = self
            .state
            .conversation_history
            .iter()
            .filter(|entry| entry.role != ConversationRole::System)
            .map(|entry| {
                let role = match entry.role {
                    ConversationRole::User => "user",
                    ConversationRole::Assistant => "assistant",
                    ConversationRole::System => "system",
                };
                (role.to_owned(), entry.text.clone())
            })
            .collect();

        merlin_deps::tracing::info!(
            "TuiApp::get_conversation_history() returning {} messages",
            history.len()
        );
        history
    }

    /// Gets the selected task ID
    pub fn get_selected_task_id(&self) -> Option<TaskId> {
        self.state.active_task_id
    }

    /// Returns the number of loaded tasks currently in the manager
    pub fn loaded_task_count(&self) -> usize {
        self.task_manager.task_order().len()
    }

    /// Deletes a task and updates UI state accordingly
    pub(super) fn delete_task(&mut self, task_id: TaskId) {
        let was_active = self.state.active_task_id == Some(task_id);

        let to_delete = self.task_manager.remove_task(task_id);

        if let Some(persistence) = &self.persistence {
            for id in &to_delete {
                if let Err(error) = persistence.delete_task_file(*id) {
                    warn!("Failed to delete task file for {:?}: {}", id, error);
                }
            }
        }

        for id in &to_delete {
            self.state.active_running_tasks.remove(id);
        }

        if !was_active {
            return;
        }

        // After deleting active task, select the next newest task
        // task_order is already sorted (oldest first, newest last)
        // All tasks are now flat (no parent-child hierarchy)
        let all_tasks: Vec<TaskId> = self.task_manager.task_order().to_vec();

        // Select the newest task (last in chronological order) if any exist
        if let Some(&new_id) = all_tasks.last() {
            self.state.active_task_id = Some(new_id);
        } else {
            self.state.active_task_id = None;
        }
    }

    /// Calculates the maximum scroll offset for the output pane
    pub(super) fn calculate_output_max_scroll(&self) -> u16 {
        let Some(task_id) = self.state.active_task_id else {
            return 0;
        };

        let Some(task) = self.task_manager.get_task(task_id) else {
            return 0;
        };

        // Use cached viewport height from actual rendering
        let viewport_height = self.layout_cache.output_viewport_height();
        if viewport_height == 0 {
            // Layout not yet cached (first render), return 0
            return 0;
        }

        let terminal_width = self.terminal.size().map(|size| size.width).unwrap_or(80);
        let text_lines = Renderer::calculate_output_line_count(task, terminal_width);
        text_lines.saturating_sub(viewport_height)
    }

    /// Auto-scrolls the output pane to the bottom if already at or near bottom
    pub(super) fn auto_scroll_output_to_bottom(&mut self) {
        if self.state.active_task_id.is_some() {
            let max_scroll = self.calculate_output_max_scroll();
            self.state.output_scroll_offset = max_scroll;
        }
    }
}
