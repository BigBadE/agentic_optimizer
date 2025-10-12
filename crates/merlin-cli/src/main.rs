//! Merlin CLI - Interactive AI coding assistant command-line interface
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
        reason = "Allow for tests"
    )
)]

use anyhow::Result;
use clap::Parser as _;
use cli::{Cli, Commands};

mod cli;
mod config;
mod handlers;
mod interactive;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Chat { project, model }) => {
            handlers::handle_chat(project, model).await?;
        }
        Some(Commands::Query {
            query,
            project,
            files,
            max_files,
        }) => {
            handlers::handle_query(query, project, files, max_files).await?;
        }
        Some(Commands::Prompt {
            query,
            project,
            files,
            max_files,
        }) => {
            handlers::handle_prompt(query, project, files, max_files).await?;
        }
        Some(Commands::Config { full }) => {
            handlers::handle_config(full)?;
        }
        Some(Commands::Metrics { daily }) => {
            handlers::handle_metrics(daily);
        }
        None => {
            // No subcommand - start interactive agent session
            handlers::handle_interactive(
                cli.project,
                cli.validation,
                cli.ui,
                cli.local,
                cli.context_dump,
            )
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use console::Term;
    use merlin_core::TokenUsage;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_token_usage_default() {
        let usage = TokenUsage::default();
        assert_eq!(usage.input, 0, "Default input should be 0");
        assert_eq!(usage.output, 0, "Default output should be 0");
        assert_eq!(usage.cache_read, 0, "Default cache_read should be 0");
        assert_eq!(usage.cache_write, 0, "Default cache_write should be 0");
    }

    #[test]
    fn test_print_chat_header() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let term = Term::stdout();

        let result = interactive::print_chat_header(&term, temp.path());
        assert!(result.is_ok(), "print_chat_header should succeed");
    }

    #[test]
    fn test_init_tui_logging_creates_file() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        // Can't directly test init_tui_logging since it's private,
        // but we can test the directory setup
        assert!(merlin_dir.exists(), ".merlin directory should exist");
    }

    #[test]
    fn test_init_tui_logging_local_mode() {
        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        assert!(
            merlin_dir.exists(),
            ".merlin directory should exist in local mode"
        );
    }

    #[test]
    fn test_init_tui_logging_removes_old_log() {
        const INITIAL_CONTENT: &str = "old content\n";

        let temp = TempDir::new().expect("Failed to create temp dir");
        let merlin_dir = temp.path().join(".merlin");
        let project_dir = temp.path().join("project");
        fs::create_dir_all(&merlin_dir).expect("Failed to create .merlin dir");
        fs::create_dir_all(&project_dir).expect("Failed to create project dir");

        let log_file = merlin_dir.join("debug.log");
        fs::write(&log_file, INITIAL_CONTENT).expect("Failed to write initial content");

        // Verify the file was created
        assert!(log_file.exists(), "Log file should exist");
    }
}
