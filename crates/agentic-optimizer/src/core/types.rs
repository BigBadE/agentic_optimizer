use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;
use crate::core::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub conversation_id: Option<String>,
    pub files_context: Vec<PathBuf>,
}

impl Query {
    pub fn new<T: Into<String>>(text: T) -> Self {
        Self {
            text: text.into(),
            conversation_id: None,
            files_context: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_files(mut self, files: Vec<PathBuf>) -> Self {
        self.files_context = files;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub text: String,
    pub confidence: f64,
    pub tokens_used: TokenUsage,
    pub provider: String,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

impl TokenUsage {
    pub const fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub files: Vec<FileContext>,
    pub system_prompt: String,
}

impl Context {
    pub fn new<T: Into<String>>(system_prompt: T) -> Self {
        Self {
            files: Vec::new(),
            system_prompt: system_prompt.into(),
        }
    }

    #[must_use]
    pub fn with_files(mut self, files: Vec<FileContext>) -> Self {
        self.files = files;
        self
    }

    pub fn files_to_string(&self) -> String {
        self.files
            .iter()
            .map(|file_ctx| format!("// File: {}\n{}\n", file_ctx.path.display(), file_ctx.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(
        clippy::integer_division,
        clippy::integer_division_remainder_used,
        reason = "approximate token count; division by 4 is intentional"
    )]
    pub fn token_estimate(&self) -> usize {
        let files_len: usize = self.files.iter().map(|file_ctx| file_ctx.content.len()).sum();
        (self.system_prompt.len() + files_len) / 4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
}

impl FileContext {
    /// # Errors
    /// Returns an error if the file cannot be read from the filesystem.
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let content = read_to_string(path)
            .map_err(|_| Error::FileNotFound(path.display().to_string()))?;

        Ok(Self {
            path: path.clone(),
            content,
        })
    }

    pub const fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}
