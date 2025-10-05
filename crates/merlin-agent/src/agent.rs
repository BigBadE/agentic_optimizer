use std::sync::Arc;

use merlin_core::ModelProvider;

use crate::{AgentConfig, AgentExecutor};

pub struct Agent {
    config: AgentConfig,
    provider: Arc<dyn ModelProvider>,
}

impl Agent {
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self {
            config: AgentConfig::default(),
            provider,
        }
    }

    pub fn with_config(provider: Arc<dyn ModelProvider>, config: AgentConfig) -> Self {
        Self { config, provider }
    }

    #[must_use]
    pub const fn config(&self) -> &AgentConfig {
        &self.config
    }

    #[must_use] 
    pub fn executor(&self) -> AgentExecutor {
        AgentExecutor::new(Arc::clone(&self.provider), self.config.clone())
    }
}

