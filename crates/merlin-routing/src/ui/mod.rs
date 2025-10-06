use crossterm::event::{self, Event, KeyCode};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    path::PathBuf,
    time::{Duration, Instant, SystemTime},
};
use textwrap;
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use crate::{TaskId, TaskResult};

pub mod events;
mod text_width;
mod output_tree;

pub use events::{MessageLevel, TaskProgress, UiEvent};
pub use text_width::{EmojiMode, calculate_width, strip_emojis, truncate_to_width, wrap_text};
use output_tree::{OutputTree, StepType};
/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    sender: mpsc::UnboundedSender<UiEvent>,
}

impl UiChannel {
    pub fn send(&self, event: UiEvent) {
        drop(self.sender.send(event));
    }
    
    pub fn task_started(&self, task_id: TaskId, description: String) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id: None,
        });
    }
    
    pub fn task_started_with_parent(&self, task_id: TaskId, description: String, parent_id: Option<TaskId>) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id,
        });
    }
    
    pub fn progress(&self, task_id: TaskId, stage: String, message: String) {
        self.send(UiEvent::TaskProgress {
            task_id,
            progress: TaskProgress {
                stage,
                current: 0,
                total: None,
                message,
            },
        });
    }
    
    pub fn output(&self, task_id: TaskId, output: String) {
        self.send(UiEvent::TaskOutput { task_id, output });
    }
    
    pub fn completed(&self, task_id: TaskId, result: TaskResult) {
        self.send(UiEvent::TaskCompleted { task_id, result });
    }
    
    pub fn failed(&self, task_id: TaskId, error: String) {
        self.send(UiEvent::TaskFailed { task_id, error });
    }
}

/// Main TUI application state
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    state: UiState,
    pending_input: Option<String>,
    input_area: TextArea<'static>,
    focused_pane: FocusedPane,
    tasks_dir: Option<std::path::PathBuf>,
    theme: Theme,
    manual_newlines: std::collections::HashSet<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FocusedPane {
    Input,
    Output,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Theme {
    Nord,
    Dracula,
    Gruvbox,
    TokyoNight,
    Catppuccin,
    Monochrome,
}

impl Theme {
    fn next(self) -> Self {
        match self {
            Theme::Nord => Theme::Dracula,
            Theme::Dracula => Theme::Gruvbox,
            Theme::Gruvbox => Theme::TokyoNight,
            Theme::TokyoNight => Theme::Catppuccin,
            Theme::Catppuccin => Theme::Monochrome,
            Theme::Monochrome => Theme::Nord,
        }
    }
    
    fn name(self) -> &'static str {
        match self {
            Theme::Nord => "Nord",
            Theme::Dracula => "Dracula",
            Theme::Gruvbox => "Gruvbox",
            Theme::TokyoNight => "Tokyo Night",
            Theme::Catppuccin => "Catppuccin",
            Theme::Monochrome => "Monochrome",
        }
    }
    
    fn focused_border(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(136, 192, 208),
            Theme::Dracula => Color::Rgb(189, 147, 249),
            Theme::Gruvbox => Color::Rgb(251, 184, 108),
            Theme::TokyoNight => Color::Rgb(122, 162, 247),
            Theme::Catppuccin => Color::Rgb(137, 180, 250),
            Theme::Monochrome => Color::Rgb(100, 200, 255),
        }
    }
    
    fn unfocused_border(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(216, 222, 233),
            Theme::Dracula => Color::Rgb(98, 114, 164),
            Theme::Gruvbox => Color::Rgb(168, 153, 132),
            Theme::TokyoNight => Color::Rgb(86, 95, 137),
            Theme::Catppuccin => Color::Rgb(108, 112, 134),
            Theme::Monochrome => Color::Rgb(128, 128, 128),
        }
    }
    
    fn text(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(236, 239, 244),
            Theme::Dracula => Color::Rgb(248, 248, 242),
            Theme::Gruvbox => Color::Rgb(235, 219, 178),
            Theme::TokyoNight => Color::Rgb(192, 202, 245),
            Theme::Catppuccin => Color::Rgb(205, 214, 244),
            Theme::Monochrome => Color::Rgb(255, 255, 255),
        }
    }
    
    fn success(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(163, 190, 140),
            Theme::Dracula => Color::Rgb(80, 250, 123),
            Theme::Gruvbox => Color::Rgb(184, 187, 38),
            Theme::TokyoNight => Color::Rgb(158, 206, 106),
            Theme::Catppuccin => Color::Rgb(166, 227, 161),
            Theme::Monochrome => Color::Rgb(100, 255, 100),
        }
    }
    
    fn error(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(191, 97, 106),
            Theme::Dracula => Color::Rgb(255, 85, 85),
            Theme::Gruvbox => Color::Rgb(251, 73, 52),
            Theme::TokyoNight => Color::Rgb(247, 118, 142),
            Theme::Catppuccin => Color::Rgb(243, 139, 168),
            Theme::Monochrome => Color::Rgb(255, 100, 100),
        }
    }
    
    fn warning(self) -> Color {
        match self {
            Theme::Nord => Color::Rgb(235, 203, 139),
            Theme::Dracula => Color::Rgb(241, 250, 140),
            Theme::Gruvbox => Color::Rgb(250, 189, 47),
            Theme::TokyoNight => Color::Rgb(224, 175, 104),
            Theme::Catppuccin => Color::Rgb(249, 226, 175),
            Theme::Monochrome => Color::Rgb(255, 200, 100),
        }
    }
    
    fn highlight(self) -> Color {
        self.focused_border()
    }
}

#[derive(Default)]
struct UiState {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
    conversation_history: Vec<ConversationEntry>,
    selected_task_index: usize,
    active_task_id: Option<TaskId>,
    loading_tasks: bool,
    active_running_tasks: std::collections::HashSet<TaskId>,
    collapsed_tasks: std::collections::HashSet<TaskId>,
    pending_delete_task_id: Option<TaskId>,
    #[allow(dead_code)]
    emoji_mode: EmojiMode,
}

#[derive(Clone)]
#[allow(dead_code)]
struct ConversationEntry {
    role: ConversationRole,
    text: String,
    timestamp: Instant,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConversationRole {
    User,
    Assistant,
    System,
}

struct TaskDisplay {
    description: String,
    status: TaskStatus,
    progress: Option<TaskProgress>,
    output_lines: Vec<String>,
    start_time: Instant,
    end_time: Option<Instant>,
    parent_id: Option<TaskId>,
    output_tree: OutputTree,
    steps: Vec<TaskStepInfo>,
}

#[derive(Clone)]
#[allow(dead_code)]
struct TaskStepInfo {
    step_id: String,
    step_type: String,
    content: String,
    timestamp: Instant,
}

#[derive(Serialize, Deserialize)]
struct SerializableTask {
    id: TaskId,
    description: String,
    status: String,
    output_text: String,
    start_time: SystemTime,
    end_time: Option<SystemTime>,
    parent_id: Option<TaskId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Running,
    Completed,
    Failed,
}


impl TuiApp {
    pub fn new() -> crate::Result<(Self, UiChannel)> {
        Self::new_with_storage(None)
    }
    
    pub fn new_with_storage(tasks_dir: impl Into<Option<PathBuf>>) -> crate::Result<(Self, UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        
        terminal.clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        
        let mut input_area = TextArea::default();
        input_area.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Input")
        );
        // Disable cursor line highlighting (no underline)
        input_area.set_cursor_line_style(Style::default());
        
        let tasks_dir = tasks_dir.into();
        let mut state = UiState::default();
        
        // Mark as loading if we have a tasks directory
        if tasks_dir.is_some() {
            state.loading_tasks = true;
        }
        
        // Load saved theme or use Tokyo Night as default
        let theme = if let Some(ref dir) = tasks_dir {
            Self::load_theme(dir).unwrap_or(Theme::TokyoNight)
        } else {
            Theme::TokyoNight
        };
        
        let app = Self {
            terminal,
            event_receiver: receiver,
            state,
            pending_input: None,
            input_area,
            focused_pane: FocusedPane::Input,
            tasks_dir,
            theme,
            manual_newlines: std::collections::HashSet::new(),
        };
        
        let channel = UiChannel { sender };
        
        Ok((app, channel))
    }
    
    pub async fn load_tasks_async(&mut self) {
        if let Some(ref tasks_dir) = self.tasks_dir {
            let dir = tasks_dir.clone();
            
            // Spawn async task loading
            let loaded_tasks = tokio::task::spawn_blocking(move || {
                Self::load_tasks(&dir)
            }).await;
            
            if let Ok(Ok(tasks)) = loaded_tasks {
                for (task_id, task_display) in tasks {
                    self.state.tasks.insert(task_id, task_display);
                    self.state.task_order.push(task_id);
                }
                
                // Wrap all loaded task outputs
                let task_ids: Vec<TaskId> = self.state.task_order.clone();
                for task_id in task_ids {
                    self.auto_wrap_output(task_id);
                }
                
                // Set selected index but don't activate any task initially
                if !self.state.task_order.is_empty() {
                    self.state.selected_task_index = self.state.task_order.len().saturating_sub(1);
                    // Don't set active_task_id - let user select a task manually
                }
            }
            
            self.state.loading_tasks = false;
        }
    }
    
    fn load_tasks(tasks_dir: &PathBuf) -> io::Result<HashMap<TaskId, TaskDisplay>> {
        use std::fs;
        
        let mut tasks = HashMap::new();
        
        if !tasks_dir.exists() {
            return Ok(tasks);
        }
        
        for entry in fs::read_dir(tasks_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Look for .gz compressed files
            if path.extension().and_then(|s| s.to_str()) == Some("gz") {
                if let Ok(file) = fs::File::open(&path) {
                    let mut decoder = GzDecoder::new(file);
                    let mut json_str = String::new();
                    
                    if decoder.read_to_string(&mut json_str).is_ok() {
                        if let Ok(serializable) = serde_json::from_str::<SerializableTask>(&json_str) {
                            let mut output_tree = OutputTree::new();
                            
                            // Convert saved text to tree nodes
                            for line in serializable.output_text.lines() {
                                if !line.is_empty() {
                                    output_tree.add_text(line.to_string());
                                }
                            }
                            
                            let status = match serializable.status.as_str() {
                                "Running" => TaskStatus::Running,
                                "Completed" => TaskStatus::Completed,
                                "Failed" => TaskStatus::Failed,
                                _ => TaskStatus::Running,
                            };
                            
                            let task_display = TaskDisplay {
                                description: serializable.description,
                                status,
                                progress: None,
                                output_lines: Vec::new(),
                                start_time: Instant::now(),
                                end_time: if serializable.end_time.is_some() { Some(Instant::now()) } else { None },
                                parent_id: serializable.parent_id,
                                output_tree,
                                steps: Vec::new(),
                            };
                            
                            tasks.insert(serializable.id, task_display);
                        }
                    }
                }
            }
        }
        
        Ok(tasks)
    }
    
    fn load_theme(tasks_dir: &PathBuf) -> io::Result<Theme> {
        let theme_file = tasks_dir.parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No parent directory"))?
            .join("theme.json");
        
        if !theme_file.exists() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "Theme file not found"));
        }
        
        let content = std::fs::read_to_string(theme_file)?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
    
    fn save_theme(&self) {
        if let Some(ref tasks_dir) = self.tasks_dir {
            if let Some(parent) = tasks_dir.parent() {
                let theme_file = parent.join("theme.json");
                if let Ok(json) = serde_json::to_string(&self.theme) {
                    drop(std::fs::write(theme_file, json));
                }
            }
        }
    }
    
    fn save_task(&self, task_id: TaskId, task: &TaskDisplay) {
        if let Some(ref tasks_dir) = self.tasks_dir {
            let status_str = match task.status {
                TaskStatus::Running => "Running",
                TaskStatus::Completed => "Completed",
                TaskStatus::Failed => "Failed",
            };
            
            let serializable = SerializableTask {
                id: task_id,
                description: task.description.clone(),
                status: status_str.to_string(),
                output_text: task.output_tree.to_text(),
                start_time: SystemTime::now(),
                end_time: if task.end_time.is_some() { Some(SystemTime::now()) } else { None },
                parent_id: task.parent_id,
            };
            
            // Clean filename: just the UUID part without "TaskId()" wrapper
            let task_id_str = format!("{:?}", task_id);
            let clean_id = task_id_str
                .strip_prefix("TaskId(")
                .and_then(|s| s.strip_suffix(")"))
                .unwrap_or(&task_id_str);
            
            let filename = format!("{}.json.gz", clean_id);
            let path = tasks_dir.join(filename);
            
            // Compress with gzip (fast compression)
            if let Ok(json) = serde_json::to_string(&serializable) {
                if let Ok(file) = std::fs::File::create(path) {
                    let mut encoder = GzEncoder::new(file, Compression::fast());
                    drop(encoder.write_all(json.as_bytes()));
                }
            }
        }
    }
    
    pub fn enable_raw_mode(&self) -> crate::Result<()> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))
    }
    
    pub fn disable_raw_mode(&mut self) -> crate::Result<()> {
        crossterm::terminal::disable_raw_mode()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        self.terminal.clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))
    }
    
    pub async fn tick(&mut self) -> crate::Result<bool> {
        // Process all pending UI events
        let mut had_events = false;
        while let Ok(event) = self.event_receiver.try_recv() {
            self.handle_event(event);
            had_events = true;
        }
        
        // Render if we had events
        if had_events {
            self.render()
                .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        }
        
        if event::poll(Duration::from_millis(50))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))? 
        {
            // Collect all buffered input events (for paste support)
            let mut events = Vec::new();
            events.push(event::read()
                .map_err(|e| crate::RoutingError::Other(e.to_string()))?);
            
            // Poll for more events with zero timeout
            while event::poll(Duration::from_millis(0))
                .map_err(|e| crate::RoutingError::Other(e.to_string()))? 
            {
                events.push(event::read()
                    .map_err(|e| crate::RoutingError::Other(e.to_string()))?);
            }
            
            // Process all events
            let mut should_quit = false;
            for event in events {
                if let Event::Key(key) = &event {
                    match key.kind {
                    crossterm::event::KeyEventKind::Press | crossterm::event::KeyEventKind::Repeat => {},
                    _ => continue,
                }

                match key.code {
                    KeyCode::Char('q') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => should_quit = true,
                    KeyCode::Char('c') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => should_quit = true,
                    // Ctrl+P for theme (P for Palette)
                    KeyCode::Char('p') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        self.theme = self.theme.next();
                        self.save_theme();
                    }
                    // Ctrl+T for tasks
                    KeyCode::Char('t') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        self.focused_pane = match self.focused_pane {
                            FocusedPane::Tasks => FocusedPane::Input,
                            _ => FocusedPane::Tasks,
                        };
                    }
                    KeyCode::Tab => {
                        // Only allow tabbing to output if there's an active task
                        if self.state.active_task_id.is_some() {
                            self.focused_pane = match self.focused_pane {
                                FocusedPane::Input => FocusedPane::Output,
                                FocusedPane::Output => FocusedPane::Input,
                                FocusedPane::Tasks => FocusedPane::Input,
                            };
                        }
                    }
                    KeyCode::Char('n') if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                        if self.focused_pane == FocusedPane::Input {
                            // Ctrl+N creates a newline
                            let (row, _) = self.input_area.cursor();
                            self.input_area.insert_newline();
                            self.manual_newlines.insert(row);
                        }
                    }
                    KeyCode::Enter if self.focused_pane == FocusedPane::Input => {
                        // Shift+Enter creates a newline (if terminal supports it), Enter submits
                        if key.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                            // Manually insert newline without using TextArea's input handler
                            let (row, _) = self.input_area.cursor();
                            self.input_area.insert_newline();
                            self.manual_newlines.insert(row);
                        } else {
                            // Only submit if NOT Shift
                            if self.submit_input() {
                                self.pending_input = Some(self.input_area.lines()[0].clone());
                                self.input_area = TextArea::default();
                                self.input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));
                                self.manual_newlines.clear();
                            }
                        }
                    }
                    KeyCode::Enter if self.focused_pane == FocusedPane::Tasks => {
                        // Toggle collapse state of selected task
                        if let Some(&task_id) = self.state.task_order.get(self.state.selected_task_index) {
                            if self.state.collapsed_tasks.contains(&task_id) {
                                self.state.collapsed_tasks.remove(&task_id);
                            } else {
                                self.state.collapsed_tasks.insert(task_id);
                            }
                        }
                    }
                    KeyCode::Up if self.focused_pane == FocusedPane::Tasks => {
                        self.state.pending_delete_task_id = None;
                        self.navigate_tasks_up();
                    }
                    KeyCode::Down if self.focused_pane == FocusedPane::Tasks => {
                        self.state.pending_delete_task_id = None;
                        self.navigate_tasks_down();
                    }
                    KeyCode::Left if self.focused_pane == FocusedPane::Tasks => {
                        // Deselect current task
                        self.state.active_task_id = None;
                        self.state.selected_task_index = usize::MAX;
                        // Clear any pending delete
                        self.state.pending_delete_task_id = None;
                    }
                    KeyCode::Backspace if self.focused_pane == FocusedPane::Tasks => {
                        // Two-step delete confirmation
                        if let Some(&selected_task_id) = self.state.task_order.get(self.state.selected_task_index) {
                            if self.state.pending_delete_task_id == Some(selected_task_id) {
                                // Second press - actually delete
                                self.delete_task(selected_task_id);
                                self.state.pending_delete_task_id = None;
                            } else {
                                // First press - mark for deletion
                                self.state.pending_delete_task_id = Some(selected_task_id);
                            }
                        }
                    }
                    _ => {
                        // Clear pending delete on any other key
                        if self.focused_pane == FocusedPane::Tasks {
                            self.state.pending_delete_task_id = None;
                        }
                        match self.focused_pane {
                            FocusedPane::Input => {
                                // Auto-wrap on text changes using textwrap
                                let should_wrap = matches!(key.code,
                                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                                );
                                
                                self.input_area.input(event);
                                
                                if should_wrap {
                                    self.auto_wrap_input();
                                }
                            }
                            FocusedPane::Output => {
                                // Tree navigation in output
                                if let Some(task_id) = self.state.active_task_id {
                                    if let Some(task) = self.state.tasks.get_mut(&task_id) {
                                        match key.code {
                                            KeyCode::Up | KeyCode::Char('k') => {
                                                task.output_tree.move_up();
                                            }
                                            KeyCode::Down | KeyCode::Char('j') => {
                                                task.output_tree.move_down();
                                            }
                                            KeyCode::Right | KeyCode::Char('l') => {
                                                task.output_tree.expand_selected();
                                            }
                                            KeyCode::Left | KeyCode::Char('h') => {
                                                task.output_tree.collapse_selected();
                                            }
                                            KeyCode::Char(' ') => {
                                                task.output_tree.toggle_selected();
                                            }
                                            KeyCode::Home => {
                                                task.output_tree.move_to_start();
                                            }
                                            KeyCode::End => {
                                                task.output_tree.move_to_end();
                                            }
                                            KeyCode::PageUp => {
                                                let page_size = 10;
                                                task.output_tree.page_up(page_size);
                                            }
                                            KeyCode::PageDown => {
                                                let page_size = 10;
                                                task.output_tree.page_down(page_size);
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            FocusedPane::Tasks => {}
                        }
                    }
                }
                }
            }
            
            // Render once after processing all events
            self.render()
                .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
            
            return Ok(should_quit);
        } else {
            // No keyboard events, but still render periodically for timers/progress
            self.render()
                .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        }
        
        Ok(false)
    }
    
    
    fn switch_to_selected_task(&mut self) {
        if let Some(&task_id) = self.state.task_order.get(self.state.selected_task_index) {
            self.state.active_task_id = Some(task_id);
        }
    }
    
    fn get_visible_tasks(&self) -> Vec<TaskId> {
        let mut visible = Vec::new();
        
        for &task_id in &self.state.task_order {
            if let Some(task) = self.state.tasks.get(&task_id) {
                // Check if any ancestor is collapsed
                let mut current_parent = task.parent_id;
                let mut is_hidden = false;
                
                while let Some(parent_id) = current_parent {
                    if self.state.collapsed_tasks.contains(&parent_id) {
                        is_hidden = true;
                        break;
                    }
                    current_parent = self.state.tasks.get(&parent_id).and_then(|t| t.parent_id);
                }
                
                if !is_hidden {
                    visible.push(task_id);
                }
            }
        }
        
        visible
    }
    
    fn navigate_tasks_up(&mut self) {
        let visible_tasks = self.get_visible_tasks();
        if visible_tasks.is_empty() {
            return;
        }
        
        // If nothing selected, select the last visible task
        if self.state.selected_task_index >= self.state.task_order.len() {
            let last_task_id = *visible_tasks.last().unwrap();
            if let Some(new_index) = self.state.task_order.iter().position(|&id| id == last_task_id) {
                self.state.selected_task_index = new_index;
                self.switch_to_selected_task();
            }
            return;
        }
        
        if let Some(current_pos) = visible_tasks.iter().position(|&id| {
            self.state.task_order.get(self.state.selected_task_index) == Some(&id)
        }) {
            if current_pos > 0 {
                let new_task_id = visible_tasks[current_pos - 1];
                if let Some(new_index) = self.state.task_order.iter().position(|&id| id == new_task_id) {
                    self.state.selected_task_index = new_index;
                    self.switch_to_selected_task();
                }
            }
        }
    }
    
    fn navigate_tasks_down(&mut self) {
        let visible_tasks = self.get_visible_tasks();
        if visible_tasks.is_empty() {
            return;
        }
        
        // If nothing selected, select the first visible task
        if self.state.selected_task_index >= self.state.task_order.len() {
            let first_task_id = visible_tasks[0];
            if let Some(new_index) = self.state.task_order.iter().position(|&id| id == first_task_id) {
                self.state.selected_task_index = new_index;
                self.switch_to_selected_task();
            }
            return;
        }
        
        if let Some(current_pos) = visible_tasks.iter().position(|&id| {
            self.state.task_order.get(self.state.selected_task_index) == Some(&id)
        }) {
            if current_pos < visible_tasks.len() - 1 {
                let new_task_id = visible_tasks[current_pos + 1];
                if let Some(new_index) = self.state.task_order.iter().position(|&id| id == new_task_id) {
                    self.state.selected_task_index = new_index;
                    self.switch_to_selected_task();
                }
            }
        }
    }
    
    fn build_tree_prefix(&self, task_id: TaskId, idx: usize) -> String {
        let task = match self.state.tasks.get(&task_id) {
            Some(t) => t,
            None => return String::new(),
        };
        
        if task.parent_id.is_none() {
            return String::new();
        }
        
        // Build ancestry chain (up to 5 levels)
        let mut ancestors = Vec::new();
        let mut current_parent = task.parent_id;
        
        while let Some(parent_id) = current_parent {
            ancestors.push(parent_id);
            if ancestors.len() >= 5 {
                break;
            }
            current_parent = self.state.tasks.get(&parent_id).and_then(|t| t.parent_id);
        }
        
        ancestors.reverse();
        
        // Build prefix string
        let mut prefix = String::new();
        
        // Add continuation lines for each ancestor level
        for (level, &ancestor_id) in ancestors.iter().enumerate() {
            // Check if this ancestor has more siblings after it
            let has_more_siblings = self.state.task_order
                .iter()
                .skip(self.state.task_order.iter().position(|&id| id == ancestor_id).unwrap_or(0) + 1)
                .filter_map(|id| self.state.tasks.get(id))
                .any(|t| {
                    // Check if it's a sibling (same parent as ancestor)
                    let ancestor_parent = self.state.tasks.get(&ancestor_id).and_then(|t| t.parent_id);
                    t.parent_id == ancestor_parent
                });
            
            if level < ancestors.len() - 1 {
                // Not the immediate parent, add continuation line
                if has_more_siblings {
                    prefix.push_str("│  ");
                } else {
                    prefix.push_str("   ");
                }
            }
        }
        
        // Add the branch character for this task
        let is_last_child = self.state.task_order
            .iter()
            .skip(idx + 1)
            .filter_map(|id| self.state.tasks.get(id))
            .all(|t| t.parent_id != task.parent_id);
        
        if is_last_child {
            prefix.push_str("└─ ");
        } else {
            prefix.push_str("├─ ");
        }
        
        prefix
    }
    
    
    fn find_cursor_position(&self, lines: &[String], cursor_pos: usize) -> (usize, usize) {
        if lines.is_empty() {
            return (0, 0);
        }
        
        let mut chars_seen = 0;
        for (row, line) in lines.iter().enumerate() {
            let line_len = line.len();
            
            if chars_seen + line_len >= cursor_pos {
                let col = cursor_pos - chars_seen;
                return (row, col);
            }
            
            chars_seen += line_len + 1; // +1 for space
        }
        
        let last_row = lines.len() - 1;
        let last_col = lines[last_row].len();
        (last_row, last_col)
    }
    
    fn delete_task(&mut self, task_id: TaskId) {
        // First, find and delete all children recursively
        let children: Vec<TaskId> = self.state.task_order
            .iter()
            .filter(|id| {
                self.state.tasks.get(id)
                    .and_then(|t| t.parent_id)
                    == Some(task_id)
            })
            .copied()
            .collect();
        
        // Recursively delete all children
        for child_id in children {
            self.delete_task(child_id);
        }
        
        // Now delete this task
        // Remove from tasks map
        self.state.tasks.remove(&task_id);
        
        // Remove from task order
        if let Some(pos) = self.state.task_order.iter().position(|&id| id == task_id) {
            self.state.task_order.remove(pos);
            
            // Adjust selected index if needed
            if self.state.selected_task_index >= self.state.task_order.len() {
                self.state.selected_task_index = self.state.task_order.len().saturating_sub(1);
            }
        }
        
        // Remove from active running tasks
        self.state.active_running_tasks.remove(&task_id);
        
        // Remove from collapsed tasks
        self.state.collapsed_tasks.remove(&task_id);
        
        // If this was the active task, clear it
        if self.state.active_task_id == Some(task_id) {
            self.state.active_task_id = None;
        }
        
        // Delete the saved task file
        if let Some(ref tasks_dir) = self.tasks_dir {
            let task_id_str = format!("{:?}", task_id);
            let clean_id = task_id_str
                .strip_prefix("TaskId(")
                .and_then(|s| s.strip_suffix(")"))
                .unwrap_or(&task_id_str);
            let task_file = tasks_dir.join(format!("{}.json.gz", clean_id));
            drop(std::fs::remove_file(task_file));
        }
    }
    
    fn auto_wrap_input(&mut self) {
        // Calculate available width
        let terminal_width = self.terminal.size().map(|s| s.width).unwrap_or(80);
        let input_width = (terminal_width as f32 * 0.7) as usize;
        let max_line_width = input_width.saturating_sub(4);
        
        // Get current state
        let lines = self.input_area.lines().to_vec();
        let (cursor_row, cursor_col) = self.input_area.cursor();
        
        // Don't wrap if only one line and it fits
        if lines.len() == 1 && lines[0].len() <= max_line_width {
            return;
        }
        
        // Split into paragraphs (separated by empty lines OR manual newlines)
        let mut paragraphs: Vec<Vec<String>> = Vec::new();
        let mut current_para: Vec<String> = Vec::new();
        
        for (idx, line) in lines.iter().enumerate() {
            if line.is_empty() {
                if !current_para.is_empty() {
                    paragraphs.push(current_para);
                    current_para = Vec::new();
                }
                paragraphs.push(vec![String::new()]); // Empty line as paragraph separator
            } else {
                current_para.push(line.clone());
                
                // Check if there's a manual newline after this line
                if self.manual_newlines.contains(&idx) {
                    // This line has a manual newline after it - end paragraph
                    paragraphs.push(current_para);
                    current_para = Vec::new();
                }
            }
        }
        if !current_para.is_empty() {
            paragraphs.push(current_para);
        }
        
        // Calculate cursor position within its paragraph
        let mut cursor_paragraph = 0;
        let mut pos_in_paragraph = 0;
        let mut line_count = 0;
        
        for (para_idx, para) in paragraphs.iter().enumerate() {
            let para_line_count = if para.len() == 1 && para[0].is_empty() {
                1
            } else {
                para.len()
            };
            
            // Check if cursor is in this paragraph
            if cursor_row < line_count + para_line_count {
                cursor_paragraph = para_idx;
                
                // Calculate position within this paragraph
                let line_in_para = cursor_row - line_count;
                for i in 0..line_in_para {
                    pos_in_paragraph += para[i].len();
                    if i > 0 {
                        pos_in_paragraph += 1; // Space between lines in paragraph
                    }
                }
                pos_in_paragraph += cursor_col;
                break;
            }
            
            line_count += para_line_count;
        }
        
        // Wrap each paragraph independently
        let mut new_lines: Vec<String> = Vec::new();
        let mut new_cursor_row = 0;
        let mut new_cursor_col = 0;
        let mut found_cursor = false;
        let mut new_manual_newlines = std::collections::HashSet::new();
        
        for (para_idx, para) in paragraphs.iter().enumerate() {
            if para.len() == 1 && para[0].is_empty() {
                // Preserve empty line
                new_lines.push(String::new());
                if para_idx < cursor_paragraph {
                    new_cursor_row += 1;
                }
            } else {
                // Wrap this paragraph
                // Join lines with spaces - they're either from previous wrapping or separate content
                let para_text = para.join(" ");
                
                // Only wrap if needed - preserve single short lines as-is
                let wrapped = if para.len() == 1 && para[0].len() < max_line_width {
                    vec![para[0].clone()]
                } else {
                    // Use textwrap for proper word wrapping
                    let ends_with_space = para_text.ends_with(' ');
                    let options = textwrap::Options::new(max_line_width)
                        .break_words(true)
                        .word_separator(textwrap::WordSeparator::AsciiSpace);
                    
                    let mut wrapped_lines: Vec<String> = textwrap::wrap(&para_text, options)
                        .into_iter()
                        .map(|cow| cow.into_owned())
                        .collect();
                    
                    if ends_with_space && !wrapped_lines.is_empty() {
                        if let Some(last) = wrapped_lines.last_mut() {
                            last.push(' ');
                        }
                    }
                    
                    wrapped_lines
                };
                
                // Find cursor in this paragraph
                if para_idx == cursor_paragraph && !found_cursor {
                    let (row_offset, col) = self.find_cursor_position(&wrapped, pos_in_paragraph);
                    new_cursor_row += row_offset;
                    new_cursor_col = col;
                    found_cursor = true;
                } else if para_idx < cursor_paragraph {
                    new_cursor_row += wrapped.len();
                }
                
                let _para_start_line = new_lines.len();
                new_lines.extend(wrapped);
                
                // Mark the last line of this paragraph as having a manual newline
                // (unless it's the last paragraph)
                if para_idx < paragraphs.len() - 1 {
                    new_manual_newlines.insert(new_lines.len() - 1);
                }
            }
        }
        
        // Update manual newlines set
        self.manual_newlines = new_manual_newlines;
        
        // If nothing changed, don't update
        if new_lines == lines {
            return;
        }
        
        // Create a completely new TextArea with the wrapped content
        let mut new_input = TextArea::new(new_lines);
        
        // Copy all styling from the old textarea
        if let Some(block) = self.input_area.block() {
            new_input.set_block(block.clone());
        }
        new_input.set_style(self.input_area.style());
        new_input.set_cursor_style(self.input_area.cursor_style());
        new_input.set_cursor_line_style(self.input_area.cursor_line_style());
        
        // Set cursor position
        new_input.move_cursor(tui_textarea::CursorMove::Jump(new_cursor_row as u16, new_cursor_col as u16));
        
        // Replace the textarea
        self.input_area = new_input;
    }
    
    fn auto_wrap_output(&mut self, _task_id: TaskId) {
        // No longer needed - output tree doesn't need wrapping
        // Tree rendering handles layout automatically
    }
    
    
    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TaskStarted { task_id, description, parent_id } => {
                let mut output_tree = OutputTree::new();
                
                // Add initial status message
                output_tree.add_text(format!("[i] Task started: {}", description));
                output_tree.add_text("[...] Processing...".to_string());
                
                let task_display = TaskDisplay {
                    description: description.clone(),
                    status: TaskStatus::Running,
                    progress: None,
                    output_lines: Vec::new(),
                    start_time: Instant::now(),
                    end_time: None,
                    parent_id,
                    output_tree,
                    steps: Vec::new(),
                };
                
                self.state.tasks.insert(task_id, task_display);
                self.state.task_order.push(task_id);
                self.state.active_running_tasks.insert(task_id); // Mark as actively running
                
                // If this task has a parent, ensure the parent is not collapsed
                if let Some(parent_id) = parent_id {
                    self.state.collapsed_tasks.remove(&parent_id);
                }
                
                // Auto-select this task if it's the first one or if we don't have an active task
                if self.state.task_order.len() == 1 || self.state.active_task_id.is_none() {
                    self.state.selected_task_index = self.state.task_order.len().saturating_sub(1);
                    self.state.active_task_id = Some(task_id);
                }
            }
            
            UiEvent::TaskProgress { task_id, progress } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.progress = Some(progress.clone());
                }
            }
            
            UiEvent::TaskOutput { task_id, output } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(output.clone());
                    task.output_tree.add_text(output);
                }
            }
            
            UiEvent::TaskCompleted { task_id, result } => {
                self.state.active_running_tasks.remove(&task_id); // No longer running
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Completed;
                    task.end_time = Some(Instant::now());
                    // Don't append text - already shown via TaskStepStarted(Output)
                }
                
                if let Some(task) = self.state.tasks.get(&task_id) {
                    self.save_task(task_id, task);
                }
                
                self.state.conversation_history.push(ConversationEntry {
                    role: ConversationRole::Assistant,
                    text: result.response.text,
                    timestamp: Instant::now(),
                });
            }
            
            UiEvent::TaskFailed { task_id, error } => {
                self.state.active_running_tasks.remove(&task_id); // No longer running
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;
                    task.end_time = Some(Instant::now());
                    
                    let error_msg = format!("Error: {error}");
                    task.output_tree.add_text(error_msg);
                }
                
                if let Some(task) = self.state.tasks.get(&task_id) {
                    self.save_task(task_id, task);
                }
            }
            
            UiEvent::SystemMessage { level, message } => {
                let prefix = match level {
                    MessageLevel::Info => "[i]",
                    MessageLevel::Warning => "[!]",
                    MessageLevel::Error => "[X]",
                    MessageLevel::Success => "[+]",
                };
                
                // Send to active task only (this is a global message)
                if let Some(task_id) = self.state.active_task_id {
                    if let Some(task) = self.state.tasks.get_mut(&task_id) {
                        task.output_tree.add_text(format!("{prefix} {message}"));
                    }
                }
                
                self.state.conversation_history.push(ConversationEntry {
                    role: ConversationRole::System,
                    text: message,
                    timestamp: Instant::now(),
                });
            }
            
            // Streaming events - Phase 2 implementation
            UiEvent::TaskStepStarted { task_id, step_id, step_type, content } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    // Store step info
                    task.steps.push(TaskStepInfo {
                        step_id: step_id.clone(),
                        step_type: step_type.clone(),
                        content: content.clone(),
                        timestamp: Instant::now(),
                    });
                    
                    // Add to output tree
                    let step_type_enum = StepType::from_str(&step_type);
                    task.output_tree.add_step(step_id, step_type_enum, content);
                }
            }
            
            UiEvent::TaskStepCompleted { task_id, step_id } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.complete_step(&step_id);
                }
            }
            
            UiEvent::ToolCallStarted { task_id, tool, args } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.add_tool_call(tool, args);
                }
            }
            
            UiEvent::ToolCallCompleted { task_id, tool, result } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.complete_tool_call(&tool, result);
                }
            }
            
            UiEvent::ThinkingUpdate { task_id: _, content: _ } => {
                // Deprecated: ThinkingUpdate is redundant with TaskStepStarted(Thinking)
                // No output insertion - already handled by TaskStepStarted
            }
            
            UiEvent::SubtaskSpawned { parent_id: _, child_id: _, description: _ } => {
                // TODO: Phase 5 - Handle hierarchical tasks
            }
        }
    }
    
    fn render(&mut self) -> io::Result<()> {
        // Count only actively running tasks (not orphaned/stuck)
        let running = self.state.tasks.iter()
            .filter(|(task_id, task)| {
                task.status == TaskStatus::Running && self.state.active_running_tasks.contains(task_id)
            })
            .count();
        let failed = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        
        // Build ancestry map for tree rendering
        let visible_tasks = self.get_visible_tasks();
        
        let task_items: Vec<ListItem> = self.state.task_order
            .iter()
            .enumerate()
            .filter_map(|(idx, task_id)| {
                // Only show visible tasks
                if !visible_tasks.contains(task_id) {
                    return None;
                }
                self.state.tasks.get(task_id).map(|task| (idx, task_id, task))
            })
            .map(|(idx, task_id, task)| {
                // Check if task is stuck (running for > 5 minutes) or orphaned (not in active set)
                let elapsed = task.end_time
                    .unwrap_or_else(Instant::now)
                    .duration_since(task.start_time)
                    .as_secs_f64();
                
                let is_orphaned = task.status == TaskStatus::Running && !self.state.active_running_tasks.contains(task_id);
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
                
                // Build tree structure prefix with proper nesting
                let tree_prefix = self.build_tree_prefix(*task_id, idx);
                
                // Add collapse indicator if task has children
                let has_children = self.state.task_order.iter()
                    .any(|id| self.state.tasks.get(id).and_then(|t| t.parent_id) == Some(*task_id));
                let collapse_indicator = if has_children {
                    if self.state.collapsed_tasks.contains(task_id) {
                        " [+]"
                    } else {
                        " [-]"
                    }
                } else {
                    ""
                };
                
                let selected = if self.state.selected_task_index < self.state.task_order.len() && idx == self.state.selected_task_index { "► " } else { "" };
                
                // Show only first line of description, truncate if too long
                let first_line = task.description.lines().next().unwrap_or("");
                let max_desc_len = 50;
                let mut description = if first_line.len() > max_desc_len {
                    format!("{}...", &first_line[..max_desc_len])
                } else {
                    first_line.to_string()
                };
                
                // Add delete confirmation prompt if this task is pending deletion
                if self.state.pending_delete_task_id == Some(*task_id) {
                    description.push_str(" [DELETE?]");
                }
                
                let mut text = if is_failed || task.status == TaskStatus::Completed || task.status == TaskStatus::Failed {
                    // No timer for failed/orphaned/completed tasks
                    format!(
                        "{}{}{} {}{}",
                        tree_prefix,
                        selected,
                        status_icon,
                        description,
                        collapse_indicator
                    )
                } else {
                    format!(
                        "{}{}{} {} ({:.0}s){}",
                        tree_prefix,
                        selected,
                        status_icon,
                        description,
                        elapsed,
                        collapse_indicator
                    )
                };
                
                if let Some(progress) = &task.progress {
                    let progress_indent = if task.parent_id.is_some() { "   " } else { "" };
                    text.push_str(&format!("\n{}   └─ {}: {}", progress_indent, progress.stage, progress.message));
                    
                    if let Some(total) = progress.total {
                        let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
                        text.push_str(&format!(" [{percent}%]"));
                    }
                }
                
                let mut style = if is_failed {
                    // Grey out failed/stuck/orphaned tasks
                    Style::default().fg(Color::DarkGray)
                } else {
                    match task.status {
                        TaskStatus::Completed => Style::default().fg(self.theme.success()),
                        TaskStatus::Failed => Style::default().fg(self.theme.error()),
                        TaskStatus::Running => Style::default().fg(self.theme.text()).add_modifier(Modifier::BOLD),
                    }
                };
                
                // Always highlight selected task, brighter when Tasks pane is focused
                if self.state.selected_task_index < self.state.task_order.len() && idx == self.state.selected_task_index {
                    if self.focused_pane == FocusedPane::Tasks {
                        style = style.fg(self.theme.highlight()).add_modifier(Modifier::BOLD);
                    } else {
                        style = style.fg(self.theme.highlight());
                    }
                }
                
                ListItem::new(text).style(style)
            })
            .collect();
        
        if self.focused_pane == FocusedPane::Input {
            self.input_area.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Input ")
                    .border_style(Style::default().fg(self.theme.focused_border()))
                    .padding(ratatui::widgets::Padding::horizontal(1))
            );
            self.input_area.set_style(Style::default().fg(self.theme.text()));
            self.input_area.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            self.input_area.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("─── Input ")
                    .border_style(Style::default().fg(self.theme.unfocused_border()))
                    .padding(ratatui::widgets::Padding::horizontal(1))
            );
            self.input_area.set_style(Style::default().fg(self.theme.text()));
            self.input_area.set_cursor_style(Style::default());
        }
        
        self.terminal.draw(|frame| {
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(70),
                    Constraint::Percentage(30),
                ])
                .split(frame.area());
            
            // Calculate input height based on number of lines (max 10)
            let input_lines = self.input_area.lines().len().max(1).min(10);
            let input_height = (input_lines + 2) as u16; // +2 for borders
            
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),
                    Constraint::Length(input_height),
                ])
                .split(main_chunks[0]);
            
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),
                    Constraint::Length(3),
                ])
                .split(main_chunks[1]);
            
            if self.state.loading_tasks {
                // Show loading indicator
                let loading_text = Paragraph::new("Loading tasks...")
                    .style(Style::default().fg(self.theme.warning()))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title("─── Output ")
                        .padding(ratatui::widgets::Padding::horizontal(1)))
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(loading_text, left_chunks[0]);
            } else if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get(&task_id) {
                    // Render output tree
                    let border_color = if self.focused_pane == FocusedPane::Output {
                        self.theme.focused_border()
                    } else {
                        self.theme.unfocused_border()
                    };
                    
                    let visible_nodes = task.output_tree.flatten_visible_nodes();
                    let selected_idx = task.output_tree.selected_index();
                    
                    let tree_items: Vec<String> = visible_nodes.iter()
                        .enumerate()
                        .map(|(idx, (node_ref, depth))| {
                            let is_selected = idx == selected_idx && self.focused_pane == FocusedPane::Output;
                            let prefix = output_tree::build_tree_prefix(*depth, node_ref.is_last, &node_ref.parent_states);
                            let is_collapsed = task.output_tree.is_collapsed(node_ref.node);
                            let icon = node_ref.node.get_icon(is_collapsed);
                            let content = node_ref.node.get_content();
                            
                            let selector = if is_selected { "► " } else { "  " };
                            format!("{}{}{} {}", selector, prefix, icon, content)
                        })
                        .collect();
                    
                    let tree_text = if tree_items.is_empty() {
                        "No output yet...".to_string()
                    } else {
                        tree_items.join("\n")
                    };
                    
                    let output_widget = Paragraph::new(tree_text)
                        .style(Style::default().fg(self.theme.text()))
                        .block(Block::default()
                            .borders(Borders::ALL)
                            .title("─── Task Output ")
                            .border_style(Style::default().fg(border_color))
                            .padding(ratatui::widgets::Padding::horizontal(1)));
                    
                    frame.render_widget(output_widget, left_chunks[0]);
                }
            } else {
                // Show instructions when no task is active
                let instructions = vec![
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
                ];
                
                let help_text = Paragraph::new(instructions.join("\n"))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .title("─── Output ")
                        .border_style(Style::default().fg(self.theme.unfocused_border()))
                        .padding(ratatui::widgets::Padding::horizontal(1)))
                    .alignment(ratatui::layout::Alignment::Left);
                frame.render_widget(help_text, left_chunks[0]);
            }
            frame.render_widget(&self.input_area, left_chunks[1]);
            
            let list_title = "─── Tasks ";
            let list_border_style = if self.focused_pane == FocusedPane::Tasks {
                Style::default().fg(self.theme.focused_border())
            } else {
                Style::default().fg(self.theme.unfocused_border())
            };
            
            // Calculate scroll offset to keep selected item visible
            let list_height = right_chunks[0].height.saturating_sub(2) as usize; // Subtract borders
            let total_items = task_items.len();
            
            let scroll_offset = if total_items > list_height {
                if self.state.selected_task_index < list_height / 2 {
                    0
                } else if self.state.selected_task_index >= total_items.saturating_sub(list_height / 2) {
                    total_items.saturating_sub(list_height)
                } else {
                    self.state.selected_task_index.saturating_sub(list_height / 2)
                }
            } else {
                0
            };
            
            // Take only visible items
            let visible_items: Vec<ListItem> = task_items
                .into_iter()
                .skip(scroll_offset)
                .take(list_height)
                .collect();
            
            let list = List::new(visible_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(list_title)
                    .border_style(list_border_style))
                .style(Style::default().fg(Color::White));
            frame.render_widget(list, right_chunks[0]);
            
            let status = Paragraph::new(format!(
                "Tasks: {running} running | {failed} failed | Theme: {}",
                self.theme.name()
            ))
            .style(Style::default().fg(self.theme.text()))
            .block(Block::default()
                .borders(Borders::ALL)
                .title("─── Status ")
                .padding(ratatui::widgets::Padding::horizontal(1)));
            frame.render_widget(status, right_chunks[1]);
        })?;
        
        Ok(())
    }
    
    
    fn submit_input(&mut self) -> bool {
        let input = self.input_area.lines()[0].trim().to_string();
        
        if !input.is_empty() {
            if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
                return true;
            }
            
            self.state.conversation_history.push(ConversationEntry {
                role: ConversationRole::User,
                text: input.clone(),
                timestamp: Instant::now(),
            });
            
            self.pending_input = Some(input);
            self.input_area = TextArea::default();
            self.input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));
            self.input_area.set_cursor_line_style(Style::default());
            self.input_area.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        }
        false
    }
    
    pub fn take_pending_input(&mut self) -> Option<String> {
        self.pending_input.take()
    }
    
    pub fn add_assistant_response(&mut self, text: String) {
        self.state.conversation_history.push(ConversationEntry {
            role: ConversationRole::Assistant,
            text: text.clone(),
            timestamp: Instant::now(),
        });
    }
    
    pub fn get_selected_task_id(&self) -> Option<TaskId> {
        self.state.task_order.get(self.state.selected_task_index).copied()
    }
    
    pub fn get_selected_task_parent(&self) -> Option<TaskId> {
        let selected_task_id = self.get_selected_task_id()?;
        self.state.tasks.get(&selected_task_id)?.parent_id
    }
    
    pub fn get_thread_context(&self) -> Vec<(TaskId, String, String)> {
        let mut context = Vec::new();
        
        // Get the parent task ID (either selected task's parent or selected task itself)
        let parent_id = self.get_selected_task_parent()
            .or_else(|| self.get_selected_task_id());
        
        if let Some(parent_id) = parent_id {
            // Add parent task
            if let Some(parent_task) = self.state.tasks.get(&parent_id) {
                let output = parent_task.output_tree.to_text();
                context.push((parent_id, parent_task.description.clone(), output));
            }
            
            // Add all sibling tasks (children of the same parent)
            for &task_id in &self.state.task_order {
                if task_id == parent_id {
                    continue;
                }
                
                if let Some(task) = self.state.tasks.get(&task_id) {
                    if task.parent_id == Some(parent_id) {
                        let output = task.output_tree.to_text();
                        context.push((task_id, task.description.clone(), output));
                    }
                }
            }
        }
        
        context
    }
    
}
