//! Main TUI application struct and core state management

use ratatui::Terminal;
use ratatui::backend::Backend;
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

/// Event handling and communication channels
pub struct EventSystem {
    /// Channel receiving UI events from background tasks
    pub receiver: mpsc::UnboundedReceiver<UiEvent>,
    /// Channel for sending UI events (kept internal)
    pub sender: mpsc::UnboundedSender<UiEvent>,
    /// Broadcast channel for UI events (observers can subscribe)
    pub broadcast: broadcast::Sender<UiEvent>,
    /// Source of input events (abstracted for testing)
    pub source: Box<dyn InputEventSource + Send>,
    /// Latest task-specific event receiver for testing
    pub last_task_receiver: Option<mpsc::Receiver<UiEvent>>,
}

/// UI component management and rendering state
pub struct UiComponents {
    /// Manages tasks and their ordering/visibility
    pub task_manager: TaskManager,
    /// Current UI state, including selections and flags
    pub state: UiState,
    /// Manages the user input area and wrapping behavior
    pub input_manager: InputManager,
    /// Responsible for drawing UI components
    pub renderer: Renderer,
    /// Which pane currently has focus
    pub focused_pane: FocusedPane,
    /// A pending input to be consumed by the app loop
    pub pending_input: Option<String>,
    /// Cache of actual rendered layout dimensions
    pub layout_cache: layout::LayoutCache,
    /// Last time the UI was rendered (for forcing periodic updates)
    pub last_render_time: Instant,
}

/// Runtime state and orchestration
pub struct RuntimeState {
    /// Thread storage and management (shared with orchestrator)
    pub thread_store: Arc<Mutex<ThreadStore>>,
    /// Orchestrator for executing tasks
    pub orchestrator: Option<Arc<RoutingOrchestrator>>,
    /// Optional task persistence handler for saving/loading tasks
    pub persistence: Option<TaskPersistence>,
    /// Log file for task execution
    pub log_file: Option<fs::File>,
}

/// Main TUI application
pub struct TuiApp<B: Backend> {
    /// Terminal instance used to render the UI
    pub terminal: Terminal<B>,
    /// Event handling and communication
    pub event_system: EventSystem,
    /// UI components and state
    pub ui_components: UiComponents,
    /// Runtime state and orchestration
    pub runtime_state: RuntimeState,
    /// Configuration manager with auto-save
    pub config_manager: ConfigManager,
}
