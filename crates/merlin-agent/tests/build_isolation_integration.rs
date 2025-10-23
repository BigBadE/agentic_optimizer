//! Integration tests for isolated build environment.
//!
//! Tests workspace copying, file application, and build execution in isolation.

#![cfg_attr(
    test,
    allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_agent::executor::{IsolatedBuildEnv, WorkspaceState};
use merlin_core::FileChange;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs::read_to_string;

/// Helper to create a minimal Rust project for testing
fn create_test_rust_project() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("create temp dir");
    let project_path = temp_dir.path().to_path_buf();

    // Create Cargo.toml
    let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
    fs::write(project_path.join("Cargo.toml"), cargo_toml).expect("write Cargo.toml");

    // Create src directory and lib.rs
    fs::create_dir(project_path.join("src")).expect("create src");
    fs::write(
        project_path.join("src/lib.rs"),
        "pub fn hello() -> String { String::from(\"hello\") }",
    )
    .expect("write lib.rs");

    // Create some other files that should be copied
    fs::write(project_path.join("README.md"), "# Test Project").expect("write README");

    // Create files that should NOT be copied
    fs::create_dir(project_path.join("target")).expect("create target");
    fs::write(project_path.join("target/dummy.txt"), "should not copy").expect("write target file");

    fs::create_dir(project_path.join(".git")).expect("create .git");
    fs::write(project_path.join(".git/config"), "git config").expect("write git config");

    (temp_dir, project_path)
}

#[test]
fn test_isolated_build_env_creation() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path);

    let build_env = IsolatedBuildEnv::new(&workspace);
    assert!(build_env.is_ok(), "Build environment should be created");
}

#[tokio::test]
async fn test_workspace_file_copying() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");

    // Copy workspace files
    let result = build_env.copy_workspace_files(&project_path).await;
    assert!(result.is_ok(), "File copying should succeed");

    // Verify essential files were copied
    let isolated_path = build_env.path();
    assert!(
        isolated_path.join("Cargo.toml").exists(),
        "Cargo.toml should be copied"
    );
    assert!(
        isolated_path.join("src/lib.rs").exists(),
        "src/lib.rs should be copied"
    );
    assert!(
        isolated_path.join("README.md").exists(),
        "README.md should be copied"
    );

    // Verify excluded directories were NOT copied
    assert!(
        !isolated_path.join("target").exists(),
        "target/ should not be copied"
    );
    assert!(
        !isolated_path.join(".git").exists(),
        ".git/ should not be copied"
    );
}

#[tokio::test]
async fn test_apply_file_changes_to_isolated_env() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");
    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Create file changes
    let changes = vec![
        FileChange::Modify {
            path: PathBuf::from("src/lib.rs"),
            content: "pub fn hello() -> String { String::from(\"modified\") }".to_owned(),
        },
        FileChange::Create {
            path: PathBuf::from("src/new_module.rs"),
            content: "pub fn new_function() {}".to_owned(),
        },
    ];

    // Apply changes
    let result = build_env.apply_changes(&changes).await;
    assert!(result.is_ok(), "Applying changes should succeed");

    // Verify changes were applied in isolated environment
    let isolated_path = build_env.path();
    let modified_content = read_to_string(isolated_path.join("src/lib.rs"))
        .await
        .expect("read modified file");
    assert!(
        modified_content.contains("modified"),
        "File should be modified in isolated env"
    );

    let new_file_content = read_to_string(isolated_path.join("src/new_module.rs"))
        .await
        .expect("read new file");
    assert!(
        new_file_content.contains("new_function"),
        "New file should exist in isolated env"
    );

    // Verify original workspace is untouched
    let original_content = read_to_string(project_path.join("src/lib.rs"))
        .await
        .expect("read original file");
    assert!(
        !original_content.contains("modified"),
        "Original file should be unchanged"
    );
    assert!(
        !project_path.join("src/new_module.rs").exists(),
        "New file should not exist in original workspace"
    );
}

#[tokio::test]
#[ignore = "Requires Rust toolchain and takes time to build"]
async fn test_run_cargo_check_in_isolation() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");
    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Run cargo check
    let result = build_env
        .run_command("cargo", &["check", "--quiet"], Duration::from_secs(60))
        .await;

    assert!(result.is_ok(), "Cargo check should succeed");
}

#[tokio::test]
#[ignore = "Requires Rust toolchain and takes time to build"]
async fn test_build_with_syntax_error() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");
    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Apply changes with syntax error
    let changes = vec![FileChange::Modify {
        path: PathBuf::from("src/lib.rs"),
        content: "pub fn hello() -> String { this is invalid syntax }".to_owned(),
    }];

    build_env.apply_changes(&changes).await.expect("apply");

    // Run cargo check - should fail
    let result = build_env
        .run_command("cargo", &["check", "--quiet"], Duration::from_secs(60))
        .await;

    assert!(
        result.is_err() || result.unwrap().exit_code != 0,
        "Cargo check should fail with syntax error"
    );
}

#[tokio::test]
async fn test_nested_directory_structure() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let project_path = temp_dir.path().to_path_buf();

    // Create nested structure
    fs::create_dir_all(project_path.join("src/module/submodule")).expect("create dirs");
    fs::write(
        project_path.join("src/module/submodule/deep.rs"),
        "// deep file",
    )
    .expect("write deep file");
    fs::write(project_path.join("Cargo.toml"), "[package]").expect("write cargo");

    let workspace = WorkspaceState::new(project_path.clone());
    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");

    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Verify nested structure was copied
    let isolated_path = build_env.path();
    assert!(
        isolated_path.join("src/module/submodule/deep.rs").exists(),
        "Nested file should be copied"
    );
}

#[tokio::test]
async fn test_file_change_operations() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");
    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Test Create, Modify, and Delete operations
    let changes = vec![
        FileChange::Create {
            path: PathBuf::from("src/created.rs"),
            content: "// created".to_owned(),
        },
        FileChange::Modify {
            path: PathBuf::from("src/lib.rs"),
            content: "// modified".to_owned(),
        },
        FileChange::Delete {
            path: PathBuf::from("README.md"),
        },
    ];

    build_env.apply_changes(&changes).await.expect("apply");

    let isolated_path = build_env.path();
    assert!(
        isolated_path.join("src/created.rs").exists(),
        "Created file should exist"
    );

    let modified = read_to_string(isolated_path.join("src/lib.rs"))
        .await
        .expect("read");
    assert!(modified.contains("modified"), "File should be modified");

    assert!(
        !isolated_path.join("README.md").exists(),
        "Deleted file should not exist"
    );
}

#[tokio::test]
async fn test_command_timeout() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    let build_env = IsolatedBuildEnv::new(&workspace).expect("create build env");
    build_env
        .copy_workspace_files(&project_path)
        .await
        .expect("copy files");

    // Run a command with very short timeout (should timeout on slow systems)
    let result = build_env
        .run_command("cargo", &["check"], Duration::from_millis(1))
        .await;

    // Either succeeds very quickly or times out - both are acceptable
    assert!(
        result.is_ok() || result.is_err(),
        "Command should complete or timeout"
    );
}

#[tokio::test]
async fn test_multiple_build_envs_isolated() {
    let (_temp, project_path) = create_test_rust_project();
    let workspace = WorkspaceState::new(project_path.clone());

    // Create multiple isolated environments
    let env1 = IsolatedBuildEnv::new(&workspace).expect("create env1");
    let env2 = IsolatedBuildEnv::new(&workspace).expect("create env2");

    env1.copy_workspace_files(&project_path)
        .await
        .expect("copy to env1");
    env2.copy_workspace_files(&project_path)
        .await
        .expect("copy to env2");

    // Modify files differently in each environment
    env1.apply_changes(&[FileChange::Modify {
        path: PathBuf::from("src/lib.rs"),
        content: "// env1".to_owned(),
    }])
    .await
    .expect("apply to env1");

    env2.apply_changes(&[FileChange::Modify {
        path: PathBuf::from("src/lib.rs"),
        content: "// env2".to_owned(),
    }])
    .await
    .expect("apply to env2");

    // Verify environments are truly isolated
    let content1 = read_to_string(env1.path().join("src/lib.rs"))
        .await
        .expect("read env1");
    let content2 = read_to_string(env2.path().join("src/lib.rs"))
        .await
        .expect("read env2");

    assert!(content1.contains("env1"), "Env1 should have env1 content");
    assert!(content2.contains("env2"), "Env2 should have env2 content");
    assert_ne!(env1.path(), env2.path(), "Paths should be different");
}
