use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};
// Formatting helpers are implemented via push methods to avoid extra allocations
use super::input::InputManager;
use super::output_tree;
use super::state::UiState;
use super::task_manager::{TaskManager, TaskStatus};
use super::theme::Theme;
use crate::TaskId;
use std::time::Instant;
use textwrap::wrap;

/// Handles rendering of the TUI
pub struct Renderer {
    theme: Theme,
}

// Parameter structs used to reduce argument count and improve clarity
struct NodeFormatParams {
    is_selected: bool,
    prefix: String,
    icon: String,
    content: String,
    available_width: usize,
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

        // Flexible vertical split: allocate a stable portion to input to keep it visible,
        // and split the remaining area between task tree and focused details.
        // Bottom input gets 20% of the height, top+middle get 80%.
        let primary_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(80), // Task tree + details
                Constraint::Percentage(20), // Input area
            ])
            .split(main_area);

        let top_middle_area = primary_split[0];
        let input_area = primary_split[1];

        // Split the top section between task tree (45%) and focused details (55%)
        let top_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Task tree
                Constraint::Percentage(55), // Focused details
            ])
            .split(top_middle_area);

        // Top: Full-width task tree
        self.render_task_tree_full(frame, top_split[0], &ctx.ui_ctx, ctx.focused);

        // Middle: Focused task details with logs and TODOs
        self.render_focused_detail_section(frame, top_split[1], &ctx.ui_ctx, ctx.focused);

        // Bottom: Input area
        self.render_input_area(frame, input_area, ctx.input, ctx.focused);
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
            let help_text = Paragraph::new("No task selected\n\nPress Ctrl+T to select a task")
                .style(Style::default().fg(Color::DarkGray))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(border_color))
                        .padding(Padding::horizontal(1)),
                )
                .alignment(Alignment::Center);
            frame.render_widget(help_text, area);
            return;
        };

        let Some(task) = ui_ctx.task_manager.get_task(active_task_id) else {
            return;
        };

        let mut text = format!("Focused task: {}\n", task.description);

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

        text.push('\n');
        text.push_str(&Self::build_tree_text(
            task,
            area.width,
            FocusedPane::Output,
        ));

        let paragraph = Paragraph::new(text)
            .style(Style::default().fg(self.theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::horizontal(1)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Builds task tree lines for rendering
    fn build_task_tree_lines(&self, ui_ctx: &UiCtx<'_>, area: Rect) -> Vec<Line<'static>> {
        let visible_tasks = ui_ctx.task_manager.get_visible_tasks();
        let mut lines = Vec::default();

        for task_id in &visible_tasks {
            let Some(task) = ui_ctx.task_manager.get_task(*task_id) else {
                continue;
            };

            let depth = Self::calculate_task_depth(*task_id, ui_ctx.task_manager);
            let indent = "  ".repeat(depth);

            let status_icon = Self::get_task_status_icon(task);
            let collapse_indicator = if ui_ctx.task_manager.has_children(*task_id) {
                if ui_ctx.task_manager.is_collapsed(*task_id) {
                    " ▶"
                } else {
                    ""
                }
            } else {
                ""
            };

            // Show subtask TODOs if any
            let todo_indicator = if ui_ctx.task_manager.has_children(*task_id) {
                let child_count = ui_ctx
                    .task_manager
                    .task_order()
                    .iter()
                    .filter(|&&id| {
                        ui_ctx
                            .task_manager
                            .get_task(id)
                            .is_some_and(|child_task| child_task.parent_id == Some(*task_id))
                    })
                    .count();
                if child_count > 0 {
                    format!("  ({child_count} subtasks)")
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let is_selected = ui_ctx.state.active_task_id == Some(*task_id);
            let task_line = format!(
                "{indent}├─ [{status_icon}] {}{}{}",
                task.description, collapse_indicator, todo_indicator
            );

            let style = if is_selected {
                Style::default()
                    .fg(self.theme.highlight())
                    .add_modifier(Modifier::BOLD)
            } else {
                match task.status {
                    TaskStatus::Running => Style::default()
                        .fg(self.theme.text())
                        .add_modifier(Modifier::BOLD),
                    TaskStatus::Completed => Style::default().fg(self.theme.success()),
                    TaskStatus::Failed => Style::default().fg(self.theme.error()),
                }
            };

            lines.push(Line::from(vec![Span::styled(task_line, style)]));

            if let Some(progress) = &task.progress {
                let progress_line = Self::render_progress_bar_line(progress, depth + 1, area.width);
                lines.push(progress_line);
            }

            if task.status == TaskStatus::Running {
                Self::add_running_task_info(&mut lines, task, &indent);
            }
        }

        if lines.is_empty() {
            lines.push(Line::from("No tasks running"));
        }

        lines
    }

    /// Adds running task info (logs, elapsed time) to lines
    fn add_running_task_info(
        lines: &mut Vec<Line<'static>>,
        task: &super::task_manager::TaskDisplay,
        indent: &str,
    ) {
        let elapsed = task
            .end_time
            .unwrap_or_else(Instant::now)
            .duration_since(task.start_time)
            .as_secs_f64();

        if let Some(last_output) = task.output_lines.last() {
            let log_line = format!("{indent}   ⤷ log: {last_output}");
            lines.push(Line::from(vec![Span::styled(
                log_line,
                Style::default().fg(Color::DarkGray),
            )]));
        }

        if elapsed > 5.0 {
            let time_line = format!("{indent}   ({elapsed:.0}s)");
            lines.push(Line::from(vec![Span::styled(
                time_line,
                Style::default().fg(Color::DarkGray),
            )]));
        }
    }

    /// Calculates task depth in hierarchy
    fn calculate_task_depth(task_id: TaskId, task_manager: &TaskManager) -> usize {
        let mut depth = 0;
        let mut current_id = task_id;

        while let Some(task) = task_manager.get_task(current_id) {
            if let Some(parent_id) = task.parent_id {
                depth += 1;
                current_id = parent_id;
            } else {
                break;
            }

            if depth > 10 {
                break;
            }
        }

        depth
    }

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
                let line = format!("{indent}  {spinner} {message}");
                Line::from(vec![Span::styled(line, Style::default().fg(Color::Cyan))])
            },
            |total| {
                let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
                let bar_width = 12usize;
                let filled = (bar_width * percent as usize / 100).min(bar_width);
                let empty = bar_width.saturating_sub(filled);

                let bar = format!(
                    "{}  ({percent}% {}{})",
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
        focused_pane: FocusedPane,
    ) {
        let mut input_area = input_manager.input_area().clone();

        let border_color = if focused_pane == FocusedPane::Input {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let cursor_style = if focused_pane == FocusedPane::Input {
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
        focused_pane: FocusedPane,
    ) -> String {
        let visible_nodes = task.output_tree.flatten_visible_nodes();
        let selected_idx = task.output_tree.selected_index();
        let available_width = width.saturating_sub(4) as usize;

        if visible_nodes.is_empty() {
            return "No output yet...".to_owned();
        }

        visible_nodes
            .iter()
            .enumerate()
            .flat_map(|(idx, (node_ref, depth))| {
                let is_selected = idx == selected_idx && focused_pane == FocusedPane::Output;
                let prefix = output_tree::build_tree_prefix(
                    *depth,
                    node_ref.is_last,
                    &node_ref.parent_states,
                );
                let is_collapsed = task.output_tree.is_collapsed(node_ref.node);
                let icon = node_ref.node.get_icon(is_collapsed).to_string();
                let content = node_ref.node.get_content();

                Self::format_tree_node(&NodeFormatParams {
                    is_selected,
                    prefix,
                    icon,
                    content,
                    available_width,
                })
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_tree_node(params: &NodeFormatParams) -> Vec<String> {
        let selector = if params.is_selected { "► " } else { "  " };
        let line_prefix = format!(
            "{selector}{prefix}{icon} ",
            prefix = params.prefix,
            icon = params.icon
        );
        let prefix_width = line_prefix.len();

        let content_width = params.available_width - prefix_width;
        if content_width < 20 {
            return vec![format!("{}{}", line_prefix, params.content)];
        }

        wrap_tree_content(&line_prefix, &params.content, content_width, prefix_width)
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

/// Wraps content lines for a tree node while preserving a prefix width
fn wrap_tree_content(
    line_prefix: &str,
    content: &str,
    content_width: usize,
    prefix_width: usize,
) -> Vec<String> {
    let wrapped = wrap(content, content_width);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                format!("{line_prefix}{line}")
            } else {
                format!("{}  {}", " ".repeat(prefix_width), line)
            }
        })
        .collect()
}
