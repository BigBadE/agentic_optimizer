use core::result::Result as CoreResult;
use thiserror::Error;

pub type Result<T> = CoreResult<T, LocalError>;

#[derive(Debug, Error)]
pub enum LocalError {
    #[error("Core error: {0}")]
    Core(#[from] merlin_core::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Ollama not available: {0}")]
    OllamaUnavailable(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Model pull failed: {0}")]
    ModelPullFailed(String),

    #[error("Inference failed: {0}")]
    InferenceFailed(String),

    #[error("{0}")]
    Other(String),
}

