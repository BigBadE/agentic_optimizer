use serde::{Deserialize, Serialize};
use std::fs::read_to_string;
use std::path::PathBuf;

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

    #[allow(clippy::min_ident_chars, reason = "standard iterator variable name")]
    pub fn files_to_string(&self) -> String {
        self.files
            .iter()
            .map(|file| format!("// File: {}\n{}\n", file.path.display(), file.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[allow(clippy::integer_division, clippy::integer_division_remainder_used, clippy::min_ident_chars, reason = "approximate token count calculation, division by 4 is intentional")]
    pub fn token_estimate(&self) -> usize {
        let files_len: usize = self.files.iter().map(|file| file.content.len()).sum();
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
    /// Returns an error if the file cannot be read
    pub fn from_path(path: &PathBuf) -> crate::Result<Self> {
        let content = read_to_string(path)
            .map_err(|_| crate::Error::FileNotFound(path.display().to_string()))?;

        Ok(Self {
            path: path.clone(),
            content,
        })
    }

    pub const fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}
