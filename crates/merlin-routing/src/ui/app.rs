use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::mpsc;

use crate::TaskId;
use super::event_handler::EventHandler;
use super::events::UiEvent;
use super::input::InputManager;
use super::persistence::TaskPersistence;
use super::renderer::{FocusedPane, Renderer};
use super::state::{ConversationEntry, ConversationRole, UiState};
use super::task_manager::TaskManager;
use super::theme::Theme;

/// Main TUI application
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    task_manager: TaskManager,
    state: UiState,
    input_manager: InputManager,
    renderer: Renderer,
    focused_pane: FocusedPane,
    pending_input: Option<String>,
    persistence: Option<TaskPersistence>,
}

impl TuiApp {
    /// Creates a new TuiApp
    pub fn new() -> crate::Result<(Self, super::UiChannel)> {
        Self::new_with_storage(None)
    }

    /// Creates a new TuiApp with task storage
    pub fn new_with_storage(
        tasks_dir: impl Into<Option<PathBuf>>,
    ) -> crate::Result<(Self, super::UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();

        let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;

        terminal
            .clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;

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
        if let Some(ref persistence) = self.persistence {
            if let Ok(tasks) = persistence.load_all_tasks().await {
                for (task_id, task_display) in tasks {
                    self.task_manager.add_task(task_id, task_display);
                }

                self.task_manager.rebuild_order();
            }

            self.state.loading_tasks = false;
        }
    }

    /// Enables raw mode
    pub fn enable_raw_mode(&self) -> crate::Result<()> {
        crossterm::terminal::enable_raw_mode()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))
    }

    /// Disables raw mode
    pub fn disable_raw_mode(&mut self) -> crate::Result<()> {
        crossterm::terminal::disable_raw_mode()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;
        self.terminal
            .clear()
            .map_err(|e| crate::RoutingError::Other(e.to_string()))
    }

    /// Processes one tick of the event loop
    pub async fn tick(&mut self) -> crate::Result<bool> {
        let had_events = self.process_ui_events();

        if had_events {
            self.render()?;
        }

        if event::poll(Duration::from_millis(50))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?
        {
            let events = self.collect_input_events()?;
            let should_quit = self.process_input_events(events)?;
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
            timestamp: std::time::Instant::now(),
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

            if let Some(task) = self.task_manager.get_task(task_id) {
                if task.parent_id == Some(parent_id) {
                    let output = task.output_tree.to_text();
                    context.push((task_id, task.description.clone(), output));
                }
            }
        }

        context
    }

    // Private methods

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

    fn collect_input_events(&self) -> crate::Result<Vec<Event>> {
        let mut events = Vec::new();
        events.push(
            event::read().map_err(|e| crate::RoutingError::Other(e.to_string()))?,
        );

        while event::poll(Duration::from_millis(0))
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?
        {
            events.push(
                event::read().map_err(|e| crate::RoutingError::Other(e.to_string()))?,
            );
        }

        Ok(events)
    }

    fn process_input_events(&mut self, events: Vec<Event>) -> crate::Result<bool> {
        let mut should_quit = false;

        for event in events {
            if let Event::Key(key) = &event {
                if !matches!(
                    key.kind,
                    crossterm::event::KeyEventKind::Press | crossterm::event::KeyEventKind::Repeat
                ) {
                    continue;
                }

                if self.handle_key_event(key)? {
                    should_quit = true;
                }
            }
        }

        Ok(should_quit)
    }

    fn handle_key_event(&mut self, key: &crossterm::event::KeyEvent) -> crate::Result<bool> {
        match key.code {
            KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Ok(true),
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.cycle_theme();
                Ok(false)
            }
            KeyCode::Char('t') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_task_focus();
                Ok(false)
            }
            KeyCode::Tab => {
                self.handle_tab();
                Ok(false)
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_ctrl_n();
                Ok(false)
            }
            KeyCode::Enter => self.handle_enter(key.modifiers.contains(KeyModifiers::SHIFT)),
            _ => {
                self.handle_other_key(key);
                Ok(false)
            }
        }
    }

    fn cycle_theme(&mut self) {
        let new_theme = self.renderer.theme().next();
        self.renderer.set_theme(new_theme);

        if let Some(ref persistence) = self.persistence {
            let dir = persistence.get_tasks_dir();
            drop(new_theme.save(&dir));
        }
    }

    fn toggle_task_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::Tasks => FocusedPane::Input,
            _ => FocusedPane::Tasks,
        };
    }

    fn handle_tab(&mut self) {
        if self.state.active_task_id.is_some() {
            self.focused_pane = match self.focused_pane {
                FocusedPane::Input => FocusedPane::Output,
                FocusedPane::Output => FocusedPane::Input,
                FocusedPane::Tasks => FocusedPane::Input,
            };
        }
    }

    fn handle_ctrl_n(&mut self) {
        if self.focused_pane == FocusedPane::Input {
            self.input_manager.input_area_mut().insert_newline();
            self.input_manager.record_manual_newline();
        }
    }

    fn handle_enter(&mut self, shift_pressed: bool) -> crate::Result<bool> {
        match self.focused_pane {
            FocusedPane::Input => {
                if shift_pressed {
                    self.input_manager.input_area_mut().insert_newline();
                    self.input_manager.record_manual_newline();
                    Ok(false)
                } else {
                    Ok(self.submit_input())
                }
            }
            FocusedPane::Tasks => {
                self.toggle_selected_task_collapse();
                Ok(false)
            }
            _ => Ok(false),
        }
    }

    fn handle_other_key(&mut self, key: &crossterm::event::KeyEvent) {
        if self.focused_pane == FocusedPane::Tasks {
            self.state.pending_delete_task_id = None;
        }

        match self.focused_pane {
            FocusedPane::Input => self.handle_input_key(key),
            FocusedPane::Output => self.handle_output_key(key),
            FocusedPane::Tasks => self.handle_task_key(key),
        }
    }

    fn handle_input_key(&mut self, key: &crossterm::event::KeyEvent) {
        let should_wrap = matches!(
            key.code,
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
        );

        self.input_manager
            .input_area_mut()
            .input(Event::Key(*key));

        if should_wrap {
            let terminal_width = self.terminal.size().map(|s| s.width).unwrap_or(80);
            let input_width = (terminal_width as f32 * 0.7) as usize;
            let max_line_width = input_width.saturating_sub(4);
            self.input_manager.auto_wrap(max_line_width);
        }
    }

    fn handle_output_key(&mut self, key: &crossterm::event::KeyEvent) {
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

    fn handle_task_key(&mut self, key: &crossterm::event::KeyEvent) {
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
            timestamp: std::time::Instant::now(),
        });

        self.pending_input = Some(input);
        self.input_manager.clear();

        false
    }

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

    fn select_last_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let last_task_id = *visible_tasks.last().unwrap();
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

    fn select_first_visible_task(&mut self, visible_tasks: &[TaskId]) {
        let first_task_id = visible_tasks[0];
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

    fn delete_task(&mut self, task_id: TaskId) {
        let was_active = self.state.active_task_id == Some(task_id);
        let deleted_pos = self
            .task_manager
            .task_order()
            .iter()
            .position(|&id| id == task_id);

        let to_delete = self.task_manager.remove_task(task_id);

        if let Some(ref persistence) = self.persistence {
            for id in &to_delete {
                drop(persistence.delete_task_file(*id));
            }
        }

        for id in &to_delete {
            self.state.active_running_tasks.remove(id);
        }

        if was_active && !self.task_manager.is_empty() {
            self.select_task_after_deletion(deleted_pos);
        } else if to_delete.contains(&self.state.active_task_id.unwrap_or(TaskId::new())) {
            self.state.active_task_id = None;
        }

        self.adjust_selected_index();
    }

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

    fn adjust_selected_index(&mut self) {
        if self.state.selected_task_index >= self.task_manager.task_order().len()
            && !self.task_manager.is_empty()
        {
            self.state.selected_task_index =
                self.task_manager.task_order().len().saturating_sub(1);
        }
    }

    fn render(&mut self) -> crate::Result<()> {
        self.terminal
            .draw(|frame| {
                self.renderer.render(
                    frame,
                    &self.task_manager,
                    &self.state,
                    &self.input_manager,
                    self.focused_pane,
                );
            })
            .map_err(|e| crate::RoutingError::Other(e.to_string()))?;

        Ok(())
    }
}

