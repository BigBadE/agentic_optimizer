//! Local inference provider integrations for the agentic optimizer.
//!
//! This crate wraps the Ollama runtime and exposes a unified interface for
//! local model execution that mirrors the remote provider APIs used elsewhere
//! in the system.

pub mod error;
pub mod inference;
pub mod manager;
pub mod models;

pub use error::{LocalError, Result};
pub use inference::LocalModelProvider;
pub use manager::OllamaManager;
pub use models::{ModelInfo, OllamaGenerateRequest, OllamaGenerateResponse, OllamaModel};
