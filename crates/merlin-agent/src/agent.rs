use std::sync::Arc;

use merlin_core::ModelProvider;

use crate::{AgentConfig, AgentExecutor};

/// Main agent that coordinates model providers and configuration
pub struct Agent {
    config: AgentConfig,
    provider: Arc<dyn ModelProvider>,
}

impl Agent {
    /// Create a new agent with default configuration
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self {
            config: AgentConfig::default(),
            provider,
        }
    }

    /// Create a new agent with custom configuration
    pub fn with_config(provider: Arc<dyn ModelProvider>, config: AgentConfig) -> Self {
        Self { config, provider }
    }

    /// Get a reference to the agent's configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Create an executor from this agent
    pub fn executor(&self) -> AgentExecutor {
        AgentExecutor::new(Arc::clone(&self.provider), self.config.clone())
    }
}
