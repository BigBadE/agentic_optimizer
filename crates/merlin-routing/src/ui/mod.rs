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
use tokio::sync::mpsc;
use tui_textarea::TextArea;
use crate::{TaskId, TaskResult};

pub mod events;

pub use events::{MessageLevel, TaskProgress, UiEvent};

/// UI update channel - REQUIRED for all task execution
#[derive(Clone)]
pub struct UiChannel {
    sender: mpsc::UnboundedSender<UiEvent>,
}

impl UiChannel {
    pub fn send(&self, event: UiEvent) {
        let _ = self.sender.send(event);
    }
    
    pub fn task_started(&self, task_id: TaskId, description: String) {
        self.send(UiEvent::TaskStarted {
            task_id,
            description,
            parent_id: None,
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
    output_area: TextArea<'static>,
    focused_pane: FocusedPane,
    tasks_dir: Option<std::path::PathBuf>,
    theme: Theme,
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
}

#[derive(Clone)]
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
    output_area: TextArea<'static>,
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
                .title(" Input ")
                .padding(ratatui::widgets::Padding::horizontal(1))
        );
        input_area.set_cursor_line_style(Style::default());
        input_area.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        input_area.set_tab_length(4);
        
        let mut output_area = TextArea::default();
        output_area.set_block(Block::default().borders(Borders::ALL).title("Output (Read-Only)"));
        
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
            output_area,
            focused_pane: FocusedPane::Input,
            tasks_dir,
            theme,
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
                if let Some(&first_task) = self.state.task_order.first() {
                    self.state.active_task_id = Some(first_task);
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
                            let mut output_area = TextArea::default();
                            output_area.set_block(Block::default().borders(Borders::ALL).title("Task Output"));
                            output_area.insert_str(&serializable.output_text);
                            
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
                                output_area,
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
                output_text: task.output_area.lines().join("\n"),
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
        while let Ok(event) = self.event_receiver.try_recv() {
            self.handle_event(event);
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
                        self.focused_pane = match self.focused_pane {
                            FocusedPane::Input => FocusedPane::Output,
                            FocusedPane::Output => FocusedPane::Input,
                            FocusedPane::Tasks => FocusedPane::Input,
                        };
                    }
                    KeyCode::Enter if self.focused_pane == FocusedPane::Input => {
                        if self.submit_input() {
                            return Ok(true);
                        }
                    }
                    KeyCode::Up if self.focused_pane == FocusedPane::Tasks => {
                        if self.state.selected_task_index > 0 {
                            self.state.selected_task_index -= 1;
                            self.switch_to_selected_task();
                        }
                    }
                    KeyCode::Down if self.focused_pane == FocusedPane::Tasks => {
                        if self.state.selected_task_index < self.state.task_order.len().saturating_sub(1) {
                            self.state.selected_task_index += 1;
                            self.switch_to_selected_task();
                        }
                    }
                    _ => {
                        match self.focused_pane {
                            FocusedPane::Input => {
                                // Only auto-wrap on text changes, not cursor movement
                                let should_wrap = matches!(key.code,
                                    KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
                                );
                                
                                self.input_area.input(event);
                                
                                if should_wrap {
                                    self.auto_wrap_input();
                                }
                            }
                            FocusedPane::Output => {
                                // Output is read-only, only allow scrolling
                                if let Some(task_id) = self.state.active_task_id {
                                    if let Some(task) = self.state.tasks.get_mut(&task_id) {
                                        match key.code {
                                            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right |
                                            KeyCode::Home | KeyCode::End | KeyCode::PageUp | KeyCode::PageDown => {
                                                task.output_area.input(event);
                                            }
                                            _ => {} // Ignore all other keys (read-only)
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
        }
        
        Ok(false)
    }
    
    
    fn switch_to_selected_task(&mut self) {
        if let Some(&task_id) = self.state.task_order.get(self.state.selected_task_index) {
            self.state.active_task_id = Some(task_id);
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
        
        // Get the full text by joining lines (they're all part of the same input)
        // Since we only have one logical paragraph, join with spaces
        let full_text = lines.join(" ");
        
        // Calculate cursor position in the full text
        let mut cursor_pos = 0;
        for (i, line) in lines.iter().enumerate() {
            if i < cursor_row {
                cursor_pos += line.len() + 1; // +1 for the space we added in join
            } else if i == cursor_row {
                cursor_pos += cursor_col;
                break;
            }
        }
        
        // Re-wrap everything from scratch
        let new_lines = self.wrap_text(&full_text, max_line_width);
        
        // If nothing changed, don't update
        if new_lines == lines {
            return;
        }
        
        // Find new cursor position
        let (new_row, new_col) = self.find_cursor_position(&new_lines, cursor_pos);
        
        // Clear all existing content
        self.input_area.move_cursor(tui_textarea::CursorMove::Head);
        for _ in 0..lines.len() {
            self.input_area.move_cursor(tui_textarea::CursorMove::End);
            self.input_area.delete_line_by_end();
            if self.input_area.cursor().0 > 0 {
                self.input_area.move_cursor(tui_textarea::CursorMove::Up);
                self.input_area.move_cursor(tui_textarea::CursorMove::End);
                self.input_area.delete_char();
            }
        }
        
        // Insert all new content
        self.input_area.move_cursor(tui_textarea::CursorMove::Head);
        self.input_area.delete_line_by_end();
        for (i, line) in new_lines.iter().enumerate() {
            if i > 0 {
                self.input_area.insert_newline();
            }
            self.input_area.insert_str(line);
        }
        
        // Restore cursor position
        self.input_area.move_cursor(tui_textarea::CursorMove::Head);
        for _ in 0..new_row {
            self.input_area.move_cursor(tui_textarea::CursorMove::Down);
        }
        for _ in 0..new_col {
            self.input_area.move_cursor(tui_textarea::CursorMove::Forward);
        }
    }
    
    fn wrap_text(&self, text: &str, max_width: usize) -> Vec<String> {
        if text.is_empty() {
            return vec![String::new()];
        }
        
        let mut lines = Vec::new();
        let mut current_line = String::new();
        let mut chars_iter = text.chars().peekable();
        let mut current_word = String::new();
        
        while let Some(ch) = chars_iter.next() {
            if ch.is_whitespace() {
                // Flush current word
                if !current_word.is_empty() {
                    if current_line.is_empty() {
                        current_line = current_word.clone();
                    } else if current_line.len() + 1 + current_word.len() <= max_width {
                        current_line.push(' ');
                        current_line.push_str(&current_word);
                    } else {
                        lines.push(current_line);
                        current_line = current_word.clone();
                    }
                    current_word.clear();
                }
                
                // Add space if we're building a line and next char isn't whitespace
                if !current_line.is_empty() && chars_iter.peek().map(|c| !c.is_whitespace()).unwrap_or(false) {
                    // Space will be added when next word is appended
                }
            } else {
                current_word.push(ch);
            }
        }
        
        // Flush final word
        if !current_word.is_empty() {
            if current_line.is_empty() {
                current_line = current_word;
            } else if current_line.len() + 1 + current_word.len() <= max_width {
                current_line.push(' ');
                current_line.push_str(&current_word);
            } else {
                lines.push(current_line);
                current_line = current_word;
            }
        }
        
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        
        if lines.is_empty() {
            lines.push(String::new());
        }
        
        lines
    }
    
    fn find_cursor_position(&self, lines: &[String], cursor_pos: usize) -> (usize, usize) {
        let mut pos = 0;
        for (row, line) in lines.iter().enumerate() {
            // Check if cursor is within this line
            if cursor_pos <= pos + line.len() {
                let col = cursor_pos.saturating_sub(pos).min(line.len());
                return (row, col);
            }
            pos += line.len() + 1; // +1 for space between lines
        }
        
        // Cursor at end
        if let Some(last_line) = lines.last() {
            (lines.len().saturating_sub(1), last_line.len())
        } else {
            (0, 0)
        }
    }
    
    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TaskStarted { task_id, description, parent_id } => {
                let mut output_area = TextArea::default();
                output_area.set_block(Block::default().borders(Borders::ALL).title("Task Output"));
                output_area.insert_str(format!("▶ Started: {description}"));
                
                let task_display = TaskDisplay {
                    description: description.clone(),
                    status: TaskStatus::Running,
                    progress: None,
                    output_lines: Vec::new(),
                    start_time: Instant::now(),
                    end_time: None,
                    parent_id,
                    output_area,
                };
                
                self.state.tasks.insert(task_id, task_display);
                self.state.task_order.push(task_id);
                
                if self.state.active_task_id.is_none() {
                    self.state.active_task_id = Some(task_id);
                }
                
                if let Some(task) = self.state.tasks.get(&task_id) {
                    self.save_task(task_id, task);
                }
            }
            
            UiEvent::TaskProgress { task_id, progress } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(format!("  {} - {}", progress.stage, progress.message));
                    task.output_area.insert_str(format!("\n  {} - {}", progress.stage, progress.message));
                    task.progress = Some(progress.clone());
                }
            }
            
            UiEvent::TaskOutput { task_id, output } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(output.clone());
                    task.output_area.insert_str(format!("\n{output}"));
                }
            }
            
            UiEvent::TaskCompleted { task_id, result } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Completed;
                    task.end_time = Some(Instant::now());
                    
                    let duration_secs = result.duration_ms as f64 / 1000.0;
                    task.output_area.insert_str(format!("\nMerlin: {}", result.response.text));
                    task.output_area.insert_str(format!("\n✓ Completed ({:.2}s)", duration_secs));
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
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;
                    task.end_time = Some(Instant::now());
                    task.output_area.insert_str(format!("\n✗ Failed: {error}"));
                }
                
                if let Some(task) = self.state.tasks.get(&task_id) {
                    self.save_task(task_id, task);
                }
            }
            
            UiEvent::SystemMessage { level, message } => {
                let prefix = match level {
                    MessageLevel::Info => "ℹ",
                    MessageLevel::Warning => "⚠",
                    MessageLevel::Error => "✗",
                    MessageLevel::Success => "✓",
                };
                
                if let Some(task_id) = self.state.active_task_id {
                    if let Some(task) = self.state.tasks.get_mut(&task_id) {
                        task.output_area.insert_str(format!("\n{prefix} {message}"));
                    }
                }
                
                self.state.conversation_history.push(ConversationEntry {
                    role: ConversationRole::System,
                    text: message,
                    timestamp: Instant::now(),
                });
            }
        }
    }
    
    fn render(&mut self) -> io::Result<()> {
        let running = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Running)
            .count();
        let completed = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();
        let failed = self.state.tasks.values()
            .filter(|t| t.status == TaskStatus::Failed)
            .count();
        
        let task_items: Vec<ListItem> = self.state.task_order
            .iter()
            .enumerate()
            .filter_map(|(idx, task_id)| {
                self.state.tasks.get(task_id).map(|task| (idx, task_id, task))
            })
            .map(|(idx, task_id, task)| {
                let status_icon = match task.status {
                    TaskStatus::Running => "▶",
                    TaskStatus::Completed => "✓",
                    TaskStatus::Failed => "✗",
                };
                
                let elapsed = task.end_time
                    .unwrap_or_else(Instant::now)
                    .duration_since(task.start_time)
                    .as_secs_f64();
                
                let indent = if task.parent_id.is_some() { "  " } else { "" };
                let selected = if idx == self.state.selected_task_index { "► " } else { "" };
                
                let mut text = format!(
                    "{}{}{} {} ({:.0}s)",
                    indent,
                    selected,
                    status_icon,
                    task.description,
                    elapsed
                );
                
                if let Some(progress) = &task.progress {
                    text.push_str(&format!("\n    └─ {}: {}", progress.stage, progress.message));
                    
                    if let Some(total) = progress.total {
                        let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
                        text.push_str(&format!(" [{percent}%]"));
                    }
                }
                
                let mut style = match task.status {
                    TaskStatus::Completed => Style::default().fg(self.theme.success()),
                    TaskStatus::Failed => Style::default().fg(self.theme.error()),
                    TaskStatus::Running => Style::default().fg(self.theme.text()).add_modifier(Modifier::BOLD),
                };
                
                if idx == self.state.selected_task_index && self.focused_pane == FocusedPane::Tasks {
                    style = style.fg(self.theme.highlight()).add_modifier(Modifier::BOLD);
                }
                
                ListItem::new(text).style(style)
            })
            .collect();
        
        if let Some(task_id) = self.state.active_task_id {
            if let Some(task) = self.state.tasks.get_mut(&task_id) {
                if self.focused_pane == FocusedPane::Output {
                    task.output_area.set_block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("─── Task Output: {} ", task.description))
                            .border_style(Style::default().fg(self.theme.focused_border()))
                            .padding(ratatui::widgets::Padding::horizontal(1))
                    );
                    task.output_area.set_style(Style::default().fg(self.theme.text()));
                    task.output_area.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
                } else {
                    task.output_area.set_block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("─── Task Output: {} ", task.description))
                            .border_style(Style::default().fg(self.theme.unfocused_border()))
                            .padding(ratatui::widgets::Padding::horizontal(1))
                    );
                    task.output_area.set_style(Style::default().fg(self.theme.text()));
                    task.output_area.set_cursor_style(Style::default());
                }
            }
        }
        
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
                        .title("─── Task Output ")
                        .padding(ratatui::widgets::Padding::horizontal(1)))
                    .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(loading_text, left_chunks[0]);
            } else if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get(&task_id) {
                    frame.render_widget(&task.output_area, left_chunks[0]);
                }
            } else {
                // Show instructions when no task is active
                let instructions = vec![
                    "Welcome to Merlin!",
                    "",
                    "Getting Started:",
                    "  • Type your request in the Input box below",
                    "  • Press ENTER to submit",
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
                        .title("─── Task Output ")
                        .border_style(Style::default().fg(self.theme.unfocused_border()))
                        .padding(ratatui::widgets::Padding::horizontal(1)))
                    .alignment(ratatui::layout::Alignment::Left);
                frame.render_widget(help_text, left_chunks[0]);
            }
            frame.render_widget(&self.input_area, left_chunks[1]);
            
            let list_title = "─── Active Tasks ";
            let list_border_style = if self.focused_pane == FocusedPane::Tasks {
                Style::default().fg(self.theme.focused_border())
            } else {
                Style::default().fg(self.theme.unfocused_border())
            };
            
            let list = List::new(task_items)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(list_title)
                    .border_style(list_border_style))
                .style(Style::default().fg(Color::White));
            frame.render_widget(list, right_chunks[0]);
            
            let status = Paragraph::new(format!(
                "Tasks: {running} running | {completed} completed | {failed} failed | Theme: {}",
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
            
            self.output_area.insert_str(format!("\nYou: {input}"));
            self.output_area.move_cursor(tui_textarea::CursorMove::End);
            
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
        
        self.output_area.insert_str(format!("\n✓ Merlin: {text}"));
    }
    
}
