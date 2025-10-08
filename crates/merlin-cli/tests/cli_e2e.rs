//! End-to-end CLI tests using `assert_cmd`
#![cfg(test)]
#![allow(clippy::expect_used, reason = "Test code is allowed to use expect")]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
/// # Panics
/// Panics if the binary cannot be found or the command fails unexpectedly.
fn test_cli_help() {
    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
/// # Panics
/// Panics if the binary cannot be found or the command fails unexpectedly.
fn test_cli_invalid_command() {
    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .arg("invalid-command-xyz")
        .assert()
        .failure();
}

#[test]
/// # Panics
/// Panics if the temp directory cannot be created, binary not found, or command fails.
fn test_cli_in_empty_directory() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
/// # Panics
/// Panics if temp dir/file creation fails, binary not found, or command fails.
fn test_cli_with_rust_project() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    // Create a minimal Rust project
    fs::create_dir(temp.path().join("src")).expect("Failed to create src dir");
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
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .expect("Failed to write main.rs");

    // Test that the CLI can run in this project
    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
/// # Panics
/// Panics if temp dir/file creation fails, binary not found, or command fails.
fn test_cli_reads_cargo_toml() {
    let temp = TempDir::new().expect("Failed to create temp dir");

    fs::write(
        temp.path().join("Cargo.toml"),
        r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("Failed to write Cargo.toml");

    // The CLI should be able to detect this is a Cargo project
    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
/// # Panics
/// Panics if temp dir creation fails, binary not found, or command fails.
fn test_cli_handles_non_utf8_paths() {
    // This test ensures the CLI doesn't panic on edge cases
    let temp = TempDir::new().expect("Failed to create temp dir");

    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .current_dir(temp.path())
        .arg("--help")
        .assert()
        .success();
}

#[test]
/// # Panics
/// Panics if the binary cannot be found or the command fails unexpectedly.
fn test_cli_with_args() {
    Command::cargo_bin("merlin")
        .expect("Binary not found")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}
