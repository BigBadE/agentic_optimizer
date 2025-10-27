#![allow(dead_code, reason = "Work in progress")]
use std::path::PathBuf;
use std::process::exit;

use pico_args::{Arguments, Error};

/// Validation mode for task execution
#[derive(Clone, Copy, Debug)]
pub enum Validation {
    /// Validation enabled
    Enabled,
    /// Validation disabled
    Disabled,
}

/// Command-line arguments for Merlin CLI
#[derive(Debug)]
pub struct Cli {
    /// Project root directory
    pub project: PathBuf,

    /// Use only local models (Ollama), disable remote tiers
    pub local: bool,

    /// Validation mode (enabled/disabled)
    pub validation: Validation,

    /// Dump full context to debug.log before each model call
    pub context_dump: bool,
}

impl Cli {
    /// Parse command-line arguments
    ///
    /// # Errors
    ///
    /// Returns an error if argument parsing fails or invalid values are provided
    pub fn parse() -> Result<Self, Error> {
        let mut pargs = Arguments::from_env();

        // Handle --help
        if pargs.contains(["-h", "--help"]) {
            print_help();
            exit(0);
        }

        let cli = Self {
            project: pargs
                .opt_value_from_str(["-p", "--project"])?
                .unwrap_or_else(|| PathBuf::from(".")),
            local: pargs.contains("--local"),
            validation: {
                let val_str: Option<String> = pargs.opt_value_from_str("--validation")?;
                match val_str.as_deref() {
                    Some("disabled") => Validation::Disabled,
                    Some("enabled") | None => Validation::Enabled,
                    Some(other) => {
                        return Err(Error::ArgumentParsingFailed {
                            cause: format!("invalid validation mode: {other}"),
                        });
                    }
                }
            },
            context_dump: pargs.contains("--context-dump"),
        };

        // Check for any remaining arguments
        let remaining = pargs.finish();
        if !remaining.is_empty() {
            merlin_deps::tracing::warn!("Unexpected arguments: {remaining:?}");
        }

        Ok(cli)
    }
}

fn print_help() {
    const HELP_TEXT: &str = "\
merlin - Intelligent AI coding assistant with multi-model routing

USAGE:
    merlin [OPTIONS]

OPTIONS:
    -p, --project <PATH>         Project root directory [default: .]
    --local                      Use only local models (Ollama), disable remote tiers
    --validation <MODE>          Validation mode (enabled/disabled) [default: enabled]
    --context-dump               Dump full context to debug.log before each model call
    -h, --help                   Print help information
";
    // Help text is printed to stdout by convention for CLI tools
    #[allow(clippy::print_stdout, reason = "Help text output")]
    {
        print!("{HELP_TEXT}");
    }
}
