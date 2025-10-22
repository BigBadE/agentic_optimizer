//! End-to-end tests for all tool operations
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

use merlin_tooling::{ToolInput, ToolRegistry};
use serde_json::json;
use std::path::PathBuf;
use tempfile::TempDir;
use tokio::fs;

/// Create a test project with files for tool operations
async fn create_test_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::with_prefix("tools_e2e").expect("Failed to create temp dir");
    let project_root = temp_dir.path().to_path_buf();

    fs::create_dir_all(project_root.join("src")).await.unwrap();
    fs::write(
        project_root.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello\");\n}\n",
    )
    .await
    .unwrap();

    fs::write(
        project_root.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )
    .await
    .unwrap();

    (temp_dir, project_root)
}

#[tokio::test]
async fn test_show_tool_reads_file() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": project_root.join("src/main.rs"),
        }),
    };

    let result = registry.execute("show", input).await.unwrap();
    assert!(result.success);
    assert!(result.message.contains("Showing lines"));

    let data = result.data.unwrap();
    let content = data["content"].as_str().unwrap();
    assert!(content.contains("fn main()"));
    assert!(content.contains("println!"));
}

#[tokio::test]
async fn test_show_tool_with_line_range() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": project_root.join("src/main.rs"),
            "start_line": 1,
            "end_line": 2,
        }),
    };

    let result = registry.execute("show", input).await.unwrap();
    assert!(result.success);

    let data = result.data.unwrap();
    let content = data["content"].as_str().unwrap();
    assert!(content.contains("fn main()"));
}

#[tokio::test]
async fn test_show_tool_nonexistent_file() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": project_root.join("nonexistent.rs"),
        }),
    };

    let result = registry.execute("show", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_edit_tool_replaces_text() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let file_path = project_root.join("src/lib.rs");

    let input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
            "old_string": "a + b",
            "new_string": "a * b",
        }),
    };

    let result = registry.execute("edit", input).await.unwrap();
    assert!(result.success);

    // Verify the change
    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("a * b"));
    assert!(!content.contains("a + b"));
}

#[tokio::test]
async fn test_edit_tool_replace_all() {
    let (_temp, project_root) = create_test_project().await;

    // Create file with multiple occurrences
    let file_path = project_root.join("test.txt");
    fs::write(&file_path, "foo bar foo baz foo").await.unwrap();

    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
            "old_string": "foo",
            "new_string": "FOO",
            "replace_all": true,
        }),
    };

    let result = registry.execute("edit", input).await.unwrap();
    assert!(result.success);

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert_eq!(content, "FOO bar FOO baz FOO");
}

#[tokio::test]
async fn test_edit_tool_fails_on_multiple_matches_without_replace_all() {
    let (_temp, project_root) = create_test_project().await;

    let file_path = project_root.join("test.txt");
    fs::write(&file_path, "foo bar foo").await.unwrap();

    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": file_path,
            "old_string": "foo",
            "new_string": "FOO",
        }),
    };

    let result = registry.execute("edit", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_tool_removes_file() {
    let (_temp, project_root) = create_test_project().await;

    let file_path = project_root.join("to_delete.txt");
    fs::write(&file_path, "temporary file").await.unwrap();
    assert!(file_path.exists());

    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
        }),
    };

    let result = registry.execute("delete", input).await.unwrap();
    assert!(result.success);
    assert!(!file_path.exists());
}

#[tokio::test]
async fn test_delete_tool_nonexistent_file() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": project_root.join("nonexistent.txt"),
        }),
    };

    let result = registry.execute("delete", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_tool_refuses_directory() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "file_path": project_root.join("src"),
        }),
    };

    let result = registry.execute("delete", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_tool_shows_directory_contents() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "directory_path": project_root.join("src"),
        }),
    };

    let result = registry.execute("list", input).await.unwrap();
    assert!(result.success);

    let data = result.data.unwrap();
    let files = data["files"].as_array().unwrap();
    let dirs = data["directories"].as_array().unwrap();

    assert_eq!(files.len(), 2); // main.rs and lib.rs
    assert!(files.iter().any(|f| f.as_str() == Some("main.rs")));
    assert!(files.iter().any(|f| f.as_str() == Some("lib.rs")));
    assert_eq!(dirs.len(), 0);
}

#[tokio::test]
async fn test_list_tool_includes_subdirectories() {
    let (_temp, project_root) = create_test_project().await;

    fs::create_dir(project_root.join("src/subdir"))
        .await
        .unwrap();

    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "directory_path": project_root.join("src"),
        }),
    };

    let result = registry.execute("list", input).await.unwrap();

    let data = result.data.unwrap();
    let dirs = data["directories"].as_array().unwrap();

    assert!(dirs.iter().any(|d| d.as_str() == Some("subdir")));
}

#[tokio::test]
async fn test_list_tool_hidden_files() {
    let (_temp, project_root) = create_test_project().await;

    fs::write(project_root.join(".hidden"), "secret")
        .await
        .unwrap();

    let registry = ToolRegistry::default();

    // Without include_hidden
    let input = ToolInput {
        params: json!({
            "directory_path": project_root.clone(),
            "include_hidden": false,
        }),
    };

    let result = registry.execute("list", input).await.unwrap();
    let data = result.data.unwrap();
    let files = data["files"].as_array().unwrap();

    assert!(!files.iter().any(|f| f.as_str() == Some(".hidden")));

    // With include_hidden
    let input = ToolInput {
        params: json!({
            "directory_path": project_root,
            "include_hidden": true,
        }),
    };

    let result = registry.execute("list", input).await.unwrap();
    let data = result.data.unwrap();
    let files = data["files"].as_array().unwrap();

    assert!(files.iter().any(|f| f.as_str() == Some(".hidden")));
}

#[tokio::test]
async fn test_list_tool_nonexistent_directory() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "directory_path": project_root.join("nonexistent"),
        }),
    };

    let result = registry.execute("list", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_tool_on_file_fails() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let input = ToolInput {
        params: json!({
            "directory_path": project_root.join("src/main.rs"),
        }),
    };

    let result = registry.execute("list", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_tool_registry_lists_all_tools() {
    let registry = ToolRegistry::default();
    let tools = registry.list_tools();

    assert!(tools.iter().any(|(name, _desc)| *name == "show"));
    assert!(tools.iter().any(|(name, _desc)| *name == "edit"));
    assert!(tools.iter().any(|(name, _desc)| *name == "delete"));
    assert!(tools.iter().any(|(name, _desc)| *name == "list"));
    assert!(tools.iter().any(|(name, _desc)| *name == "bash"));

    assert!(tools.len() >= 5);
}

#[tokio::test]
async fn test_tool_registry_unknown_tool() {
    let registry = ToolRegistry::default();

    let input = ToolInput { params: json!({}) };

    let result = registry.execute("nonexistent_tool", input).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_complete_workflow_read_edit_verify() {
    let (_temp, project_root) = create_test_project().await;
    let registry = ToolRegistry::default();

    let file_path = project_root.join("src/lib.rs");

    // Step 1: Read the file
    let show_input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
        }),
    };

    let show_result = registry.execute("show", show_input).await.unwrap();
    assert!(show_result.success);
    let original_content = show_result.data.unwrap()["content"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(original_content.contains("a + b"));

    // Step 2: Edit the file
    let edit_input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
            "old_string": "a + b",
            "new_string": "a - b",
        }),
    };

    let edit_result = registry.execute("edit", edit_input).await.unwrap();
    assert!(edit_result.success);

    // Step 3: Read again to verify
    let verify_input = ToolInput {
        params: json!({
            "file_path": file_path,
        }),
    };

    let verify_result = registry.execute("show", verify_input).await.unwrap();
    assert!(verify_result.success);
    let verify_data = verify_result.data.unwrap();
    let new_content = verify_data["content"].as_str().unwrap();
    assert!(new_content.contains("a - b"));
    assert!(!new_content.contains("a + b"));
}

#[tokio::test]
async fn test_complete_workflow_create_list_delete() {
    let (_temp, project_root) = create_test_project().await;

    // Create some test files
    for i in 1..=5 {
        fs::write(
            project_root.join(format!("test{i}.txt")),
            format!("Test file {i}"),
        )
        .await
        .unwrap();
    }

    let registry = ToolRegistry::default();

    // Step 1: List files
    let list_input = ToolInput {
        params: json!({
            "directory_path": project_root.clone(),
        }),
    };

    let list_result = registry.execute("list", list_input).await.unwrap();
    let files = list_result.data.unwrap()["files"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(files.len(), 5);

    // Step 2: Delete all test files
    for i in 1..=5 {
        let delete_input = ToolInput {
            params: json!({
                "file_path": project_root.join(format!("test{i}.txt")),
            }),
        };

        let delete_result = registry.execute("delete", delete_input).await.unwrap();
        assert!(delete_result.success);
    }

    // Step 3: List again to verify deletion
    let verify_input = ToolInput {
        params: json!({
            "directory_path": project_root,
        }),
    };

    let verify_result = registry.execute("list", verify_input).await.unwrap();
    let remaining_files = verify_result.data.unwrap()["files"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(remaining_files.len(), 0);
}

#[tokio::test]
async fn test_edit_preserves_file_structure() {
    let (_temp, project_root) = create_test_project().await;

    let file_path = project_root.join("multiline.txt");
    let original = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\n";
    fs::write(&file_path, original).await.unwrap();

    let registry = ToolRegistry::default();

    // Edit middle line
    let input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
            "old_string": "Line 3",
            "new_string": "Modified Line 3",
        }),
    };

    registry.execute("edit", input).await.unwrap();

    let content = fs::read_to_string(&file_path).await.unwrap();
    assert!(content.contains("Line 1"));
    assert!(content.contains("Line 2"));
    assert!(content.contains("Modified Line 3"));
    assert!(content.contains("Line 4"));
    assert!(content.contains("Line 5"));
}

#[tokio::test]
async fn test_tools_handle_unicode() {
    let (_temp, project_root) = create_test_project().await;

    let file_path = project_root.join("unicode.txt");
    let unicode_content = "Hello 世界 🌍\nRust 🦀 is great!\n";
    fs::write(&file_path, unicode_content).await.unwrap();

    let registry = ToolRegistry::default();

    // Show tool with unicode
    let show_input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
        }),
    };

    let show_result = registry.execute("show", show_input).await.unwrap();
    assert!(show_result.success);
    let show_data = show_result.data.unwrap();
    let content = show_data["content"].as_str().unwrap();
    assert!(content.contains("世界"));
    assert!(content.contains("🌍"));
    assert!(content.contains("🦀"));

    // Edit with unicode
    let edit_input = ToolInput {
        params: json!({
            "file_path": file_path.clone(),
            "old_string": "世界",
            "new_string": "World",
        }),
    };

    let edit_result = registry.execute("edit", edit_input).await.unwrap();
    assert!(edit_result.success);

    let final_content = fs::read_to_string(&file_path).await.unwrap();
    assert!(final_content.contains("World"));
    assert!(!final_content.contains("世界"));
}
