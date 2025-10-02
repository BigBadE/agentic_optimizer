use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub conversation_id: Option<String>,
    pub files_context: Vec<PathBuf>,
}

impl Query {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            conversation_id: None,
            files_context: Vec::new(),
        }
    }

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
    pub fn total(&self) -> u64 {
        self.input + self.output + self.cache_read + self.cache_write
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub files: Vec<FileContext>,
    pub system_prompt: String,
}

impl Context {
    pub fn new(system_prompt: impl Into<String>) -> Self {
        Self {
            files: Vec::new(),
            system_prompt: system_prompt.into(),
        }
    }

    pub fn with_files(mut self, files: Vec<FileContext>) -> Self {
        self.files = files;
        self
    }

    pub fn files_to_string(&self) -> String {
        self.files
            .iter()
            .map(|f| format!("// File: {}\n{}\n", f.path.display(), f.content))
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn token_estimate(&self) -> usize {
        let files_len: usize = self.files.iter().map(|f| f.content.len()).sum();
        (self.system_prompt.len() + files_len) / 4
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
}

impl FileContext {
    pub fn from_path(path: &PathBuf) -> crate::core::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|_| crate::core::Error::FileNotFound(path.display().to_string()))?;

        Ok(Self {
            path: path.clone(),
            content,
        })
    }

    pub fn new(path: PathBuf, content: String) -> Self {
        Self { path, content }
    }
}
