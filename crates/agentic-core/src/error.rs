use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML deserialization error: {0}")]
    Toml(#[from] toml::de::Error),

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
}

impl Error {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Request(_) | Error::Provider(_)
        )
    }
}
