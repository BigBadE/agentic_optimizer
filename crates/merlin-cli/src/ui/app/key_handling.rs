//! Keyboard input handling and dispatch

use super::input_handler;
use super::tui_app::TuiApp;
use crate::ui::app::navigation::{NavigationContext, navigate_tasks_down, navigate_tasks_up};
use crate::ui::renderer::FocusedPane;
use merlin_deps::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use merlin_deps::ratatui::backend::Backend;
use std::collections::HashSet;
use std::hash::Hash;

/// Toggle a value in a `HashSet` (remove if present, insert if absent)
fn toggle_set<T: Eq + Hash>(set: &mut HashSet<T>, value: T) {
    if !set.remove(&value) {
        set.insert(value);
    }
}

impl<B: Backend> TuiApp<B> {
    /// Handles a single key event and returns true if the app should quit
    pub(super) fn handle_key_event(&mut self, key: &KeyEvent) -> bool {
        // Handle cancel/queue prompt keys if queued input exists
        if self.state.queued_input.is_some() {
            return match key.code {
                KeyCode::Char('c') => {
                    // Cancel current work and submit queued input
                    self.state.cancel_requested = true;
                    if let Some(queued) = self.state.queued_input.take() {
                        self.state.processing_status = Some("[Cancelling work...]".to_string());
                        self.pending_input = Some(queued);
                    }
                    false
                }
                KeyCode::Char('a') => {
                    // Accept queue - just keep the queued input
                    self.state.processing_status =
                        Some("[Input queued, will run after current work]".to_string());
                    false
                }
                KeyCode::Esc => {
                    // Discard queued input
                    self.state.queued_input = None;
                    self.state.processing_status = None;
                    false
                }
                _ => {
                    // Ignore other keys when prompt is showing
                    false
                }
            };
        }

        match key.code {
            KeyCode::Char('q' | 'c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_theme();
                false
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    // Ctrl+Shift+T: Toggle thread pane focus
                    input_handler::toggle_thread_focus(&mut self.focused_pane);
                } else {
                    // Ctrl+T: Toggle task focus
                    input_handler::toggle_task_focus(&mut self.focused_pane);
                }
                false
            }
            KeyCode::Tab => {
                input_handler::handle_tab(&mut self.focused_pane, self.state.active_task_id);
                false
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                input_handler::handle_ctrl_n(self.focused_pane, &mut self.input_manager);
                false
            }
            KeyCode::Enter => self.handle_enter_key(key.modifiers.contains(KeyModifiers::SHIFT)),
            _ => {
                self.handle_other_key(key);
                false
            }
        }
    }

    /// Handles the Enter key press
    pub(super) fn handle_enter_key(&mut self, shift_pressed: bool) -> bool {
        match self.focused_pane {
            FocusedPane::Input => {
                if shift_pressed {
                    self.input_manager.insert_newline_at_cursor();
                    self.input_manager.record_manual_newline();
                    false
                } else {
                    self.submit_input()
                }
            }
            FocusedPane::Tasks => {
                self.handle_tasks_enter_key();
                false
            }
            FocusedPane::Output | FocusedPane::Threads => false,
        }
    }

    /// Handles Enter key when Tasks pane is focused
    fn handle_tasks_enter_key(&mut self) {
        let Some(selected_id) = self.state.active_task_id else {
            return;
        };

        // Check if selected task has steps
        let has_steps = self
            .task_manager
            .get_task(selected_id)
            .is_some_and(|task| !task.steps.is_empty());

        if has_steps {
            // Toggle step expansion for tasks with steps
            toggle_set(&mut self.state.expanded_steps, selected_id);
        } else {
            // Toggle conversation expansion for tasks without steps
            toggle_set(&mut self.state.expanded_conversations, selected_id);
        }
    }

    /// Handles any other key events depending on the focused pane
    pub(super) fn handle_other_key(&mut self, key: &KeyEvent) {
        if self.focused_pane == FocusedPane::Tasks && key.code != KeyCode::Backspace {
            self.state.pending_delete_task_id = None;
        }

        match self.focused_pane {
            FocusedPane::Input => self.handle_input_pane_key(key),
            FocusedPane::Output => self.handle_output_pane_key(key),
            FocusedPane::Tasks => self.handle_tasks_pane_key(key),
            FocusedPane::Threads => self.handle_threads_pane_key(key),
        }
    }

    fn handle_input_pane_key(&mut self, key: &KeyEvent) {
        let terminal_width = self.terminal.size().map(|size| size.width).unwrap_or(80);
        input_handler::handle_input_key(key, &mut self.input_manager, terminal_width);
    }

    fn handle_output_pane_key(&mut self, key: &KeyEvent) {
        let max_scroll = self.calculate_output_max_scroll();
        input_handler::handle_output_key(
            key,
            self.state.active_task_id,
            &mut self.state.output_scroll_offset,
            max_scroll,
        );
    }

    fn handle_tasks_pane_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up => self.navigate_tasks_up_handler(),
            KeyCode::Down => self.navigate_tasks_down_handler(),
            KeyCode::Backspace => {
                if let Some(task_id_to_delete) = input_handler::handle_backspace_in_tasks(
                    self.state.active_task_id,
                    &mut self.state.pending_delete_task_id,
                ) {
                    self.delete_task(task_id_to_delete);
                }
            }
            _ => {
                input_handler::handle_task_key(
                    key,
                    &mut self.state.active_task_id,
                    &mut self.state.pending_delete_task_id,
                    &mut self.state.expanded_conversations,
                    &self.task_manager,
                );
            }
        }
    }

    fn handle_threads_pane_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.navigate_threads_up(),
            KeyCode::Down | KeyCode::Char('j') => self.navigate_threads_down(),
            KeyCode::Char('n') => self.create_new_thread(),
            KeyCode::Char('b') => self.branch_from_current(),
            KeyCode::Delete | KeyCode::Char('d') => self.archive_selected_thread(),
            _ => {}
        }
    }

    fn navigate_tasks_up_handler(&mut self) {
        let terminal_height = self.terminal.size().map(|size| size.height).unwrap_or(30);
        navigate_tasks_up(
            &self.task_manager,
            &mut NavigationContext {
                active_task_id: &mut self.state.active_task_id,
                expanded_conversations: &self.state.expanded_conversations,
                task_list_scroll_offset: &mut self.state.task_list_scroll_offset,
                task_output_scroll: &mut self.state.task_output_scroll,
                output_scroll_offset: &mut self.state.output_scroll_offset,
            },
            terminal_height,
            self.focused_pane == FocusedPane::Tasks,
        );
    }

    fn navigate_tasks_down_handler(&mut self) {
        let terminal_height = self.terminal.size().map(|size| size.height).unwrap_or(30);
        navigate_tasks_down(
            &self.task_manager,
            &mut NavigationContext {
                active_task_id: &mut self.state.active_task_id,
                expanded_conversations: &self.state.expanded_conversations,
                task_list_scroll_offset: &mut self.state.task_list_scroll_offset,
                task_output_scroll: &mut self.state.task_output_scroll,
                output_scroll_offset: &mut self.state.output_scroll_offset,
            },
            terminal_height,
            self.focused_pane == FocusedPane::Tasks,
        );
    }
}
