//! Task tree line building logic
//!
//! Builds the flattened list of task tree lines for rendering,
//! handling both focused and unfocused views with different layout strategies.

use merlin_deps::ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use super::super::task_manager::{TaskDisplay, TaskStatus};
use super::super::theme::Theme;
use super::helpers as render_helpers;
use super::task_rendering;
use super::{FocusedPane, UiCtx};
use merlin_routing::TaskId;

/// Builds task list lines for rendering (top tasks panel)
pub fn build_task_tree_lines(
    ui_ctx: &UiCtx<'_>,
    area: Rect,
    focused: FocusedPane,
    theme: Theme,
) -> Vec<Line<'static>> {
    let mut lines = Vec::default();

    // Get all tasks in the order maintained by TaskManager (oldest first - newest at bottom)
    // TaskManager maintains task_order which is properly sorted by timestamp
    let all_tasks: Vec<_> = ui_ctx
        .task_manager
        .task_order()
        .iter()
        .filter_map(|&task_id| {
            ui_ctx
                .task_manager
                .get_task(task_id)
                .map(|task| (task_id, task))
        })
        .collect();

    // All tasks are now root-level (no hierarchy)
    let root_tasks: Vec<_> = all_tasks.iter().collect();

    if focused == FocusedPane::Tasks {
        build_focused_task_lines(
            &FocusedTaskLinesContext {
                ui_ctx,
                area,
                all_tasks: &all_tasks,
                root_tasks: &root_tasks,
                theme,
            },
            &mut lines,
        );
    } else {
        build_unfocused_task_lines(
            &UnfocusedTaskLinesContext {
                ui_ctx,
                area,
                all_tasks: &all_tasks,
                root_tasks: &root_tasks,
                theme,
            },
            &mut lines,
        );
    }

    lines
}

/// Context for building focused task lines
struct FocusedTaskLinesContext<'ctx> {
    ui_ctx: &'ctx UiCtx<'ctx>,
    area: Rect,
    all_tasks: &'ctx [(TaskId, &'ctx TaskDisplay)],
    root_tasks: &'ctx [&'ctx (TaskId, &'ctx TaskDisplay)],
    theme: Theme,
}

/// Builds task list lines when Tasks pane is focused
fn build_focused_task_lines(ctx: &FocusedTaskLinesContext<'_>, lines: &mut Vec<Line<'static>>) {
    let FocusedTaskLinesContext {
        ui_ctx,
        area,
        all_tasks,
        root_tasks,
        theme,
    } = *ctx;
    let max_width = area.width.saturating_sub(2) as usize;

    // Note: In the flat task system, we don't track a primary root task anymore.
    // All tasks are displayed equally in a flat list.

    // Build flat list of visible items (roots + expanded children) in display order
    let mut visible_items: Vec<(TaskId, bool)> = Vec::new();
    for (root_id, _) in root_tasks {
        visible_items.push((*root_id, false)); // false = is_child (all tasks are root-level now)

        // No children in the new flat system
        // Expansion is now used for showing steps or thread messages within a task
    }

    // Calculate visible window with scroll offset
    // scroll_offset = 0 means show oldest items at top
    // scroll_offset > 0 means scroll down to show newer items
    // Placeholder is always at the bottom (after all items)
    // area.height - 2 to account for top/bottom borders
    let max_visible = (area.height.saturating_sub(2) as usize).max(1);
    let scroll_offset = ui_ctx.state.task_list_scroll_offset;

    let total_visible_items = visible_items.len();
    let total_items = total_visible_items + 1; // +1 for placeholder
    let start_index = scroll_offset.min(total_items);
    let end_index = (start_index + max_visible).min(total_items);

    // Determine how many items to show (might be less than max if placeholder is visible)
    let items_end = end_index.min(total_visible_items);
    let items_to_show: Vec<_> = visible_items
        .iter()
        .skip(start_index)
        .take(items_end - start_index)
        .collect();

    // Check if placeholder should be visible based on scroll window
    let show_placeholder = end_index > total_visible_items;

    // Render each visible task/child
    let render_ctx = task_rendering::RenderTasksContext {
        items_to_show: &items_to_show,
        all_tasks,
        ui_ctx,
        max_width,
        theme,
    };
    task_rendering::render_visible_tasks(&render_ctx, lines);

    // Show placeholder if it's in the visible window
    if show_placeholder {
        let is_placeholder_selected = ui_ctx.state.active_task_id.is_none();
        let (prefix, placeholder_style) = if is_placeholder_selected {
            (
                "▶ ",
                Style::default()
                    .fg(theme.highlight())
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", Style::default().fg(Color::DarkGray))
        };
        lines.push(Line::from(vec![Span::styled(
            format!("{prefix}Start a new conversation..."),
            placeholder_style,
        )]));
    }
}

/// Context for building unfocused task lines
struct UnfocusedTaskLinesContext<'ctx> {
    ui_ctx: &'ctx UiCtx<'ctx>,
    area: Rect,
    all_tasks: &'ctx [(TaskId, &'ctx TaskDisplay)],
    root_tasks: &'ctx [&'ctx (TaskId, &'ctx TaskDisplay)],
    theme: Theme,
}

/// Builds task list lines when Tasks pane is NOT focused
fn build_unfocused_task_lines(ctx: &UnfocusedTaskLinesContext<'_>, lines: &mut Vec<Line<'static>>) {
    let UnfocusedTaskLinesContext {
        ui_ctx,
        area,
        all_tasks,
        root_tasks,
        theme,
    } = *ctx;
    // Show placeholder when no tasks exist
    if root_tasks.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  Start a new conversation...",
            Style::default().fg(Color::DarkGray),
        )]));
        return;
    }

    // Show placeholder when it's selected AND there are no running tasks
    // (if there are running tasks, show them instead)
    if ui_ctx.state.active_task_id.is_none() && ui_ctx.state.active_running_tasks.is_empty() {
        lines.push(Line::from(vec![Span::styled(
            "  Start a new conversation...",
            Style::default().fg(Color::DarkGray),
        )]));
        return;
    }

    // Determine which root task to display as primary
    let primary_root_task_id =
        task_rendering::determine_root_task_to_display(all_tasks, root_tasks, ui_ctx);

    // Show only the primary task with its children
    let Some((_, root_task)) = all_tasks.iter().find(|(id, _)| *id == primary_root_task_id) else {
        return;
    };

    let is_active = ui_ctx
        .state
        .active_running_tasks
        .contains(&primary_root_task_id);
    let is_selected = ui_ctx.state.active_task_id == Some(primary_root_task_id);
    let is_primary_expanded = ui_ctx
        .state
        .expanded_conversations
        .contains(&primary_root_task_id);

    render_unfocused_root_and_children(
        &UnfocusedRootContext {
            root_task,
            is_active,
            is_selected,
            is_primary_expanded,
            area,
            theme,
        },
        lines,
    );
}

/// Context for rendering unfocused root task and children
struct UnfocusedRootContext<'ctx> {
    root_task: &'ctx TaskDisplay,
    is_active: bool,
    is_selected: bool,
    is_primary_expanded: bool,
    area: Rect,
    theme: Theme,
}

/// Renders the root task and its children in unfocused view
fn render_unfocused_root_and_children(
    ctx: &UnfocusedRootContext<'_>,
    lines: &mut Vec<Line<'static>>,
) {
    let UnfocusedRootContext {
        root_task,
        is_active,
        is_selected,
        is_primary_expanded,
        area,
        theme,
    } = *ctx;
    let status_icon = render_helpers::task_status_icon(root_task, is_active);

    // In the new flat system, tasks don't have children
    // Expansion is now used for showing steps or thread messages
    let has_children = false;

    // Add expand indicator if task has expandable content
    let expand_indicator = render_helpers::expansion_indicator(has_children, is_primary_expanded);

    let task_line = root_task.progress.as_ref().map_or_else(
        || {
            format!(
                "{expand_indicator}[{status_icon}] {}",
                root_task.description
            )
        },
        |progress| {
            format!(
                "{expand_indicator}[{status_icon}] {} [{}]",
                root_task.description, progress.stage
            )
        },
    );

    let max_width = area.width.saturating_sub(2) as usize;
    let truncated_line = task_rendering::truncate_text(&task_line, max_width);

    let style = render_helpers::selection_style(is_selected, theme);

    lines.push(Line::from(vec![Span::styled(truncated_line, style)]));

    // Show progress bar for running tasks with progress
    if root_task.status == TaskStatus::Running
        && let Some(progress) = &root_task.progress
    {
        let progress_line = task_rendering::render_progress_bar_line(progress, 1, area.width);
        lines.push(progress_line);
    }

    // Render current step as a child if present and not expanded
    if !is_primary_expanded && let Some(step) = &root_task.current_step {
        let step_line = format!(" └─ ● {}", step.content);
        let truncated_step = task_rendering::truncate_text(&step_line, max_width);
        let step_style = render_helpers::step_style();
        lines.push(Line::from(vec![Span::styled(truncated_step, step_style)]));
    }

    // No children in the new flat system
    // Tasks are independent and don't have parent-child relationships
    // Expansion is now used for showing steps or thread messages within a single task
}
