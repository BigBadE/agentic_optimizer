//! Test helpers and accessors for `TuiApp`
//!
//! These methods provide controlled access to internal state for testing purposes.
//! They are available when compiling tests or when the `test-util` feature is enabled.
//!
//! # Testing Pattern
//!
//! When writing UI tests:
//! 1. Create a `TuiApp` instance with a test backend
//! 2. Use `set_event_source()` to inject a custom `InputEventSource`
//! 3. Use accessor methods to verify state after event processing
//!
//! Never manipulate internal state directly - use the provided methods.

#[cfg(any(test, feature = "test-util"))]
use merlin_deps::ratatui::backend::Backend;

#[cfg(any(test, feature = "test-util"))]
use super::tui_app::TuiApp;
#[cfg(any(test, feature = "test-util"))]
use crate::ui::event_source::InputEventSource;
#[cfg(any(test, feature = "test-util"))]
use crate::ui::renderer::FocusedPane;
#[cfg(any(test, feature = "test-util"))]
use crate::ui::state::{ConversationEntry, ConversationRole, UiState};
#[cfg(any(test, feature = "test-util"))]
use crate::ui::task_manager::TaskManager;
#[cfg(any(test, feature = "test-util"))]
use merlin_core::ThreadId;
#[cfg(any(test, feature = "test-util"))]
use merlin_routing::TaskId;

#[cfg(any(test, feature = "test-util"))]
#[allow(dead_code, reason = "Test utilities")]
impl<B: Backend> TuiApp<B> {
    /// Gets a reference to the terminal backend
    ///
    /// # Testing Only
    /// This method is intended for test assertions only.
    pub fn backend(&self) -> &B {
        self.terminal.backend()
    }

    /// Gets immutable access to task manager
    ///
    /// # Testing Only
    /// Use this to verify task state in tests.
    pub fn task_manager(&self) -> &TaskManager {
        &self.task_manager
    }

    /// Gets mutable access to task manager for test setup
    ///
    /// # Testing Only
    /// Use this to set up task state before running test scenarios.
    pub fn task_manager_mut(&mut self) -> &mut TaskManager {
        &mut self.task_manager
    }

    /// Gets immutable access to UI state
    ///
    /// # Testing Only
    /// Use this to verify UI state in tests.
    pub fn state(&self) -> &UiState {
        &self.state
    }

    /// Gets mutable access to UI state for test setup
    ///
    /// # Testing Only
    /// Use this to set up UI state before running test scenarios.
    pub fn state_mut(&mut self) -> &mut UiState {
        &mut self.state
    }

    /// Sets the focused pane
    ///
    /// # Testing Only
    /// Use this to test pane-specific behavior.
    pub fn set_focused_pane(&mut self, pane: FocusedPane) {
        self.focused_pane = pane;
    }

    /// Gets the current input text as a single string
    ///
    /// # Testing Only
    /// Use this to verify input content in tests.
    pub fn get_input_text(&self) -> String {
        self.input_manager.input_area().lines().join("\n")
    }

    /// Gets the input lines as a vector
    ///
    /// # Testing Only
    /// Use this to verify multi-line input behavior.
    pub fn get_input_lines(&self) -> Vec<String> {
        self.input_manager.input_area().lines().to_vec()
    }

    /// Replaces the input event source
    ///
    /// # Testing Only
    /// Use this to inject a custom event source that provides synthetic events.
    /// This is the primary mechanism for testing UI behavior without user interaction.
    ///
    /// # Example
    /// ```ignore
    /// let mut app = create_test_app();
    /// app.set_event_source(Box::new(TestEventSource::new(vec![...])));
    /// ```
    pub fn set_event_source(&mut self, source: Box<dyn InputEventSource + Send>) {
        self.event_source = source;
    }

    /// Adds an assistant response to conversation history
    ///
    /// # Testing Only
    /// Use this to set up conversation state for testing conversation features.
    ///
    /// TODO: This is used for testing conversation threading features (not yet implemented).
    pub fn add_assistant_response(&mut self, text: String) {
        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::Assistant,
            text,
        });
    }

    /// Gets the thread ID of the selected task
    ///
    /// # Testing Only
    /// Use this to verify task thread membership in tests.
    #[allow(dead_code, reason = "Test utility for future thread-based features")]
    pub fn get_selected_task_thread(&self) -> Option<ThreadId> {
        let selected_task_id = self.get_selected_task_id()?;
        self.task_manager.get_task(selected_task_id)?.thread_id
    }

    /// Gets thread context for the selected task
    ///
    /// # Testing Only
    /// Use this to verify conversation threading.
    ///
    /// TODO: This is for testing conversation threading features (not yet implemented).
    pub fn get_thread_context(&self) -> Vec<(TaskId, String, String)> {
        super::conversation::get_thread_context(self.state.active_task_id, &self.task_manager)
    }

    /// Gets the current theme name
    ///
    /// # Testing Only
    /// Use this to verify theme cycling.
    pub fn get_theme_name(&self) -> String {
        format!("{:?}", self.renderer.theme())
    }

    /// Gets the currently focused pane
    ///
    /// # Testing Only
    /// Use this to verify focus state.
    pub fn get_focused_pane(&self) -> FocusedPane {
        self.focused_pane
    }

    /// Gets the set of expanded conversation IDs
    ///
    /// # Testing Only
    /// Use this to verify conversation expansion state.
    pub fn get_expanded_conversations(&self) -> Vec<TaskId> {
        self.state.expanded_conversations.iter().copied().collect()
    }

    /// Gets the set of expanded step IDs
    ///
    /// # Testing Only
    /// Use this to verify step expansion state.
    pub fn get_expanded_steps(&self) -> Vec<TaskId> {
        self.state.expanded_steps.iter().copied().collect()
    }

    /// Gets the output scroll offset
    ///
    /// # Testing Only
    /// Use this to verify scrolling behavior.
    pub fn get_output_scroll(&self) -> u16 {
        self.state.output_scroll_offset
    }

    /// Gets the task list scroll offset
    ///
    /// # Testing Only
    /// Use this to verify task list scrolling.
    pub fn get_task_list_scroll(&self) -> usize {
        self.state.task_list_scroll_offset
    }

    /// Gets the processing status message
    ///
    /// # Testing Only
    /// Use this to verify status display.
    pub fn get_processing_status(&self) -> Option<String> {
        self.state.processing_status.clone()
    }

    /// Gets the pending delete task ID
    ///
    /// # Testing Only
    /// Use this to verify delete confirmation flow.
    pub fn get_pending_delete_task_id(&self) -> Option<TaskId> {
        self.state.pending_delete_task_id
    }
}
