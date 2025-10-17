//! Keyboard input handling and dispatch

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::Backend;

use super::conversation;
use super::input_handler;
use super::navigation;
use super::tui_app::TuiApp;
use crate::ui::renderer::FocusedPane;

impl<B: Backend> TuiApp<B> {
    /// Handles a single key event and returns true if the app should quit
    pub(super) fn handle_key_event(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q' | 'c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_theme();
                false
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                input_handler::toggle_task_focus(&mut self.focused_pane);
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
                // Toggle expand/collapse for the selected conversation
                if let Some(selected_id) = self.state.active_task_id {
                    let root_id =
                        conversation::find_root_conversation(selected_id, &self.task_manager);
                    if self.state.expanded_conversations.contains(&root_id) {
                        self.state.expanded_conversations.remove(&root_id);
                    } else {
                        self.state.expanded_conversations.insert(root_id);
                    }
                }
                false
            }
            FocusedPane::Output => false,
        }
    }

    /// Handles any other key events depending on the focused pane
    pub(super) fn handle_other_key(&mut self, key: &KeyEvent) {
        if self.focused_pane == FocusedPane::Tasks && key.code != KeyCode::Backspace {
            self.state.pending_delete_task_id = None;
        }

        match self.focused_pane {
            FocusedPane::Input => {
                let terminal_width = self.terminal.size().map(|size| size.width).unwrap_or(80);
                input_handler::handle_input_key(key, &mut self.input_manager, terminal_width);
            }
            FocusedPane::Output => {
                let max_scroll = self.calculate_output_max_scroll();
                input_handler::handle_output_key(
                    key,
                    self.state.active_task_id,
                    &mut self.state.output_scroll_offset,
                    max_scroll,
                );
            }
            FocusedPane::Tasks => match key.code {
                KeyCode::Up => {
                    let terminal_height =
                        self.terminal.size().map(|size| size.height).unwrap_or(30);
                    navigation::navigate_tasks_up(
                        &self.task_manager,
                        navigation::NavigationContext {
                            active_task_id: &mut self.state.active_task_id,
                            expanded_conversations: &self.state.expanded_conversations,
                            task_list_scroll_offset: &mut self.state.task_list_scroll_offset,
                        },
                        terminal_height,
                        self.focused_pane == FocusedPane::Tasks,
                    );
                }
                KeyCode::Down => {
                    let terminal_height =
                        self.terminal.size().map(|size| size.height).unwrap_or(30);
                    navigation::navigate_tasks_down(
                        &self.task_manager,
                        navigation::NavigationContext {
                            active_task_id: &mut self.state.active_task_id,
                            expanded_conversations: &self.state.expanded_conversations,
                            task_list_scroll_offset: &mut self.state.task_list_scroll_offset,
                        },
                        terminal_height,
                        self.focused_pane == FocusedPane::Tasks,
                    );
                }
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
            },
        }
    }
}
