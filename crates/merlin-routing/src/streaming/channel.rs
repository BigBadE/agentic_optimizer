use super::StreamingEvent;
use tokio::sync::mpsc;

/// Channel for streaming execution events
#[derive(Clone)]
pub struct StreamingChannel {
    sender: mpsc::UnboundedSender<StreamingEvent>,
}

impl StreamingChannel {
    #[must_use]
    pub fn new() -> (Self, mpsc::UnboundedReceiver<StreamingEvent>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Self { sender }, receiver)
    }

    pub fn send(&self, event: StreamingEvent) {
        drop(self.sender.send(event));
    }
}

impl Default for StreamingChannel {
    fn default() -> Self {
        Self::new().0
    }
}
