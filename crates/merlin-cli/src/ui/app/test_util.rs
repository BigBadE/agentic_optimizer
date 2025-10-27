//! Test utilities for TUI application
//!
//! This module is only compiled when the `test-util` feature is enabled.
//! It provides read-only access to internal TUI state for integration testing.

use ratatui::backend::Backend;

use super::tui_app::TuiApp;
use crate::ui::event_source::InputEventSource;
use crate::ui::input::InputManager;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use crate::ui::theme::Theme;
use crate::ui::{UiChannel, layout};
use merlin_agent::ThreadStore;
use merlin_routing::{Result, RoutingError};
use ratatui::Terminal;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

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
    ) -> Result<(Self, UiChannel)> {
        let (sender, receiver) = mpsc::unbounded_channel();

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
        };

        let channel = UiChannel::from_sender(sender);

        Ok((app, channel))
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
}
