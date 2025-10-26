//! Application lifecycle operations (constructors, initialization, raw mode)

use crossterm::terminal;
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use std::io;
use std::path::PathBuf;
use std::time::Instant;
use tokio::sync::mpsc;

use super::tui_app::TuiApp;
use crate::ui::UiChannel;
use crate::ui::event_source::CrosstermEventSource;
use crate::ui::input::InputManager;
use crate::ui::layout;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use crate::ui::theme::Theme;
use merlin_agent::ThreadStore;
use merlin_routing::{Result, RoutingError};

impl TuiApp<CrosstermBackend<io::Stdout>> {
    /// Creates a new `TuiApp` with task storage
    ///
    /// # Errors
    /// Returns an error if terminal initialization or clearing fails.
    pub fn new_with_storage(tasks_dir: impl Into<Option<PathBuf>>) -> Result<(Self, UiChannel)> {
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
            event_source: Box::new(CrosstermEventSource),
            last_render_time: Instant::now(),
            layout_cache: layout::LayoutCache::new(),
            thread_store,
        };

        let channel = UiChannel::from_sender(sender);

        Ok((app, channel))
    }

    /// Enables raw mode
    ///
    /// # Errors
    /// Returns an error if enabling raw mode fails.
    pub fn enable_raw_mode() -> Result<()> {
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
                // Adjust scroll to show placeholder at bottom (selected by default)
                self.adjust_task_list_scroll();
            }

            tracing::info!("Loaded {} tasks from persistence", loaded_count);
            self.state.loading_tasks = false;
        }
    }
}
