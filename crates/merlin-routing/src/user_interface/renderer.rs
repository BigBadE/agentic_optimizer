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

        // Check if we have a running task with progress to determine dynamic sizing
        let has_running_task_with_progress = ctx
            .ui_ctx
            .task_manager
            .iter_tasks()
            .any(|(_, task)| task.status == TaskStatus::Running && task.progress.is_some());

        // Determine task panel height dynamically
        let task_height = if has_running_task_with_progress {
            Constraint::Min(5) // Larger for running tasks with progress
        } else {
            Constraint::Min(3) // Minimal for completed tasks
        };

        // If no task is selected, use minimal space for tasks panel
        if ctx.ui_ctx.state.active_task_id.is_none() {
            let primary_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    task_height,                // Task tree (dynamic)
                    Constraint::Percentage(80), // Input area (take most space)
                ])
                .split(main_area);

            self.render_task_tree_full(frame, primary_split[0], &ctx.ui_ctx, ctx.focused);
            self.render_input_area(frame, primary_split[1], ctx.input, ctx);
        } else {
            // With selection, split between tasks, focused details, and input
            let primary_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    task_height,                // Task tree (dynamic)
                    Constraint::Percentage(60), // Focused details
                    Constraint::Percentage(20), // Input area
                ])
                .split(main_area);

            self.render_task_tree_full(frame, primary_split[0], &ctx.ui_ctx, ctx.focused);
            self.render_focused_detail_section(frame, primary_split[1], &ctx.ui_ctx, ctx.focused);
            self.render_input_area(frame, primary_split[2], ctx.input, ctx);
        }
    }

    // Rendering methods

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

            text.push('(');
            text.push_str(&percent.to_string());
            text.push_str("% ");
            text.push_str(&"▓".repeat(filled));
            text.push_str(&"░".repeat(empty));
            text.push_str(" ETA 0:");
            if eta_secs < 10 {
                text.push('0');
            }
            text.push_str(&eta_secs.to_string());
            text.push_str("s)\n");
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

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("─── Focused - {} ", task.description))
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
            let status_icon = Self::get_task_status_icon(task);
            let is_selected = ui_ctx.state.active_task_id == Some(*task_id);

            // Show status with task description and optional stage
            let task_line = task.progress.as_ref().map_or_else(
                || format!("└─ [{status_icon}] {}", task.description),
                |progress| {
                    format!(
                        "└─ [{status_icon}] {} [{}]",
                        task.description, progress.stage
                    )
                },
            );

            let style = if is_selected {
                Style::default()
                    .fg(self.theme.highlight())
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.text())
            };

            lines.push(Line::from(vec![Span::styled(task_line, style)]));

            // Show progress bar for running tasks with progress
            if task.status == TaskStatus::Running
                && let Some(progress) = &task.progress
            {
                let progress_line = Self::render_progress_bar_line(progress, 1, area.width);
                lines.push(progress_line);
            }

            // Show task position indicator if there are multiple tasks
            if all_tasks.len() > 1 {
                let current_index = all_tasks
                    .iter()
                    .position(|(id, _)| *id == *task_id)
                    .unwrap_or(0);
                let total = all_tasks.len();
                lines.push(Line::from(vec![Span::styled(
                    format!(" ({}/{} tasks)", current_index + 1, total),
                    Style::default().fg(Color::DarkGray),
                )]));
            }
        }

        lines
    }

    // Removed unused helper functions to satisfy clippy

    /// Gets status icon for task
    fn get_task_status_icon(task: &super::task_manager::TaskDisplay) -> &'static str {
        // Check if task hasn't started yet (pending/queued)
        if task.status == TaskStatus::Running
            && task.output_lines.is_empty()
            && task.progress.is_none()
        {
            return " "; // Pending/queued
        }

        match task.status {
            TaskStatus::Running => "▶",
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

        // Create title with optional status indicator
        let title = ctx.ui_ctx.state.processing_status.as_ref().map_or_else(
            || "─── Input ".to_string(),
            |status| format!("─── Input {status} "),
        );

        input_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
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
