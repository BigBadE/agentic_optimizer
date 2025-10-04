use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
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
}

#[derive(Default)]
struct UiState {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,
    output_buffer: Vec<OutputLine>,
    scroll_offset: usize,
    input_buffer: String,
    input_mode: InputMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    Editing,
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Normal
    }
}

struct TaskDisplay {
    id: TaskId,
    description: String,
    status: TaskStatus,
    progress: Option<TaskProgress>,
    output_lines: Vec<String>,
    start_time: Instant,
    end_time: Option<Instant>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStatus {
    Running,
    Completed,
    Failed,
}

struct OutputLine {
    task_id: Option<TaskId>,
    timestamp: Instant,
    level: MessageLevel,
    text: String,
}

impl TuiApp {
    pub fn new() -> crate::Result<(Self, UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();
        
        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        
        terminal.clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        
        let app = Self {
            terminal,
            event_receiver: receiver,
            state: UiState::default(),
        };
        
        let channel = UiChannel { sender };
        
        Ok((app, channel))
    }
    
    pub async fn run(mut self) -> crate::Result<()> {
        loop {
            while let Ok(event) = self.event_receiver.try_recv() {
                self.handle_event(event);
            }
            
            self.render()
                .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
            
            if event::poll(Duration::from_millis(50))
                .map_err(|e| crate::RoutingError::Other(e.to_string()))? 
            {
                if let Event::Key(key) = event::read()
                    .map_err(|e| crate::RoutingError::Other(e.to_string()))?
                {
                    match self.state.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Char('i') => {
                                    self.state.input_mode = InputMode::Editing;
                                }
                                KeyCode::Up => self.scroll_up(),
                                KeyCode::Down => self.scroll_down(),
                                KeyCode::PageUp => self.page_up(),
                                KeyCode::PageDown => self.page_down(),
                                _ => {}
                            }
                        }
                        InputMode::Editing => {
                            match key.code {
                                KeyCode::Esc => {
                                    self.state.input_mode = InputMode::Normal;
                                }
                                KeyCode::Enter => {
                                    self.submit_input();
                                    self.state.input_mode = InputMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    self.state.input_buffer.pop();
                                }
                                KeyCode::Char(c) => {
                                    self.state.input_buffer.push(c);
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        
        self.terminal.clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TaskStarted { task_id, description, parent_id: _ } => {
                let task_display = TaskDisplay {
                    id: task_id,
                    description: description.clone(),
                    status: TaskStatus::Running,
                    progress: None,
                    output_lines: Vec::new(),
                    start_time: Instant::now(),
                    end_time: None,
                };
                
                self.state.tasks.insert(task_id, task_display);
                self.state.task_order.push(task_id);
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: format!("▶ Started: {}", description),
                });
            }
            
            UiEvent::TaskProgress { task_id, progress } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(format!("  {} - {}", progress.stage, progress.message));
                    task.progress = Some(progress.clone());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: format!("  {} - {}", progress.stage, progress.message),
                });
            }
            
            UiEvent::TaskOutput { task_id, output } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_lines.push(output.clone());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Info,
                    text: output,
                });
            }
            
            UiEvent::TaskCompleted { task_id, result } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Completed;
                    task.end_time = Some(Instant::now());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Success,
                    text: format!("✓ Completed ({}ms)", result.duration_ms),
                });
            }
            
            UiEvent::TaskFailed { task_id, error } => {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.status = TaskStatus::Failed;
                    task.end_time = Some(Instant::now());
                }
                
                self.state.output_buffer.push(OutputLine {
                    task_id: Some(task_id),
                    timestamp: Instant::now(),
                    level: MessageLevel::Error,
                    text: format!("✗ Failed: {}", error),
                });
            }
            
            UiEvent::SystemMessage { level, message } => {
                self.state.output_buffer.push(OutputLine {
                    task_id: None,
                    timestamp: Instant::now(),
                    level,
                    text: message,
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
        
        let visible_lines = 30;
        let start = self.state.scroll_offset;
        let end = (start + visible_lines).min(self.state.output_buffer.len());
        
        let output_lines: Vec<Line> = self.state.output_buffer[start..end]
            .iter()
            .map(|line| {
                let level_icon = match line.level {
                    MessageLevel::Info => "ℹ",
                    MessageLevel::Warning => "⚠",
                    MessageLevel::Error => "✗",
                    MessageLevel::Success => "✓",
                };
                
                let style = match line.level {
                    MessageLevel::Error => Style::default().fg(Color::Red),
                    MessageLevel::Warning => Style::default().fg(Color::Yellow),
                    MessageLevel::Success => Style::default().fg(Color::Green),
                    MessageLevel::Info => Style::default(),
                };
                
                Line::from(vec![
                    Span::styled(format!("{} ", level_icon), style),
                    Span::raw(&line.text),
                ])
            })
            .collect();
        
        let task_items: Vec<ListItem> = self.state.task_order
            .iter()
            .filter_map(|task_id| self.state.tasks.get(task_id))
            .map(|task| {
                let status_icon = match task.status {
                    TaskStatus::Running => "▶",
                    TaskStatus::Completed => "✓",
                    TaskStatus::Failed => "✗",
                };
                
                let elapsed = task.end_time
                    .unwrap_or_else(Instant::now)
                    .duration_since(task.start_time)
                    .as_millis();
                
                let mut text = format!(
                    "{} {} ({}ms)",
                    status_icon,
                    task.description,
                    elapsed
                );
                
                if let Some(progress) = &task.progress {
                    text.push_str(&format!("\n    └─ {}: {}", progress.stage, progress.message));
                    
                    if let Some(total) = progress.total {
                        let percent = (progress.current as f64 / total as f64 * 100.0) as u16;
                        text.push_str(&format!(" [{}%]", percent));
                    }
                }
                
                let style = match task.status {
                    TaskStatus::Completed => Style::default().fg(Color::Green),
                    TaskStatus::Failed => Style::default().fg(Color::Red),
                    TaskStatus::Running => Style::default().add_modifier(Modifier::BOLD),
                };
                
                ListItem::new(text).style(style)
            })
            .collect();
        
        let input_text = if self.state.input_mode == InputMode::Editing {
            format!("> {}_", self.state.input_buffer)
        } else {
            format!("> {} (press 'i' to edit)", self.state.input_buffer)
        };
        
        let help = match self.state.input_mode {
            InputMode::Normal => "i: Input | ↑/↓: Scroll | PgUp/PgDn: Page | q: Quit",
            InputMode::Editing => "ESC: Cancel | ENTER: Submit",
        };
        
        self.terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Percentage(30),
                    Constraint::Min(10),
                    Constraint::Length(3),
                    Constraint::Length(3),
                ])
                .split(frame.area());
            
            let header = Paragraph::new(format!(
                "Agentic Optimizer - Tasks: {} running | {} completed | {} failed",
                running, completed, failed
            ))
            .block(Block::default().borders(Borders::ALL).title("Status"));
            frame.render_widget(header, chunks[0]);
            
            let paragraph = Paragraph::new(output_lines)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Output (scroll: {}/{})", start, self.state.output_buffer.len())));
            frame.render_widget(paragraph, chunks[1]);
            
            let list = List::new(task_items)
                .block(Block::default().borders(Borders::ALL).title("Active Tasks"));
            frame.render_widget(list, chunks[2]);
            
            let input = Paragraph::new(input_text)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .title(match self.state.input_mode {
                        InputMode::Normal => "Input (Normal Mode)",
                        InputMode::Editing => "Input (Editing - ESC to cancel, ENTER to submit)",
                    }));
            frame.render_widget(input, chunks[3]);
            
            let status = Paragraph::new(help)
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(status, chunks[4]);
        })?;
        
        Ok(())
    }
    
    
    fn submit_input(&mut self) {
        if !self.state.input_buffer.is_empty() {
            self.state.output_buffer.push(OutputLine {
                task_id: None,
                timestamp: Instant::now(),
                level: MessageLevel::Info,
                text: format!("User input: {}", self.state.input_buffer),
            });
            
            self.state.input_buffer.clear();
        }
    }
    
    fn scroll_up(&mut self) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(1);
    }
    
    fn scroll_down(&mut self) {
        let max_scroll = self.state.output_buffer.len().saturating_sub(1);
        self.state.scroll_offset = (self.state.scroll_offset + 1).min(max_scroll);
    }
    
    fn page_up(&mut self) {
        self.state.scroll_offset = self.state.scroll_offset.saturating_sub(10);
    }
    
    fn page_down(&mut self) {
        let max_scroll = self.state.output_buffer.len().saturating_sub(1);
        self.state.scroll_offset = (self.state.scroll_offset + 10).min(max_scroll);
    }
}
