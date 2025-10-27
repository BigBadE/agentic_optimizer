//! Merlin CLI - Interactive AI coding assistant command-line interface

use clap::Parser as _;
use cli::Cli;
use merlin_deps::anyhow::Result;
use tokio;

mod cli;
mod handlers;
mod interactive;
mod ui;
mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    handlers::handle_interactive(cli.project, cli.validation, cli.local, cli.context_dump).await?;

    Ok(())
}
