//! Fixture-based event source for testing.
//!
//! This module provides an `InputEventSource` implementation that feeds events
//! from test fixtures instead of reading from the terminal.

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use merlin_cli::InputEventSource;
use std::collections::VecDeque;
use std::io;
use std::time::Duration;

use super::fixture::{TestEvent, TestFixture};

/// Fixture-based event source that provides events from a test fixture
pub struct FixtureEventSource {
    /// Queue of events to be returned
    events: VecDeque<Event>,
    /// Whether we've exhausted all events
    exhausted: bool,
}

impl FixtureEventSource {
    /// Create a new fixture event source from a test fixture
    #[must_use]
    pub fn new(fixture: &TestFixture) -> Self {
        let mut events = VecDeque::new();

        // Convert fixture events to crossterm events
        for event in &fixture.events {
            match event {
                TestEvent::UserInput(input_event) => {
                    // Add character events for each character in the text
                    for character in input_event.data.text.chars() {
                        events.push_back(Event::Key(KeyEvent {
                            code: KeyCode::Char(character),
                            modifiers: KeyModifiers::empty(),
                            kind: KeyEventKind::Press,
                            state: KeyEventState::empty(),
                        }));
                    }

                    // If submit is true, add an Enter key event
                    if input_event.data.submit {
                        events.push_back(Event::Key(KeyEvent {
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
                            KeyCode::Char(single.chars().next().unwrap_or(' '))
                        }
                        _ => KeyCode::Null,
                    };

                    events.push_back(Event::Key(KeyEvent {
                        code,
                        modifiers: KeyModifiers::empty(),
                        kind: KeyEventKind::Press,
                        state: KeyEventState::empty(),
                    }));
                }
                TestEvent::LlmResponse(_) | TestEvent::Wait(_) => {
                    // These events don't produce terminal input events
                }
            }
        }

        Self {
            events,
            exhausted: false,
        }
    }

    /// Check if there are more events to process
    #[must_use]
    #[allow(dead_code, reason = "Will be used when event loop is implemented")]
    pub fn has_events(&self) -> bool {
        !self.events.is_empty()
    }
}

impl InputEventSource for FixtureEventSource {
    fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
        // Always return immediately with whether we have events
        Ok(!self.events.is_empty())
    }

    fn read(&mut self) -> io::Result<Event> {
        // Return next event from queue
        self.events.pop_front().ok_or_else(|| {
            self.exhausted = true;
            io::Error::new(io::ErrorKind::WouldBlock, "No more events in fixture")
        })
    }
}
