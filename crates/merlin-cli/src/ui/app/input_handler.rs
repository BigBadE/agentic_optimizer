//! Input event handling for TUI
//!
//! Handles keyboard input events, pane navigation, and key combinations.

use crossterm::event::{Event, KeyCode, KeyEvent};
use merlin_routing::TaskId;
use std::collections::HashSet;

use crate::ui::input::InputManager;
use crate::ui::renderer::FocusedPane;
use crate::ui::task_manager::TaskManager;

/// Handles key events when the input pane is focused
pub fn handle_input_key(key: &KeyEvent, input_manager: &mut InputManager, terminal_width: u16) {
    let input_width = (f32::from(terminal_width) * 0.7) as usize;
    let max_line_width = input_width.saturating_sub(4);

    input_manager.handle_input(&Event::Key(*key), Some(max_line_width));
}

/// Handles key events when the output pane is focused
pub fn handle_output_key(
    key: &KeyEvent,
    active_task_id: Option<TaskId>,
    output_scroll_offset: &mut u16,
    max_scroll: u16,
) {
    if active_task_id.is_none() {
        return;
    }

    match key.code {
        // Arrow keys and vim-style navigation scroll the text output
        KeyCode::Up | KeyCode::Char('k') => {
            *output_scroll_offset = output_scroll_offset.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *output_scroll_offset = (*output_scroll_offset).saturating_add(1).min(max_scroll);
        }
        // Home/End scroll to top/bottom of output
        KeyCode::Home => {
            *output_scroll_offset = 0;
        }
        KeyCode::End => {
            *output_scroll_offset = max_scroll;
        }
        // PageUp/PageDown for faster scrolling
        KeyCode::PageUp => {
            *output_scroll_offset = output_scroll_offset.saturating_sub(10);
        }
        KeyCode::PageDown => {
            *output_scroll_offset = (*output_scroll_offset).saturating_add(10).min(max_scroll);
        }
        _ => {}
    }
}

/// Handles key events when the tasks pane is focused
pub fn handle_task_key(
    key: &KeyEvent,
    active_task_id: &mut Option<TaskId>,
    pending_delete_task_id: &mut Option<TaskId>,
    expanded_conversations: &mut HashSet<TaskId>,
    task_manager: &TaskManager,
) {
    match key.code {
        KeyCode::Right => {
            // Toggle expand/collapse for the selected conversation
            if let Some(selected_id) = *active_task_id {
                let root_id =
                    super::conversation::find_root_conversation(selected_id, task_manager);
                if expanded_conversations.contains(&root_id) {
                    expanded_conversations.remove(&root_id);
                } else {
                    expanded_conversations.insert(root_id);
                }
            }
        }
        KeyCode::Left => {
            *active_task_id = None;
            *pending_delete_task_id = None;
        }
        _ => {}
    }
}

/// Handles Ctrl-N to insert a manual newline in the input pane
pub fn handle_ctrl_n(focused_pane: FocusedPane, input_manager: &mut InputManager) {
    if focused_pane == FocusedPane::Input {
        input_manager.insert_newline_at_cursor();
        input_manager.record_manual_newline();
    }
}

/// Handles Tab navigation between input and output panes when a task is active
pub fn handle_tab(focused_pane: &mut FocusedPane, active_task_id: Option<TaskId>) {
    if active_task_id.is_some() {
        *focused_pane = match *focused_pane {
            FocusedPane::Input => FocusedPane::Output,
            FocusedPane::Output | FocusedPane::Tasks => FocusedPane::Input,
        };
    }
}

/// Toggles focus between the tasks pane and the input pane
pub fn toggle_task_focus(focused_pane: &mut FocusedPane) {
    *focused_pane = match *focused_pane {
        FocusedPane::Tasks => FocusedPane::Input,
        _ => FocusedPane::Tasks,
    };
}

/// Handles backspace behavior in the tasks pane (two-step delete)
/// Returns true if a task was deleted
pub fn handle_backspace_in_tasks(
    active_task_id: Option<TaskId>,
    pending_delete_task_id: &mut Option<TaskId>,
) -> Option<TaskId> {
    let selected_task_id = active_task_id?;

    if *pending_delete_task_id == Some(selected_task_id) {
        *pending_delete_task_id = None;
        Some(selected_task_id)
    } else {
        *pending_delete_task_id = Some(selected_task_id);
        None
    }
}
