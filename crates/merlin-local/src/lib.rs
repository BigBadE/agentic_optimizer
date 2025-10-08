//! Local inference provider integrations for the agentic optimizer.
//!
//! This crate wraps the Ollama runtime and exposes a unified interface for
//! local model execution that mirrors the remote provider APIs used elsewhere
//! in the system.
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

/// Error types for local provider operations.
pub mod error;
/// Local model provider implementation using Ollama.
pub mod inference;
/// Ollama service management utilities.
pub mod manager;
/// Data types for Ollama models and API interactions.
pub mod models;

pub use error::{LocalError, Result};
pub use inference::LocalModelProvider;
pub use manager::OllamaManager;
pub use models::{ModelInfo, OllamaGenerateRequest, OllamaGenerateResponse, OllamaModel};
