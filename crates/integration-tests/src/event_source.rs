//! Fixture-based event source for testing.
//!
//! This module provides an `InputEventSource` implementation that feeds events
//! from test fixtures instead of reading from the terminal.

use merlin_cli::InputEventSource;
use merlin_deps::async_trait::async_trait;
use merlin_deps::crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers,
};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex};

use super::fixture::{TestEvent, TestFixture};

/// Internal state for fixture event source
struct FixtureEventState {
    /// All fixture events
    fixture_events: Vec<TestEvent>,
    /// Current fixture event index
    current_index: usize,
    /// Queue of crossterm events for the current fixture event
    current_events: VecDeque<Event>,
    /// Whether we've exhausted all events
    exhausted: bool,
}

/// Fixture-based event source that provides events from a test fixture on-demand
pub struct FixtureEventSource {
    /// Shared state
    state: Arc<Mutex<FixtureEventState>>,
}

/// Handle to control the fixture event source
pub struct FixtureEventController {
    /// Shared state
    state: Arc<Mutex<FixtureEventState>>,
}

impl FixtureEventState {
    /// Load crossterm events for the current fixture event
    fn load_current_event(&mut self) {
        if self.current_index >= self.fixture_events.len() {
            self.exhausted = true;
            return;
        }

        let event = &self.fixture_events[self.current_index];
        match event {
            TestEvent::UserInput(input_event) => {
                // Add character events for each character in the text
                for character in input_event.data.text.chars() {
                    self.current_events.push_back(Event::Key(KeyEvent {
                        code: KeyCode::Char(character),
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                        state: KeyEventState::empty(),
                    }));
                }

                // If submit is true, add an Enter key event
                if input_event.data.submit {
                    self.current_events.push_back(Event::Key(KeyEvent {
                        code: KeyCode::Enter,
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                        state: KeyEventState::empty(),
                    }));
                }
            }
            TestEvent::KeyPress(key_event) => {
                // Convert string key name to KeyCode
                let code = match key_event.data.key.as_str() {
                    "Tab" => KeyCode::Tab,
                    "Enter" => KeyCode::Enter,
                    "Esc" | "Escape" => KeyCode::Esc,
                    "Backspace" => KeyCode::Backspace,
                    "Delete" => KeyCode::Delete,
                    "Up" => KeyCode::Up,
                    "Down" => KeyCode::Down,
                    "Left" => KeyCode::Left,
                    "Right" => KeyCode::Right,
                    "Home" => KeyCode::Home,
                    "End" => KeyCode::End,
                    "PageUp" => KeyCode::PageUp,
                    "PageDown" => KeyCode::PageDown,
                    single if single.len() == 1 => {
                        // We've just checked that single has exactly 1 character
                        // This must succeed, but we use Null as fallback to satisfy clippy
                        single.chars().next().map_or(KeyCode::Null, KeyCode::Char)
                    }
                    _ => KeyCode::Null,
                };

                // Parse modifiers
                let mut modifiers = KeyModifiers::empty();
                for modifier in &key_event.data.modifiers {
                    match modifier.to_lowercase().as_str() {
                        "ctrl" | "control" => modifiers.insert(KeyModifiers::CONTROL),
                        "shift" => modifiers.insert(KeyModifiers::SHIFT),
                        "alt" => modifiers.insert(KeyModifiers::ALT),
                        _ => {}
                    }
                }

                self.current_events.push_back(Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press,
                    state: KeyEventState::empty(),
                }));
            }
            TestEvent::LlmResponse(_) | TestEvent::Wait(_) => {
                // These events don't produce terminal input events
                // Don't add any events - the runner will call advance() when it's processed
            }
        }
    }
}

impl FixtureEventSource {
    /// Create a new fixture event source from a test fixture
    ///
    /// Returns the event source and a controller handle
    #[must_use]
    pub fn new(fixture: &TestFixture) -> (Self, FixtureEventController) {
        let state = Arc::new(Mutex::new(FixtureEventState {
            fixture_events: fixture.events.clone(),
            current_index: 0,
            current_events: VecDeque::new(),
            exhausted: false,
        }));

        // Load first event batch
        if let Ok(mut guard) = state.lock() {
            guard.load_current_event();
        }

        let source = Self {
            state: Arc::clone(&state),
        };
        let controller = FixtureEventController { state };

        (source, controller)
    }
}

impl FixtureEventController {
    /// Advance to the next fixture event
    ///
    /// This should be called by the test runner after processing the current event
    pub fn advance(&self) {
        if let Ok(mut state) = self.state.lock() {
            // Only advance if we've consumed all current events
            if state.current_events.is_empty() {
                state.current_index += 1;
                state.load_current_event();
            }
        }
    }
}

#[async_trait]
impl InputEventSource for FixtureEventSource {
    async fn next_event(&mut self) -> io::Result<Option<Event>> {
        // Get next event from current queue (non-blocking)
        let event_opt = self
            .state
            .lock()
            .ok()
            .and_then(|mut state| state.current_events.pop_front());

        event_opt.map_or(Ok(None), |evt| Ok(Some(evt)))
    }
}
