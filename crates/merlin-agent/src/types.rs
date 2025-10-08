use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use merlin_core::TokenUsage;

/// Request to execute an agent query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRequest {
    /// The query or task to execute
    pub query: String,
    /// Root directory of the workspace
    pub workspace_root: PathBuf,
    /// Optional list of context files to include
    pub context_files: Vec<PathBuf>,
    /// Optional maximum number of tokens to use
    pub max_tokens: Option<usize>,
}

impl AgentRequest {
    /// Create a new agent request with the given query and workspace root
    pub fn new<T: Into<String>>(query: T, workspace_root: PathBuf) -> Self {
        Self {
            query: query.into(),
            workspace_root,
            context_files: Vec::default(),
            max_tokens: None,
        }
    }

    /// Add a list of context files to include
    #[must_use]
    pub fn with_context_files(mut self, files: Vec<PathBuf>) -> Self {
        self.context_files = files;
        self
    }

    /// Set the maximum number of tokens to use
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }
}

/// Response from an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Generated response content
    pub content: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f64,
    /// Name of the provider that generated the response
    pub provider_used: String,
    /// Token usage statistics
    pub tokens_used: TokenUsage,
    /// Latency in milliseconds
    pub latency_ms: u64,
    /// Files that were included in the context
    pub context_files_used: Vec<PathBuf>,
}

/// Complete result of an agent execution including response and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// The agent's response
    pub response: AgentResponse,
    /// Execution performance metadata
    pub metadata: ExecutionMetadata,
}

/// Performance metadata for an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    /// Time spent building context in milliseconds
    pub context_build_time_ms: u64,
    /// Time spent calling the provider in milliseconds
    pub provider_call_time_ms: u64,
    /// Total execution time in milliseconds
    pub total_time_ms: u64,
    /// Estimated token count of the context
    pub context_token_estimate: usize,
}
