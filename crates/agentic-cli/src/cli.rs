use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "agentic-optimizer")]
#[command(about = "Cost-optimized AI coding agent", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
