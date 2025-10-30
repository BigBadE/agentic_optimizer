//! Task tree rendering logic
//!
//! Handles rendering of task lists, task trees with expand/collapse,
//! and individual task items with their status and progress indicators.

use merlin_deps::ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::task_manager::TaskDisplay;
use super::super::theme::Theme;
use super::UiCtx;
use super::helpers as render_helpers;
use merlin_routing::{TaskId, TaskProgress};

/// Determines which root task to display based on selection and running state
pub fn determine_root_task_to_display(
    _all_tasks: &[(TaskId, &TaskDisplay)],
    root_tasks: &[&(TaskId, &TaskDisplay)],
    ui_ctx: &UiCtx<'_>,
) -> TaskId {
    ui_ctx.state.active_task_id.map_or_else(
        || {
            // No selection - show the first running task, or just the first
            root_tasks
                .iter()
                .find(|(id, _)| ui_ctx.state.active_running_tasks.contains(id))
                .map_or_else(
                    || {
                        root_tasks
                            .first()
                            .map_or_else(TaskId::default, |(id, _)| *id)
                    },
                    |(id, _)| *id,
                )
        },
        |active_id| {
            // If a task is selected, return it (no hierarchy to navigate)
            // If selected task not found, default to first root
            if root_tasks.iter().any(|(id, _)| *id == active_id) {
                active_id
            } else {
                root_tasks
                    .first()
                    .map_or_else(TaskId::default, |(id, _)| *id)
            }
        },
    )
}

/// Context for rendering visible tasks
pub struct RenderTasksContext<'ctx> {
    /// Items to show (task IDs and child flags)
    pub items_to_show: &'ctx [&'ctx (TaskId, bool)],
    /// All tasks available for rendering
    pub all_tasks: &'ctx [(TaskId, &'ctx TaskDisplay)],
    /// UI context with state and task manager
    pub ui_ctx: &'ctx UiCtx<'ctx>,
    /// Primary root task ID
    pub primary_root_task_id: TaskId,
    /// Maximum width for rendering
    pub max_width: usize,
    /// Theme for styling
    pub theme: Theme,
}

/// Renders visible tasks and children into the lines vector
pub fn render_visible_tasks(ctx: &RenderTasksContext<'_>, lines: &mut Vec<Line<'static>>) {
    let RenderTasksContext {
        items_to_show,
        all_tasks,
        ui_ctx,
        primary_root_task_id,
        max_width,
        theme,
    } = ctx;
    for &&(task_id, is_child) in *items_to_show {
        let Some((_, task)) = all_tasks.iter().find(|(id, _)| *id == task_id) else {
            continue;
        };

        let is_active = ui_ctx.state.active_running_tasks.contains(&task_id);
        let status_icon = render_helpers::task_status_icon(task, is_active);
        let is_selected = ui_ctx.state.active_task_id == Some(task_id);
        let is_primary = task_id == *primary_root_task_id;

        if is_child {
            render_child_task(
                &ChildTaskContext {
                    task,
                    task_id,
                    status_icon,
                    is_selected,
                    ui_ctx,
                    max_width: *max_width,
                    theme: *theme,
                },
                lines,
            );
        } else {
            render_root_task(
                &RootTaskContext {
                    task,
                    task_id,
                    status_icon,
                    is_selected,
                    _is_primary: is_primary,
                    ui_ctx,
                    all_tasks,
                    max_width: *max_width,
                    theme: *theme,
                },
                lines,
            );
        }
    }
}

/// Context for rendering a child task
struct ChildTaskContext<'ctx> {
    task: &'ctx TaskDisplay,
    task_id: TaskId,
    status_icon: &'ctx str,
    is_selected: bool,
    ui_ctx: &'ctx UiCtx<'ctx>,
    max_width: usize,
    theme: Theme,
}

/// Renders a child task with indentation
fn render_child_task(ctx: &ChildTaskContext<'_>, lines: &mut Vec<Line<'static>>) {
    let ChildTaskContext {
        task,
        task_id,
        status_icon,
        is_selected,
        ui_ctx,
        max_width,
        theme,
    } = *ctx;
    let child_symbol = "├─";
    let retry_suffix = if task.retry_count > 0 {
        format!(" (retry {})", task.retry_count)
    } else {
        String::new()
    };
    let child_line = format!(
        " {child_symbol} [{}] {}{}",
        status_icon, task.description, retry_suffix
    );

    let truncated_line = truncate_text(&child_line, max_width);
    let style = render_helpers::selection_style(is_selected, theme);

    lines.push(Line::from(vec![Span::styled(truncated_line, style)]));

    // Show delete confirmation if this task is pending deletion
    if is_selected && ui_ctx.state.pending_delete_task_id == Some(task_id) {
        let confirm_line = "  │  └─ ⚠ Press Backspace again to confirm deletion";
        let truncated_confirm = truncate_text(confirm_line, max_width);
        let confirm_style = render_helpers::delete_confirmation_style();
        lines.push(Line::from(vec![Span::styled(
            truncated_confirm,
            confirm_style,
        )]));
    }
    // Render all steps if expanded
    else if ui_ctx.state.expanded_steps.contains(&task_id) && !task.steps.is_empty() {
        for (index, step) in task.steps.iter().enumerate() {
            let step_icon = render_helpers::step_type_icon(&step.step_type);
            let is_last_step = index == task.steps.len() - 1;
            let step_connector = if is_last_step { "└─" } else { "├─" };
            let step_line = format!("  │  {step_connector} {} {}", step_icon, step.content);
            let truncated_step = truncate_text(&step_line, max_width);
            let step_style = render_helpers::step_style_with_status(&step.step_type, step.status);
            lines.push(Line::from(vec![Span::styled(truncated_step, step_style)]));
        }
    }
    // Render current step as a sub-child if present
    else if let Some(step) = &task.current_step {
        let step_icon = render_helpers::step_type_icon(&step.step_type);
        let step_line = format!("  │  └─ {} {}", step_icon, step.content);
        let truncated_step = truncate_text(&step_line, max_width);
        let step_style = render_helpers::step_style_with_status(&step.step_type, step.status);
        lines.push(Line::from(vec![Span::styled(truncated_step, step_style)]));
    }
}

/// Context for rendering a root task
struct RootTaskContext<'ctx> {
    task: &'ctx TaskDisplay,
    task_id: TaskId,
    status_icon: &'ctx str,
    is_selected: bool,
    _is_primary: bool,
    ui_ctx: &'ctx UiCtx<'ctx>,
    all_tasks: &'ctx [(TaskId, &'ctx TaskDisplay)],
    max_width: usize,
    theme: Theme,
}

/// Renders a root task conversation
fn render_root_task(ctx: &RootTaskContext<'_>, lines: &mut Vec<Line<'static>>) {
    let RootTaskContext {
        task,
        task_id,
        status_icon,
        is_selected,
        _is_primary,
        ui_ctx,
        max_width,
        theme,
        ..
    } = *ctx;
    // In the new flat system, tasks don't have children
    // Expansion is now used for showing steps or thread messages
    let has_children = false;
    let is_expanded = ui_ctx.state.expanded_conversations.contains(&task_id);

    // Add expand indicator if task has expandable content
    let expand_indicator = render_helpers::expansion_indicator(has_children, is_expanded);

    let retry_suffix = if task.retry_count > 0 {
        format!(" (retry {})", task.retry_count)
    } else {
        String::new()
    };

    let task_line = task.progress.as_ref().map_or_else(
        || {
            format!(
                "{expand_indicator}[{status_icon}] {}{}",
                task.description, retry_suffix
            )
        },
        |progress| {
            format!(
                "{expand_indicator}[{status_icon}] {}{} [{}]",
                task.description, retry_suffix, progress.stage
            )
        },
    );

    let truncated_line = truncate_text(&task_line, max_width);
    let style = render_helpers::selection_style(is_selected, theme);

    lines.push(Line::from(vec![Span::styled(truncated_line, style)]));

    // Show delete confirmation if this task is pending deletion
    if is_selected && ui_ctx.state.pending_delete_task_id == Some(task_id) {
        let confirm_line = " └─ ⚠ Press Backspace again to confirm deletion";
        let truncated_confirm = truncate_text(confirm_line, max_width);
        let confirm_style = render_helpers::delete_confirmation_style();
        lines.push(Line::from(vec![Span::styled(
            truncated_confirm,
            confirm_style,
        )]));
    }
    // Render current step as a child if present and not expanded
    // (if expanded, step will show under the actual task children)
    else if !is_expanded && let Some(step) = &task.current_step {
        let step_icon = render_helpers::step_type_icon(&step.step_type);
        let step_line = format!(" └─ {} {}", step_icon, step.content);
        let truncated_step = truncate_text(&step_line, max_width);
        let step_style = render_helpers::step_style_with_status(&step.step_type, step.status);
        lines.push(Line::from(vec![Span::styled(truncated_step, step_style)]));
    }
}

/// Renders a progress bar line
pub fn render_progress_bar_line(
    progress: &TaskProgress,
    depth: usize,
    _width: u16,
) -> Line<'static> {
    let indent = "  ".repeat(depth);

    progress.total.map_or_else(
        || {
            let spinner = get_spinner();
            let message = &progress.message;
            let line = format!("{indent} {spinner} {message}");
            Line::from(vec![Span::styled(line, Style::default().fg(Color::Cyan))])
        },
        |total| {
            let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
            let bar_width = 12usize;
            let filled = (bar_width * percent as usize / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);

            let bar = format!(
                "{} ({percent}% {}{})",
                indent,
                "▓".repeat(filled),
                "░".repeat(empty)
            );

            Line::from(vec![Span::styled(bar, Style::default().fg(Color::Cyan))])
        },
    )
}

/// Gets a simple spinner character
fn get_spinner() -> char {
    let frames = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let now = SystemTime::now();
    let elapsed = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let index = (elapsed.as_millis() / 100) as usize % frames.len();
    frames[index]
}

/// Truncates text to fit within `max_width`, adding "..." if truncated
pub fn truncate_text(text: &str, max_width: usize) -> String {
    use merlin_deps::unicode_width::{UnicodeWidthChar as _, UnicodeWidthStr as _};

    let text_width = text.width();
    if text_width <= max_width {
        return text.to_string();
    }

    // Need to truncate - reserve space for "..."
    let target_width = max_width.saturating_sub(3);
    let mut result = String::new();
    let mut current_width = 0;

    for character in text.chars() {
        let char_width = character.width().unwrap_or(0);
        if current_width + char_width > target_width {
            break;
        }
        result.push(character);
        current_width += char_width;
    }

    result.push_str("...");
    result
}

/// Adds lines for child tasks
///
/// This function is currently unused in the flat task system but kept for future
/// when thread-based grouping is implemented.

pub fn add_child_task_lines(
    area: Rect,
    lines: &mut Vec<Line<'static>>,
    children: &[&(TaskId, &TaskDisplay)],
    ui_ctx: &UiCtx<'_>,
    theme: Theme,
) {
    let max_width = area.width.saturating_sub(2) as usize;

    for (index, (child_id, child_task)) in children.iter().enumerate() {
        let is_active = ui_ctx.state.active_running_tasks.contains(child_id);
        let child_icon = render_helpers::task_status_icon(child_task, is_active);
        let is_last = index == children.len() - 1;
        let prefix = if is_last { " └─" } else { " ├─" };
        let is_selected = Some(*child_id) == ui_ctx.state.active_task_id;

        let child_line = format!("{prefix} [{child_icon}] {}", child_task.description);
        let truncated_line = truncate_text(&child_line, max_width);

        let style = render_helpers::child_task_style(is_selected, theme);

        lines.push(Line::from(vec![Span::styled(truncated_line, style)]));

        // Render current step for this child if present
        if let Some(step) = &child_task.current_step {
            let connector = if is_last { "    " } else { " │  " };
            let step_icon = render_helpers::step_type_icon(&step.step_type);
            let step_line = format!("{connector}└─ {} {}", step_icon, step.content);
            let truncated_step = truncate_text(&step_line, max_width);
            let step_style = render_helpers::step_style_with_status(&step.step_type, step.status);
            lines.push(Line::from(vec![Span::styled(truncated_step, step_style)]));
        }
    }
}
