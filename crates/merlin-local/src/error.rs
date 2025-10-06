use core::result::Result as CoreResult;
use merlin_core::Error as CoreError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

pub type Result<T> = CoreResult<T, LocalError>;

#[derive(Debug, Error)]
pub enum LocalError {
    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("HTTP error: {0}")]
    Http(#[from] ReqwestError),

    #[error("JSON error: {0}")]
    Json(#[from] SerdeJsonError),

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

