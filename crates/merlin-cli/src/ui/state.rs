use merlin_routing::TaskId;
use std::collections::{HashMap, HashSet};

/// Maximum number of conversation entries to retain
const MAX_CONVERSATION_HISTORY: usize = 50;

/// Main UI state
#[derive(Default)]
pub struct UiState {
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
    /// Status message to display when processing input
    pub processing_status: Option<String>,
    /// Vertical scroll offset for the output pane
    pub output_scroll_offset: u16,
    /// Background embedding index progress (current, total)
    pub embedding_progress: Option<(u64, u64)>,
    /// Task ID to continue conversation from (when submitting with a task selected)
    pub continuing_conversation_from: Option<TaskId>,
    /// Scroll offset for the task list (0 = bottom/newest, higher = scroll up to older)
    pub task_list_scroll_offset: usize,
    /// Set of expanded conversation IDs (showing child messages)
    pub expanded_conversations: HashSet<TaskId>,
    /// Set of task IDs with expanded steps (showing step details)
    pub expanded_steps: HashSet<TaskId>,
    /// Per-task output scroll positions for preserving scroll when switching tasks
    pub task_output_scroll: HashMap<TaskId, u16>,
    /// Flag to auto-scroll output to bottom on next render
    pub auto_scroll_output_to_bottom: bool,
}

impl UiState {
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
}

/// Conversation entry
#[derive(Clone)]
pub struct ConversationEntry {
    /// Role of the speaker
    pub role: ConversationRole,
    /// Text content of the message
    pub text: String,
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
mod tests {
    use super::*;

    #[test]
    fn test_add_conversation_entry() {
        let mut state = UiState::default();

        let entry = ConversationEntry {
            role: ConversationRole::User,
            text: "Hello".to_owned(),
        };

        state.add_conversation_entry(entry);
        assert_eq!(state.conversation_history.len(), 1);
        assert_eq!(state.conversation_history[0].text, "Hello");
    }

    #[test]
    fn test_conversation_history_limit() {
        let mut state = UiState::default();

        // Add more than MAX_CONVERSATION_HISTORY entries
        for index in 0..(MAX_CONVERSATION_HISTORY + 20) {
            let entry = ConversationEntry {
                role: ConversationRole::User,
                text: format!("Message {index}"),
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
        for index in 0..5 {
            state.add_conversation_entry(ConversationEntry {
                role: ConversationRole::User,
                text: format!("Message {index}"),
            });
        }

        assert_eq!(state.conversation_history.len(), 5);

        // Clear by directly accessing the field
        state.conversation_history.clear();
        assert_eq!(state.conversation_history.len(), 0);
    }

    #[test]
    fn test_conversation_entry_role_accessors() {
        let entry = ConversationEntry {
            role: ConversationRole::Assistant,
            text: "Test".to_owned(),
        };

        // Access fields directly, not as methods
        assert_eq!(entry.role, ConversationRole::Assistant);
        assert_eq!(entry.text, "Test");
    }
}
