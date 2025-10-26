//! Main TUI application struct and core state management

use ratatui::Terminal;
use ratatui::backend::Backend;
use std::time::Instant;
use tokio::sync::mpsc;

use crate::ui::event_source::InputEventSource;
use crate::ui::input::InputManager;
use crate::ui::layout;
use crate::ui::persistence::TaskPersistence;
use crate::ui::renderer::{FocusedPane, Renderer};
use crate::ui::state::UiState;
use crate::ui::task_manager::TaskManager;
use merlin_agent::ThreadStore;
use merlin_routing::UiEvent;

/// Main TUI application
pub struct TuiApp<B: Backend> {
    /// Terminal instance used to render the UI
    pub(super) terminal: Terminal<B>,
    /// Channel receiving UI events from background tasks
    pub(super) event_receiver: mpsc::UnboundedReceiver<UiEvent>,
    /// Manages tasks and their ordering/visibility
    pub(super) task_manager: TaskManager,
    /// Current UI state, including selections and flags
    pub(super) state: UiState,
    /// Manages the user input area and wrapping behavior
    pub(super) input_manager: InputManager,
    /// Responsible for drawing UI components
    pub(super) renderer: Renderer,
    /// Which pane currently has focus
    pub(super) focused_pane: FocusedPane,
    /// A pending input to be consumed by the app loop
    pub(super) pending_input: Option<String>,
    /// Optional task persistence handler for saving/loading tasks
    pub(super) persistence: Option<TaskPersistence>,
    /// Source of input events (abstracted for testing)
    pub(super) event_source: Box<dyn InputEventSource + Send>,
    /// Last time the UI was rendered (for forcing periodic updates)
    pub(super) last_render_time: Instant,
    /// Cache of actual rendered layout dimensions
    pub(super) layout_cache: layout::LayoutCache,
    /// Thread storage and management
    #[allow(dead_code, reason = "Will be used in Phase 5")]
    pub(super) thread_store: ThreadStore,
}

// Note: all input is sourced from `event_source` to allow test injection without
// altering application behavior.
