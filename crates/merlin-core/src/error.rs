use core::result::Result as CoreResult;
use std::io::Error as IoError;

use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use toml::de::Error as TomlError;

/// Result type for core operations.
pub type Result<T> = CoreResult<T, Error>;

/// Errors that can occur in the core library.
#[derive(Debug, Error)]
pub enum Error {
    /// An I/O operation failed.
    #[error("IO error: {0}")]
    Io(#[from] IoError),

    /// An HTTP request failed.
    #[error("HTTP request failed: {0}")]
    Request(#[from] ReqwestError),

    /// JSON serialization or deserialization failed.
    #[error("JSON serialization error: {0}")]
    Json(#[from] SerdeJsonError),

    /// TOML deserialization failed.
    #[error("TOML deserialization error: {0}")]
    Toml(#[from] TomlError),

    /// Configuration is invalid or missing.
    #[error("Configuration error: {0}")]
    Config(String),

    /// A model provider encountered an error.
    #[error("Provider error: {0}")]
    Provider(String),

    /// Required API key was not found.
    #[error("API key not found: {0}")]
    MissingApiKey(String),

    /// Model provider returned an invalid response.
    #[error("Invalid response from provider: {0}")]
    InvalidResponse(String),

    /// Failed to build context for model invocation.
    #[error("Context building failed: {0}")]
    ContextBuild(String),

    /// The specified file does not exist.
    #[error("File not found: {0}")]
    FileNotFound(String),

    /// A general error not covered by other variants.
    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Determines whether this error may succeed if retried.
    ///
    /// Returns `true` for transient errors like network failures or provider errors.
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Request(_) | Self::Provider(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value as JsonValue, from_str};
    use std::io;

    #[test]
    fn test_error_display() {
        let error1 = Error::Config("invalid config".to_owned());
        assert_eq!(error1.to_string(), "Configuration error: invalid config");

        let error2 = Error::Provider("model failed".to_owned());
        assert_eq!(error2.to_string(), "Provider error: model failed");

        let error3 = Error::MissingApiKey("OPENAI_API_KEY".to_owned());
        assert_eq!(error3.to_string(), "API key not found: OPENAI_API_KEY");
    }

    #[test]
    fn test_error_is_retryable() {
        // Retryable errors
        let error1 = Error::Provider("timeout".to_owned());
        assert!(error1.is_retryable());

        // Non-retryable errors
        let error2 = Error::Config("bad config".to_owned());
        assert!(!error2.is_retryable());

        let error3 = Error::MissingApiKey("KEY".to_owned());
        assert!(!error3.is_retryable());

        let error4 = Error::FileNotFound("test.txt".to_owned());
        assert!(!error4.is_retryable());
    }

    #[test]
    fn test_error_from_io() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn test_error_from_json() {
        let json_error = from_str::<JsonValue>("invalid json").unwrap_err();
        let error: Error = json_error.into();
        assert!(matches!(error, Error::Json(_)));
    }

    #[test]
    fn test_result_type() {
        fn returns_result() -> String {
            "success".to_owned()
        }

        fn returns_error() -> Result<String> {
            Err(Error::Other("failed".to_owned()))
        }

        returns_result();
        returns_error().unwrap_err();
    }
}
