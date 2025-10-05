use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub system_prompt: String,
    pub max_context_tokens: usize,
    pub temperature: f64,
    pub top_k_context_files: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            system_prompt: "You are a helpful AI assistant that analyzes code and provides insights.".to_owned(),
            max_context_tokens: 100_000,
            temperature: 0.7,
            top_k_context_files: 10,
        }
    }
}

impl AgentConfig {
    #[must_use] 
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_system_prompt<T: Into<String>>(mut self, prompt: T) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    #[must_use]
    pub const fn with_max_context_tokens(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    #[must_use]
    pub const fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    #[must_use]
    pub const fn with_top_k_context_files(mut self, top_k: usize) -> Self {
        self.top_k_context_files = top_k;
        self
    }
}
