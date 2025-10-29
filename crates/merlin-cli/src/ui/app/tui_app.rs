//! Main TUI application struct and core state management

use merlin_deps::ratatui::Terminal;
use merlin_deps::ratatui::backend::Backend;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

use crate::config::ConfigManager;
use crate::ui::event_source::InputEventSource;
use crate::ui::input::InputManager;
use crate::ui::layout;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_routing::UiEvent;

/// Main TUI application
pub struct TuiApp<B: Backend> {
    /// Terminal instance used to render the UI
    #[cfg_attr(
        feature = "test-util",
        allow(dead_code, reason = "Exposed for test inspection")
    )]
    pub(crate) terminal: Terminal<B>,
    /// Channel receiving UI events from background tasks
    pub(crate) event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    /// Channel for sending UI events (kept internal)
    pub(crate) event_sender: mpsc::UnboundedSender<UiEvent>,
    /// Broadcast channel for UI events (observers can subscribe)
    pub(crate) ui_event_broadcast: broadcast::Sender<UiEvent>,
    /// Manages tasks and their ordering/visibility
    pub(crate) task_manager: TaskManager,
    /// Current UI state, including selections and flags
    pub(crate) state: UiState,
    /// Manages the user input area and wrapping behavior
    pub(crate) input_manager: InputManager,
    /// Responsible for drawing UI components
    pub(crate) renderer: Renderer,
    /// Which pane currently has focus
    pub(crate) focused_pane: FocusedPane,
    /// A pending input to be consumed by the app loop
    pub(crate) pending_input: Option<String>,
    /// Optional task persistence handler for saving/loading tasks
    pub(crate) persistence: Option<TaskPersistence>,
    /// Source of input events (abstracted for testing)
    pub(crate) event_source: Box<dyn InputEventSource + Send>,
    /// Last time the UI was rendered (for forcing periodic updates)
    pub(crate) last_render_time: Instant,
    /// Cache of actual rendered layout dimensions
    pub(crate) layout_cache: layout::LayoutCache,
    /// Thread storage and management (shared with orchestrator)
    pub(crate) thread_store: Arc<Mutex<ThreadStore>>,
    /// Orchestrator for executing tasks
    pub(crate) orchestrator: Option<Arc<RoutingOrchestrator>>,
    /// Log file for task execution
    pub(crate) log_file: Option<fs::File>,
    /// Configuration manager with auto-save
    pub(crate) config_manager: ConfigManager,
}

// Note: all input is sourced from `event_source` to allow test injection without
// altering application behavior.
//
// Test utilities (new_for_test, test_state, etc.) are in app/test_util.rs,
// only compiled with the test-util feature.
