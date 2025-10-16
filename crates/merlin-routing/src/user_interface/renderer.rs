use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};
// Formatting helpers are implemented via push methods to avoid extra allocations
use super::input::InputManager;
use super::state::UiState;
use super::task_manager::{TaskManager, TaskStatus};
use super::theme::Theme;
use std::time::{SystemTime, UNIX_EPOCH};
use textwrap::wrap;

/// Handles rendering of the TUI
pub struct Renderer {
    theme: Theme,
}

/// Shared UI context used to reduce argument count for render helpers
pub struct UiCtx<'ctx> {
    /// Task manager reference
    pub task_manager: &'ctx TaskManager,
    /// UI state reference
    pub state: &'ctx UiState,
}

/// Rendering context with all necessary references
pub struct RenderCtx<'ctx> {
    /// UI context
    pub ui_ctx: UiCtx<'ctx>,
    /// Input manager reference
    pub input: &'ctx InputManager,
    /// Currently focused pane
    pub focused: FocusedPane,
}

impl Renderer {
    /// Creates a new Renderer with the specified theme
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Gets the current theme
    pub fn theme(&self) -> Theme {
        self.theme
    }

    /// Sets the theme
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }
    /// Renders the entire UI
    pub fn render(&self, frame: &mut Frame, ctx: &RenderCtx<'_>) {
        let main_area = frame.area();

        // Determine maximum task area height based on focus (BEFORE calculating content)
        let max_task_area_height = if ctx.focused == FocusedPane::Tasks {
            // When Tasks pane is focused, allow up to 60% of screen height with minimum input space
            let max_height = (main_area.height * 60) / 100;
            // Ensure at least 10 lines remain for input
            max_height.min(main_area.height.saturating_sub(10))
        } else if ctx.focused == FocusedPane::Output && ctx.ui_ctx.state.active_task_id.is_some() {
            // When Output pane is focused, limit task list to max 3 lines + borders
            5
        } else {
            // Default: use full height
            main_area.height
        };

        // Create constrained area for calculating task content
        let constrained_task_area = Rect {
            height: max_task_area_height,
            ..main_area
        };

        // Calculate actual content heights using constrained area
        let task_content_lines =
            self.calculate_task_tree_height(&ctx.ui_ctx, constrained_task_area, ctx.focused);
        let input_content_lines = ctx.input.input_area().lines().len() as u16;

        // Determine final task height - always size to content but don't exceed max
        // +2 for borders
        let task_height = (task_content_lines + 2).min(max_task_area_height);

        let input_height = input_content_lines + 2;

        // If no task is selected, use minimal space for tasks panel and let input fill the rest
        if ctx.ui_ctx.state.active_task_id.is_none() {
            let primary_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(task_height),
                    Constraint::Min(input_height), // Input fills remaining space
                ])
                .split(main_area);

            self.render_task_tree_full(frame, primary_split[0], &ctx.ui_ctx, ctx.focused);
            self.render_input_area(frame, primary_split[1], ctx.input, ctx);
        } else {
            // With selection, split between tasks, focused details, and input
            let primary_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(task_height),
                    Constraint::Min(10), // Focused details gets remaining space
                    Constraint::Length(input_height),
                ])
                .split(main_area);

            self.render_task_tree_full(frame, primary_split[0], &ctx.ui_ctx, ctx.focused);
            self.render_focused_detail_section(frame, primary_split[1], &ctx.ui_ctx, ctx.focused);
            self.render_input_area(frame, primary_split[2], ctx.input, ctx);
        }
    }

    // Rendering methods

    /// Calculates the height needed for the task tree content
    fn calculate_task_tree_height(
        &self,
        ui_ctx: &UiCtx<'_>,
        area: Rect,
        focused: FocusedPane,
    ) -> u16 {
        let lines = self.build_task_tree_lines(ui_ctx, area, focused);
        lines.len() as u16
    }

    /// Renders full-width task tree at the top
    fn render_task_tree_full(
        &self,
        frame: &mut Frame,
        area: Rect,
        ui_ctx: &UiCtx<'_>,
        focused: FocusedPane,
    ) {
        let border_color = if focused == FocusedPane::Tasks {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let lines = self.build_task_tree_lines(ui_ctx, area, focused);

        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Tasks ")
                    .border_style(Style::default().fg(border_color)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Renders the focused task output section
    fn render_focused_detail_section(
        &self,
        frame: &mut Frame,
        area: Rect,
        ui_ctx: &UiCtx<'_>,
        focused: FocusedPane,
    ) {
        let border_color = if focused == FocusedPane::Output {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let Some(active_task_id) = ui_ctx.state.active_task_id else {
            // When no task selected, the focused box is omitted by caller; nothing to render
            return;
        };

        let Some(task) = ui_ctx.task_manager.get_task(active_task_id) else {
            return;
        };

        let mut text = String::new();

        if let Some(progress) = &task.progress
            && let Some(total) = progress.total
        {
            let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
            let bar_width = 30;
            let filled = (bar_width * percent as usize / 100).min(bar_width);
            let empty = bar_width.saturating_sub(filled);
            let eta_secs = (total.saturating_sub(progress.current)) / 2;

            // Convert seconds to minutes:seconds format
            let eta_minutes = eta_secs / 60;
            let eta_remaining_secs = eta_secs % 60;

            text.push('(');
            text.push_str(&percent.to_string());
            text.push_str("% ");
            text.push_str(&"▓".repeat(filled));
            text.push_str(&"░".repeat(empty));
            text.push_str(" ETA ");
            text.push_str(&eta_minutes.to_string());
            text.push(':');
            if eta_remaining_secs < 10 {
                text.push('0');
            }
            text.push_str(&eta_remaining_secs.to_string());
            text.push_str(")\n");
        }

        text.push_str(&Self::build_tree_text(
            task,
            area.width,
            FocusedPane::Output,
        ));

        // Calculate content height and clamp scroll offset
        // Account for borders (2) and padding (2)
        let content_height = area.height.saturating_sub(4);
        let text_lines = text.lines().count() as u16;
        let max_scroll = text_lines.saturating_sub(content_height);
        let clamped_scroll = ui_ctx.state.output_scroll_offset.min(max_scroll);

        // Build title with optional embedding progress indicator
        let title = if let Some((current, total)) = ui_ctx.state.embedding_progress {
            let percent = (current as f64 / total as f64 * 100.0) as u16;
            let base_title = format!(
                "─── Focused - {}  [Indexing: {}%] ",
                task.description, percent
            );
            // Truncate title to fit in area width
            Self::truncate_text(&base_title, area.width.saturating_sub(2) as usize)
        } else {
            let base_title = format!("─── Focused - {} ", task.description);
            // Truncate title to fit in area width
            Self::truncate_text(&base_title, area.width.saturating_sub(2) as usize)
        };

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::horizontal(1)),
            )
            .wrap(Wrap { trim: false })
            .scroll((clamped_scroll, 0));

        frame.render_widget(paragraph, area);
    }

    /// Determines which root task to display based on selection and running state
    fn determine_root_task_to_display(
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        root_tasks: &[&(super::TaskId, &super::task_manager::TaskDisplay)],
        ui_ctx: &UiCtx<'_>,
    ) -> super::TaskId {
        ui_ctx.state.active_task_id.map_or_else(
            || {
                // No selection - show the first running root, or just the first
                root_tasks
                    .iter()
                    .find(|(id, _)| ui_ctx.state.active_running_tasks.contains(id))
                    .map_or_else(
                        || {
                            root_tasks
                                .first()
                                .map_or_else(super::TaskId::default, |(id, _)| *id)
                        },
                        |(id, _)| *id,
                    )
            },
            |active_id| {
                // If a task is selected, find its root
                all_tasks
                    .iter()
                    .find(|(id, _)| *id == active_id)
                    .and_then(|(_, task)| {
                        if task.parent_id.is_none() {
                            // Selected task is a root conversation
                            Some(active_id)
                        } else {
                            // Selected task is a child - return its parent (the root conversation)
                            task.parent_id
                        }
                    })
                    // If selected task not found, default to first root
                    .unwrap_or_else(|| {
                        root_tasks
                            .first()
                            .map_or_else(super::TaskId::default, |(id, _)| *id)
                    })
            },
        )
    }

    /// Renders visible tasks and children into the lines vector
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn render_visible_tasks(
        &self,
        items_to_show: &[&(super::TaskId, bool)],
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        ui_ctx: &UiCtx<'_>,
        primary_root_task_id: super::TaskId,
        max_width: usize,
        lines: &mut Vec<Line<'static>>,
    ) {
        for &&(task_id, is_child) in items_to_show {
            let Some((_, task)) = all_tasks.iter().find(|(id, _)| *id == task_id) else {
                continue;
            };

            let is_active = ui_ctx.state.active_running_tasks.contains(&task_id);
            let status_icon = Self::get_task_status_icon(task, is_active);
            let is_selected = ui_ctx.state.active_task_id == Some(task_id);
            let is_primary = task_id == primary_root_task_id;

            if is_child {
                self.render_child_task(task, status_icon, is_selected, max_width, lines);
            } else {
                self.render_root_task(
                    task,
                    task_id,
                    status_icon,
                    is_selected,
                    is_primary,
                    ui_ctx,
                    all_tasks,
                    max_width,
                    lines,
                );
            }
        }
    }

    /// Renders a child task with indentation
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn render_child_task(
        &self,
        task: &super::task_manager::TaskDisplay,
        status_icon: &str,
        is_selected: bool,
        max_width: usize,
        lines: &mut Vec<Line<'static>>,
    ) {
        let child_symbol = "├─";
        let child_line = format!(" {child_symbol} [{}] {}", status_icon, task.description);

        let truncated_line = Self::truncate_text(&child_line, max_width);
        let style = if is_selected {
            Style::default()
                .fg(self.theme.highlight())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };

        lines.push(Line::from(vec![Span::styled(truncated_line, style)]));
    }

    /// Renders a root task conversation
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn render_root_task(
        &self,
        task: &super::task_manager::TaskDisplay,
        task_id: super::TaskId,
        status_icon: &str,
        is_selected: bool,
        _is_primary: bool,
        ui_ctx: &UiCtx<'_>,
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        max_width: usize,
        lines: &mut Vec<Line<'static>>,
    ) {
        // Check if this conversation has children
        let has_children = all_tasks
            .iter()
            .any(|(_, task_item)| task_item.parent_id == Some(task_id));
        let is_expanded = ui_ctx.state.expanded_conversations.contains(&task_id);

        // Add expand indicator if conversation has children
        let expand_indicator = match (has_children, is_expanded) {
            (true, true) => "▼ ",
            (true, false) => "▶ ",
            (false, _) => "",
        };

        let task_line = task.progress.as_ref().map_or_else(
            || format!("{expand_indicator}[{status_icon}] {}", task.description),
            |progress| {
                format!(
                    "{expand_indicator}[{status_icon}] {} [{}]",
                    task.description, progress.stage
                )
            },
        );

        let truncated_line = Self::truncate_text(&task_line, max_width);

        let style = if is_selected {
            Style::default()
                .fg(self.theme.highlight())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };

        lines.push(Line::from(vec![Span::styled(truncated_line, style)]));
    }

    /// Builds task list lines for rendering (top tasks panel)
    fn build_task_tree_lines(
        &self,
        ui_ctx: &UiCtx<'_>,
        area: Rect,
        focused: FocusedPane,
    ) -> Vec<Line<'static>> {
        let mut lines = Vec::default();

        // Get all tasks sorted by start time (oldest first - newest at bottom)
        let mut all_tasks: Vec<_> = ui_ctx.task_manager.iter_tasks().collect();
        all_tasks.sort_by(|(_, task_a), (_, task_b)| {
            task_a.start_time.cmp(&task_b.start_time) // Chronological order
        });

        // Get all root tasks (conversations) sorted by start time (oldest first, newest at bottom)
        let mut root_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|(_, task)| task.parent_id.is_none())
            .collect();
        root_tasks.sort_by(|(_, task_a), (_, task_b)| task_a.start_time.cmp(&task_b.start_time));

        if focused == FocusedPane::Tasks {
            self.build_focused_task_lines(ui_ctx, area, &all_tasks, &root_tasks, &mut lines);
        } else {
            self.build_unfocused_task_lines(ui_ctx, area, &all_tasks, &root_tasks, &mut lines);
        }

        lines
    }

    /// Builds task list lines when Tasks pane is focused
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn build_focused_task_lines(
        &self,
        ui_ctx: &UiCtx<'_>,
        area: Rect,
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        root_tasks: &[&(super::TaskId, &super::task_manager::TaskDisplay)],
        lines: &mut Vec<Line<'static>>,
    ) {
        let max_width = area.width.saturating_sub(2) as usize;

        // Determine which root task to display as primary
        let primary_root_task_id = if root_tasks.is_empty() {
            super::TaskId::default()
        } else {
            Self::determine_root_task_to_display(all_tasks, root_tasks, ui_ctx)
        };

        // Build flat list of visible items (roots + expanded children) in display order
        let mut visible_items: Vec<(super::TaskId, bool)> = Vec::new();
        for (root_id, _) in root_tasks {
            visible_items.push((*root_id, false)); // false = is_child

            // If expanded, add children
            if ui_ctx.state.expanded_conversations.contains(root_id) {
                let mut children: Vec<_> = all_tasks
                    .iter()
                    .filter(|(_, task)| task.parent_id == Some(*root_id))
                    .collect();
                children
                    .sort_by(|(_, task_a), (_, task_b)| task_a.start_time.cmp(&task_b.start_time));

                for (child_id, _) in children {
                    visible_items.push((*child_id, true)); // true = is_child
                }
            }
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
        self.render_visible_tasks(
            &items_to_show,
            all_tasks,
            ui_ctx,
            primary_root_task_id,
            max_width,
            lines,
        );

        // Show placeholder if it's in the visible window
        if show_placeholder {
            let is_placeholder_selected = ui_ctx.state.active_task_id.is_none();
            let (prefix, placeholder_style) = if is_placeholder_selected {
                (
                    "▶ ",
                    Style::default()
                        .fg(self.theme.highlight())
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

    /// Builds task list lines when Tasks pane is NOT focused
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn build_unfocused_task_lines(
        &self,
        ui_ctx: &UiCtx<'_>,
        area: Rect,
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        root_tasks: &[&(super::TaskId, &super::task_manager::TaskDisplay)],
        lines: &mut Vec<Line<'static>>,
    ) {
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
            Self::determine_root_task_to_display(all_tasks, root_tasks, ui_ctx);

        // Show only the primary task with its children
        let Some((_, root_task)) = all_tasks.iter().find(|(id, _)| *id == primary_root_task_id)
        else {
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

        self.render_unfocused_root_and_children(
            root_task,
            all_tasks,
            primary_root_task_id,
            is_active,
            is_selected,
            is_primary_expanded,
            area,
            ui_ctx,
            lines,
        );
    }

    /// Renders the root task and its children in unfocused view
    #[allow(clippy::too_many_arguments, reason = "Helper method needs all context")]
    fn render_unfocused_root_and_children(
        &self,
        root_task: &super::task_manager::TaskDisplay,
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        primary_root_task_id: super::TaskId,
        is_active: bool,
        is_selected: bool,
        is_primary_expanded: bool,
        area: Rect,
        ui_ctx: &UiCtx<'_>,
        lines: &mut Vec<Line<'static>>,
    ) {
        let status_icon = Self::get_task_status_icon(root_task, is_active);

        // Check if this conversation has children
        let has_children = all_tasks
            .iter()
            .any(|(_, task)| task.parent_id == Some(primary_root_task_id));

        // Add expand indicator if conversation has children
        let expand_indicator = match (has_children, is_primary_expanded) {
            (true, true) => "▼ ",
            (true, false) => "▶ ",
            (false, _) => "",
        };

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
        let truncated_line = Self::truncate_text(&task_line, max_width);

        let style = if is_selected {
            Style::default()
                .fg(self.theme.highlight())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };

        lines.push(Line::from(vec![Span::styled(truncated_line, style)]));

        // Show progress bar for running tasks with progress
        if root_task.status == TaskStatus::Running
            && let Some(progress) = &root_task.progress
        {
            let progress_line = Self::render_progress_bar_line(progress, 1, area.width);
            lines.push(progress_line);
        }

        // Show children only if conversation is expanded OR if they're currently running
        let children: Vec<_> = all_tasks
            .iter()
            .filter(|(id, task)| {
                task.parent_id == Some(primary_root_task_id)
                    && (is_primary_expanded
                        || ui_ctx.state.active_running_tasks.contains(id)
                        || task.status == TaskStatus::Running)
            })
            .collect();

        if !children.is_empty() {
            self.add_child_task_lines(area, lines, &children, ui_ctx);
        }
    }

    /// Adds lines for child tasks
    fn add_child_task_lines(
        &self,
        area: Rect,
        lines: &mut Vec<Line<'static>>,
        children: &[&(super::TaskId, &super::task_manager::TaskDisplay)],
        ui_ctx: &UiCtx<'_>,
    ) {
        let max_width = area.width.saturating_sub(2) as usize;

        for (index, (child_id, child_task)) in children.iter().enumerate() {
            let is_active = ui_ctx.state.active_running_tasks.contains(child_id);
            let child_icon = Self::get_task_status_icon(child_task, is_active);
            let is_last = index == children.len() - 1;
            let prefix = if is_last { " └─" } else { " ├─" };
            let is_selected = Some(*child_id) == ui_ctx.state.active_task_id;

            let child_line = format!("{prefix} [{child_icon}] {}", child_task.description);
            let truncated_line = Self::truncate_text(&child_line, max_width);

            let style = if is_selected {
                Style::default()
                    .fg(self.theme.highlight())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            lines.push(Line::from(vec![Span::styled(truncated_line, style)]));
        }
    }

    /// Truncates text to fit within `max_width`, adding "..." if truncated
    fn truncate_text(text: &str, max_width: usize) -> String {
        use unicode_width::{UnicodeWidthChar as _, UnicodeWidthStr as _};

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

    /// Gets status icon for task
    fn get_task_status_icon(
        task: &super::task_manager::TaskDisplay,
        is_active: bool,
    ) -> &'static str {
        match task.status {
            TaskStatus::Running => {
                // Check if task has output or progress
                if !task.output_lines.is_empty() || task.progress.is_some() {
                    "▶" // Running with output
                } else if is_active {
                    "◉" // Active but no output yet
                } else {
                    " " // Pending/queued
                }
            }
            TaskStatus::Completed => "✔",
            TaskStatus::Failed => "✗",
        }
    }

    /// Renders a progress bar line
    fn render_progress_bar_line(
        progress: &super::events::TaskProgress,
        depth: usize,
        _width: u16,
    ) -> Line<'static> {
        let indent = "  ".repeat(depth);

        progress.total.map_or_else(
            || {
                let spinner = Self::get_spinner();
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

    fn render_input_area(
        &self,
        frame: &mut Frame,
        area: Rect,
        input_manager: &InputManager,
        ctx: &RenderCtx<'_>,
    ) {
        let mut input_area = input_manager.input_area().clone();

        let border_color = if ctx.focused == FocusedPane::Input {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let cursor_style = if ctx.focused == FocusedPane::Input {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };

        input_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("─── Input ")
                .border_style(Style::default().fg(border_color))
                .padding(Padding::horizontal(1)),
        );
        input_area.set_style(Style::default().fg(self.theme.text()));
        input_area.set_cursor_style(cursor_style);

        frame.render_widget(&input_area, area);
    }

    // Helper methods

    fn build_tree_text(
        task: &super::task_manager::TaskDisplay,
        width: u16,
        _focused_pane: FocusedPane,
    ) -> String {
        let available_width = width.saturating_sub(4) as usize;
        if task.output_lines.is_empty() {
            return "No output yet...".to_owned();
        }

        task.output_lines
            .iter()
            .filter(|line| !line.trim_start().starts_with("Prompt:"))
            .flat_map(|line| {
                wrap(line, available_width)
                    .into_iter()
                    .map(|cow| cow.to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Focused pane identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Text input pane on the left
    Input,
    /// Output pane displaying task tree
    Output,
    /// Tasks list pane on the right
    Tasks,
}

// Helper functions

// removed old wrap_tree_content and node formatting; focused output prints raw text
