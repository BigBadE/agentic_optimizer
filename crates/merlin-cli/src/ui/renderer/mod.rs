//! UI rendering module
//!
//! Organized into focused sub-modules for better maintainability.

mod helpers;
mod task_rendering;
mod task_tree_builder;

use merlin_deps::ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

use std::sync::{Arc, Mutex};

use merlin_agent::ThreadStore;
use merlin_core::{Thread, ThreadId};
use merlin_deps::ratatui::text::Line;

use super::input::InputManager;
use super::layout;
use super::scroll;
use super::state::{PanelFocus, UiState};
use super::task_manager::{TaskDisplay, TaskManager};
use super::theme::Theme;

// Layout constants
const TASKS_PANE_MAX_HEIGHT_PERCENT: u16 = 60;
const MIN_REMAINING_HEIGHT: u16 = 10;
const UNFOCUSED_TASK_LIST_HEIGHT: u16 = 5;
const MIN_FOCUSED_DETAIL_HEIGHT: u16 = 10;

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
    /// Layout cache to populate with actual rendered dimensions
    pub layout_cache: &'ctx mut layout::LayoutCache,
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
    pub fn render(&self, frame: &mut Frame, ctx: &mut RenderCtx<'_>) {
        let main_area = frame.area();

        // Check if thread mode should be active
        let thread_mode = ctx.focused == FocusedPane::Threads
            || ctx.ui_ctx.state.active_thread_id.is_some()
            || ctx.ui_ctx.state.focused_panel == PanelFocus::ThreadList;

        if thread_mode {
            // Thread mode: side-by-side layout (threads | work + input)
            self.render_thread_mode(frame, main_area, ctx);
        } else {
            // Classic mode: tasks + output + input (vertical layout)
            self.render_classic_mode(frame, main_area, ctx);
        }
    }

    /// Renders the classic task-based layout
    fn render_classic_mode(&self, frame: &mut Frame, main_area: Rect, ctx: &mut RenderCtx<'_>) {
        // Calculate task content lines
        let max_task_area_height = if ctx.focused == FocusedPane::Tasks {
            let max_height = (main_area.height * TASKS_PANE_MAX_HEIGHT_PERCENT) / 100;
            max_height.min(main_area.height.saturating_sub(MIN_REMAINING_HEIGHT))
        } else if ctx.focused == FocusedPane::Output && ctx.ui_ctx.state.active_task_id.is_some() {
            UNFOCUSED_TASK_LIST_HEIGHT
        } else {
            main_area.height
        };

        let constrained_task_area = Rect {
            height: max_task_area_height,
            ..main_area
        };
        let task_content_lines =
            self.calculate_task_tree_height(&ctx.ui_ctx, constrained_task_area, ctx.focused);
        let input_content_lines = ctx.input.input_area().lines().len() as u16;

        // Use centralized layout calculations
        let task_height =
            layout::calculate_task_area_height(main_area.height, task_content_lines, ctx.focused);
        let input_height = layout::calculate_input_area_height(input_content_lines);

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
                    Constraint::Min(MIN_FOCUSED_DETAIL_HEIGHT), // Focused details gets remaining space
                    Constraint::Length(input_height),
                ])
                .split(main_area);

            self.render_task_tree_full(frame, primary_split[0], &ctx.ui_ctx, ctx.focused);

            // Cache the actual output area dimensions from ratatui's layout
            ctx.layout_cache
                .set_output_area(primary_split[1].width, primary_split[1].height);

            self.render_focused_detail_section(frame, primary_split[1], &ctx.ui_ctx, ctx.focused);
            self.render_input_area(frame, primary_split[2], ctx.input, ctx);
        }
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

    /// Calculates the height needed for the task tree content
    fn calculate_task_tree_height(
        &self,
        ui_ctx: &UiCtx<'_>,
        area: Rect,
        focused: FocusedPane,
    ) -> u16 {
        let lines = task_tree_builder::build_task_tree_lines(ui_ctx, area, focused, self.theme);
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

        let lines = task_tree_builder::build_task_tree_lines(ui_ctx, area, focused, self.theme);

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

        // Get plain text output from task
        let text = task.output.clone();

        // Calculate content height and clamp scroll offset
        // Account for borders only (2) - horizontal padding doesn't affect height
        let viewport_height = area.height.saturating_sub(2);
        let text_lines = scroll::count_text_lines(&text);
        let max_scroll = text_lines.saturating_sub(viewport_height);
        let clamped_scroll = ui_ctx.state.output_scroll_offset.min(max_scroll);

        // Build title without embedding progress (moved to input box)
        let base_title = format!("─── Focused - {} ", task.description);
        let title =
            task_rendering::truncate_text(&base_title, area.width.saturating_sub(2) as usize);

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
        use merlin_deps::ratatui::text::Span;

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
            for thread in threads {
                lines.push(self.build_thread_line(thread, selected_thread_id));
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

    /// Builds a single thread line with selection, color, name, and status
    fn build_thread_line(
        &self,
        thread: &Thread,
        selected_thread_id: Option<ThreadId>,
    ) -> Line<'static> {
        use merlin_deps::ratatui::text::Span;

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

        // Color emoji
        spans.push(Span::raw(format!("{} ", thread.color.emoji())));

        // Thread name
        let name_style = if is_selected {
            Style::default()
                .fg(self.theme.text())
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text())
        };
        spans.push(Span::styled(thread.name.clone(), name_style));

        // Show message count
        let msg_count = thread.messages.len();
        if msg_count > 0 {
            spans.push(Span::styled(
                format!(" ({msg_count})"),
                Style::default()
                    .fg(self.theme.text())
                    .add_modifier(Modifier::DIM),
            ));
        }

        // Show latest work status if any
        if let Some(last_msg) = thread.last_message()
            && let Some(ref work) = last_msg.work
        {
            spans.push(Span::raw(" "));
            spans.push(Span::raw(work.status.emoji()));
        }

        Line::from(spans)
    }

    // Helper methods

    /// Calculate the number of lines that will be rendered for a task's output
    pub fn calculate_output_line_count(task: &TaskDisplay, _width: u16) -> u16 {
        task.output.lines().count() as u16
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
    /// Threads list pane (side-by-side mode)
    Threads,
}
