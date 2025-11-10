//! Task operations, deletion, and scroll management

use ratatui::backend::Backend;
use tracing::warn;

use super::conversation;
use super::tui_app::TuiApp;
use crate::ui::renderer::Renderer;
use crate::ui::state::ConversationRole;
use merlin_routing::TaskId;

impl<B: Backend> TuiApp<B> {
    /// Gets conversation history in (role, content) format for context building
    pub fn get_conversation_history(&self) -> Vec<(String, String)> {
        // If continuing a conversation, load history from that task
        if let Some(task_id) = self.ui_components.state.continuing_conversation_from {
            tracing::info!(
                "TuiApp::get_conversation_history() - continuing from task {:?}",
                task_id
            );
            return conversation::get_conversation_history_from_task(
                task_id,
                &self.ui_components.task_manager,
            );
        }

        // Otherwise, use current conversation history, filtering out system messages
        let history: Vec<(String, String)> = self
            .ui_components
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

        tracing::info!(
            "TuiApp::get_conversation_history() returning {} messages",
            history.len()
        );
        history
    }

    /// Returns the number of loaded tasks currently in the manager
    pub fn loaded_task_count(&self) -> usize {
        self.ui_components.task_manager.task_order().len()
    }

    /// Deletes a task and updates UI state accordingly
    pub(super) fn delete_task(&mut self, task_id: TaskId) {
        let was_active = self.ui_components.state.active_task_id == Some(task_id);

        let to_delete = self.ui_components.task_manager.remove_task(task_id);

        if let Some(persistence) = &self.runtime_state.persistence {
            for id in &to_delete {
                if let Err(error) = persistence.delete_task_file(*id) {
                    warn!("Failed to delete task file for {:?}: {}", id, error);
                }
            }
        }

        for id in &to_delete {
            self.ui_components.state.active_running_tasks.remove(id);
        }

        if !was_active {
            return;
        }

        // After deleting active task, select the next newest task
        // task_order is already sorted (oldest first, newest last)
        // All tasks are now flat (no parent-child hierarchy)
        let all_tasks: Vec<TaskId> = self.ui_components.task_manager.task_order().to_vec();

        // Select the newest task (last in chronological order) if any exist
        if let Some(&new_id) = all_tasks.last() {
            self.ui_components.state.active_task_id = Some(new_id);
        } else {
            self.ui_components.state.active_task_id = None;
        }
    }

    /// Calculates the maximum scroll offset for the output pane
    pub(super) fn calculate_output_max_scroll(&self) -> u16 {
        let Some(task_id) = self.ui_components.state.active_task_id else {
            return 0;
        };

        let Some(task) = self.ui_components.task_manager.get_task(task_id) else {
            return 0;
        };

        // Use cached viewport height from actual rendering
        let viewport_height = self.ui_components.layout_cache.output_viewport_height();
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
        if self.ui_components.state.active_task_id.is_some() {
            let max_scroll = self.calculate_output_max_scroll();
            self.ui_components.state.output_scroll_offset = max_scroll;
        }
    }
}
