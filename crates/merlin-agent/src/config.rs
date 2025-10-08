use serde::{Deserialize, Serialize};

/// Configuration for agent behavior and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// System prompt that defines agent behavior
    pub system_prompt: String,
    /// Maximum number of tokens allowed in context
    pub max_context_tokens: usize,
    /// Temperature parameter for LLM generation (0.0-1.0)
    pub temperature: f64,
    /// Maximum number of context files to include
    pub top_k_context_files: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt:
                "You are a helpful AI assistant that analyzes code and provides insights."
                    .to_owned(),
            max_context_tokens: 100_000,
            temperature: 0.7,
            top_k_context_files: 10,
        }
    }
}

impl AgentConfig {
    /// Set the system prompt
    #[must_use]
    pub fn with_system_prompt<T: Into<String>>(mut self, prompt: T) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Set the maximum context tokens
    #[must_use]
    pub fn with_max_context_tokens(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    /// Set the temperature parameter
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set the maximum number of context files
    #[must_use]
    pub fn with_top_k_context_files(mut self, top_k: usize) -> Self {
        self.top_k_context_files = top_k;
        self
    }
}
