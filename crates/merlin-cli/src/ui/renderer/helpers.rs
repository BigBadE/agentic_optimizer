//! Helper utilities for rendering UI components
//!
//! Provides reusable functions for style calculation, status icons,
//! and other common rendering operations to reduce code duplication.

use merlin_deps::ratatui::style::{Color, Modifier, Style};

use super::super::task_manager::{TaskDisplay, TaskStatus};
use super::super::theme::Theme;

/// Calculates expansion indicator based on children and expansion state
///
/// # Arguments
/// * `has_children` - Whether the task has children
/// * `is_expanded` - Whether the task is currently expanded
///
/// # Returns
/// String slice with the appropriate indicator ("▼ ", "▶ ", or "")
pub fn expansion_indicator(has_children: bool, is_expanded: bool) -> &'static str {
    match (has_children, is_expanded) {
        (true, true) => "▼ ",
        (true, false) => "▶ ",
        (false, _) => "",
    }
}

/// Calculates the text style based on selection state
///
/// # Arguments
/// * `is_selected` - Whether the item is selected
/// * `theme` - Current theme
///
/// # Returns
/// Ratatui `Style` with appropriate foreground color and modifiers
pub fn selection_style(is_selected: bool, theme: Theme) -> Style {
    if is_selected {
        Style::default()
            .fg(theme.highlight())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text())
    }
}

/// Calculates the text style for child tasks
///
/// # Arguments
/// * `is_selected` - Whether the child task is selected
/// * `theme` - Current theme
///
/// # Returns
/// Ratatui `Style` with appropriate colors for child tasks
#[allow(
    dead_code,
    reason = "Used by add_child_task_lines, kept for future thread-based grouping"
)]
pub fn child_task_style(is_selected: bool, theme: Theme) -> Style {
    if is_selected {
        Style::default()
            .fg(theme.highlight())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

/// Gets status icon for task based on state and activity
///
/// # Arguments
/// * `task` - Task display information
/// * `is_active` - Whether the task is currently active
///
/// # Returns
/// String slice with appropriate status icon
pub fn task_status_icon(task: &TaskDisplay, is_active: bool) -> &'static str {
    match task.status {
        TaskStatus::Pending => "⋯", // Pending/waiting for dependencies
        TaskStatus::Running => {
            // Check if task has output or progress
            if !task.output_lines.is_empty() || task.progress.is_some() {
                "▶" // Running with output
            } else if is_active {
                "◉" // Active but no output yet
            } else {
                " " // Running but no output
            }
        }
        TaskStatus::Completed => "✔",
        TaskStatus::Failed => "✗",
    }
}

/// Calculates step style (for current step display)
///
/// # Returns
/// Ratatui `Style` for step indicators
pub fn step_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// Calculates delete confirmation style
///
/// # Returns
/// Ratatui `Style` for delete confirmation messages
pub fn delete_confirmation_style() -> Style {
    Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD)
}

/// Gets icon for a step type
///
/// # Arguments
/// * `step_type` - Type of step (e.g., `thinking`, `tool_call`, `validation`)
///
/// # Returns
/// Unicode icon representing the step type
pub fn step_type_icon(step_type: &str) -> &'static str {
    match step_type {
        "thinking" => "💭",
        "tool_call" => "🔧",
        "validation" => "✓",
        "error" => "❌",
        "planning" => "📋",
        _ => "●",
    }
}

/// Gets style for a step based on its status
///
/// # Arguments
/// * `_step_type` - Type of step (e.g., `thinking`, `tool_call`, `validation`) - reserved for future use
/// * `status` - Status of the step
///
/// # Returns
/// Ratatui `Style` for the step
pub fn step_style_with_status(
    _step_type: &str,
    status: super::super::task_manager::TaskStepStatus,
) -> Style {
    use super::super::task_manager::TaskStepStatus;

    let color = match status {
        TaskStepStatus::Running => Color::Gray,
        TaskStepStatus::Completed => Color::Green,
        TaskStepStatus::Failed => Color::Red,
    };

    Style::default().fg(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test expansion indicator symbols
    ///
    /// # Panics
    /// Panics if test assertions fail
    #[test]
    fn test_expansion_indicator() {
        assert_eq!(expansion_indicator(true, true), "▼ ");
        assert_eq!(expansion_indicator(true, false), "▶ ");
        assert_eq!(expansion_indicator(false, true), "");
        assert_eq!(expansion_indicator(false, false), "");
    }

    /// Test selection styling
    ///
    /// # Panics
    /// Panics if test assertions fail
    #[test]
    fn test_selection_style() {
        let theme = Theme::default();
        let selected = selection_style(true, theme);
        let unselected = selection_style(false, theme);

        // Selected should have highlight color and bold
        assert_eq!(selected.fg, Some(theme.highlight()));

        // Unselected should have normal text color
        assert_eq!(unselected.fg, Some(theme.text()));
    }
}
