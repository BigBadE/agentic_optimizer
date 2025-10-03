use std::sync::Arc;
use std::time::Instant;

use agentic_context::ContextBuilder;
use agentic_core::{Context, ModelProvider, Query};
use agentic_languages::LanguageProvider;
use tracing::{info, warn};

use crate::{AgentConfig, AgentRequest, AgentResponse, ExecutionMetadata, ExecutionResult};

pub struct AgentExecutor {
    provider: Arc<dyn ModelProvider>,
    config: AgentConfig,
    context_builder: Option<ContextBuilder>,
}

impl AgentExecutor {
    pub fn new(provider: Arc<dyn ModelProvider>, config: AgentConfig) -> Self {
        Self {
            provider,
            config,
            context_builder: None,
        }
    }

    pub fn with_language_backend(mut self, backend: Box<dyn LanguageProvider>) -> Self {
        let builder = if let Some(mut builder) = self.context_builder.take() {
            builder = builder.with_language_backend(backend);
            builder
        } else {
            ContextBuilder::new(std::env::current_dir().unwrap_or_default())
                .with_language_backend(backend)
        };
        self.context_builder = Some(builder);
        self
    }

    pub async fn execute(&mut self, request: AgentRequest) -> anyhow::Result<ExecutionResult> {
        let total_start = Instant::now();

        info!("Executing agent request: {}", request.query);

        let context_start = Instant::now();
        let context = self.build_context(&request).await?;
        let context_build_time = context_start.elapsed().as_millis() as u64;

        info!(
            "Context built: {} files, ~{} tokens",
            context.files.len(),
            context.token_estimate()
        );

        let provider_start = Instant::now();
        let query = Query::new(&request.query);
        let response = self.provider.generate(&query, &context).await?;
        let provider_call_time = provider_start.elapsed().as_millis() as u64;

        let total_time = total_start.elapsed().as_millis() as u64;

        let agent_response = AgentResponse {
            content: response.text,
            confidence: response.confidence,
            provider_used: response.provider,
            tokens_used: response.tokens_used,
            latency_ms: response.latency_ms,
            context_files_used: context.files.iter().map(|f| f.path.clone()).collect(),
        };

        let metadata = ExecutionMetadata {
            context_build_time_ms: context_build_time,
            provider_call_time_ms: provider_call_time,
            total_time_ms: total_time,
            context_token_estimate: context.token_estimate(),
        };

        Ok(ExecutionResult {
            response: agent_response,
            metadata,
        })
    }

    async fn build_context(&mut self, request: &AgentRequest) -> anyhow::Result<Context> {
        if self.context_builder.is_none() {
            self.context_builder = Some(
                ContextBuilder::new(request.workspace_root.clone())
                    .with_max_files(self.config.top_k_context_files),
            );
        }

        let builder = self.context_builder.as_mut().unwrap();

        let query = Query::new(&request.query).with_files(request.context_files.clone());

        let mut context = builder.build_context(&query).await?;

        context.system_prompt = self.config.system_prompt.clone();

        let token_count = context.token_estimate();
        if token_count > self.config.max_context_tokens {
            warn!(
                "Context exceeds max tokens ({} > {}), truncating files",
                token_count, self.config.max_context_tokens
            );

            let ratio = self.config.max_context_tokens as f64 / token_count as f64;
            let target_files = (context.files.len() as f64 * ratio).ceil() as usize;
            context.files.truncate(target_files.max(1));
        }

        Ok(context)
    }
}
