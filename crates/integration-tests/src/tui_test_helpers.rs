//! Test helper functions for TUI testing

use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_cli::TuiApp;
use merlin_cli::config::ConfigManager;
use merlin_cli::ui::event_source::InputEventSource;
use merlin_cli::ui::input::InputManager;
use merlin_cli::ui::layout;
use merlin_cli::ui::persistence::TaskPersistence;
use merlin_cli::ui::renderer::{FocusedPane, Renderer};
use merlin_cli::ui::state::UiState;
use merlin_cli::ui::task_manager::TaskManager;
use merlin_deps::crossterm::event::{Event as CrosstermEvent, KeyEventKind};
use merlin_deps::ratatui::Terminal;
use merlin_deps::ratatui::backend::Backend;
use merlin_routing::{Result, RoutingError, UiEvent};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

/// Creates a new `TuiApp` for testing with custom backend and event source
///
/// # Errors
/// Returns an error if terminal initialization fails.
pub async fn new_test_app<B: Backend>(
    backend: B,
    event_source: Box<dyn InputEventSource + Send>,
    tasks_dir: impl Into<Option<PathBuf>>,
    orchestrator: Option<Arc<RoutingOrchestrator>>,
) -> Result<TuiApp<B>> {
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

    // Initialize config manager for tests
    let config_manager = ConfigManager::new()
        .await
        .map_err(|err| RoutingError::Other(format!("Failed to create config manager: {err}")))?;

    let theme = config_manager
        .get()
        .map_err(|err| RoutingError::Other(format!("Failed to read config: {err}")))?
        .theme;

    let persistence = tasks_dir
        .as_ref()
        .map(|dir| TaskPersistence::new(dir.join(".merlin").join("tasks")));

    // Use orchestrator's thread store if available, otherwise create new one
    let thread_store = if let Some(ref orch) = orchestrator
        && let Some(store_arc) = orch.thread_store()
    {
        Arc::clone(&store_arc)
    } else {
        let thread_storage_path = tasks_dir.as_ref().map_or_else(
            || PathBuf::from(".merlin/threads"),
            |dir| dir.join(".merlin").join("threads"),
        );

        let store = ThreadStore::new(thread_storage_path)
            .map_err(|err| RoutingError::Other(format!("Failed to create thread store: {err}")))?;
        Arc::new(Mutex::new(store))
    };

    let app = TuiApp {
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
        config_manager,
        last_task_receiver: None,
    };

    Ok(app)
}

/// Process pending UI events for testing (non-blocking)
pub fn process_ui_events<B: Backend>(app: &mut TuiApp<B>) {
    while let Ok(ui_event) = app.event_receiver.try_recv() {
        // Broadcast to observers
        drop(app.ui_event_broadcast.send(ui_event.clone()));

        // Handle the event
        let persistence = app.persistence.as_ref();
        let mut handler = merlin_cli::ui::event_handler::EventHandler::new(
            &mut app.task_manager,
            &mut app.state,
            persistence,
        );
        handler.handle_event(ui_event);
    }
}

/// Get next input event from fixture for testing
///
/// # Errors
/// Returns error if reading event fails
pub async fn next_input_event<B: Backend>(app: &mut TuiApp<B>) -> Result<Option<CrosstermEvent>> {
    app.event_source
        .next_event()
        .await
        .map_err(|err| RoutingError::Other(err.to_string()))
}

/// Handle input event for testing
pub fn handle_input<B: Backend>(app: &mut TuiApp<B>, event: &CrosstermEvent) {
    // Process any pending UI events first
    process_ui_events(app);

    // Handle the input
    if let CrosstermEvent::Key(key) = event
        && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
    {
        app.handle_key_event(key);
    }

    // Process any UI events triggered by the input
    process_ui_events(app);
}

/// Get the last task-specific event receiver for testing
///
/// This returns the receiver created by the most recent `spawn_task_execution` call.
///
/// # Errors
/// Returns error if no task has been spawned yet
pub fn get_task_receiver<B: Backend>(app: &mut TuiApp<B>) -> Result<mpsc::Receiver<UiEvent>> {
    app.last_task_receiver.take().ok_or_else(|| {
        RoutingError::Other("No task receiver available - did you spawn a task?".to_owned())
    })
}
