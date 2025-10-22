//! Task navigation logic for TUI
//!
//! Handles task list navigation (up/down arrows), task selection,
//! and scroll management for the task list pane.

use merlin_routing::TaskId;
use std::collections::{HashMap, HashSet};

use crate::ui::task_manager::TaskManager;

/// Builds a flat list of all visible tasks in display order.
///
/// Includes root conversations and their children if expanded.
/// Returns Vec of (`task_id`, `is_child`) tuples in chronological order (oldest first, newest last).
pub fn build_visible_task_list(
    task_manager: &TaskManager,
    expanded_conversations: &HashSet<TaskId>,
) -> Vec<(TaskId, bool)> {
    // Use task_order which is already sorted correctly (oldest first, newest last)
    let mut visible_tasks = Vec::new();

    for &task_id in task_manager.task_order() {
        let Some(task) = task_manager.get_task(task_id) else {
            continue;
        };

        if task.parent_id.is_none() {
            // Root conversation - always visible
            visible_tasks.push((task_id, false));

            // If expanded, children will follow in task_order
        } else {
            // Child task - only visible if parent is expanded
            let Some(parent_id) = task.parent_id else {
                continue;
            };
            if expanded_conversations.contains(&parent_id) {
                visible_tasks.push((task_id, true));
            }
        }
    }

    visible_tasks
}

/// Context for navigation operations
///
/// Contains mutable references that must be consumed by value.
/// The `clippy::needless_pass_by_value` lint is disabled for functions using this
/// because the context contains mutable references that cannot be borrowed.
pub struct NavigationContext<'nav> {
    /// Active task identifier
    pub active_task_id: &'nav mut Option<TaskId>,
    /// Set of expanded conversation IDs
    pub expanded_conversations: &'nav HashSet<TaskId>,
    /// Current scroll offset in task list
    pub task_list_scroll_offset: &'nav mut usize,
    /// Per-task output scroll positions
    pub task_output_scroll: &'nav mut HashMap<TaskId, u16>,
    /// Current output scroll offset
    pub output_scroll_offset: &'nav mut u16,
}

/// Context for scroll adjustment operations
pub struct ScrollContext<'scroll> {
    /// Active task identifier to keep visible
    pub active_task_id: Option<&'scroll TaskId>,
    /// Set of expanded conversation IDs
    pub expanded_conversations: &'scroll HashSet<TaskId>,
    /// Current scroll offset to be updated
    pub task_list_scroll_offset: &'scroll mut usize,
    /// Task manager for building visible list
    pub task_manager: &'scroll TaskManager,
    /// Terminal height for viewport calculation
    pub terminal_height: u16,
    /// Whether the tasks pane currently has focus
    pub focused_pane_is_tasks: bool,
}

/// Moves selection up within the visible tasks, updating active selection.
///
/// Navigates through both root conversations and expanded children.
/// Up moves to older tasks (up the screen).
pub fn navigate_tasks_up(
    task_manager: &TaskManager,
    ctx: &mut NavigationContext<'_>,
    terminal_height: u16,
    focused_pane_is_tasks: bool,
) {
    let visible_tasks = build_visible_task_list(task_manager, ctx.expanded_conversations);

    if visible_tasks.is_empty() {
        // No tasks, stay on placeholder
        return;
    }

    // If nothing selected (placeholder at bottom), select the newest visible task (last in list)
    if ctx.active_task_id.is_none() {
        if let Some((last_id, _)) = visible_tasks.last() {
            *ctx.active_task_id = Some(*last_id);
            // Restore scroll position for this task
            if let Some(&scroll_pos) = ctx.task_output_scroll.get(last_id) {
                *ctx.output_scroll_offset = scroll_pos;
            } else {
                *ctx.output_scroll_offset = 0;
            }
            adjust_task_list_scroll(&mut ScrollContext {
                active_task_id: ctx.active_task_id.as_ref(),
                expanded_conversations: ctx.expanded_conversations,
                task_list_scroll_offset: ctx.task_list_scroll_offset,
                task_manager,
                terminal_height,
                focused_pane_is_tasks,
            });
        }
        return;
    }

    // Find current task in the visible list
    let Some(current_id) = *ctx.active_task_id else {
        return;
    };

    // Save current output scroll position before switching
    ctx.task_output_scroll
        .insert(current_id, *ctx.output_scroll_offset);

    // Find the previous task in the visible list (older, up the screen)
    if let Some(current_pos) = visible_tasks.iter().position(|(id, _)| *id == current_id)
        && current_pos > 0
    {
        let (prev_id, _) = visible_tasks[current_pos - 1];
        *ctx.active_task_id = Some(prev_id);
        // Restore scroll position for the new task
        if let Some(&scroll_pos) = ctx.task_output_scroll.get(&prev_id) {
            *ctx.output_scroll_offset = scroll_pos;
        } else {
            *ctx.output_scroll_offset = 0;
        }
        adjust_task_list_scroll(&mut ScrollContext {
            active_task_id: ctx.active_task_id.as_ref(),
            expanded_conversations: ctx.expanded_conversations,
            task_list_scroll_offset: ctx.task_list_scroll_offset,
            task_manager,
            terminal_height,
            focused_pane_is_tasks,
        });
    }
}

/// Moves selection down within the visible tasks, updating active selection.
///
/// Navigates through both root conversations and expanded children.
/// Down moves to newer tasks (down the screen) or to placeholder.
pub fn navigate_tasks_down(
    task_manager: &TaskManager,
    ctx: &mut NavigationContext<'_>,
    terminal_height: u16,
    focused_pane_is_tasks: bool,
) {
    let visible_tasks = build_visible_task_list(task_manager, ctx.expanded_conversations);

    if visible_tasks.is_empty() {
        // No tasks, stay on placeholder
        return;
    }

    // If nothing selected (placeholder at bottom), don't move (stop at boundary)
    if ctx.active_task_id.is_none() {
        return;
    }

    // Find current task in the visible list
    let Some(current_id) = *ctx.active_task_id else {
        return;
    };

    // Save current output scroll position before switching
    ctx.task_output_scroll
        .insert(current_id, *ctx.output_scroll_offset);

    // Find the next task in the visible list (newer, down the screen)
    if let Some(current_pos) = visible_tasks.iter().position(|(id, _)| *id == current_id) {
        if current_pos + 1 < visible_tasks.len() {
            let (next_id, _) = visible_tasks[current_pos + 1];
            *ctx.active_task_id = Some(next_id);
            // Restore scroll position for the new task
            if let Some(&scroll_pos) = ctx.task_output_scroll.get(&next_id) {
                *ctx.output_scroll_offset = scroll_pos;
            } else {
                *ctx.output_scroll_offset = 0;
            }
        } else {
            // At the newest visible task, move to placeholder
            *ctx.active_task_id = None;
        }
        adjust_task_list_scroll(&mut ScrollContext {
            active_task_id: ctx.active_task_id.as_ref(),
            expanded_conversations: ctx.expanded_conversations,
            task_list_scroll_offset: ctx.task_list_scroll_offset,
            task_manager,
            terminal_height,
            focused_pane_is_tasks,
        });
    }
}

/// Adjusts task list scroll to keep the selected task visible
pub fn adjust_task_list_scroll(ctx: &mut ScrollContext<'_>) {
    // Get all visible tasks in display order (includes expanded children)
    let visible_tasks = build_visible_task_list(ctx.task_manager, ctx.expanded_conversations);
    let total_visible = visible_tasks.len();

    // Calculate how many items can be shown based on current terminal size and focus
    let task_area_height = if ctx.focused_pane_is_tasks {
        let max_height = (ctx.terminal_height * 60) / 100;
        max_height.min(ctx.terminal_height.saturating_sub(10))
    } else if ctx.active_task_id.is_some() {
        5
    } else {
        ctx.terminal_height
    };

    let max_visible = (task_area_height.saturating_sub(2) as usize).max(1);

    // Find the current position of the selected item
    let selected_index = ctx.active_task_id.map_or(
        total_visible, // Placeholder is selected - it's at index total_visible (after all tasks)
        |&selected_id| {
            // A task is selected - find its position
            visible_tasks
                .iter()
                .position(|(id, _)| *id == selected_id)
                .unwrap_or(total_visible) // If not found, treat like placeholder
        },
    );

    // Special case: if placeholder is selected (active_task_id is None), always scroll to show it at the bottom
    if ctx.active_task_id.is_none() && total_visible >= max_visible {
        *ctx.task_list_scroll_offset = total_visible.saturating_sub(max_visible - 1);
        return;
    }

    // Only adjust scroll if the selected item is outside the current visible window
    let current_scroll = *ctx.task_list_scroll_offset;
    let window_start = current_scroll;
    let window_end = current_scroll + max_visible;

    // If selected item is above the visible window, scroll up to show it at the top
    if selected_index < window_start {
        *ctx.task_list_scroll_offset = selected_index;
    }
    // If selected item is below the visible window, scroll down to show it at the bottom
    else if selected_index >= window_end {
        *ctx.task_list_scroll_offset = selected_index.saturating_sub(max_visible - 1);
    }
    // Otherwise, the item is already visible - don't change scroll
}
