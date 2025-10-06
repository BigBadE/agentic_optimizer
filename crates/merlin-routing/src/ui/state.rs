use std::collections::HashSet;
use std::time::Instant;
use crate::TaskId;
use super::text_width::EmojiMode;

/// Main UI state
#[derive(Default)]
pub struct UiState {
    pub selected_task_index: usize,
    pub active_task_id: Option<TaskId>,
    pub active_running_tasks: HashSet<TaskId>,
    pub pending_delete_task_id: Option<TaskId>,
    pub loading_tasks: bool,
    pub conversation_history: Vec<ConversationEntry>,
    pub emoji_mode: EmojiMode,
}

impl UiState {
    /// Access `emoji_mode`
    pub fn emoji_mode(&self) -> &EmojiMode {
        &self.emoji_mode
    }
}

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    pub role: ConversationRole,
    pub text: String,
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
}
