use crossterm::event::{self, Event};
use std::time::Duration;

/// Abstraction over the input event source used by the TUI.
///
/// Implementations must mirror crossterm's semantics:
/// - `poll(timeout)` waits up to timeout for an event and returns whether one is available.
/// - `read()` blocks until an event is available and returns it.
pub trait InputEventSource {
    /// Wait up to `timeout` for an event to become available.
    /// Returns `true` if an event is ready to be read.
    fn poll(&mut self, timeout: Duration) -> bool;

    /// Block until an input `Event` is available and return it.
    fn read(&mut self) -> Event;
}

/// Default event source backed by crossterm.
pub struct CrosstermEventSource;

impl InputEventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> bool {
        event::poll(timeout).unwrap_or(false)
    }

    fn read(&mut self) -> Event {
        event::read().unwrap_or(Event::Resize(0, 0))
    }
}
