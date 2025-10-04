pub mod error;
pub mod inference;
pub mod manager;
pub mod models;

pub use error::{LocalError, Result};
pub use inference::LocalModelProvider;
pub use manager::OllamaManager;
pub use models::{ModelInfo, OllamaGenerateRequest, OllamaGenerateResponse, OllamaModel};
