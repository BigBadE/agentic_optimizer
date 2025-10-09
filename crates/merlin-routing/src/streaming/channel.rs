use super::StreamingEvent;
use tokio::sync::mpsc;

/// Channel for streaming execution events
#[derive(Clone)]
pub struct StreamingChannel {
    sender: mpsc::UnboundedSender<StreamingEvent>,
}

impl StreamingChannel {
    /// Creates a streaming channel from an existing sender (for testing)
    pub fn from_sender(sender: mpsc::UnboundedSender<StreamingEvent>) -> Self {
        Self { sender }
    }

    /// Sends a streaming event through the channel.
    ///
    /// Events are dropped if the receiver has been closed.
    pub fn send(&self, event: StreamingEvent) {
        drop(self.sender.send(event));
    }
}

impl Default for StreamingChannel {
    fn default() -> Self {
        let (sender, _receiver) = mpsc::unbounded_channel();
        Self { sender }
    }
}
