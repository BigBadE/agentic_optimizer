use core::result::Result as CoreResult;
use merlin_core::Error as CoreError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

/// Result type for local provider operations.
pub type Result<T> = CoreResult<T, LocalError>;

/// Errors that can occur when using the local model provider.
#[derive(Debug, Error)]
pub enum LocalError {
    /// An error from the core library.
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    /// An HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] ReqwestError),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] SerdeJsonError),

    /// Ollama service is not available or unreachable.
    #[error("Ollama not available: {0}")]
    OllamaUnavailable(String),

    /// The requested model was not found.
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// Failed to pull the model from the registry.
    #[error("Model pull failed: {0}")]
    ModelPullFailed(String),

    /// Model inference failed.
    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    /// A general error not covered by other variants.
    #[error("{0}")]
    Other(String),
}
