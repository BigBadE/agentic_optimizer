//! Thread, message, and conversation types.

use serde::{Deserialize, Serialize};

use super::ThreadColor;
use super::ids::{MessageId, ThreadId};
use super::work::WorkUnit;

/// A conversation thread containing messages and their associated work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    /// Unique identifier for this thread
    pub id: ThreadId,
    /// Display name for this thread (user-editable)
    pub name: String,
    /// Color for visual identification
    pub color: ThreadColor,
    /// Messages in this thread (ordered chronologically)
    pub messages: Vec<Message>,
    /// Parent thread if this was branched (None for root threads)
    pub parent_thread: Option<BranchPoint>,
    /// Whether this thread is archived (hidden from main view)
    pub archived: bool,
}

impl Thread {
    /// Creates a new thread with the given name and color
    pub fn new(name: String, color: ThreadColor) -> Self {
        Self {
            id: ThreadId::new(),
            name,
            color,
            messages: Vec::new(),
            parent_thread: None,
            archived: false,
        }
    }

    /// Creates a new thread branched from another thread at a specific message
    pub fn branched_from(
        name: String,
        color: ThreadColor,
        parent_thread_id: ThreadId,
        parent_message_id: MessageId,
    ) -> Self {
        Self {
            id: ThreadId::new(),
            name,
            color,
            messages: Vec::new(),
            parent_thread: Some(BranchPoint {
                thread_id: parent_thread_id,
                message_id: parent_message_id,
            }),
            archived: false,
        }
    }

    /// Adds a message to this thread
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Returns the most recent message in this thread
    #[must_use]
    pub fn last_message(&self) -> Option<&Message> {
        self.messages.last()
    }
}

/// Reference to a parent thread and message where a branch occurred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchPoint {
    /// ID of the parent thread
    pub thread_id: ThreadId,
    /// ID of the message in the parent thread where this branch started
    pub message_id: MessageId,
}

/// A user message in a thread that spawns work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique identifier for this message
    pub id: MessageId,
    /// User input text
    pub content: String,
    /// Work unit spawned by this message (None if cancelled before work started)
    pub work: Option<WorkUnit>,
}

impl Message {
    /// Creates a new message with the given content
    pub fn new(content: String) -> Self {
        Self {
            id: MessageId::new(),
            content,
            work: None,
        }
    }

    /// Attaches work to this message
    pub fn attach_work(&mut self, work: WorkUnit) {
        self.work = Some(work);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_creation() {
        let thread = Thread::new("Test Thread".to_owned(), ThreadColor::Blue);
        assert_eq!(thread.name, "Test Thread");
        assert_eq!(thread.color, ThreadColor::Blue);
        assert!(thread.messages.is_empty());
        assert!(thread.parent_thread.is_none());
        assert!(!thread.archived);
    }

    #[test]
    fn test_thread_branching() {
        let parent_id = ThreadId::new();
        let parent_msg_id = MessageId::new();
        let thread = Thread::branched_from(
            "Branch".to_owned(),
            ThreadColor::Green,
            parent_id,
            parent_msg_id,
        );

        assert!(thread.parent_thread.is_some());
        let branch_point = thread.parent_thread.unwrap();
        assert_eq!(branch_point.thread_id, parent_id);
        assert_eq!(branch_point.message_id, parent_msg_id);
    }

    #[test]
    fn test_message_creation() {
        let message = Message::new("Hello".to_owned());
        assert_eq!(message.content, "Hello");
        assert!(message.work.is_none());
    }
}
