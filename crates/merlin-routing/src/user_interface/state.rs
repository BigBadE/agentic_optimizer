use super::text_width::EmojiMode;
use crate::TaskId;
use std::collections::HashSet;
use std::time::Instant;

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
    /// History of conversation entries
    pub conversation_history: Vec<ConversationEntry>,
    #[allow(
        dead_code,
        reason = "Field is part of public API for emoji mode configuration"
    )]
    /// Emoji display mode
    pub emoji_mode: EmojiMode,
}

impl UiState {
    /// Access `emoji_mode`
    #[allow(dead_code, reason = "Accessor method for public API")]
    pub fn emoji_mode(&self) -> &EmojiMode {
        &self.emoji_mode
    }
}

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    #[allow(
        dead_code,
        reason = "Field is part of public API for conversation tracking"
    )]
    /// Role of the speaker
    pub role: ConversationRole,
    #[allow(
        dead_code,
        reason = "Field is part of public API for conversation tracking"
    )]
    /// Text content of the message
    pub text: String,
    #[allow(
        dead_code,
        reason = "Field is part of public API for conversation tracking"
    )]
    /// Timestamp when the entry was created
    pub timestamp: Instant,
}

impl ConversationEntry {
    /// Access role
    #[allow(dead_code, reason = "Accessor method for public API")]
    pub fn role(&self) -> ConversationRole {
        self.role
    }

    /// Access text
    #[allow(dead_code, reason = "Accessor method for public API")]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Access timestamp
    #[allow(dead_code, reason = "Accessor method for public API")]
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }
}

/// Conversation role
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversationRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message
    System,
}
