use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::TaskId;
use crate::{Result, RoutingError};
use crossterm::event::{KeyEvent, KeyEventKind};
use std::time::Instant;
use super::event_handler::EventHandler;
use super::events::UiEvent;
use super::input::InputManager;
use super::persistence::TaskPersistence;
use super::renderer::{FocusedPane, Renderer, RenderCtx, UiCtx};
use super::state::{ConversationEntry, ConversationRole, UiState};
use super::task_manager::TaskManager;
use super::theme::Theme;

/// Main TUI application
pub struct TuiApp {
    /// Terminal instance used to render the UI
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
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
}

impl TuiApp {
    /// Creates a new `TuiApp`
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

        let persistence = tasks_dir.as_ref().map(|dir| TaskPersistence::new(dir.clone()));

        let app = Self {
            terminal,
            event_receiver: receiver,
            task_manager: TaskManager::new(),
            state,
            input_manager: InputManager::new(),
            renderer: Renderer::new(theme),
            focused_pane: FocusedPane::Input,
            pending_input: None,
            persistence,
        };

        let channel = super::UiChannel { sender };

        Ok((app, channel))
    }

    /// Loads tasks asynchronously
    pub async fn load_tasks_async(&mut self) {
        if let Some(persistence) = &self.persistence {
            if let Ok(tasks) = persistence.load_all_tasks().await {
                for (task_id, task_display) in tasks {
                    self.task_manager.insert_task_for_load(task_id, task_display);
                }

                self.task_manager.rebuild_order();
            }

            self.state.loading_tasks = false;
        }
    }

    /// Enables raw mode
    ///
    /// # Errors
    /// Returns an error if enabling raw mode fails.
    pub fn enable_raw_mode(&self) -> Result<()> {
        terminal::enable_raw_mode()
            .map_err(|err| RoutingError::Other(err.to_string()))
    }

    /// Disables raw mode
    ///
    /// # Errors
    /// Returns an error if disabling raw mode or clearing the terminal fails.
    pub fn disable_raw_mode(&mut self) -> Result<()> {
        terminal::disable_raw_mode()
            .map_err(|err| RoutingError::Other(err.to_string()))?;
        self.terminal
            .clear()
            .map_err(|err| RoutingError::Other(err.to_string()))
    }

    /// Processes one tick of the event loop
    ///
    /// # Errors
    /// Returns an error if polling or rendering the terminal fails.
    pub fn tick(&mut self) -> Result<bool> {
        let had_events = self.process_ui_events();

        if had_events {
            self.render()?;
        }

        if event::poll(Duration::from_millis(50))
            .map_err(|err| RoutingError::Other(err.to_string()))?
        {
            let events = Self::collect_input_events()?;
            let should_quit = self.process_input_events(events);
            self.render()?;
            return Ok(should_quit);
        }

        self.render()?;
        Ok(false)
    }

    /// Takes pending input from the user
    pub fn take_pending_input(&mut self) -> Option<String> {
        self.pending_input.take()
    }

    /// Adds an assistant response to conversation history
    pub fn add_assistant_response(&mut self, text: String) {
        self.state.conversation_history.push(ConversationEntry {
            role: ConversationRole::Assistant,
            text,
            timestamp: Instant::now(),
        });
    }

    /// Gets the selected task ID
    pub fn get_selected_task_id(&self) -> Option<TaskId> {
        if self.state.active_task_id.is_some() {
            self.task_manager
                .task_order()
                .get(self.state.selected_task_index)
                .copied()
        } else {
            None
        }
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
            return Vec::new();
        };

        let mut context = Vec::new();

        if let Some(parent_task) = self.task_manager.get_task(parent_id) {
            let output = parent_task.output_tree.to_text();
            context.push((parent_id, parent_task.description.clone(), output));
        }

        for &task_id in self.task_manager.task_order() {
            if task_id == parent_id {
                continue;
            }

            if let Some(task) = self.task_manager.get_task(task_id)
                && task.parent_id == Some(parent_id) {
                    let output = task.output_tree.to_text();
                    context.push((task_id, task.description.clone(), output));
                }
        }

        context
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

    /// Collects input events from the terminal.
    ///
    /// # Errors
    /// Returns an error if reading or polling events fails.
    fn collect_input_events() -> Result<Vec<Event>> {
        let mut events = Vec::new();
        events.push(event::read().map_err(|err| RoutingError::Other(err.to_string()))?);

        while event::poll(Duration::from_millis(0))
            .map_err(|err| RoutingError::Other(err.to_string()))?
        {
            events.push(event::read().map_err(|err| RoutingError::Other(err.to_string()))?);
        }

        Ok(events)
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
            drop(new_theme.save(dir));
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
            self.input_manager.input_area_mut().insert_newline();
            self.input_manager.record_manual_newline();
        }
    }

    /// Handles Enter key depending on pane and modifier; returns true if should quit
    fn handle_enter(&mut self, shift_pressed: bool) -> bool {
        match self.focused_pane {
            FocusedPane::Input => {
                if shift_pressed {
                    self.input_manager.input_area_mut().insert_newline();
                    self.input_manager.record_manual_newline();
                    false
                } else {
                    self.submit_input()
                }
            }
            FocusedPane::Tasks => {
                self.toggle_selected_task_collapse();
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
        let should_wrap = matches!(
            key.code,
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
        );

        self.input_manager
            .input_area_mut()
            .input(Event::Key(*key));

        if should_wrap {
            let terminal_width = self.terminal.size().map(|size| size.width).unwrap_or(80);
            let input_width = (f32::from(terminal_width) * 0.7) as usize;
            let max_line_width = input_width.saturating_sub(4);
            self.input_manager.auto_wrap(max_line_width);
        }
    }

    /// Handles key events when the output pane is focused
    fn handle_output_key(&mut self, key: &KeyEvent) {
        let Some(task_id) = self.state.active_task_id else {
            return;
        };

        let Some(task) = self.task_manager.get_task_mut(task_id) else {
            return;
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => task.output_tree.move_up(),
            KeyCode::Down | KeyCode::Char('j') => task.output_tree.move_down(),
            KeyCode::Right | KeyCode::Char('l') => task.output_tree.expand_selected(),
            KeyCode::Left | KeyCode::Char('h') => task.output_tree.collapse_selected(),
            KeyCode::Char(' ') => task.output_tree.toggle_selected(),
            KeyCode::Home => task.output_tree.move_to_start(),
            KeyCode::End => task.output_tree.move_to_end(),
            KeyCode::PageUp => task.output_tree.page_up(10),
            KeyCode::PageDown => task.output_tree.page_down(10),
            _ => {}
        }
    }

    /// Handles key events when the tasks pane is focused
    fn handle_task_key(&mut self, key: &KeyEvent) {
        match key.code {
            KeyCode::Up => {
                self.state.pending_delete_task_id = None;
                self.navigate_tasks_up();
            }
            KeyCode::Down => {
                self.state.pending_delete_task_id = None;
                self.navigate_tasks_down();
            }
            KeyCode::Left => {
                self.state.active_task_id = None;
                self.state.selected_task_index = usize::MAX;
                self.state.pending_delete_task_id = None;
            }
            KeyCode::Backspace => self.handle_backspace_in_tasks(),
            _ => {}
        }
    }

    /// Handles backspace behavior in the tasks pane (two-step delete)
    fn handle_backspace_in_tasks(&mut self) {
        let Some(&selected_task_id) = self
            .task_manager
            .task_order()
            .get(self.state.selected_task_index)
        else {
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
        let input = self.input_manager.input_area().lines()[0].trim().to_string();

        if input.is_empty() {
            return false;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            return true;
        }

        self.state.conversation_history.push(ConversationEntry {
            role: ConversationRole::User,
            text: input.clone(),
            timestamp: Instant::now(),
        });

        self.pending_input = Some(input);
        self.input_manager.clear();

        false
    }

    /// Collapses or expands the currently selected task in the tasks pane
    fn toggle_selected_task_collapse(&mut self) {
        let Some(&task_id) = self
            .task_manager
            .task_order()
            .get(self.state.selected_task_index)
        else {
            return;
        };

        self.task_manager.toggle_collapse(task_id);
    }

    /// Moves selection up within the visible tasks, updating active selection
    fn navigate_tasks_up(&mut self) {
        let visible_tasks = self.task_manager.get_visible_tasks();
        if visible_tasks.is_empty() {
            return;
        }

        if self.state.active_task_id.is_none() {
            self.select_last_visible_task(&visible_tasks);
            return;
        }

        if self.state.selected_task_index >= self.task_manager.task_order().len() {
            self.select_last_visible_task(&visible_tasks);
            return;
        }

        self.select_previous_visible_task(&visible_tasks);
    }

    /// Moves selection down within the visible tasks, updating active selection
    fn navigate_tasks_down(&mut self) {
        let visible_tasks = self.task_manager.get_visible_tasks();
        if visible_tasks.is_empty() {
            return;
        }

        if self.state.active_task_id.is_none() {
            self.select_last_visible_task(&visible_tasks);
            return;
        }

        if self.state.selected_task_index >= self.task_manager.task_order().len() {
            self.select_first_visible_task(&visible_tasks);
            return;
        }

        self.select_next_visible_task(&visible_tasks);
    }

    /// Selects the last task from the provided `visible_tasks` if available
    fn select_last_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let Some(&last_task_id) = visible_tasks.last() else { return; };
        if let Some(new_index) = self
            .task_manager
            .task_order()
            .iter()
            .position(|&id| id == last_task_id)
        {
            self.state.selected_task_index = new_index;
            self.state.active_task_id = Some(last_task_id);
        }
    }

    /// Selects the first task from the provided `visible_tasks` if available
    fn select_first_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let Some(&first_task_id) = visible_tasks.first() else { return; };
        if let Some(new_index) = self
            .task_manager
            .task_order()
            .iter()
            .position(|&id| id == first_task_id)
        {
            self.state.selected_task_index = new_index;
            self.state.active_task_id = Some(first_task_id);
        }
    }

    /// Moves selection to the previous task within the `visible_tasks` list if possible
    fn select_previous_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let current_task_id = self.task_manager.task_order()[self.state.selected_task_index];
        let Some(current_pos) = visible_tasks.iter().position(|&id| id == current_task_id) else {
            return;
        };

        if current_pos > 0 {
            let new_task_id = visible_tasks[current_pos - 1];
            if let Some(new_index) = self
                .task_manager
                .task_order()
                .iter()
                .position(|&id| id == new_task_id)
            {
                self.state.selected_task_index = new_index;
                self.state.active_task_id = Some(new_task_id);
            }
        }
    }

    /// Moves selection to the next task within the `visible_tasks` list if possible
    fn select_next_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let current_task_id = self.task_manager.task_order()[self.state.selected_task_index];
        let Some(current_pos) = visible_tasks.iter().position(|&id| id == current_task_id) else {
            return;
        };

        if current_pos < visible_tasks.len() - 1 {
            let new_task_id = visible_tasks[current_pos + 1];
            if let Some(new_index) = self
                .task_manager
                .task_order()
                .iter()
                .position(|&id| id == new_task_id)
            {
                self.state.selected_task_index = new_index;
                self.state.active_task_id = Some(new_task_id);
            }
        }
    }

    /// Deletes a task and updates UI state accordingly
    fn delete_task(&mut self, task_id: TaskId) {
        let was_active = self.state.active_task_id == Some(task_id);
        let deleted_pos = self
            .task_manager
            .task_order()
            .iter()
            .position(|&id| id == task_id);

        let to_delete = self.task_manager.remove_task(task_id);

        if let Some(persistence) = &self.persistence {
            for id in &to_delete {
                drop(persistence.delete_task_file(*id));
            }
        }

        for id in &to_delete {
            self.state.active_running_tasks.remove(id);
        }

        if was_active && !self.task_manager.is_empty() {
            self.select_task_after_deletion(deleted_pos);
        } else if to_delete.contains(&self.state.active_task_id.unwrap_or_default()) {
            self.state.active_task_id = None;
        }

        self.adjust_selected_index();
    }

    /// Selects the appropriate task after a deletion, preserving a valid selection
    fn select_task_after_deletion(&mut self, deleted_pos: Option<usize>) {
        let Some(pos) = deleted_pos else {
            return;
        };

        if pos < self.task_manager.task_order().len() {
            self.state.selected_task_index = pos;
        } else {
            self.state.selected_task_index = self.task_manager.task_order().len().saturating_sub(1);
        }

        if let Some(&new_task_id) = self
            .task_manager
            .task_order()
            .get(self.state.selected_task_index)
        {
            self.state.active_task_id = Some(new_task_id);
        } else {
            self.state.active_task_id = None;
        }
    }

    /// Adjusts the selected index to stay within bounds of the current task order
    fn adjust_selected_index(&mut self) {
        if self.state.selected_task_index >= self.task_manager.task_order().len()
            && !self.task_manager.is_empty()
        {
            self.state.selected_task_index =
                self.task_manager.task_order().len().saturating_sub(1);
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
                    ui_ctx: UiCtx { task_manager: &self.task_manager, state: &self.state },
                    input: &self.input_manager,
                    focused: self.focused_pane,
                };
                self.renderer.render(frame, &ctx);
            })
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        Ok(())
    }
}

