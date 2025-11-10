//! UI rendering module
//!
//! Handles rendering of the thread-based UI layout.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

use std::sync::{Arc, Mutex};

use merlin_agent::ThreadStore;
use merlin_core::{Thread, ThreadId};
use ratatui::text::Line;

use super::input::InputManager;
use super::layout;
use super::scroll;
use super::state::UiState;
use super::task_manager::{TaskDisplay, TaskManager};
use super::theme::Theme;

// Layout constants
const MIN_REMAINING_HEIGHT: u16 = 10;

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
    /// Thread store reference (shared with orchestrator)
    pub thread_store: &'ctx Arc<Mutex<ThreadStore>>,
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

        // Always use thread mode (side-by-side layout with threads | work + input)
        self.render_thread_mode(frame, main_area, ctx);
    }

    /// Renders the thread-based side-by-side layout
    fn render_thread_mode(&self, frame: &mut Frame, main_area: Rect, ctx: &RenderCtx<'_>) {
        let input_content_lines = ctx.input.input_area().lines().len() as u16;
        let input_height = layout::calculate_input_area_height(input_content_lines);

        // Split horizontally: threads (30%) | work details (70%)
        let horizontal_split = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(main_area);

        // For the work details side, split vertically: tasks + output | input
        let right_side_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(MIN_REMAINING_HEIGHT),
                Constraint::Length(input_height),
            ])
            .split(horizontal_split[1]);

        // Render thread list on left
        self.render_thread_list(frame, horizontal_split[0], ctx);

        // Render work details on top right
        self.render_focused_detail_section(frame, right_side_split[0], &ctx.ui_ctx, ctx.focused);

        // Render input on bottom right
        self.render_input_area(frame, right_side_split[1], ctx.input, ctx);
    }

    // Rendering methods

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

        // Get task output if a task is selected
        let (text, title) = if let Some(active_task_id) = ui_ctx.state.active_task_id
            && let Some(task) = ui_ctx.task_manager.get_task(active_task_id)
        {
            // Get plain text output from task
            let text = task.output.clone();

            // Build title without embedding progress (moved to input box)
            let base_title = format!("─── Focused - {} ", task.description);
            let title = truncate_text(&base_title, area.width.saturating_sub(2) as usize);

            (text, title)
        } else {
            // No task selected - show empty output pane with generic title
            (String::new(), "─── Focused ".to_owned())
        };

        // Calculate content height and clamp scroll offset
        // Account for borders only (2) - horizontal padding doesn't affect height
        let viewport_height = area.height.saturating_sub(2);
        let text_lines = scroll::count_text_lines(&text);
        let max_scroll = text_lines.saturating_sub(viewport_height);
        let clamped_scroll = ui_ctx.state.output_scroll_offset.min(max_scroll);

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

        // Build title with optional embedding progress indicator
        let title = if let Some((current, total)) = ctx.ui_ctx.state.embedding_progress {
            let percent = (current as f64 / total as f64 * 100.0) as u16;
            format!("─── Input  [Indexing: {percent}%] ")
        } else {
            "─── Input ".to_owned()
        };

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

    fn render_thread_list(&self, frame: &mut Frame, area: Rect, ctx: &RenderCtx<'_>) {
        let focused = ctx.focused;
        let selected_thread_id = ctx.ui_ctx.state.active_thread_id;
        let thread_store = ctx.thread_store;

        let border_color = if focused == FocusedPane::Threads {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let lines = thread_store.lock().ok().map_or_else(Vec::new, |store| {
            let threads = store.active_threads();
            self.build_thread_list_lines(&threads, selected_thread_id, focused)
        });

        let paragraph = Paragraph::new(lines)
            .style(Style::default().fg(self.theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Threads ")
                    .border_style(Style::default().fg(border_color))
                    .padding(Padding::horizontal(1)),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(paragraph, area);
    }

    /// Builds the lines for thread list display
    fn build_thread_list_lines(
        &self,
        threads: &[&Thread],
        selected_thread_id: Option<ThreadId>,
        focused: FocusedPane,
    ) -> Vec<Line<'static>> {
        use ratatui::text::Span;

        let mut lines = Vec::new();

        if threads.is_empty() {
            lines.push(Line::from(Span::styled(
                "No threads yet",
                Style::default().fg(self.theme.text()),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Press 'n' to create a new thread",
                Style::default().fg(self.theme.text()),
            )));
        } else {
            for (index, thread) in threads.iter().enumerate() {
                lines.push(self.build_thread_line(thread, selected_thread_id, index + 1));
            }
        }

        // Add help text at bottom
        if focused == FocusedPane::Threads {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "n:new b:branch d:delete ↑↓:navigate",
                Style::default()
                    .fg(self.theme.text())
                    .add_modifier(Modifier::DIM),
            )));
        }

        lines
    }

    /// Builds a single thread line with selection, number, name, and status
    fn build_thread_line(
        &self,
        thread: &Thread,
        selected_thread_id: Option<ThreadId>,
        thread_number: usize,
    ) -> Line<'static> {
        use ratatui::text::Span;

        let is_selected = selected_thread_id == Some(thread.id);
        let mut spans = Vec::new();

        // Selection indicator
        if is_selected {
            spans.push(Span::styled(
                "> ",
                Style::default()
                    .fg(self.theme.focused_border())
                    .add_modifier(Modifier::BOLD),
            ));
        } else {
            spans.push(Span::raw("  "));
        }

        // Thread number in brackets
        spans.push(Span::styled(
            format!("[{thread_number}] "),
            Style::default()
                .fg(self.theme.text())
                .add_modifier(Modifier::DIM),
        ));

        // Thread name
        let name_style = if is_selected {
            Style::default()
                .fg(self.theme.text())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };
        spans.push(Span::styled(thread.name.clone(), name_style));

        // Check if thread has in-progress work and show running indicator
        let is_running = thread
            .last_message()
            .and_then(|msg| msg.work.as_ref())
            .is_some_and(|work| {
                use merlin_core::WorkStatus;
                matches!(work.status, WorkStatus::InProgress | WorkStatus::Retrying)
            });

        if is_running {
            spans.push(Span::styled(
                " [...]",
                Style::default()
                    .fg(self.theme.warning())
                    .add_modifier(Modifier::DIM),
            ));
        }

        // Show message count with status color if work exists
        let msg_count = thread.messages.len();
        if msg_count > 0 {
            let count_text = format!(" ({msg_count})");

            // Apply status color if there's work
            let count_style = if let Some(last_msg) = thread.last_message()
                && let Some(ref work) = last_msg.work
            {
                use merlin_core::WorkStatus;
                let status_color = match work.status {
                    WorkStatus::Completed => self.theme.success(),
                    WorkStatus::Failed => self.theme.error(),
                    WorkStatus::InProgress | WorkStatus::Retrying => self.theme.warning(),
                    WorkStatus::Cancelled => self.theme.text(),
                };
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::DIM)
            } else {
                Style::default()
                    .fg(self.theme.text())
                    .add_modifier(Modifier::DIM)
            };

            spans.push(Span::styled(count_text, count_style));
        }

        Line::from(spans)
    }

    // Helper methods

    /// Calculate the number of lines that will be rendered for a task's output
    pub fn calculate_output_line_count(task: &TaskDisplay, _width: u16) -> u16 {
        task.output.lines().count() as u16
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

/// Focused pane identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    /// Text input pane on the left
    Input,
    /// Output pane displaying task tree
    Output,
    /// Tasks list pane on the right
    Tasks,
    /// Threads list pane (side-by-side mode)
    Threads,
}
