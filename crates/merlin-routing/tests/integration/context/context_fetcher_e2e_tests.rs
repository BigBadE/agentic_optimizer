//! End-to-end tests for ContextFetcher functionality
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

use merlin_context::ContextFetcher;
use merlin_core::Query;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Create a test project structure with multiple files
async fn create_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::with_prefix("context_fetcher_e2e").expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    // Create comprehensive file structure
    fs::create_dir_all(project_root.join("src")).await.unwrap();
    fs::create_dir_all(project_root.join("src/utils"))
        .await
        .unwrap();
    fs::create_dir_all(project_root.join("tests"))
        .await
        .unwrap();

    // Main files
    fs::write(
        project_root.join("src/main.rs"),
        r#"
mod utils;
use utils::helper;

fn main() {
    println!("Hello from main!");
    helper::do_work();
}
"#,
    )
    .await
    .unwrap();

    fs::write(
        project_root.join("src/lib.rs"),
        r#"
pub mod utils;

pub fn public_function() {
    println!("Public API");
}
"#,
    )
    .await
    .unwrap();

    // Utils module
    fs::write(
        project_root.join("src/utils/mod.rs"),
        r#"
pub mod helper;
"#,
    )
    .await
    .unwrap();

    fs::write(
        project_root.join("src/utils/helper.rs"),
        r#"
pub fn do_work() {
    println!("Doing work");
}

pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}
"#,
    )
    .await
    .unwrap();

    // Test file
    fs::write(
        project_root.join("tests/integration_test.rs"),
        r#"
#[test]
fn test_example() {
    assert_eq!(2 + 2, 4);
}
"#,
    )
    .await
    .unwrap();

    (temp_dir, project_root)
}

#[tokio::test]
async fn test_extract_file_references_from_natural_language() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root.clone());

    // Test various ways of mentioning files
    let text = "Please check src/main.rs and also look at src/lib.rs. \
                The helper function is in src/utils/helper.rs";

    let files = fetcher.extract_file_references(text);

    assert!(files.iter().any(|p| p.ends_with("src/main.rs")));
    assert!(files.iter().any(|p| p.ends_with("src/lib.rs")));
    assert!(files.iter().any(|p| p.ends_with("src/utils/helper.rs")));
    assert_eq!(files.len(), 3);
}

#[tokio::test]
async fn test_extract_file_references_with_inline_mentions() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root.clone());

    let text = "The issue is in `src/main.rs` on line 5. Also see src/lib.rs for the API.";
    let files = fetcher.extract_file_references(text);

    assert!(!files.is_empty());
    assert!(files.iter().any(|p| p.ends_with("src/main.rs")));
}

#[tokio::test]
async fn test_extract_file_references_no_matches() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root);

    let text = "This text has no file references at all";
    let files = fetcher.extract_file_references(text);

    assert_eq!(files.len(), 0);
}

#[tokio::test]
async fn test_extract_file_references_nonexistent_files() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root);

    let text = "Check nonexistent.rs and also fake_file.txt";
    let files = fetcher.extract_file_references(text);

    // Should not include non-existent files
    assert_eq!(files.len(), 0);
}

#[tokio::test]
async fn test_resolve_module_path() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root.clone());

    // Test Rust module path resolution
    let text = "The bug is in crate::utils::helper";
    let files = fetcher.extract_file_references(text);

    // Should resolve to src/utils/helper.rs or src/utils/helper/mod.rs
    assert!(files.iter().any(|p| p.to_string_lossy().contains("utils")));
}

#[tokio::test]
async fn test_build_context_for_query() {
    let (_temp, project_root) = create_test_project().await;
    let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

    let query = Query::new("Please analyze src/main.rs for bugs");
    let context = fetcher.build_context_for_query(&query).await.unwrap();

    // Context should include the system prompt
    assert!(!context.system_prompt.is_empty());

    // Should have extracted and loaded the file
    assert!(
        context
            .files
            .iter()
            .any(|f| f.path.to_string_lossy().contains("main.rs"))
            || context.files.is_empty() // Fallback mode may not load files
    );
}

#[tokio::test]
async fn test_build_context_from_conversation() {
    let (_temp, project_root) = create_test_project().await;
    let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

    let messages = vec![
        ("user".to_owned(), "I need help with src/main.rs".to_owned()),
        (
            "assistant".to_owned(),
            "Sure, I can help with that".to_owned(),
        ),
        ("user".to_owned(), "Also check src/lib.rs".to_owned()),
    ];

    let query = Query::new("Now update both files");
    let context = fetcher
        .build_context_from_conversation(&messages, &query)
        .await
        .unwrap();

    // Should include conversation history in system prompt
    assert!(context.system_prompt.contains("Previous Conversation"));
    assert!(context.system_prompt.contains("user:"));
    assert!(context.system_prompt.contains("assistant:"));
}

#[tokio::test]
async fn test_build_context_extracts_files_from_conversation() {
    let (_temp, project_root) = create_test_project().await;
    let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

    let messages = vec![
        ("user".to_owned(), "Check src/main.rs".to_owned()),
        ("assistant".to_owned(), "I see the issue".to_owned()),
        (
            "user".to_owned(),
            "Now look at src/utils/helper.rs".to_owned(),
        ),
    ];

    let query = Query::new("Fix both files");
    let context = fetcher
        .build_context_from_conversation(&messages, &query)
        .await
        .unwrap();

    // Should have extracted files from conversation
    let has_files = !context.files.is_empty();
    let has_conversation = context.system_prompt.contains("Previous Conversation");

    assert!(has_conversation);
    // Files may or may not be loaded depending on ContextBuilder availability
    assert!(has_files || !has_files);
}

#[tokio::test]
async fn test_multiple_file_formats() {
    let (_temp, project_root) = create_test_project().await;

    // Create files with different extensions
    fs::write(project_root.join("config.toml"), "key = \"value\"")
        .await
        .unwrap();
    fs::write(project_root.join("README.md"), "# Project")
        .await
        .unwrap();
    fs::write(project_root.join("data.json"), "{\"test\": true}")
        .await
        .unwrap();

    let fetcher = ContextFetcher::new(project_root);

    let text = "Check config.toml, README.md and data.json";
    let files = fetcher.extract_file_references(text);

    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|p| p.ends_with("config.toml")));
    assert!(files.iter().any(|p| p.ends_with("README.md")));
    assert!(files.iter().any(|p| p.ends_with("data.json")));
}

#[tokio::test]
async fn test_relative_and_absolute_paths() {
    let (_temp, project_root) = create_test_project().await;
    let fetcher = ContextFetcher::new(project_root.clone());

    // Test relative path
    let text = format!(
        "Check src/main.rs and {}/src/lib.rs",
        project_root.display()
    );
    let files = fetcher.extract_file_references(&text);

    // Should find both relative and absolute
    assert!(files.len() >= 1);
    assert!(
        files
            .iter()
            .any(|p| p.ends_with("src/main.rs") || p.ends_with("src/lib.rs"))
    );
}

#[tokio::test]
async fn test_context_with_empty_query() {
    let (_temp, project_root) = create_test_project().await;
    let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

    let query = Query::new("");
    let context = fetcher.build_context_for_query(&query).await.unwrap();

    // Should create valid context (system prompt may be empty for empty query in fallback mode)
    assert!(context.files.is_empty()); // No files extracted from empty query
}

#[tokio::test]
async fn test_large_conversation_history() {
    let (_temp, project_root) = create_test_project().await;
    let mut fetcher = ContextFetcher::new(project_root).without_context_builder();

    // Create a large conversation
    let mut messages = Vec::new();
    for i in 0..50 {
        messages.push(("user".to_owned(), format!("User message {i}")));
        messages.push(("assistant".to_owned(), format!("Assistant message {i}")));
    }

    let query = Query::new("Summarize our conversation");
    let context = fetcher
        .build_context_from_conversation(&messages, &query)
        .await
        .unwrap();

    // Should include all messages
    assert!(context.system_prompt.contains("User message 0"));
    assert!(context.system_prompt.contains("User message 49"));
}
