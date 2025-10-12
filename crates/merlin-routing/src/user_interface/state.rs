use super::text_width::EmojiMode;
use crate::TaskId;
use std::collections::HashSet;
use std::time::Instant;

/// Maximum number of conversation entries to retain
const MAX_CONVERSATION_HISTORY: usize = 50;

/// Main UI state
#[derive(Default)]
pub struct UiState {
    /// Currently selected task index
    pub selected_task_index: usize,
    /// Active task identifier
    pub active_task_id: Option<TaskId>,
    /// Set of currently running tasks
    pub active_running_tasks: HashSet<TaskId>,
    /// Task pending deletion
    pub pending_delete_task_id: Option<TaskId>,
    /// Whether tasks are currently loading
    pub loading_tasks: bool,
    /// History of conversation entries (limited to `MAX_CONVERSATION_HISTORY`)
    pub conversation_history: Vec<ConversationEntry>,
    /// Emoji display mode
    pub emoji_mode: EmojiMode,
    /// Status message to display when processing input
    pub processing_status: Option<String>,
    /// Vertical scroll offset for the output pane
    pub output_scroll_offset: u16,
}

impl UiState {
    /// Access `emoji_mode`
    pub fn emoji_mode(&self) -> &EmojiMode {
        &self.emoji_mode
    }

    /// Add a conversation entry and trim history if needed
    pub fn add_conversation_entry(&mut self, entry: ConversationEntry) {
        tracing::info!(
            "UiState::add_conversation_entry() - role: {:?}, text length: {}",
            entry.role,
            entry.text.len()
        );
        self.conversation_history.push(entry);

        // Trim oldest entries if we exceed the limit
        if self.conversation_history.len() > MAX_CONVERSATION_HISTORY {
            let excess = self.conversation_history.len() - MAX_CONVERSATION_HISTORY;
            self.conversation_history.drain(0..excess);
        }

        tracing::info!(
            "UiState now has {} conversation entries",
            self.conversation_history.len()
        );
    }

    /// Clear all conversation history
    pub fn clear_conversation_history(&mut self) {
        self.conversation_history.clear();
    }
}

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    /// Role of the speaker
    pub role: ConversationRole,
    /// Text content of the message
    pub text: String,
    /// Timestamp when the entry was created
    pub timestamp: Instant,
}

impl ConversationEntry {
    /// Access role
    pub fn role(&self) -> ConversationRole {
        self.role
    }

    /// Access text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Access timestamp
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// Conversation role
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConversationRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message
    System,
}

#[cfg(test)]
#[allow(
    clippy::min_ident_chars,
    reason = "Test code uses short variable names for clarity"
)]
mod tests {
    use super::*;

    #[test]
    fn test_add_conversation_entry() {
        let mut state = UiState::default();

        let entry = ConversationEntry {
            role: ConversationRole::User,
            text: "Hello".to_owned(),
            timestamp: Instant::now(),
        };

        state.add_conversation_entry(entry);
        assert_eq!(state.conversation_history.len(), 1);
        assert_eq!(state.conversation_history[0].text, "Hello");
    }

    #[test]
    fn test_conversation_history_limit() {
        let mut state = UiState::default();

        // Add more than MAX_CONVERSATION_HISTORY entries
        for i in 0..(MAX_CONVERSATION_HISTORY + 20) {
            let entry = ConversationEntry {
                role: ConversationRole::User,
                text: format!("Message {i}"),
                timestamp: Instant::now(),
            };
            state.add_conversation_entry(entry);
        }

        // Should be capped at MAX_CONVERSATION_HISTORY
        assert_eq!(state.conversation_history.len(), MAX_CONVERSATION_HISTORY);

        // Should have kept the most recent messages (oldest trimmed)
        assert_eq!(
            state.conversation_history[0].text,
            format!("Message {}", 20)
        );
        assert_eq!(
            state.conversation_history.last().unwrap().text,
            format!("Message {}", MAX_CONVERSATION_HISTORY + 19)
        );
    }

    #[test]
    fn test_clear_conversation_history() {
        let mut state = UiState::default();

        // Add some entries
        for i in 0..5 {
            state.add_conversation_entry(ConversationEntry {
                role: ConversationRole::User,
                text: format!("Message {i}"),
                timestamp: Instant::now(),
            });
        }

        assert_eq!(state.conversation_history.len(), 5);

        // Clear
        state.clear_conversation_history();
        assert_eq!(state.conversation_history.len(), 0);
    }

    #[test]
    fn test_conversation_entry_role_accessors() {
        let entry = ConversationEntry {
            role: ConversationRole::Assistant,
            text: "Test".to_owned(),
            timestamp: Instant::now(),
        };

        assert_eq!(entry.role(), ConversationRole::Assistant);
        assert_eq!(entry.text(), "Test");
    }
}
