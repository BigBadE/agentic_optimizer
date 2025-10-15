//! Tests for conversation continuation functionality
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_routing::TaskId;
use merlin_routing::user_interface::state::{ConversationEntry, ConversationRole, UiState};
use std::time::Instant;

#[test]
fn test_continuing_conversation_from_field() {
    let mut state = UiState::default();

    // Initially None
    assert_eq!(state.continuing_conversation_from, None);

    // Set a task ID
    let task_id = TaskId::default();
    state.continuing_conversation_from = Some(task_id);
    assert_eq!(state.continuing_conversation_from, Some(task_id));

    // Take it
    let taken = state.continuing_conversation_from.take();
    assert_eq!(taken, Some(task_id));
    assert_eq!(state.continuing_conversation_from, None);
}

#[test]
fn test_conversation_entry_creation() {
    let entry = ConversationEntry {
        role: ConversationRole::User,
        text: "Test message".to_string(),
        timestamp: Instant::now(),
    };

    assert_eq!(entry.role, ConversationRole::User);
    assert_eq!(entry.text, "Test message");
}

#[test]
fn test_add_conversation_entry() {
    let mut state = UiState::default();

    state.add_conversation_entry(ConversationEntry {
        role: ConversationRole::User,
        text: "User message".to_string(),
        timestamp: Instant::now(),
    });

    state.add_conversation_entry(ConversationEntry {
        role: ConversationRole::Assistant,
        text: "Assistant response".to_string(),
        timestamp: Instant::now(),
    });

    assert_eq!(state.conversation_history.len(), 2);
    assert_eq!(state.conversation_history[0].role, ConversationRole::User);
    assert_eq!(
        state.conversation_history[1].role,
        ConversationRole::Assistant
    );
}
