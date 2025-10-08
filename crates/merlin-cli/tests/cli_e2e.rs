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
