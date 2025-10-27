use merlin_deps::crossterm::event::{self, Event};
use std::io;
use std::time::Duration;

/// Abstraction over the input event source used by the TUI.
///
/// Implementations must mirror crossterm's semantics:
/// - `poll(timeout)` waits up to timeout for an event and returns whether one is available.
/// - `read()` blocks until an event is available and returns it.
pub trait InputEventSource: Send + Sync {
    /// Wait up to `timeout` for an event to become available.
    ///
    /// # Errors
    /// Returns an error if the event polling operation fails.
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;

    /// Block until an input `Event` is available and return it.
    ///
    /// # Errors
    /// Returns an error if reading the event fails.
    fn read(&mut self) -> io::Result<Event>;
}

/// Default event source backed by crossterm.
pub struct CrosstermEventSource;

impl InputEventSource for CrosstermEventSource {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<Event> {
        event::read()
    }
}
