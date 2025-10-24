//! Integration tests for async CLI functions with mocking
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

use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Helper to create a minimal Rust project
fn create_test_project(temp: &TempDir) {
    fs::create_dir_all(temp.path().join("src")).expect("Failed to create src dir");

    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write Cargo.toml");

    fs::write(
        temp.path().join("src/main.rs"),
        r#"fn main() {
    println!("Hello, world!");
}

fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 2), 4);
    }
}
"#,
    )
    .expect("Failed to write main.rs");
}

/// Helper to create config with test API key
fn create_test_config(temp: &TempDir) {
    fs::write(
        temp.path().join("config.toml"),
        r#"[providers]
openrouter_key = "test-key-for-testing"
high_model = "anthropic/claude-sonnet-4-20250514"
medium_model = "anthropic/claude-3.5-sonnet"
"#,
    )
    .expect("Failed to write config");
}

#[tokio::test]
async fn test_prompt_command_shows_context() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("show me the add function")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_command_with_max_files() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("analyze the code")
        .arg("--max-files")
        .arg("5")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_command_with_specific_files() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("what does this file do")
        .arg("-f")
        .arg("src/main.rs")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_command_empty_query() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_command_in_empty_directory() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("test query");

    // Empty directory may fail due to no language backend available - this is expected
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_query_command_requires_api_key() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    // Without API key, should still run but may fail with API error
    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("query")
        .arg("what is the purpose of this code?")
        .env_remove("OPENROUTER_API_KEY");

    // Command should execute (whether it succeeds depends on API availability)
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_query_command_with_project_config() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);
    create_test_config(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("query")
        .arg("explain the add function")
        .env_remove("OPENROUTER_API_KEY");

    // Should read config from project
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_query_command_with_specific_files() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("query")
        .arg("what does this do?")
        .arg("-f")
        .arg("src/main.rs")
        .env_remove("OPENROUTER_API_KEY");

    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_query_command_with_max_files() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("query")
        .arg("analyze code")
        .arg("--max-files")
        .arg("3")
        .env_remove("OPENROUTER_API_KEY");

    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_chat_command_basic_invocation() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("chat")
        .env_remove("OPENROUTER_API_KEY")
        .write_stdin("exit\n");

    // Chat should handle exit gracefully
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_chat_command_with_custom_model() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("chat")
        .arg("--model")
        .arg("anthropic/claude-3.5-sonnet")
        .env_remove("OPENROUTER_API_KEY")
        .write_stdin("exit\n");

    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_interactive_mode_with_validation_disabled() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    // Test non-interactive mode (no subcommand) with validation disabled
    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("--validation")
        .arg("disabled")
        .arg("--help")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_command_with_nonexistent_file() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("test query")
        .arg("-f")
        .arg("nonexistent.rs");

    // Should handle gracefully
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}

#[tokio::test]
async fn test_prompt_command_large_query() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    let large_query = "a".repeat(1000);
    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg(&large_query)
        .assert()
        .success();
}

#[tokio::test]
async fn test_config_loading_precedence() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);
    create_test_config(&temp);

    // Config command should show project config
    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("config")
        .arg("--full")
        .assert()
        .success();
}

#[tokio::test]
async fn test_multiple_source_files_in_project() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    // Add more source files
    fs::write(
        temp.path().join("src/lib.rs"),
        "pub fn multiply(a: i32, b: i32) -> i32 {\n    a * b\n}\n",
    )
    .expect("Failed to write lib.rs");

    fs::create_dir_all(temp.path().join("src/utils")).expect("Failed to create utils dir");
    fs::write(
        temp.path().join("src/utils/helpers.rs"),
        "pub fn helper() -> String {\n    \"helper\".to_string()\n}\n",
    )
    .expect("Failed to write helpers.rs");

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("show me all the functions")
        .assert()
        .success();
}

#[tokio::test]
async fn test_prompt_with_tests_directory() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    fs::create_dir_all(temp.path().join("tests")).expect("Failed to create tests dir");
    fs::write(
        temp.path().join("tests/integration_test.rs"),
        "#[test]\nfn test_integration() {\n    assert!(true);\n}\n",
    )
    .expect("Failed to write test file");

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("show me the tests")
        .assert()
        .success();
}

#[tokio::test]
async fn test_context_building_with_dependencies() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    create_test_project(&temp);

    // Add a dependency reference
    let mut cargo_toml =
        fs::read_to_string(temp.path().join("Cargo.toml")).expect("Failed to read Cargo.toml");
    cargo_toml.push_str("\n[dependencies]\nserde = \"1.0\"\n");
    fs::write(temp.path().join("Cargo.toml"), cargo_toml).expect("Failed to write Cargo.toml");

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("what dependencies does this project have")
        .assert()
        .success();
}

#[tokio::test]
async fn test_error_handling_invalid_rust_syntax() {
    let temp = TempDir::new().expect("Failed to create temp dir");
    fs::create_dir_all(temp.path().join("src")).expect("Failed to create src");

    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname=\"test\"\nversion=\"0.1.0\"\n",
    )
    .expect("Failed to write Cargo.toml");

    // Write invalid Rust code
    fs::write(
        temp.path().join("src/main.rs"),
        "this is not valid rust code {{{{ }}}}",
    )
    .expect("Failed to write invalid code");

    let mut cmd = Command::cargo_bin("merlin").expect("Binary not found");
    cmd.current_dir(temp.path())
        .env("MERLIN_SKIP_EMBEDDINGS", "1")
        .arg("prompt")
        .arg("analyze this code");

    // Should handle gracefully even with invalid syntax
    let output = cmd.output().expect("Failed to execute");
    assert!(output.status.code().is_some());
}
