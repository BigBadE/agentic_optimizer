use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::TaskId;
use super::input::InputManager;
use super::output_tree;
use super::state::UiState;
use super::task_manager::{TaskManager, TaskStatus};
use super::theme::Theme;

/// Handles rendering of the TUI
pub struct Renderer {
    theme: Theme,
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
    pub fn render(
        &self,
        frame: &mut Frame,
        task_manager: &TaskManager,
        state: &UiState,
        input_manager: &InputManager,
        focused_pane: FocusedPane,
    ) {
        let chunks = self.create_main_layout(frame.area(), input_manager);
        let (left_chunks, right_chunks) = self.split_chunks(&chunks);

        self.render_output_area(frame, &left_chunks[0], task_manager, state, focused_pane);
        self.render_input_area(frame, &left_chunks[1], input_manager, focused_pane);
        self.render_task_list(frame, &right_chunks[0], task_manager, state, focused_pane);
        self.render_status_bar(frame, &right_chunks[1], task_manager, state);
    }

    // Layout methods

    fn create_main_layout(&self, area: Rect, input_manager: &InputManager) -> Vec<Rect> {
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let input_lines = input_manager.input_area().lines().len().max(1).min(10);
        let input_height = (input_lines + 2) as u16;

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(input_height)])
            .split(main_chunks[0]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(5), Constraint::Length(3)])
            .split(main_chunks[1]);

        vec![
            left_chunks[0],
            left_chunks[1],
            right_chunks[0],
            right_chunks[1],
        ]
    }

    fn split_chunks(&self, chunks: &[Rect]) -> ([Rect; 2], [Rect; 2]) {
        let left = [chunks[0], chunks[1]];
        let right = [chunks[2], chunks[3]];
        (left, right)
    }

    // Rendering methods

    fn render_output_area(
        &self,
        frame: &mut Frame,
        area: &Rect,
        task_manager: &TaskManager,
        state: &UiState,
        focused_pane: FocusedPane,
    ) {
        if state.loading_tasks {
            self.render_loading(frame, area);
        } else if let Some(task_id) = state.active_task_id {
            self.render_task_output(frame, area, task_manager, task_id, focused_pane);
        } else {
            self.render_welcome(frame, area);
        }
    }

    fn render_loading(&self, frame: &mut Frame, area: &Rect) {
        let loading_text = Paragraph::new("Loading tasks...")
            .style(Style::default().fg(self.theme.warning()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Output ")
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .alignment(Alignment::Center);

        frame.render_widget(loading_text, *area);
    }

    fn render_task_output(
        &self,
        frame: &mut Frame,
        area: &Rect,
        task_manager: &TaskManager,
        task_id: TaskId,
        focused_pane: FocusedPane,
    ) {
        let Some(task) = task_manager.get_task(task_id) else {
            return;
        };

        let border_color = if focused_pane == FocusedPane::Output {
            self.theme.focused_border()
        } else {
            self.theme.unfocused_border()
        };

        let tree_text = self.build_tree_text(task, area.width, focused_pane);

        let output_widget = Paragraph::new(tree_text)
            .style(Style::default().fg(self.theme.text()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Task Output ")
                    .border_style(Style::default().fg(border_color))
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            );

        frame.render_widget(output_widget, *area);
    }

    fn render_welcome(&self, frame: &mut Frame, area: &Rect) {
        let instructions = self.get_welcome_text();

        let help_text = Paragraph::new(instructions.join("\n"))
            .style(Style::default().fg(Color::DarkGray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Output ")
                    .border_style(Style::default().fg(self.theme.unfocused_border()))
                    .padding(ratatui::widgets::Padding::horizontal(1)),
            )
            .alignment(Alignment::Left);

        frame.render_widget(help_text, *area);
    }

    fn render_input_area(
        &self,
        frame: &mut Frame,
        area: &Rect,
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
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );
        input_area.set_style(Style::default().fg(self.theme.text()));
        input_area.set_cursor_style(cursor_style);

        frame.render_widget(&input_area, *area);
    }

    fn render_task_list(
        &self,
        frame: &mut Frame,
        area: &Rect,
        task_manager: &TaskManager,
        state: &UiState,
        focused_pane: FocusedPane,
    ) {
        let visible_tasks = task_manager.get_visible_tasks();
        let task_items = self.build_task_items(task_manager, state, &visible_tasks);

        let border_style = if focused_pane == FocusedPane::Tasks {
            Style::default().fg(self.theme.focused_border())
        } else {
            Style::default().fg(self.theme.unfocused_border())
        };

        let scroll_offset = self.calculate_scroll_offset(
            area,
            task_items.len(),
            state.selected_task_index,
            state.active_task_id,
        );

        let visible_items: Vec<ListItem> = task_items
            .into_iter()
            .skip(scroll_offset)
            .take(area.height.saturating_sub(2) as usize)
            .collect();

        let list = List::new(visible_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Tasks ")
                    .border_style(border_style),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(list, *area);
    }

    fn render_status_bar(
        &self,
        frame: &mut Frame,
        area: &Rect,
        task_manager: &TaskManager,
        state: &UiState,
    ) {
        let (running, failed) = self.count_tasks(task_manager, state);

        let status = Paragraph::new(format!(
            "Tasks: {running} running | {failed} failed | Theme: {}",
            self.theme.name()
        ))
        .style(Style::default().fg(self.theme.text()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("─── Status ")
                .padding(ratatui::widgets::Padding::horizontal(1)),
        );

        frame.render_widget(status, *area);
    }

    // Helper methods

    fn build_tree_text(
        &self,
        task: &super::task_manager::TaskDisplay,
        width: u16,
        focused_pane: FocusedPane,
    ) -> String {
        let visible_nodes = task.output_tree.flatten_visible_nodes();
        let selected_idx = task.output_tree.selected_index();
        let available_width = width.saturating_sub(4) as usize;

        if visible_nodes.is_empty() {
            return "No output yet...".to_string();
        }

        visible_nodes
            .iter()
            .enumerate()
            .flat_map(|(idx, (node_ref, depth))| {
                self.format_tree_node(
                    idx,
                    node_ref,
                    *depth,
                    selected_idx,
                    available_width,
                    focused_pane,
                    &task.output_tree,
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_tree_node(
        &self,
        idx: usize,
        node_ref: &output_tree::OutputNodeRef,
        depth: usize,
        selected_idx: usize,
        available_width: usize,
        focused_pane: FocusedPane,
        tree: &super::output_tree::OutputTree,
    ) -> Vec<String> {
        let is_selected = idx == selected_idx && focused_pane == FocusedPane::Output;
        let prefix = output_tree::build_tree_prefix(depth, node_ref.is_last, &node_ref.parent_states);
        let is_collapsed = tree.is_collapsed(node_ref.node);
        let icon = node_ref.node.get_icon(is_collapsed);
        let content = node_ref.node.get_content();

        let selector = if is_selected { "► " } else { "  " };
        let line_prefix = format!("{}{}{} ", selector, prefix, icon);
        let prefix_width = line_prefix.len();

        let content_width = available_width.saturating_sub(prefix_width);
        if content_width < 20 {
            return vec![format!("{}{}", line_prefix, content)];
        }

        wrap_tree_content(&line_prefix, &content, content_width, prefix_width)
    }

    fn build_task_items(
        &self,
        task_manager: &TaskManager,
        state: &UiState,
        visible_tasks: &[TaskId],
    ) -> Vec<ListItem<'_>> {
        task_manager
            .task_order()
            .iter()
            .enumerate()
            .filter_map(|(idx, task_id)| {
                if !visible_tasks.contains(task_id) {
                    return None;
                }
                task_manager.get_task(*task_id).map(|task| (idx, task_id, task))
            })
            .map(|(idx, task_id, task)| {
                self.build_task_item(idx, *task_id, task, task_manager, state)
            })
            .collect()
    }

    fn build_task_item(
        &self,
        idx: usize,
        task_id: TaskId,
        task: &super::task_manager::TaskDisplay,
        task_manager: &TaskManager,
        state: &UiState,
    ) -> ListItem<'_> {
        let (status_icon, is_failed, elapsed) =
            self.calculate_task_status(task_id, task, state);

        let tree_prefix = build_task_tree_prefix(task_id, idx, task_manager);
        let collapse_indicator = self.get_collapse_indicator(task_id, task_manager);
        let selected = self.get_selection_marker(idx, state);
        let description = self.format_task_description(task, task_id, state);

        let text = format_task_text(
            &tree_prefix,
            &selected,
            status_icon,
            &description,
            &collapse_indicator,
            is_failed,
            elapsed,
            task,
        );

        let style = self.calculate_task_style(idx, task, state, is_failed);
        ListItem::new(text).style(style)
    }

    fn calculate_task_status(
        &self,
        task_id: TaskId,
        task: &super::task_manager::TaskDisplay,
        state: &UiState,
    ) -> (&'static str, bool, f64) {
        let elapsed = task
            .end_time
            .unwrap_or_else(std::time::Instant::now)
            .duration_since(task.start_time)
            .as_secs_f64();

        let is_orphaned = task.status == TaskStatus::Running
            && !state.active_running_tasks.contains(&task_id);
        let is_stuck = task.status == TaskStatus::Running && elapsed > 300.0;
        let is_failed = is_orphaned || is_stuck;

        let status_icon = if is_failed {
            "[X]"
        } else {
            match task.status {
                TaskStatus::Running => "[>]",
                TaskStatus::Completed => "[+]",
                TaskStatus::Failed => "[X]",
            }
        };

        (status_icon, is_failed, elapsed)
    }

    fn get_collapse_indicator(&self, task_id: TaskId, task_manager: &TaskManager) -> String {
        if !task_manager.has_children(task_id) {
            return String::new();
        }

        if task_manager.is_collapsed(task_id) {
            " [+]".to_string()
        } else {
            " [-]".to_string()
        }
    }

    fn get_selection_marker(&self, idx: usize, state: &UiState) -> String {
        if state.active_task_id.is_some() && state.selected_task_index == idx {
            "► ".to_string()
        } else {
            "  ".to_string()
        }
    }

    fn format_task_description(
        &self,
        task: &super::task_manager::TaskDisplay,
        task_id: TaskId,
        state: &UiState,
    ) -> String {
        let first_line = task.description.lines().next().unwrap_or("");
        let max_desc_len = 50;

        let mut description = if first_line.len() > max_desc_len {
            format!("{}...", &first_line[..max_desc_len])
        } else {
            first_line.to_string()
        };

        if state.pending_delete_task_id == Some(task_id) {
            description.push_str(" [DELETE?]");
        }

        description
    }

    fn calculate_task_style(
        &self,
        idx: usize,
        task: &super::task_manager::TaskDisplay,
        state: &UiState,
        is_failed: bool,
    ) -> Style {
        let mut style = if is_failed {
            Style::default().fg(Color::DarkGray)
        } else {
            match task.status {
                TaskStatus::Completed => Style::default().fg(self.theme.success()),
                TaskStatus::Failed => Style::default().fg(self.theme.error()),
                TaskStatus::Running => Style::default()
                    .fg(self.theme.text())
                    .add_modifier(Modifier::BOLD),
            }
        };

        if state.active_task_id.is_some() && state.selected_task_index == idx {
            style = style.fg(self.theme.highlight()).add_modifier(Modifier::BOLD);
        }

        style
    }

    fn calculate_scroll_offset(
        &self,
        area: &Rect,
        total_items: usize,
        selected_index: usize,
        active_task_id: Option<TaskId>,
    ) -> usize {
        let list_height = area.height.saturating_sub(2) as usize;

        if total_items <= list_height {
            return 0;
        }

        if active_task_id.is_none() {
            return total_items.saturating_sub(list_height);
        }

        if selected_index < list_height / 2 {
            0
        } else if selected_index >= total_items.saturating_sub(list_height / 2) {
            total_items.saturating_sub(list_height)
        } else {
            selected_index.saturating_sub(list_height / 2)
        }
    }

    fn count_tasks(
        &self,
        task_manager: &TaskManager,
        state: &UiState,
    ) -> (usize, usize) {
        let running = task_manager
            .iter_tasks()
            .filter(|(task_id, task)| {
                task.status == TaskStatus::Running
                    && state.active_running_tasks.contains(task_id)
            })
            .count();

        let failed = task_manager
            .iter_tasks()
            .filter(|(_, task)| task.status == TaskStatus::Failed)
            .count();

        (running, failed)
    }

    fn get_welcome_text(&self) -> Vec<&'static str> {
        vec![
            "Welcome to Merlin!",
            "",
            "Getting Started:",
            "  • Type your request in the Input box below",
            "  • Press ENTER to submit",
            "  • Ctrl+N or Shift+Enter: New line (multi-line input)",
            "  • Each request creates a new task",
            "",
            "Navigation:",
            "  • TAB: Switch between Input and Output",
            "  • Ctrl+T: Focus task list",
            "  • Ctrl+P: Change theme (Palette)",
            "  • ↑/↓: Navigate tasks (when task list focused)",
            "",
            "The output pane will show the selected task's progress.",
        ]
    }
}

/// Focused pane identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Input,
    Output,
    Tasks,
}

// Helper functions

fn wrap_tree_content(
    line_prefix: &str,
    content: &str,
    content_width: usize,
    prefix_width: usize,
) -> Vec<String> {
    let wrapped = textwrap::wrap(content, content_width);
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                format!("{}{}", line_prefix, line)
            } else {
                format!("{}  {}", " ".repeat(prefix_width), line)
            }
        })
        .collect()
}

fn build_task_tree_prefix(
    task_id: TaskId,
    idx: usize,
    task_manager: &TaskManager,
) -> String {
    let Some(task) = task_manager.get_task(task_id) else {
        return String::new();
    };

    if task.parent_id.is_none() {
        return String::new();
    }

    let mut ancestors = collect_ancestors(task_id, task_manager);
    ancestors.reverse();

    let mut prefix = String::new();

    for (level, &ancestor_id) in ancestors.iter().enumerate() {
        let has_more_siblings = check_more_siblings(ancestor_id, task_manager);

        if level < ancestors.len() - 1 {
            prefix.push_str(if has_more_siblings { "│  " } else { "   " });
        }
    }

    let is_last_child = check_is_last_child(task_id, idx, task_manager);
    prefix.push_str(if is_last_child { "└─ " } else { "├─ " });

    prefix
}

fn collect_ancestors(task_id: TaskId, task_manager: &TaskManager) -> Vec<TaskId> {
    let mut ancestors = Vec::new();
    let mut current_parent = task_manager.get_task(task_id).and_then(|t| t.parent_id);

    while let Some(parent_id) = current_parent {
        ancestors.push(parent_id);
        if ancestors.len() >= 5 {
            break;
        }
        current_parent = task_manager.get_task(parent_id).and_then(|t| t.parent_id);
    }

    ancestors
}

fn check_more_siblings(ancestor_id: TaskId, task_manager: &TaskManager) -> bool {
    let Some(ancestor) = task_manager.get_task(ancestor_id) else {
        return false;
    };

    let ancestor_parent = ancestor.parent_id;
    let ancestor_pos = task_manager
        .task_order()
        .iter()
        .position(|&id| id == ancestor_id)
        .unwrap_or(0);

    task_manager
        .task_order()
        .iter()
        .skip(ancestor_pos + 1)
        .filter_map(|id| task_manager.get_task(*id))
        .any(|t| t.parent_id == ancestor_parent)
}

fn check_is_last_child(
    task_id: TaskId,
    idx: usize,
    task_manager: &TaskManager,
) -> bool {
    let Some(task) = task_manager.get_task(task_id) else {
        return false;
    };

    task_manager
        .task_order()
        .iter()
        .skip(idx + 1)
        .filter_map(|id| task_manager.get_task(*id))
        .all(|t| t.parent_id != task.parent_id)
}

fn format_task_text(
    tree_prefix: &str,
    selected: &str,
    status_icon: &str,
    description: &str,
    collapse_indicator: &str,
    is_failed: bool,
    elapsed: f64,
    task: &super::task_manager::TaskDisplay,
) -> String {
    let mut text = if is_failed
        || task.status == TaskStatus::Completed
        || task.status == TaskStatus::Failed
    {
        format!(
            "{}{}{} {}{}",
            tree_prefix, selected, status_icon, description, collapse_indicator
        )
    } else {
        format!(
            "{}{}{} {} ({:.0}s){}",
            tree_prefix, selected, status_icon, description, elapsed, collapse_indicator
        )
    };

    if let Some(progress) = &task.progress {
        let progress_indent = if task.parent_id.is_some() { "   " } else { "" };
        text.push_str(&format!(
            "\n{}   └─ {}: {}",
            progress_indent, progress.stage, progress.message
        ));

        if let Some(total) = progress.total {
            let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
            text.push_str(&format!(" [{percent}%]"));
        }
    }

    text
}
