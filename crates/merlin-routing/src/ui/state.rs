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

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    pub role: ConversationRole,
    pub text: String,
    pub timestamp: Instant,
}

/// Conversation role
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ConversationRole {
    User,
    Assistant,
    System,
}
