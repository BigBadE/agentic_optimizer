use merlin_deps::async_trait::async_trait;
use merlin_deps::crossterm::event::{Event, EventStream};
use merlin_deps::futures::StreamExt as _;
use std::io;

/// Abstraction over the input event source used by the TUI.
///
/// Async trait for receiving input events. Implementations provide
/// asynchronous event streams that can be awaited without polling.
#[async_trait]
pub trait InputEventSource: Send {
    /// Wait for the next input event to arrive.
    ///
    /// Returns `None` when the event stream is exhausted (only for fixtures).
    ///
    /// # Errors
    /// Returns an error if reading the event fails.
    async fn next_event(&mut self) -> io::Result<Option<Event>>;
}

/// Default event source backed by crossterm.
pub struct CrosstermEventSource {
    /// Crossterm event stream
    stream: EventStream,
}

impl CrosstermEventSource {
    /// Create new crossterm event source
    #[must_use]
    pub fn new() -> Self {
        Self {
            stream: EventStream::new(),
        }
    }
}

impl Default for CrosstermEventSource {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InputEventSource for CrosstermEventSource {
    async fn next_event(&mut self) -> io::Result<Option<Event>> {
        match self.stream.next().await {
            Some(Ok(event)) => Ok(Some(event)),
            Some(Err(error)) => Err(error),
            None => Ok(None), // Stream ended (shouldn't happen in practice)
        }
    }
}
