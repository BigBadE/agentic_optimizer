use crossterm::event::{Event, EventStream};
use futures::StreamExt as _;
use std::future::Future;
use std::io;
use std::pin::Pin;

/// Future type returned by `InputEventSource::next_event`
pub type EventFuture<'fut> = Pin<Box<dyn Future<Output = io::Result<Option<Event>>> + Send + 'fut>>;

/// Abstraction over the input event source used by the TUI.
///
/// Trait for receiving input events. Implementations provide
/// asynchronous event streams that can be awaited without polling.
///
/// Note: Uses explicit Future return types instead of `async fn` for dyn compatibility.
pub trait InputEventSource: Send {
    /// Wait for the next input event to arrive.
    ///
    /// Returns `None` when the event stream is exhausted (only for fixtures).
    ///
    /// # Errors
    /// Returns an error if reading the event fails.
    fn next_event(&mut self) -> EventFuture<'_>;
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

impl InputEventSource for CrosstermEventSource {
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(async move {
            match self.stream.next().await {
                Some(Ok(event)) => Ok(Some(event)),
                Some(Err(error)) => Err(error),
                None => Ok(None), // Stream ended (shouldn't happen in practice)
            }
        })
    }
}
