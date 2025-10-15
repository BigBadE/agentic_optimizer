use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::warn;

use super::event_handler::EventHandler;
use super::event_source::{CrosstermEventSource, InputEventSource};
use super::events::UiEvent;
use super::input::InputManager;
use super::persistence::TaskPersistence;
use super::renderer::{FocusedPane, RenderCtx, Renderer, UiCtx};
use super::state::{ConversationEntry, ConversationRole, UiState};
use super::task_manager::TaskManager;
use super::theme::Theme;
use crate::TaskId;
use crate::{Result, RoutingError};

/// No-op event source for testing - always returns no events
#[derive(Default)]
struct NoOpEventSource;

impl InputEventSource for NoOpEventSource {
    fn poll(&mut self, _timeout: Duration) -> bool {
        false
    }

    fn read(&mut self) -> Event {
        Event::Key(KeyEvent::new(KeyCode::Null, KeyModifiers::NONE))
    }
}

/// Main TUI application
pub struct TuiApp<B: Backend> {
    /// Terminal instance used to render the UI
    terminal: Terminal<B>,
    /// Channel receiving UI events from background tasks
    event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    /// Manages tasks and their ordering/visibility
    task_manager: TaskManager,
    /// Current UI state, including selections and flags
    state: UiState,
    /// Manages the user input area and wrapping behavior
    input_manager: InputManager,
    /// Responsible for drawing UI components
    renderer: Renderer,
    /// Which pane currently has focus
    focused_pane: FocusedPane,
    /// A pending input to be consumed by the app loop
    pending_input: Option<String>,
    /// Optional task persistence handler for saving/loading tasks
    persistence: Option<TaskPersistence>,
    /// Source of input events (abstracted for testing)
    event_source: Box<dyn InputEventSource + Send>,
    /// Last time the UI was rendered (for forcing periodic updates)
    last_render_time: Instant,
}

// Note: all input is sourced from `event_source` to allow test injection without
// altering application behavior.

impl TuiApp<CrosstermBackend<io::Stdout>> {
    /// Creates a new `TuiApp` with Crossterm backend
    ///
    /// # Errors
    /// Returns an error if terminal initialization or clearing fails.
    pub fn new() -> Result<(Self, super::UiChannel)> {
        Self::new_with_storage(None)
    }

    /// Creates a new `TuiApp` with task storage
    ///
    /// # Errors
    /// Returns an error if terminal initialization or clearing fails.
    pub fn new_with_storage(
        tasks_dir: impl Into<Option<PathBuf>>,
    ) -> Result<(Self, super::UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        terminal
            .clear()
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        let tasks_dir = tasks_dir.into();
        let mut state = UiState::default();

        if tasks_dir.is_some() {
            state.loading_tasks = true;
        }

        let theme = tasks_dir
            .as_ref()
            .and_then(|dir| Theme::load(dir).ok())
            .unwrap_or_default();

        let persistence = tasks_dir
            .as_ref()
            .map(|dir| TaskPersistence::new(dir.clone()));

        let mut app = Self {
            terminal,
            event_receiver: receiver,
            task_manager: TaskManager::default(),
            state,
            input_manager: InputManager::default(),
            renderer: Renderer::new(theme),
            focused_pane: FocusedPane::Input,
            pending_input: None,
            persistence,
            event_source: Box::new(CrosstermEventSource),
            last_render_time: Instant::now(),
        };

        // Initialize scroll to show placeholder at bottom (selected by default)
        app.adjust_task_list_scroll();

        let channel = super::UiChannel { sender };

        Ok((app, channel))
    }

    /// Enables raw mode
    ///
    /// # Errors
    /// Returns an error if enabling raw mode fails.
    pub fn enable_raw_mode(&self) -> Result<()> {
        terminal::enable_raw_mode().map_err(|err| RoutingError::Other(err.to_string()))
    }

    /// Disables raw mode
    ///
    /// # Errors
    /// Returns an error if disabling raw mode or clearing the terminal fails.
    pub fn disable_raw_mode(&mut self) -> Result<()> {
        terminal::disable_raw_mode().map_err(|err| RoutingError::Other(err.to_string()))?;
        self.terminal
            .clear()
            .map_err(|err| RoutingError::Other(err.to_string()))
    }
}

impl<B: Backend> TuiApp<B> {
    /// Creates a test-friendly `TuiApp` with a generic backend (for testing only)
    ///
    /// # Errors
    /// Returns an error if the app initialization fails
    pub fn new_for_test(terminal: Terminal<B>) -> Result<(Self, super::UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let app = Self {
            terminal,
            event_receiver: receiver,
            task_manager: TaskManager::default(),
            state: UiState::default(),
            input_manager: InputManager::default(),
            renderer: Renderer::new(Theme::default()),
            focused_pane: FocusedPane::Input,
            pending_input: None,
            persistence: None,
            event_source: Box::new(NoOpEventSource),
            last_render_time: Instant::now(),
        };

        let channel = super::UiChannel { sender };

        Ok((app, channel))
    }

    /// Replaces the input event source. Useful for tests to inject a synthetic source
    /// that mirrors crossterm semantics.
    pub fn set_event_source(&mut self, source: Box<dyn InputEventSource + Send>) {
        self.event_source = source;
    }

    /// Loads tasks asynchronously
    pub async fn load_tasks_async(&mut self) {
        if let Some(persistence) = &self.persistence {
            let mut loaded_count = 0usize;
            if let Ok(tasks) = persistence.load_all_tasks().await {
                loaded_count = tasks.len();
                for (task_id, task_display) in tasks {
                    self.task_manager
                        .insert_task_for_load(task_id, task_display);
                }

                self.task_manager.rebuild_order();

                // Don't auto-select any task on load - user should manually select
            }

            tracing::info!("Loaded {} tasks from persistence", loaded_count);
            self.state.loading_tasks = false;
        }
    }

    /// Processes one tick of the event loop
    ///
    /// # Errors
    /// Returns an error if polling or rendering the terminal fails.
    pub fn tick(&mut self) -> Result<bool> {
        let had_events = self.process_ui_events();

        if had_events {
            self.render()?;
            self.last_render_time = Instant::now();
        }

        if self.event_source.poll(Duration::from_millis(50)) {
            let events = self.collect_input_events();
            let should_quit = self.process_input_events(events);
            self.render()?;
            self.last_render_time = Instant::now();
            return Ok(should_quit);
        }

        // Force periodic renders when there are active tasks with progress
        // This ensures progress bars and timers update smoothly every tick (50ms)
        let has_active_progress = self.task_manager.has_tasks_with_progress();
        let time_since_render = self.last_render_time.elapsed();
        let should_force_render =
            has_active_progress && time_since_render >= Duration::from_millis(50);

        // Unconditionally render once at the end of the tick loop to keep UI fresh.
        // The previous conditional branches were identical.
        let _ = should_force_render; // document the intent without branching
        self.render()?;
        self.last_render_time = Instant::now();

        Ok(false)
    }

    /// Takes pending input from the user
    pub fn take_pending_input(&mut self) -> Option<String> {
        self.pending_input.take()
    }

    /// Takes the task ID to continue conversation from (clears it after taking)
    pub fn take_continuing_conversation_from(&mut self) -> Option<TaskId> {
        self.state.continuing_conversation_from.take()
    }

    /// Adds an assistant response to conversation history
    pub fn add_assistant_response(&mut self, text: String) {
        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::Assistant,
            text,
            timestamp: Instant::now(),
        });
    }

    /// Gets conversation history in (role, content) format for context building
    pub fn get_conversation_history(&self) -> Vec<(String, String)> {
        // If continuing a conversation, load history from that task
        if let Some(task_id) = self.state.continuing_conversation_from {
            tracing::info!(
                "TuiApp::get_conversation_history() - continuing from task {:?}",
                task_id
            );
            return self.get_conversation_history_from_task(task_id);
        }

        // Otherwise, use current conversation history, filtering out system messages
        let history: Vec<(String, String)> = self
            .state
            .conversation_history
            .iter()
            .filter(|entry| entry.role != ConversationRole::System)
            .map(|entry| {
                let role = match entry.role {
                    ConversationRole::User => "user",
                    ConversationRole::Assistant => "assistant",
                    ConversationRole::System => "system",
                };
                (role.to_owned(), entry.text.clone())
            })
            .collect();

        tracing::info!(
            "TuiApp::get_conversation_history() returning {} messages",
            history.len()
        );
        history
    }

    /// Gets conversation history from a specific task and its ancestors
    fn get_conversation_history_from_task(&self, task_id: TaskId) -> Vec<(String, String)> {
        let mut history = Vec::new();

        // Find the root task
        let mut current_id = task_id;
        let root_id = loop {
            if let Some(task) = self.task_manager.get_task(current_id) {
                if let Some(parent_id) = task.parent_id {
                    current_id = parent_id;
                } else {
                    break current_id;
                }
            } else {
                break task_id;
            }
        };

        // Collect all tasks in the conversation chain (root and its children)
        let mut conversation_tasks = vec![root_id];
        for (id, task) in self.task_manager.iter_tasks() {
            if task.parent_id == Some(root_id) {
                conversation_tasks.push(id);
            }
        }

        // Sort by start time to maintain chronological order
        conversation_tasks.sort_by(|task_a, task_b| {
            let time_a = self
                .task_manager
                .get_task(*task_a)
                .map_or_else(Instant::now, |task| task.start_time);
            let time_b = self
                .task_manager
                .get_task(*task_b)
                .map_or_else(Instant::now, |task| task.start_time);
            time_a.cmp(&time_b)
        });

        // Extract conversation from each task's description and output
        for id in conversation_tasks {
            if let Some(task) = self.task_manager.get_task(id) {
                // Add user message (task description)
                if !task.description.is_empty()
                    && !task.description.starts_with("Saving task")
                    && !task.description.starts_with("Loading task")
                {
                    history.push(("user".to_string(), task.description.clone()));
                }

                // Add assistant response from output tree
                let output_text = task.output_tree.to_text();
                if !output_text.is_empty()
                    && !output_text.contains("Saving task")
                    && !output_text.contains("Loading task")
                {
                    history.push(("assistant".to_string(), output_text));
                }
            }
        }

        tracing::info!(
            "TuiApp::get_conversation_history_from_task() returning {} messages from task chain",
            history.len()
        );
        history
    }

    /// Gets the selected task ID
    pub fn get_selected_task_id(&self) -> Option<TaskId> {
        self.state.active_task_id
    }

    /// Gets the parent of the selected task
    pub fn get_selected_task_parent(&self) -> Option<TaskId> {
        let selected_task_id = self.get_selected_task_id()?;
        self.task_manager.get_task(selected_task_id)?.parent_id
    }

    /// Gets thread context for the selected task
    pub fn get_thread_context(&self) -> Vec<(TaskId, String, String)> {
        let parent_id = self
            .get_selected_task_parent()
            .or_else(|| self.get_selected_task_id());

        let Some(parent_id) = parent_id else {
            return Vec::default();
        };

        let mut context = Vec::default();

        if let Some(parent_task) = self.task_manager.get_task(parent_id) {
            let output = parent_task.output_tree.to_text();
            context.push((parent_id, parent_task.description.clone(), output));
        }

        for &task_id in self.task_manager.task_order() {
            if task_id == parent_id {
                continue;
            }

            if let Some(task) = self.task_manager.get_task(task_id)
                && task.parent_id == Some(parent_id)
            {
                let output = task.output_tree.to_text();
                context.push((task_id, task.description.clone(), output));
            }
        }

        context
    }

    /// Gets a reference to the terminal backend (for testing only)
    pub fn backend(&self) -> &B {
        self.terminal.backend()
    }

    /// Gets immutable access to task manager (for testing only)
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Gets mutable access to task manager for test setup (for testing only)
    pub fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Gets immutable access to UI state (for testing only)
    pub fn state(&self) -> &UiState {
        &self.state
    }

    /// Gets mutable access to UI state for test setup (for testing only)
    pub fn state_mut(&mut self) -> &mut UiState {
        &mut self.state
    }

    /// Sets the focused pane (for testing only)
    pub fn set_focused_pane(&mut self, pane: FocusedPane) {
        self.focused_pane = pane;
    }

    /// Gets the current input text (for testing only)
    pub fn get_input_text(&self) -> String {
        self.input_manager.input_area().lines().join("\n")
    }

    /// Gets the input lines (for testing only)
    pub fn get_input_lines(&self) -> Vec<String> {
        self.input_manager.input_area().lines().to_vec()
    }

    /// Returns the number of loaded tasks currently in the manager
    pub fn loaded_task_count(&self) -> usize {
        self.task_manager.task_order().len()
    }

    // Private methods

    /// Processes any pending UI events from the channel and updates state
    fn process_ui_events(&mut self) -> bool {
        let mut had_events = false;

        while let Ok(event) = self.event_receiver.try_recv() {
            let persistence = self.persistence.as_ref();
            let mut handler =
                EventHandler::new(&mut self.task_manager, &mut self.state, persistence);
            handler.handle_event(event);
            had_events = true;
        }

        had_events
    }

    /// Collects input events from the terminal (via the configured event source).
    ///
    /// # Errors
    /// Returns a vector of collected events (blocking for the first, then draining immediately available ones).
    fn collect_input_events(&mut self) -> Vec<Event> {
        let mut events = Vec::default();
        // blocking read of at least one event
        let first = self.event_source.read();
        events.push(first);

        // drain the buffer of any immediately available events
        while self.event_source.poll(Duration::from_millis(0)) {
            events.push(self.event_source.read());
        }

        events
    }

    /// Processes a batch of input events and returns true if the app should quit
    fn process_input_events(&mut self, events: Vec<Event>) -> bool {
        let mut should_quit = false;

        for event in events {
            if let Event::Key(key) = &event {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }

                if self.handle_key_event(key) {
                    should_quit = true;
                }
            }
        }

        should_quit
    }

    /// Handles a single key event and returns true if the app should quit
    fn handle_key_event(&mut self, key: &KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('q' | 'c') if key.modifiers.contains(KeyModifiers::CONTROL) => true,
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_theme();
                false
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_task_focus();
                false
            }
            KeyCode::Tab => {
                self.handle_tab();
                false
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_ctrl_n();
                false
            }
            KeyCode::Enter => self.handle_enter(key.modifiers.contains(KeyModifiers::SHIFT)),
            _ => {
                self.handle_other_key(key);
                false
            }
        }
    }

    /// Cycles to the next theme and persists it on disk if persistence is enabled
    fn cycle_theme(&mut self) {
        let new_theme = self.renderer.theme().next();
        self.renderer.set_theme(new_theme);

        if let Some(persistence) = &self.persistence {
            let dir = persistence.get_tasks_dir();
            if let Err(error) = new_theme.save(dir) {
                warn!("Failed to save theme: {}", error);
            }
        }
    }

    /// Toggles focus between the tasks pane and the input pane
    fn toggle_task_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Tasks => FocusedPane::Input,
            _ => FocusedPane::Tasks,
        };
    }

    /// Handles Tab navigation between input and output panes when a task is active
    fn handle_tab(&mut self) {
        if self.state.active_task_id.is_some() {
            self.focused_pane = match self.focused_pane {
                FocusedPane::Input => FocusedPane::Output,
                FocusedPane::Output | FocusedPane::Tasks => FocusedPane::Input,
            };
        }
    }

    /// Handles Ctrl-N to insert a manual newline in the input pane
    fn handle_ctrl_n(&mut self) {
        if self.focused_pane == FocusedPane::Input {
            self.input_manager.insert_newline_at_cursor();
            self.input_manager.record_manual_newline();
        }
    }

    /// Handles Enter key depending on pane and modifier; returns true if should quit
    fn handle_enter(&mut self, shift_pressed: bool) -> bool {
        match self.focused_pane {
            FocusedPane::Input => {
                if shift_pressed {
                    self.input_manager.insert_newline_at_cursor();
                    self.input_manager.record_manual_newline();
                    false
                } else {
                    self.submit_input()
                }
            }
            FocusedPane::Tasks => {
                // Toggle expand/collapse for the selected conversation
                if let Some(selected_id) = self.state.active_task_id {
                    let root_id = self.find_root_conversation(selected_id);
                    if self.state.expanded_conversations.contains(&root_id) {
                        self.state.expanded_conversations.remove(&root_id);
                    } else {
                        self.state.expanded_conversations.insert(root_id);
                    }
                }
                false
            }
            FocusedPane::Output => false,
        }
    }

    /// Handles any other key events depending on the focused pane
    fn handle_other_key(&mut self, key: &KeyEvent) {
        if self.focused_pane == FocusedPane::Tasks && key.code != KeyCode::Backspace {
            self.state.pending_delete_task_id = None;
        }

        match self.focused_pane {
            FocusedPane::Input => self.handle_input_key(key),
            FocusedPane::Output => self.handle_output_key(key),
            FocusedPane::Tasks => self.handle_task_key(key),
        }
    }

    /// Handles key events when the input pane is focused
    fn handle_input_key(&mut self, key: &KeyEvent) {
        let terminal_width = self.terminal.size().map(|size| size.width).unwrap_or(80);
        let input_width = (f32::from(terminal_width) * 0.7) as usize;
        let max_line_width = input_width.saturating_sub(4);

        self.input_manager
            .handle_input(&Event::Key(*key), Some(max_line_width));
    }

    /// Handles key events when the output pane is focused
    fn handle_output_key(&mut self, key: &KeyEvent) {
        let Some(task_id) = self.state.active_task_id else {
            return;
        };

        let Some(task) = self.task_manager.get_task_mut(task_id) else {
            return;
        };

        // Calculate max scroll based on task output (approximate)
        let text_lines = task
            .output_lines
            .iter()
            .map(|line| line.lines().count())
            .sum::<usize>() as u16;
        let viewport_height = 10u16; // Approximate viewport height
        let max_scroll = text_lines.saturating_sub(viewport_height);

        match key.code {
            // Arrow keys and vim-style navigation scroll the text output
            KeyCode::Up | KeyCode::Char('k') => {
                self.state.output_scroll_offset = self.state.output_scroll_offset.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let new_offset = self.state.output_scroll_offset.saturating_add(1);
                self.state.output_scroll_offset = new_offset.min(max_scroll);
            }
            // Right/Left expand/collapse output tree
            KeyCode::Right | KeyCode::Char('l') => task.output_tree.expand_selected(),
            KeyCode::Left | KeyCode::Char('h') => task.output_tree.collapse_selected(),
            // Space toggles output tree node
            KeyCode::Char(' ') => task.output_tree.toggle_selected(),
            // Home/End scroll to top/bottom of output
            KeyCode::Home => {
                self.state.output_scroll_offset = 0;
            }
            KeyCode::End => {
                self.state.output_scroll_offset = max_scroll;
            }
            // PageUp/PageDown for faster scrolling
            KeyCode::PageUp => {
                self.state.output_scroll_offset =
                    self.state.output_scroll_offset.saturating_sub(10);
            }
            KeyCode::PageDown => {
                let new_offset = self.state.output_scroll_offset.saturating_add(10);
                self.state.output_scroll_offset = new_offset.min(max_scroll);
            }
            _ => {}
        }
    }

    /// Handles key events when the tasks pane is focused
    fn handle_task_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up => self.navigate_tasks_up(),
            KeyCode::Down => self.navigate_tasks_down(),
            KeyCode::Right => {
                self.toggle_selected_task_collapse();
            }
            KeyCode::Left => {
                self.state.active_task_id = None;
                self.state.pending_delete_task_id = None;
            }
            KeyCode::Backspace => self.handle_backspace_in_tasks(),
            _ => {}
        }
    }

    /// Finds the root conversation for a given task ID
    fn find_root_conversation(&self, task_id: TaskId) -> TaskId {
        let mut current_id = task_id;
        while let Some(task) = self.task_manager.get_task(current_id) {
            if let Some(parent_id) = task.parent_id {
                current_id = parent_id;
            } else {
                return current_id;
            }
        }
        task_id
    }

    /// Handles backspace behavior in the tasks pane (two-step delete)
    fn handle_backspace_in_tasks(&mut self) {
        let Some(selected_task_id) = self.state.active_task_id else {
            return;
        };

        if self.state.pending_delete_task_id == Some(selected_task_id) {
            self.delete_task(selected_task_id);
            self.state.pending_delete_task_id = None;
        } else {
            self.state.pending_delete_task_id = Some(selected_task_id);
        }
    }

    /// Submits the current input if non-empty and returns true if it indicates quitting
    fn submit_input(&mut self) -> bool {
        let input = self.input_manager.input_area().lines()[0]
            .trim()
            .to_string();

        if input.is_empty() {
            return false;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            return true;
        }

        // If a task is selected, we're continuing that conversation
        // Clear the selection so the new message starts a fresh conversation context
        if self.state.active_task_id.is_some() {
            // Keep the conversation history from the selected task's root
            // The orchestrator will use this to continue the conversation
            self.state.continuing_conversation_from = self.state.active_task_id;
            self.state.active_task_id = None;
        }

        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::User,
            text: input.clone(),
            timestamp: Instant::now(),
        });

        // Set processing status
        self.state.processing_status = Some("[Processing...]".to_string());

        self.pending_input = Some(input);
        self.input_manager.clear();

        false
    }

    /// Toggles collapse state of the selected task
    fn toggle_selected_task_collapse(&mut self) {
        let Some(task_id) = self.state.active_task_id else {
            return;
        };

        self.task_manager.toggle_collapse(task_id);
    }

    /// Builds a flat list of all visible tasks in display order
    /// Includes root conversations and their children if expanded
    /// Returns Vec of (`task_id`, `is_child`) tuples in chronological order
    fn build_visible_task_list(&self) -> Vec<(TaskId, bool)> {
        // Get all root conversations sorted by start time (oldest first)
        let mut root_conversations: Vec<_> = self
            .task_manager
            .iter_tasks()
            .filter(|(_, task)| task.parent_id.is_none())
            .collect();
        root_conversations
            .sort_by(|(_, task_a), (_, task_b)| task_a.start_time.cmp(&task_b.start_time));

        let mut visible_tasks = Vec::new();

        for (root_id, _) in &root_conversations {
            // Add the root conversation
            visible_tasks.push((*root_id, false));

            // If expanded, add its children
            if self.state.expanded_conversations.contains(root_id) {
                let mut children: Vec<_> = self
                    .task_manager
                    .iter_tasks()
                    .filter(|(_, task)| task.parent_id == Some(*root_id))
                    .collect();
                children
                    .sort_by(|(_, task_a), (_, task_b)| task_a.start_time.cmp(&task_b.start_time));

                for (child_id, _) in children {
                    visible_tasks.push((child_id, true));
                }
            }
        }

        visible_tasks
    }

    /// Moves selection up within the visible tasks, updating active selection
    /// Navigates through both root conversations and expanded children
    /// Up moves to older tasks (up the screen)
    fn navigate_tasks_up(&mut self) {
        let visible_tasks = self.build_visible_task_list();

        if visible_tasks.is_empty() {
            // No tasks, stay on placeholder
            return;
        }

        // If nothing selected (placeholder at bottom), select the newest visible task (last in list)
        if self.state.active_task_id.is_none() {
            if let Some((last_id, _)) = visible_tasks.last() {
                self.state.active_task_id = Some(*last_id);
                self.adjust_task_list_scroll();
            }
            return;
        }

        // Find current task in the visible list
        let Some(current_id) = self.state.active_task_id else {
            return;
        };

        // Find the previous task in the visible list (older, up the screen)
        if let Some(current_pos) = visible_tasks.iter().position(|(id, _)| *id == current_id)
            && current_pos > 0
        {
            let (prev_id, _) = visible_tasks[current_pos - 1];
            self.state.active_task_id = Some(prev_id);
            self.adjust_task_list_scroll();
        }
    }

    /// Moves selection down within the visible tasks, updating active selection
    /// Navigates through both root conversations and expanded children
    /// Down moves to newer tasks (down the screen) or to placeholder
    fn navigate_tasks_down(&mut self) {
        let visible_tasks = self.build_visible_task_list();

        if visible_tasks.is_empty() {
            // No tasks, stay on placeholder
            return;
        }

        // If nothing selected (placeholder at bottom), don't move (stop at boundary)
        if self.state.active_task_id.is_none() {
            return;
        }

        // Find current task in the visible list
        let Some(current_id) = self.state.active_task_id else {
            return;
        };

        // Find the next task in the visible list (newer, down the screen)
        if let Some(current_pos) = visible_tasks.iter().position(|(id, _)| *id == current_id) {
            if current_pos + 1 < visible_tasks.len() {
                let (next_id, _) = visible_tasks[current_pos + 1];
                self.state.active_task_id = Some(next_id);
            } else {
                // At the newest visible task, move to placeholder
                self.state.active_task_id = None;
            }
            self.adjust_task_list_scroll();
        }
    }

    /// Adjusts task list scroll to keep the selected task visible
    fn adjust_task_list_scroll(&mut self) {
        // Get all visible tasks in display order (includes expanded children)
        let visible_tasks = self.build_visible_task_list();

        let total_visible = visible_tasks.len();
        let max_visible = 3; // Can show up to 3 items (tasks + placeholder)

        // If nothing selected (placeholder is selected)
        if self.state.active_task_id.is_none() {
            // Scroll to show placeholder at bottom
            // Placeholder is conceptually at index total_visible (after all visible tasks)
            let placeholder_index = total_visible;
            if placeholder_index >= max_visible {
                self.state.task_list_scroll_offset =
                    placeholder_index.saturating_sub(max_visible - 1);
            } else {
                self.state.task_list_scroll_offset = 0;
            }
            return;
        }

        // Find the selected task in the visible list
        let Some(selected_id) = self.state.active_task_id else {
            return;
        };

        // Find the index of the selected task in the visible list
        let Some(selected_index) = visible_tasks.iter().position(|(id, _)| *id == selected_id)
        else {
            return;
        };

        // Display shows oldest at top (chronological order)
        // scroll_offset = 0 means show the oldest visible tasks (start of list)
        // scroll_offset = N means skip the first N oldest visible tasks

        // Calculate which tasks are currently visible
        let visible_start = self.state.task_list_scroll_offset;
        let visible_end = (visible_start + max_visible).min(total_visible + 1); // +1 for placeholder

        // Adjust scroll if selected task is outside visible window
        if selected_index < visible_start {
            // Selected task is older than visible window start, scroll up to show it
            self.state.task_list_scroll_offset = selected_index;
        } else if selected_index >= visible_end {
            // Selected task is newer than visible window end, scroll down to show it
            self.state.task_list_scroll_offset = selected_index.saturating_sub(max_visible - 1);
        }
    }

    /// Deletes a task and updates UI state accordingly
    fn delete_task(&mut self, task_id: TaskId) {
        let was_active = self.state.active_task_id == Some(task_id);

        let to_delete = self.task_manager.remove_task(task_id);

        if let Some(persistence) = &self.persistence {
            for id in &to_delete {
                if let Err(error) = persistence.delete_task_file(*id) {
                    warn!("Failed to delete task file for {:?}: {}", id, error);
                }
            }
        }

        for id in &to_delete {
            self.state.active_running_tasks.remove(id);
        }

        if !was_active {
            return;
        }

        // After deleting active task, select the next newest conversation
        let mut root_conversations: Vec<_> = self
            .task_manager
            .iter_tasks()
            .filter(|(_, task)| task.parent_id.is_none())
            .collect();
        root_conversations
            .sort_by(|(_, task_a), (_, task_b)| task_a.start_time.cmp(&task_b.start_time));

        // Select the newest conversation (last in chronological order) if any exist
        if let Some((new_id, _)) = root_conversations.last() {
            self.state.active_task_id = Some(*new_id);
        } else {
            self.state.active_task_id = None;
        }
    }

    /// Renders the UI to the terminal
    ///
    /// # Errors
    /// Returns an error if drawing to the terminal fails.
    fn render(&mut self) -> Result<()> {
        self.terminal
            .draw(|frame| {
                let ctx = RenderCtx {
                    ui_ctx: UiCtx {
                        task_manager: &self.task_manager,
                        state: &self.state,
                    },
                    input: &self.input_manager,
                    focused: self.focused_pane,
                };
                self.renderer.render(frame, &ctx);
            })
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        Ok(())
    }
}
