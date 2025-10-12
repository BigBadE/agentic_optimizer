//! Integration tests for conversation context handling
//!
//! These tests verify that conversation history is properly tracked,
//! passed through the system, and used in context building.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::min_ident_chars,
    clippy::missing_panics_doc,
    reason = "Test code is allowed to use expect/unwrap and doesn't need panic docs"
)]

use merlin_core::Query;
use merlin_routing::{ContextFetcher, RoutingConfig, RoutingOrchestrator, Task, UiChannel};
use std::fs;
use tempfile::TempDir;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_conversation_history_in_context() {
    // Create a temp directory for testing
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    // Create test files for context fetching
    fs::create_dir_all(project_root.join("src")).expect("Failed to create src dir");
    fs::write(
        project_root.join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }",
    )
    .expect("Failed to write test file");

    // Test conversation history extraction and context building
    let mut fetcher = ContextFetcher::new(project_root.clone());

    let conversation = vec![
        ("user".to_owned(), "Fix the bug in src/main.rs".to_owned()),
        (
            "assistant".to_owned(),
            "I've fixed the bug in the main function".to_owned(),
        ),
        ("user".to_owned(), "Now add error handling".to_owned()),
    ];

    let current_query = Query::new("Make sure it handles all edge cases");

    // Build context from conversation
    let context = fetcher
        .build_context_from_conversation(&conversation, &current_query)
        .await
        .expect("Failed to build context from conversation");

    // Verify conversation history is in the context
    assert!(
        context.system_prompt.contains("Previous Conversation"),
        "Context should include conversation history section"
    );
    assert!(
        context.system_prompt.contains("Fix the bug in src/main.rs"),
        "Context should include first user message"
    );
    assert!(
        context.system_prompt.contains("Now add error handling"),
        "Context should include second user message"
    );
}

#[tokio::test]
async fn test_empty_conversation_history() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    fs::create_dir_all(project_root.join("src")).expect("Failed to create src dir");

    let mut fetcher = ContextFetcher::new(project_root);

    let empty_conversation: Vec<(String, String)> = vec![];
    let query = Query::new("Simple query");

    // Should work with empty conversation
    let context = fetcher
        .build_context_from_conversation(&empty_conversation, &query)
        .await
        .expect("Failed to build context with empty conversation");

    // Should not have conversation section with empty history
    assert!(
        !context.system_prompt.contains("Previous Conversation")
            || context
                .system_prompt
                .contains("=== Previous Conversation ===\n=== End"),
        "Empty conversation should not add meaningful conversation section"
    );
}

#[tokio::test]
async fn test_conversation_file_reference_extraction() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    // Create multiple test files
    fs::create_dir_all(project_root.join("src")).expect("Failed to create src dir");
    fs::write(project_root.join("src/main.rs"), "fn main() {}").expect("Failed to write main.rs");
    fs::write(project_root.join("src/lib.rs"), "pub mod utils;").expect("Failed to write lib.rs");

    let mut fetcher = ContextFetcher::new(project_root.clone());

    // Conversation mentions multiple files
    let conversation = vec![
        ("user".to_owned(), "Check src/main.rs for issues".to_owned()),
        (
            "assistant".to_owned(),
            "I found an issue in src/lib.rs".to_owned(),
        ),
    ];

    let query = Query::new("Fix both files");

    let context = fetcher
        .build_context_from_conversation(&conversation, &query)
        .await
        .expect("Failed to build context");

    // Conversation history should be present in system prompt
    assert!(
        context.system_prompt.contains("Previous Conversation"),
        "Context should include conversation history"
    );
    assert!(
        context.system_prompt.contains("Check src/main.rs"),
        "Context should include conversation content"
    );
}

#[tokio::test]
async fn test_orchestrator_with_conversation_history() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    fs::create_dir_all(project_root.join("src")).expect("Failed to create src dir");
    fs::write(project_root.join("src/main.rs"), "fn main() {}").expect("Failed to write test file");

    let mut config = RoutingConfig::default();
    config.workspace.root_path = project_root;
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;
    config.tiers.local_enabled = true;

    let orchestrator = RoutingOrchestrator::new(config);

    let task = Task::new("Say hello".to_owned());
    let (sender, _receiver) = mpsc::unbounded_channel();
    let ui_channel = UiChannel::from_sender(sender);

    let conversation_history = vec![
        ("user".to_owned(), "Hi there".to_owned()),
        ("assistant".to_owned(), "Hello! How can I help?".to_owned()),
    ];

    // This should not panic - it should handle conversation history
    let result = orchestrator
        .execute_task_streaming_with_history(task, ui_channel, conversation_history)
        .await;

    // We expect this to succeed or fail gracefully (local model might not be available)
    match result {
        Ok(_) => {
            // Success - conversation history was processed
        }
        Err(error) => {
            // Acceptable errors: model not available, connection issues
            let error_str = error.to_string().to_lowercase();
            assert!(
                error_str.contains("ollama")
                    || error_str.contains("connection")
                    || error_str.contains("not available")
                    || error_str.contains("model"),
                "Unexpected error: {error}"
            );
        }
    }
}

#[tokio::test]
async fn test_context_fetcher_conversation_limit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    let mut fetcher = ContextFetcher::new(project_root);

    // Create a very long conversation (100 messages)
    let mut long_conversation = Vec::new();
    for i in 0..100 {
        long_conversation.push(("user".to_owned(), format!("Message number {i}")));
    }

    let query = Query::new("Final question");

    // Should handle long conversations without panicking
    let context = fetcher
        .build_context_from_conversation(&long_conversation, &query)
        .await
        .expect("Failed to build context with long conversation");

    // Context should be built successfully
    assert!(
        context.system_prompt.contains("Previous Conversation"),
        "Long conversation should be included in context"
    );

    // Verify all messages are present (or at least a reasonable number)
    let message_count = (0..100)
        .filter(|i| {
            context
                .system_prompt
                .contains(&format!("Message number {i}"))
        })
        .count();

    assert!(
        message_count >= 50,
        "Should include at least 50 messages from long conversation, got {message_count}"
    );
}

#[tokio::test]
async fn test_conversation_history_ordering() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    let mut fetcher = ContextFetcher::new(project_root);

    let conversation = vec![
        ("user".to_owned(), "First message".to_owned()),
        ("assistant".to_owned(), "First response".to_owned()),
        ("user".to_owned(), "Second message".to_owned()),
        ("assistant".to_owned(), "Second response".to_owned()),
    ];

    let query = Query::new("Third message");

    let context = fetcher
        .build_context_from_conversation(&conversation, &query)
        .await
        .expect("Failed to build context");

    // Verify messages appear in order
    let prompt = &context.system_prompt;
    let first_pos = prompt
        .find("First message")
        .expect("First message not found");
    let second_pos = prompt
        .find("Second message")
        .expect("Second message not found");
    let first_resp_pos = prompt
        .find("First response")
        .expect("First response not found");
    let second_resp_pos = prompt
        .find("Second response")
        .expect("Second response not found");

    assert!(
        first_pos < first_resp_pos,
        "First message should come before first response"
    );
    assert!(
        first_resp_pos < second_pos,
        "First response should come before second message"
    );
    assert!(
        second_pos < second_resp_pos,
        "Second message should come before second response"
    );
}
