//! Main event loop and event processing logic

use merlin_deps::crossterm::event::{Event, KeyEventKind};
use merlin_deps::ratatui::backend::Backend;
use merlin_routing::{Result, RoutingError, UiEvent};
use std::sync::Arc;
use std::time::Instant;

use super::navigation;
use super::task_execution::TaskExecutionParams;
use super::tui_app::TuiApp;
use crate::ui::app::navigation::ScrollContext;
use crate::ui::event_handler::EventHandler;
use crate::ui::renderer::{FocusedPane, RenderCtx, UiCtx};
use crate::ui::state::{ConversationEntry, ConversationRole};

impl<B: Backend> TuiApp<B> {
    /// Run the main event loop until quit
    ///
    /// This processes both input events (from crossterm `EventStream`) and UI events
    /// (from the orchestrator) concurrently using `tokio::select`!.
    ///
    /// # Errors
    /// Returns an error if event processing or rendering fails.
    pub async fn run_event_loop(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                // Wait for input event from async stream
                event_result = self.event_source.next_event() => {
                    match event_result {
                        Ok(Some(event)) => {
                            if self.handle_input(&event) {
                                break; // Quit requested
                            }
                        }
                        Ok(None) => {
                            // Event source exhausted (shouldn't happen for crossterm)
                            break;
                        }
                        Err(error) => {
                            return Err(RoutingError::Other(error.to_string()));
                        }
                    }
                }

                // Wait for UI event from orchestrator
                Some(ui_event) = self.event_receiver.recv() => {
                    self.handle_ui_event(ui_event);
                }
            }

            // Render after processing any event
            self.render()?;
            self.last_render_time = Instant::now();
        }

        Ok(())
    }

    /// Handle an input event and return true if the app should quit
    fn handle_input(&mut self, event: &Event) -> bool {
        if let Event::Key(key) = event
            && matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            return self.handle_key_event(key);
        }
        false
    }

    /// Handle a UI event from the orchestrator
    fn handle_ui_event(&mut self, ui_event: UiEvent) {
        // Broadcast to observers
        drop(self.ui_event_broadcast.send(ui_event.clone()));

        // Handle the event
        let persistence = self.persistence.as_ref();
        let mut handler = EventHandler::new(&mut self.task_manager, &mut self.state, persistence);
        handler.handle_event(ui_event);

        self.adjust_task_list_scroll();
    }

    /// Submits the current input if non-empty and returns true if it indicates quitting
    pub(super) fn submit_input(&mut self) -> bool {
        let input = self.input_manager.input_area().lines()[0]
            .trim()
            .to_string();

        if input.is_empty() {
            return false;
        }

        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            return true;
        }

        // Check if there's already work running
        let has_running_work = !self.state.active_running_tasks.is_empty();

        if has_running_work {
            // Queue the input for later processing
            self.state.queued_input = Some(input);
            self.state.processing_status =
                Some("[Work in progress. Press 'c' to cancel, 'a' to queue]".to_string());
            self.input_manager.clear();
            return false;
        }

        // If a task is selected, we're continuing that conversation
        if self.state.active_task_id.is_some() {
            self.state.continuing_conversation_from = self.state.active_task_id;
            self.state.active_task_id = None;
        }

        self.state.add_conversation_entry(ConversationEntry {
            role: ConversationRole::User,
            text: input.clone(),
        });

        self.state.processing_status = Some("[Processing...]".to_string());

        if let Some(ref orchestrator) = self.orchestrator {
            let conversation_history = self.get_conversation_history();
            let parent_task_id = self.state.continuing_conversation_from;

            self.spawn_task_execution(TaskExecutionParams {
                orchestrator: Arc::clone(orchestrator),
                user_input: input,
                parent_task_id,
                conversation_history,
                thread_id: self.state.active_thread_id,
            });
        } else {
            self.pending_input = Some(input);
        }

        self.input_manager.clear();
        false
    }

    /// Cycles to the next theme and auto-saves via `ConfigManager`
    pub(super) fn cycle_theme(&mut self) {
        let new_theme = self.renderer.theme().next();
        self.renderer.set_theme(new_theme);

        // Update config (auto-saves when guard is dropped)
        if let Ok(mut config) = self.config_manager.get_mut() {
            config.theme = new_theme;
        } // Drop happens here, triggering async save
    }

    /// Adjusts task list scroll to keep the selected task visible
    pub(super) fn adjust_task_list_scroll(&mut self) {
        let terminal_height = self.terminal.size().map(|size| size.height).unwrap_or(30);
        navigation::adjust_task_list_scroll(&mut ScrollContext {
            active_task_id: self.state.active_task_id.as_ref(),
            expanded_conversations: &self.state.expanded_conversations,
            task_list_scroll_offset: &mut self.state.task_list_scroll_offset,
            task_manager: &self.task_manager,
            terminal_height,
            focused_pane_is_tasks: self.focused_pane == FocusedPane::Tasks,
        });
    }

    /// Renders the UI to the terminal
    ///
    /// # Errors
    /// Returns an error if drawing to the terminal fails.
    pub(super) fn render(&mut self) -> Result<()> {
        // Ensure scroll is correct before rendering (handles initial state)
        self.adjust_task_list_scroll();

        // Auto-scroll output to bottom if flag is set
        if self.state.auto_scroll_output_to_bottom {
            self.auto_scroll_output_to_bottom();
            self.state.auto_scroll_output_to_bottom = false;
        }

        let layout_cache = &mut self.layout_cache;
        let renderer = &self.renderer;
        let task_manager = &self.task_manager;
        let state = &self.state;
        let input_manager = &self.input_manager;
        let focused_pane = self.focused_pane;
        let thread_store = &self.thread_store;

        self.terminal
            .draw(|frame| {
                let mut ctx = RenderCtx {
                    ui_ctx: UiCtx {
                        task_manager,
                        state,
                    },
                    input: input_manager,
                    focused: focused_pane,
                    layout_cache,
                    thread_store,
                };
                renderer.render(frame, &mut ctx);
            })
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        Ok(())
    }
}
