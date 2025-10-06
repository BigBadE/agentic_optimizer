use std::env;
use std::time::Instant;

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use merlin_core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};

/// `OpenRouter` API endpoint URL.
const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// Default model for `OpenRouter`.
const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-20250514";

/// Provider implementation for `OpenRouter` API.
pub struct OpenRouterProvider {
    /// HTTP client for API requests.
    client: Client,
    /// `OpenRouter` API key.
    api_key: String,
    /// Model name to use.
    model: String,
}

impl OpenRouterProvider {
    /// Creates a new `OpenRouterProvider` with the given API key.
    ///
    /// # Errors
    /// Returns an error if the provided API key is empty.
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(Error::MissingApiKey("OPENROUTER_API_KEY".to_owned()));
        }

        Ok(Self {
            client: Client::new(),
            api_key,
            model: DEFAULT_MODEL.to_owned(),
        })
    }

    /// Creates a new `OpenRouterProvider` from environment variables.
    ///
    /// # Errors
    /// Returns an error if the env var is missing.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENROUTER_API_KEY")
            .map_err(|_| Error::MissingApiKey("OPENROUTER_API_KEY".to_owned()))?;
        Self::new(api_key)
    }

    /// Creates a new `OpenRouterProvider` from config or environment.
    ///
    /// # Errors
    /// Returns an error if the API key is not provided.
    pub fn from_config_or_env(config_key: Option<String>) -> Result<Self> {
        let api_key = config_key
            .or_else(|| env::var("OPENROUTER_API_KEY").ok())
            .ok_or_else(|| Error::MissingApiKey("OPENROUTER_API_KEY or config.toml openrouter_key".to_owned()))?;
        Self::new(api_key)
    }

    /// Sets the model to use for generation.
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
}

/// Response payload returned by the `OpenRouter` API.
#[derive(Deserialize)]
struct OpenRouterResponse {
    /// List of generated choices.
    choices: Vec<Choice>,
    /// Optional token usage statistics returned by the service.
    usage: Option<Usage>,
}

/// Individual completion choice from `OpenRouter`.
#[derive(Deserialize)]
struct Choice {
    /// Message payload representing the completion text.
    message: Message,
}

/// Message structure containing generated content.
#[derive(Deserialize)]
struct Message {
    /// Text content produced by the model.
    content: String,
}

/// Token accounting information for a response.
#[derive(Deserialize)]
struct Usage {
    /// Number of prompt tokens billed for the request.
    prompt_tokens: u64,
    /// Number of completion tokens returned by the model.
    completion_tokens: u64,
    #[serde(default)]
    /// Detailed prompt token usage, when available.
    prompt_tokens_details: Option<PromptTokensDetails>,
}

/// Detailed prompt token usage breakdown.
#[derive(Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    /// Count of cached tokens supplied via the API.
    cached_tokens: u64,
}

#[async_trait]
impl ModelProvider for OpenRouterProvider {
    fn name(&self) -> &'static str {
        "openrouter"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();

        let mut messages = vec![
            json!({
                "role": "system",
                "content": context.system_prompt
            }),
        ];

        if !context.files.is_empty() {
            messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "text",
                    "text": format!("Context:\n{}", context.files_to_string()),
                    "cache_control": {"type": "ephemeral"}
                }]
            }));
        }

        messages.push(json!({
            "role": "user",
            "content": query.text
        }));

        let request_body = json!({
            "model": self.model,
            "messages": messages,
            "max_tokens": 4096,
        });

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/BigBadE/agentic_optimizer")
            .header("X-Title", "Agentic Optimizer")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "OpenRouter API request failed with status {status}: {error_text}"
            )));
        }

        let api_response: OpenRouterResponse = response.json().await?;

        let text = api_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| Error::Provider("No response from OpenRouter".to_owned()))?;

        let tokens_used = if let Some(usage) = api_response.usage {
            let cache_read = usage
                .prompt_tokens_details
                .as_ref()
                .map_or(0, |details| details.cached_tokens);

            TokenUsage {
                input: usage.prompt_tokens.saturating_sub(cache_read),
                output: usage.completion_tokens,
                cache_read,
                cache_write: 0,
            }
        } else {
            TokenUsage::default()
        };

        let latency_ms = start.elapsed().as_millis() as u64;

        Ok(Response {
            text,
            confidence: 1.0,
            tokens_used,
            provider: self.name().to_owned(),
            latency_ms,
        })
    }

    fn estimate_cost(&self, context: &Context) -> f64 {
        let tokens = context.token_estimate() as f64;
        tokens * 3.0 / 1_000_000.0
    }
}

