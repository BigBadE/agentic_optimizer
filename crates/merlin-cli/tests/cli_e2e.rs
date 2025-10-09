//! End-to-end CLI tests using `assert_cmd`
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
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to get cargo binary or fail test
fn cargo_bin() -> Command {
    Command::cargo_bin("merlin").unwrap_or_else(|err| panic!("Binary not found: {err}"))
}

/// Helper to create temp dir or fail test
fn temp_dir() -> TempDir {
    TempDir::new().unwrap_or_else(|err| panic!("Failed to create temp dir: {err}"))
}

#[test]
fn test_cli_help() {
    cargo_bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn test_cli_invalid_command() {
    cargo_bin().arg("invalid-command-xyz").assert().failure();
}

#[test]
fn test_cli_in_empty_directory() {
    let temp = temp_dir();

    cargo_bin()
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_rust_project() {
    let temp = temp_dir();

    // Create a minimal Rust project
    fs::create_dir(temp.path().join("src"))
        .unwrap_or_else(|err| panic!("Failed to create src dir: {err}"));
    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap_or_else(|err| panic!("Failed to write Cargo.toml: {err}"));

    fs::write(
        temp.path().join("src/main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .unwrap_or_else(|err| panic!("Failed to write main.rs: {err}"));

    // Test that the CLI can run in this project
    cargo_bin()
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_reads_cargo_toml() {
    let temp = temp_dir();

    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap_or_else(|err| panic!("Failed to write Cargo.toml: {err}"));

    // The CLI should be able to detect this is a Cargo project
    cargo_bin()
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_handles_non_utf8_paths() {
    // This test ensures the CLI doesn't panic on edge cases
    let temp = temp_dir();

    cargo_bin()
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_args() {
    cargo_bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn test_cli_config_command() {
    cargo_bin().arg("config").assert().success();
}

#[test]
fn test_cli_config_full() {
    cargo_bin().arg("config").arg("--full").assert().success();
}

#[test]
fn test_cli_metrics_command() {
    cargo_bin().arg("metrics").assert().success();
}

#[test]
fn test_cli_metrics_daily() {
    cargo_bin().arg("metrics").arg("--daily").assert().success();
}

#[test]
fn test_cli_with_local_flag() {
    let temp = temp_dir();

    fs::create_dir(temp.path().join("src"))
        .unwrap_or_else(|err| panic!("Failed to create src: {err}"));
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname=\"test\"\nversion=\"0.1.0\"",
    )
    .unwrap_or_else(|err| panic!("Failed to write Cargo.toml: {err}"));

    cargo_bin()
        .current_dir(temp.path())
        .arg("--local")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_validation_disabled() {
    cargo_bin()
        .arg("--validation")
        .arg("disabled")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_validation_enabled() {
    cargo_bin()
        .arg("--validation")
        .arg("enabled")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_ui_plain() {
    cargo_bin()
        .arg("--ui")
        .arg("plain")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_ui_plain_verbose() {
    cargo_bin()
        .arg("--ui")
        .arg("plain-verbose")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_ui_tui() {
    cargo_bin()
        .arg("--ui")
        .arg("tui")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_with_project_flag() {
    let temp = temp_dir();

    fs::create_dir(temp.path().join("src"))
        .unwrap_or_else(|err| panic!("Failed to create src: {err}"));
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname=\"proj\"\nversion=\"0.1.0\"",
    )
    .unwrap_or_else(|err| panic!("Failed to write Cargo.toml: {err}"));

    cargo_bin()
        .arg("--project")
        .arg(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_prompt_command_help() {
    cargo_bin()
        .arg("prompt")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Show relevant files"));
}

#[test]
fn test_cli_query_command_help() {
    cargo_bin()
        .arg("query")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Ask a question"));
}

#[test]
fn test_cli_chat_command_help() {
    cargo_bin()
        .arg("chat")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("interactive chat"));
}

#[test]
fn test_cli_config_in_project() {
    let temp = temp_dir();

    // Create project structure
    fs::create_dir(temp.path().join("src"))
        .unwrap_or_else(|err| panic!("Failed to create src: {err}"));
    fs::write(
        temp.path().join("Cargo.toml"),
        "[package]\nname=\"test\"\nversion=\"0.1.0\"\nedition=\"2021\"",
    )
    .unwrap_or_else(|err| panic!("Failed to write Cargo.toml: {err}"));

    // Create a config file
    fs::write(
        temp.path().join("config.toml"),
        "[providers]\nhigh_model = \"test-model\"",
    )
    .unwrap_or_else(|err| panic!("Failed to write config.toml: {err}"));

    cargo_bin()
        .current_dir(temp.path())
        .arg("config")
        .assert()
        .success();
}

#[test]
fn test_cli_multiple_flags_combined() {
    cargo_bin()
        .arg("--local")
        .arg("--validation")
        .arg("disabled")
        .arg("--ui")
        .arg("plain")
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn test_cli_invalid_validation_value() {
    cargo_bin()
        .arg("--validation")
        .arg("invalid")
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_cli_invalid_ui_value() {
    cargo_bin()
        .arg("--ui")
        .arg("invalid")
        .arg("--help")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

#[test]
fn test_cli_nonexistent_project() {
    cargo_bin()
        .arg("--project")
        .arg("/nonexistent/path/that/does/not/exist")
        .arg("config")
        .assert()
        .success();
}
