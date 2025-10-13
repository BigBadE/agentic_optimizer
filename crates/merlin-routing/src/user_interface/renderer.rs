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
use std::time::Instant;
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

        // Calculate actual content heights
        let task_content_lines = self.calculate_task_tree_height(&ctx.ui_ctx, main_area);
        let input_content_lines = ctx.input.input_area().lines().len() as u16;

        // Add borders (2) to content lines
        let task_height = task_content_lines + 2;
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
    fn calculate_task_tree_height(&self, ui_ctx: &UiCtx<'_>, area: Rect) -> u16 {
        let lines = self.build_task_tree_lines(ui_ctx, area);
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

        let lines = self.build_task_tree_lines(ui_ctx, area);

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

    /// Builds task list lines for rendering (top tasks panel)
    fn build_task_tree_lines(&self, ui_ctx: &UiCtx<'_>, area: Rect) -> Vec<Line<'static>> {
        let mut lines = Vec::default();

        // Get all tasks sorted by start time (oldest first - newest at bottom)
        let mut all_tasks: Vec<_> = ui_ctx.task_manager.iter_tasks().collect();
        all_tasks.sort_by(|(_, task_a), (_, task_b)| {
            task_a.start_time.cmp(&task_b.start_time) // Chronological order
        });

        if all_tasks.is_empty() {
            lines.push(Line::from(vec![Span::styled(
                " No tasks",
                Style::default().fg(Color::DarkGray),
            )]));
            return lines;
        }

        // Show only one task - either the selected one or the most recent
        let task_to_show = ui_ctx.state.active_task_id.map_or_else(
            || all_tasks.last(),
            |selected_id| all_tasks.iter().find(|(id, _)| *id == selected_id),
        );

        if let Some((task_id, task)) = task_to_show {
            let is_active = ui_ctx.state.active_running_tasks.contains(task_id);
            let status_icon = Self::get_task_status_icon(task, is_active);
            let is_selected = ui_ctx.state.active_task_id == Some(*task_id);

            // Show status with task description and optional stage
            let task_line = task.progress.as_ref().map_or_else(
                || format!("[{status_icon}] {}", task.description),
                |progress| format!("[{status_icon}] {} [{}]", task.description, progress.stage),
            );

            // Truncate if needed to fit in the area (account for borders and padding)
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
            if task.status == TaskStatus::Running
                && let Some(progress) = &task.progress
            {
                let progress_line = Self::render_progress_bar_line(progress, 1, area.width);
                lines.push(progress_line);
            }

            // Show other tasks if there are multiple tasks
            if all_tasks.len() > 1 {
                Self::add_other_task_lines(
                    area,
                    &mut lines,
                    &all_tasks,
                    ui_ctx.state.active_task_id,
                    ui_ctx,
                );
            }
        }

        lines
    }

    /// Adds lines for other tasks (not the currently focused one)
    fn add_other_task_lines(
        area: Rect,
        lines: &mut Vec<Line<'static>>,
        all_tasks: &[(super::TaskId, &super::task_manager::TaskDisplay)],
        active_task_id: Option<super::TaskId>,
        ui_ctx: &UiCtx<'_>,
    ) {
        let other_tasks: Vec<_> = all_tasks
            .iter()
            .filter(|(id, _)| Some(*id) != active_task_id)
            .collect();

        let max_width = area.width.saturating_sub(2) as usize;

        for (index, (other_id, other_task)) in other_tasks.iter().enumerate() {
            let is_active = ui_ctx.state.active_running_tasks.contains(other_id);
            let other_icon = Self::get_task_status_icon(other_task, is_active);
            let is_last = index == other_tasks.len() - 1;
            let prefix = if is_last { " └─" } else { " ├─" };
            let other_line = format!("{prefix} [{other_icon}] {}", other_task.description);
            let truncated_line = Self::truncate_text(&other_line, max_width);

            lines.push(Line::from(vec![Span::styled(
                truncated_line,
                Style::default().fg(Color::DarkGray),
            )]));
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
        let now = Instant::now();
        let index = (now.elapsed().as_millis() / 100) as usize % frames.len();
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
