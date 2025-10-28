//! Test utilities for TUI application
//!
//! This module is only compiled when the `test-util` feature is enabled.
//! It provides read-only access to internal TUI state for integration testing.

use merlin_deps::ratatui::backend::Backend;

use super::tui_app::TuiApp;
use crate::ui::event_source::InputEventSource;
use crate::ui::input::InputManager;
use crate::ui::layout;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use crate::ui::theme::Theme;
use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_deps::crossterm::event::{Event as CrosstermEvent, KeyEventKind};
use merlin_deps::ratatui::Terminal;
use merlin_routing::{Result, RoutingError, UiEvent};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

#[allow(dead_code, reason = "Temporary allow")]
impl<B: Backend> TuiApp<B> {
    /// Creates a new `TuiApp` for testing with custom backend and event source
    ///
    /// This is exposed for integration testing purposes only.
    ///
    /// # Errors
    /// Returns an error if terminal initialization fails.
    pub fn new_for_test(
        backend: B,
        event_source: Box<dyn InputEventSource + Send>,
        tasks_dir: impl Into<Option<PathBuf>>,
        orchestrator: Option<Arc<RoutingOrchestrator>>,
    ) -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (broadcast_sender, _) = broadcast::channel(100);

        let mut terminal =
            Terminal::new(backend).map_err(|err| RoutingError::Other(err.to_string()))?;

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

        let thread_storage_path = tasks_dir.as_ref().map_or_else(
            || PathBuf::from(".merlin/threads"),
            |dir| dir.join("threads"),
        );

        let thread_store = ThreadStore::new(thread_storage_path)
            .map_err(|err| RoutingError::Other(format!("Failed to create thread store: {err}")))?;

        let app = Self {
            terminal,
            event_receiver: receiver,
            event_sender: sender,
            ui_event_broadcast: broadcast_sender,
            task_manager: TaskManager::default(),
            state,
            input_manager: InputManager::default(),
            renderer: Renderer::new(theme),
            focused_pane: FocusedPane::Input,
            pending_input: None,
            persistence,
            event_source,
            last_render_time: Instant::now(),
            layout_cache: layout::LayoutCache::new(),
            thread_store,
            orchestrator,
            log_file: None,
        };

        Ok(app)
    }

    /// Get read-only access to UI state for testing
    pub fn test_state(&self) -> &UiState {
        &self.state
    }

    /// Get read-only access to task manager for testing
    pub fn test_task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Get read-only access to thread store for testing
    pub fn test_thread_store(&self) -> &ThreadStore {
        &self.thread_store
    }

    /// Get read-only access to input manager for testing
    pub fn test_input_manager(&self) -> &InputManager {
        &self.input_manager
    }

    /// Get focused pane for testing
    pub fn test_focused_pane(&self) -> FocusedPane {
        self.focused_pane
    }

    /// Subscribe to UI events for testing
    pub fn test_subscribe_ui_events(&self) -> broadcast::Receiver<UiEvent> {
        self.ui_event_broadcast.subscribe()
    }

    /// Process pending UI events for testing (non-blocking)
    ///
    /// # Errors
    /// Returns error if event processing fails
    pub fn test_process_ui_events(&mut self) -> Result<()> {
        while let Ok(ui_event) = self.event_receiver.try_recv() {
            // Broadcast to observers
            drop(self.ui_event_broadcast.send(ui_event.clone()));

            // Handle the event
            let persistence = self.persistence.as_ref();
            let mut handler = crate::ui::event_handler::EventHandler::new(
                &mut self.task_manager,
                &mut self.state,
                persistence,
            );
            handler.handle_event(ui_event);
        }
        Ok(())
    }

    /// Get next input event from fixture for testing
    ///
    /// # Errors
    /// Returns error if reading event fails
    pub async fn test_next_input_event(&mut self) -> Result<Option<CrosstermEvent>> {
        self.event_source
            .next_event()
            .await
            .map_err(|err| RoutingError::Other(err.to_string()))
    }

    /// Handle input event for testing
    ///
    /// # Errors
    /// Returns error if event processing fails
    pub fn test_handle_input(&mut self, event: &CrosstermEvent) -> Result<()> {
        // Process any pending UI events first
        self.test_process_ui_events()?;

        // Handle the input
        if let CrosstermEvent::Key(key) = event
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            self.handle_key_event(key);
        }

        // Process any UI events triggered by the input
        self.test_process_ui_events()?;

        Ok(())
    }
}
