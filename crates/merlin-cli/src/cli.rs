use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Validation {
    Enabled,
    Disabled,
}

#[derive(Parser)]
#[command(name = "merlin")]
#[command(about = "Intelligent AI coding assistant with multi-model routing", long_about = None)]
pub struct Cli {
    /// Project root directory
    #[arg(short, long, default_value = ".")]
    pub project: PathBuf,

    /// Use only local models (Ollama), disable remote tiers
    #[arg(long)]
    pub local: bool,

    /// Validation mode (enabled/disabled)
    #[arg(long, value_enum, default_value = "enabled")]
    pub validation: Validation,

    /// Dump full context to debug.log before each model call
    #[arg(long)]
    pub context_dump: bool,
}
