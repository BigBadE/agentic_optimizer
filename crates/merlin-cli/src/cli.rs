use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "merlin")]
#[command(about = "Intelligent AI coding assistant with multi-model routing", long_about = None)]
pub struct Cli {
    /// Project root directory
    #[arg(short, long, default_value = ".", global = true)]
    pub project: PathBuf,

    /// Use only local models (Ollama), disable remote tiers
    #[arg(long, global = true)]
    pub local: bool,

    /// Disable validation pipeline (enabled by default)
    #[arg(long, global = true)]
    pub no_validate: bool,

    /// Show detailed routing decisions
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Disable TUI mode, use plain terminal output
    #[arg(long, global = true)]
    pub no_tui: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Start interactive chat session")]
    Chat {
        #[arg(short, long, default_value = ".", help = "Project root directory")]
        project: PathBuf,

        #[arg(long, help = "Model to use (overrides config)")]
        model: Option<String>,
    },

    #[command(about = "Ask a question or request code changes")]
    Query {
        #[arg(help = "The query to send to the agent")]
        query: String,

        #[arg(short, long, default_value = ".", help = "Project root directory")]
        project: PathBuf,

        #[arg(short, long, help = "Specific files to include in context")]
        files: Vec<PathBuf>,

        #[arg(long, help = "Maximum number of files to include")]
        max_files: Option<usize>,
    },

    #[command(about = "Show relevant files for a prompt without sending to LLM")]
    Prompt {
        #[arg(help = "The prompt/query to analyze")]
        query: String,

        #[arg(short, long, default_value = ".", help = "Project root directory")]
        project: PathBuf,

        #[arg(short, long, help = "Specific files to include in context")]
        files: Vec<PathBuf>,

        #[arg(long, help = "Maximum number of files to include")]
        max_files: Option<usize>,
    },

    #[command(about = "Show configuration")]
    Config {
        #[arg(long, help = "Show full configuration including defaults")]
        full: bool,
    },

    #[command(about = "Show metrics and cost tracking")]
    Metrics {
        #[arg(long, help = "Show daily metrics")]
        daily: bool,
    },

}

