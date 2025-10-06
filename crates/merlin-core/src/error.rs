use core::result::Result as CoreResult;
use std::io::Error as IoError;

use thiserror::Error;
use toml::de::Error as TomlError;

pub type Result<T> = CoreResult<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] IoError),

    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML deserialization error: {0}")]
    Toml(#[from] TomlError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("API key not found: {0}")]
    MissingApiKey(String),

    #[error("Invalid response from provider: {0}")]
    InvalidResponse(String),

    #[error("Context building failed: {0}")]
    ContextBuild(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    #[must_use] 
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Request(_) | Self::Provider(_)
        )
    }
}
