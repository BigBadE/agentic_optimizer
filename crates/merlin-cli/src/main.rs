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

use cli::Cli;
use merlin_deps::anyhow::{Context as _, Result};

mod cli;
mod handlers;
mod interactive;
mod ui;
mod utils;

/// Main entry point for Merlin CLI
///
/// # Errors
/// Returns error if CLI parsing or handler execution fails
///
/// # Panics
/// May panic if Tokio runtime initialization fails
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse().context("Failed to parse command-line arguments")?;

    handlers::handle_interactive(cli.project, cli.validation, cli.local, cli.context_dump).await?;

    Ok(())
}
