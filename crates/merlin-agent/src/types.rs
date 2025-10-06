use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use merlin_core::TokenUsage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    pub query: String,
    pub workspace_root: PathBuf,
    pub context_files: Vec<PathBuf>,
    pub max_tokens: Option<usize>,
}

impl AgentRequest {
    pub fn new<T: Into<String>>(query: T, workspace_root: PathBuf) -> Self {
        Self {
            query: query.into(),
            workspace_root,
            context_files: Vec::new(),
            max_tokens: None,
        }
    }

    #[must_use]
    pub fn with_context_files(mut self, files: Vec<PathBuf>) -> Self {
        self.context_files = files;
        self
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    pub content: String,
    pub confidence: f64,
    pub provider_used: String,
    pub tokens_used: TokenUsage,
    pub latency_ms: u64,
    pub context_files_used: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub response: AgentResponse,
    pub metadata: ExecutionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub context_build_time_ms: u64,
    pub provider_call_time_ms: u64,
    pub total_time_ms: u64,
    pub context_token_estimate: usize,
}

