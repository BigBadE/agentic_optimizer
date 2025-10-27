//! Main event loop and event processing logic

use merlin_deps::crossterm::event::{Event, KeyEventKind};
use merlin_deps::ratatui::backend::Backend;
use merlin_deps::tracing::warn;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::navigation;
use super::tui_app::TuiApp;
use crate::ui::app::navigation::ScrollContext;
use crate::ui::event_handler::EventHandler;
use crate::ui::renderer::{FocusedPane, RenderCtx, UiCtx};
use crate::ui::state::{ConversationEntry, ConversationRole};
use merlin_routing::{Result, RoutingError};

impl<B: Backend> TuiApp<B> {
    /// Processes one tick of the event loop
    ///
    /// # Errors
    /// Returns an error if polling or rendering the terminal fails.
    pub fn tick(&mut self) -> Result<bool> {
        let had_events = self.process_ui_events();

        if had_events {
            self.render()?;
            self.last_render_time = Instant::now();
        }

        let has_event = self
            .event_source
            .poll(Duration::from_millis(50))
            .map_err(|err| RoutingError::Other(err.to_string()))?;

        if has_event {
            let events = self.collect_input_events()?;
            let should_quit = self.process_input_events(events);
            self.render()?;
            self.last_render_time = Instant::now();
            return Ok(should_quit);
        }

        // Force periodic renders when there are active tasks with progress
        // This ensures progress bars and timers update smoothly every tick (50ms)
        let has_active_progress = self.task_manager.has_tasks_with_progress();
        let time_since_render = self.last_render_time.elapsed();
        let should_force_render =
            has_active_progress && time_since_render >= Duration::from_millis(50);

        // Unconditionally render once at the end of the tick loop to keep UI fresh.
        // The previous conditional branches were identical.
        let _ = should_force_render; // document the intent without branching
        self.render()?;
        self.last_render_time = Instant::now();

        Ok(false)
    }

    /// Processes any pending UI events from the channel and updates state
    pub(super) fn process_ui_events(&mut self) -> bool {
        let mut had_events = false;

        while let Ok(event) = self.event_receiver.try_recv() {
            // Broadcast to test event tap if present
            #[cfg(feature = "test-util")]
            if let Some(ref tap) = self.test_event_tap {
                drop(tap.send(event.clone()));
            }

            let persistence = self.persistence.as_ref();
            let mut handler =
                EventHandler::new(&mut self.task_manager, &mut self.state, persistence);
            handler.handle_event(event);
            had_events = true;
        }

        // Adjust scroll after processing events (in case tasks were added/removed)
        if had_events {
            self.adjust_task_list_scroll();
        }

        had_events
    }

    /// Collects input events from the terminal (via the configured event source).
    ///
    /// # Errors
    /// Returns an error if reading events from the event source fails.
    pub(super) fn collect_input_events(&mut self) -> Result<Vec<Event>> {
        let mut events = Vec::default();

        // blocking read of at least one event
        let first = self
            .event_source
            .read()
            .map_err(|err| RoutingError::Other(err.to_string()))?;
        events.push(first);

        // drain the buffer of any immediately available events
        loop {
            let has_event = self
                .event_source
                .poll(Duration::from_millis(0))
                .map_err(|err| RoutingError::Other(err.to_string()))?;

            if !has_event {
                break;
            }

            let event = self
                .event_source
                .read()
                .map_err(|err| RoutingError::Other(err.to_string()))?;
            events.push(event);
        }

        Ok(events)
    }

    /// Processes a batch of input events and returns true if the app should quit
    pub(super) fn process_input_events(&mut self, events: Vec<Event>) -> bool {
        let mut should_quit = false;

        for event in events {
            if let Event::Key(key) = &event {
                if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                    continue;
                }

                if self.handle_key_event(key) {
                    should_quit = true;
                }
            }
        }

        should_quit
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

            self.spawn_task_execution(
                Arc::clone(orchestrator),
                input,
                parent_task_id,
                conversation_history,
            );
        } else {
            self.pending_input = Some(input);
        }

        self.input_manager.clear();
        false
    }

    /// Cycles to the next theme and persists it on disk if persistence is enabled
    pub(super) fn cycle_theme(&mut self) {
        let new_theme = self.renderer.theme().next();
        self.renderer.set_theme(new_theme);

        if let Some(persistence) = &self.persistence {
            let dir = persistence.get_tasks_dir();
            if let Err(error) = new_theme.save(dir) {
                warn!("Failed to save theme: {}", error);
            }
        }
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
