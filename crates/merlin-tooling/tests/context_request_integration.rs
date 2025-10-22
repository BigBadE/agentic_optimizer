//! Integration tests for `ContextRequestTool`.
//!
//! Tests the tool's integration with TypeScript runtime, file finding,
//! and context tracking functionality.

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

use merlin_tooling::{
    ContextRequestResult, ContextRequestTool, ContextTracker, Tool, ToolInput, TypeScriptRuntime,
};
use serde_json::{Value, from_value};
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create a realistic project structure
fn create_test_project() -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let project_root = temp_dir.path();

    // Create directory structure
    let src_dir = project_root.join("src");
    create_dir_all(&src_dir).expect("Failed to create src dir");

    let tests_dir = project_root.join("tests");
    create_dir_all(&tests_dir).expect("Failed to create tests dir");

    let docs_dir = project_root.join("docs");
    create_dir_all(&docs_dir).expect("Failed to create docs dir");

    // Create source files
    write(
        src_dir.join("lib.rs"),
        r"//! Main library
pub mod executor;
pub mod parser;
",
    )
    .expect("Failed to write lib.rs");

    write(
        src_dir.join("executor.rs"),
        r"//! Executor module
pub struct Executor;
",
    )
    .expect("Failed to write executor.rs");

    write(
        src_dir.join("parser.rs"),
        r"//! Parser module
pub struct Parser;
",
    )
    .expect("Failed to write parser.rs");

    write(
        src_dir.join("utils.rs"),
        r"//! Utility functions
pub fn helper() {}
",
    )
    .expect("Failed to write utils.rs");

    // Create test files
    write(
        tests_dir.join("integration.rs"),
        r"#[test]
fn test_integration() {}
",
    )
    .expect("Failed to write integration.rs");

    // Create doc files
    write(docs_dir.join("README.md"), "# Project Documentation")
        .expect("Failed to write README.md");

    write(
        docs_dir.join("DESIGN.md"),
        "# Design Document\n\nArchitecture details...",
    )
    .expect("Failed to write DESIGN.md");

    // Create config files
    write(
        project_root.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
"#,
    )
    .expect("Failed to write Cargo.toml");

    temp_dir
}

#[tokio::test]
async fn test_context_request_exact_path() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "src/lib.rs",
            "reason": "Need to see library structure"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(output.success, "Should successfully find exact file");

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(result.success);
    assert_eq!(result.files.len(), 1);
    assert!(result.files[0].path.ends_with("lib.rs"));
    assert!(result.files[0].content.contains("Main library"));
}

#[tokio::test]
async fn test_context_request_glob_pattern_all_rust() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "**/*.rs",
            "reason": "Need all Rust files",
            "max_files": 10
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(output.success);

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(result.success);
    assert!(result.files.len() >= 4, "Should find at least 4 .rs files");

    // Verify all returned files are .rs
    for file in &result.files {
        assert!(file.path.extension().is_some_and(|ext| ext == "rs"));
    }
}

#[tokio::test]
async fn test_context_request_glob_pattern_markdown() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "**/*.md",
            "reason": "Need documentation files",
            "max_files": 5
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(output.success);

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(result.success);
    assert!(result.files.len() >= 2, "Should find at least 2 .md files");

    // Verify we got markdown files
    let has_readme = result.files.iter().any(|f| f.path.ends_with("README.md"));
    assert!(has_readme, "Should include README.md");
}

#[tokio::test]
async fn test_context_request_directory_specific() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "src/**/*.rs",
            "reason": "Need source files only",
            "max_files": 10
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(output.success);

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(result.success);

    // All files should be from src directory
    for file in &result.files {
        let path_str = file.path.to_string_lossy();
        assert!(
            path_str.contains("src"),
            "File should be from src directory: {path_str}"
        );
    }
}

#[tokio::test]
async fn test_context_request_max_files_limit() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "**/*.rs",
            "reason": "Testing max_files limit",
            "max_files": 2
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(output.success);

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(result.success);
    assert!(
        result.files.len() <= 2,
        "Should respect max_files limit of 2"
    );
}

#[tokio::test]
async fn test_context_request_no_matches() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "**/*.xyz",
            "reason": "Looking for non-existent file type"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    // Should return error result
    assert!(!output.success, "Should fail when no files found");
}

#[tokio::test]
async fn test_context_request_nonexistent_file() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "nonexistent.rs",
            "reason": "Looking for file that doesn't exist"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    assert!(!output.success, "Should fail for nonexistent file");
}

#[tokio::test]
async fn test_context_tracker_integration() {
    let temp_dir = create_test_project();
    let tracker = ContextTracker::new();
    let tool = ContextRequestTool::with_tracker(temp_dir.path().to_path_buf(), tracker.clone());

    // Request first file
    let input1 = ToolInput {
        params: serde_json::json!({
            "pattern": "src/lib.rs",
            "reason": "First request"
        }),
    };

    tool.execute(input1).await.expect("Tool execution failed");

    // Request second file
    let input2 = ToolInput {
        params: serde_json::json!({
            "pattern": "src/executor.rs",
            "reason": "Second request"
        }),
    };

    tool.execute(input2).await.expect("Tool execution failed");

    // Check tracker
    let requested = tracker.get_requested().await;

    assert!(requested.len() >= 2, "Should track both requests");
    assert!(
        requested.iter().any(|path| path.ends_with("lib.rs")),
        "Should track lib.rs"
    );
    assert!(
        requested.iter().any(|path| path.ends_with("executor.rs")),
        "Should track executor.rs"
    );
}

#[tokio::test]
async fn test_context_tracker_deduplication() {
    let temp_dir = create_test_project();
    let tracker = ContextTracker::new();
    let tool = ContextRequestTool::with_tracker(temp_dir.path().to_path_buf(), tracker.clone());

    // Request same file twice
    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "src/lib.rs",
            "reason": "Requesting same file"
        }),
    };

    tool.execute(input.clone())
        .await
        .expect("First execution failed");
    tool.execute(input).await.expect("Second execution failed");

    // Check tracker - should only have one entry
    let requested = tracker.get_requested().await;

    let lib_count = requested
        .iter()
        .filter(|path| path.ends_with("lib.rs"))
        .count();

    assert_eq!(lib_count, 1, "Should deduplicate same file request");
}

#[tokio::test]
async fn test_context_tracker_clear() {
    let tracker = ContextTracker::new();

    tracker.add_requested(PathBuf::from("file1.rs")).await;
    tracker.add_requested(PathBuf::from("file2.rs")).await;

    let before_clear = tracker.get_requested().await;
    assert_eq!(before_clear.len(), 2);

    tracker.clear().await;

    let after_clear = tracker.get_requested().await;
    assert_eq!(
        after_clear.len(),
        0,
        "Clear should remove all tracked files"
    );
}

#[tokio::test]
async fn test_context_request_file_size_metadata() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "src/lib.rs",
            "reason": "Check file metadata"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(!result.files.is_empty());

    let file = &result.files[0];
    assert!(file.size > 0, "File size should be tracked");
    assert_eq!(
        file.size,
        file.content.len(),
        "Size should match content length"
    );
}

#[tokio::test]
async fn test_context_request_max_file_size_limit() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create a large file
    let large_content = "x".repeat(200_000); // 200KB
    write(temp_dir.path().join("large.txt"), large_content).expect("Failed to write large file");

    // Create tool with small max file size
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf()).with_max_file_size(100_000);

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "large.txt",
            "reason": "Testing file size limit"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    // Should fail or return empty result due to size limit
    assert!(!output.success, "Should fail for oversized file");
}

#[tokio::test]
async fn test_context_request_typescript_signature() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    // Verify TypeScript signature is valid
    let signature = tool.typescript_signature();

    assert!(signature.contains("requestContext"));
    assert!(signature.contains("pattern: string"));
    assert!(signature.contains("reason: string"));
    assert!(signature.contains("max_files?"));
    assert!(signature.contains("Promise<"));
}

#[tokio::test]
async fn test_context_request_with_typescript_runtime() {
    let temp_dir = create_test_project();
    let tool = Arc::new(ContextRequestTool::new(temp_dir.path().to_path_buf())) as Arc<dyn Tool>;

    let mut runtime = TypeScriptRuntime::new();

    // Register the tool
    runtime.register_tool(Arc::clone(&tool));

    // Execute TypeScript code that calls the tool
    let code = r#"
const result = await requestContext("src/lib.rs", "Testing from TypeScript");
return {
    done: true,
    found: result.success,
    fileCount: result.files.length
};
"#;

    let result = runtime.execute(code).await.expect("Execution failed");

    // Verify the result
    let result_obj = result.as_object().expect("Result should be object");

    assert_eq!(result_obj.get("done"), Some(&serde_json::json!(true)));
    assert_eq!(result_obj.get("found"), Some(&serde_json::json!(true)));
    assert!(result_obj.get("fileCount").is_some());
}

#[tokio::test]
async fn test_context_request_glob_with_typescript() {
    let temp_dir = create_test_project();
    let tool = Arc::new(ContextRequestTool::new(temp_dir.path().to_path_buf()));

    let mut runtime = TypeScriptRuntime::new();
    runtime.register_tool(tool);

    let code = r#"
const result = await requestContext("**/*.rs", "Get all Rust files", 10);
return {
    done: true,
    success: result.success,
    count: result.files.length,
    message: result.message
};
"#;

    let result = runtime.execute(code).await.expect("Execution failed");

    let result_obj = result.as_object().expect("Result should be object");

    assert_eq!(result_obj.get("success"), Some(&serde_json::json!(true)));

    let count = result_obj
        .get("count")
        .and_then(Value::as_u64)
        .expect("Should have count");

    assert!(count >= 4, "Should find multiple Rust files");
}

#[tokio::test]
async fn test_context_request_error_handling() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    // Test invalid JSON params
    let input = ToolInput {
        params: serde_json::json!({
            "invalid_field": "value"
        }),
    };

    let result = tool.execute(input).await;

    assert!(result.is_err(), "Should fail with invalid input parameters");
}

#[tokio::test]
async fn test_context_request_content_verification() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "src/executor.rs",
            "reason": "Verify content is read correctly"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    assert!(!result.files.is_empty());

    let file = &result.files[0];
    assert!(file.content.contains("Executor module"));
    assert!(file.content.contains("pub struct Executor"));
}

#[tokio::test]
async fn test_tool_name_and_description() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    assert_eq!(tool.name(), "requestContext");

    let description = tool.description();
    assert!(description.contains("additional context"));
    assert!(description.contains("glob"));
}

#[tokio::test]
async fn test_context_request_default_max_files() {
    let temp_dir = create_test_project();
    let tool = ContextRequestTool::new(temp_dir.path().to_path_buf());

    // Don't specify max_files - should use default
    let input = ToolInput {
        params: serde_json::json!({
            "pattern": "**/*.rs",
            "reason": "Testing default max_files"
        }),
    };

    let output = tool.execute(input).await.expect("Tool execution failed");

    let result: ContextRequestResult =
        from_value(output.data.expect("No data")).expect("Failed to deserialize result");

    // Default is 5, but project might have fewer files
    assert!(
        result.files.len() <= 5,
        "Should respect default max_files of 5"
    );
}
