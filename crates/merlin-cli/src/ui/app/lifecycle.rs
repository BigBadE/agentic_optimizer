//! Application lifecycle operations (constructors, initialization, raw mode)

use merlin_deps::crossterm::terminal;
use merlin_deps::ratatui::Terminal;
use merlin_deps::ratatui::backend::{Backend, CrosstermBackend};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

use super::tui_app::TuiApp;
use crate::config::ConfigManager;
use crate::ui::event_source::CrosstermEventSource;
use crate::ui::input::InputManager;
use crate::ui::layout;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_routing::{Result, RoutingError};

impl TuiApp<CrosstermBackend<io::Stdout>> {
    /// Creates a new `TuiApp` with task storage and orchestrator
    ///
    /// # Errors
    /// Returns an error if terminal initialization or clearing fails.
    pub async fn new_with_storage(
        tasks_dir: impl Into<Option<PathBuf>>,
        orchestrator: Option<Arc<RoutingOrchestrator>>,
        log_file: Option<fs::File>,
    ) -> Result<Self> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let (broadcast_sender, _) = broadcast::channel(100);

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

        // Initialize config manager and load theme from ~/.merlin/config.toml
        let config_manager = ConfigManager::new().await.map_err(|err| {
            RoutingError::Other(format!("Failed to create config manager: {err}"))
        })?;

        let theme = config_manager
            .get()
            .map_err(|err| RoutingError::Other(format!("Failed to read config: {err}")))?
            .theme;

        let persistence = tasks_dir
            .as_ref()
            .map(|dir| TaskPersistence::new(dir.clone()));

        let thread_storage_path = tasks_dir.as_ref().map_or_else(
            || PathBuf::from(".merlin/threads"),
            |dir| dir.join("threads"),
        );

        let store = ThreadStore::new(thread_storage_path)
            .map_err(|err| RoutingError::Other(format!("Failed to create thread store: {err}")))?;
        let thread_store = Arc::new(Mutex::new(store));

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
            event_source: Box::new(CrosstermEventSource::new()),
            last_render_time: Instant::now(),
            layout_cache: layout::LayoutCache::new(),
            thread_store,
            orchestrator,
            log_file,
            config_manager,
            last_task_receiver: None,
        };

        Ok(app)
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

// Test utilities (new_for_test) moved to app/test_util.rs, only compiled with test-util feature

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

            merlin_deps::tracing::info!("Loaded {} tasks from persistence", loaded_count);
            self.state.loading_tasks = false;
        }
    }

    /// Loads threads from disk
    ///
    /// # Errors
    /// Returns an error if thread loading fails
    pub fn load_threads(&self) -> Result<()> {
        let mut store = self
            .thread_store
            .lock()
            .map_err(|err| RoutingError::Other(format!("Thread store lock error: {err}")))?;
        let loaded_count = store.active_threads().len();
        store.load_all()?;
        let new_count = store.active_threads().len();
        drop(store);
        merlin_deps::tracing::info!(
            "Loaded {} threads from disk ({} new)",
            new_count,
            new_count.saturating_sub(loaded_count)
        );
        Ok(())
    }
}
