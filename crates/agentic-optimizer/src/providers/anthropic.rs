use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::env;
use std::time::Instant;

use crate::core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};

/// Anthropic Messages endpoint URL.
const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
/// Default model used for generation.
const MODEL: &str = "claude-sonnet-4-20250514";
/// Anthropic API version header value.
const API_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    /// HTTP client used for requests.
    client: Client,
    /// API key for Anthropic.
    api_key: String,
}

impl AnthropicProvider {
    /// Create a new provider instance.
    ///
    /// # Errors
    /// Returns an error if the provided API key is empty.
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(Error::MissingApiKey("ANTHROPIC_API_KEY".to_owned()));
        }

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    /// Create a provider by reading `ANTHROPIC_API_KEY` from the environment.
    ///
    /// # Errors
    /// Returns an error if the env var is missing.
    pub fn from_env() -> Result<Self> {
        let api_key = env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::MissingApiKey("ANTHROPIC_API_KEY".to_owned()))?;
        Self::new(api_key)
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic-sonnet-4"
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();

        let user_message = if context.files.is_empty() {
            query.text.clone()
        } else {
            format!(
                "{}\n\nContext:\n{}",
                query.text,
                context.files_to_string()
            )
        };

        let request_body = json!({
            "model": MODEL,
            "max_tokens": 4096i32,
            "system": context.system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_message
                }
            ]
        });

        let response = self
            .client
            .post(ANTHROPIC_API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "API request failed with status {status}: {error_text}"
            )));
        }

        let api_response: AnthropicResponse = response.json().await?;

        let text = api_response
            .content
            .first()
            .map(|block| {
                let ContentBlock::Text { text } = block;
                text.clone()
            })
            .ok_or_else(|| Error::InvalidResponse("No text content in response".to_owned()))?;

        let tokens_used = TokenUsage {
            input: api_response.usage.input,
            output: api_response.usage.output,
            cache_read: api_response.usage.cache_read_input.unwrap_or(0),
            cache_write: api_response.usage.cache_creation_input.unwrap_or(0),
        };

        Ok(Response {
            text,
            confidence: 0.9,
            tokens_used,
            provider: self.name().to_owned(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn estimate_cost(&self, context: &Context) -> f64 {
        let tokens = context.token_estimate() as f64;
        tokens * 3.0 / 1_000_000.0
    }
}

#[derive(Debug, Deserialize)]
/// Root payload from Anthropic Messages API.
struct AnthropicResponse {
    /// Content blocks from the model.
    content: Vec<ContentBlock>,
    /// Token usage information.
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
/// Message content returned by Anthropic.
enum ContentBlock {
    /// A text block.
    Text { 
        /// The text content.
        text: String 
    },
}

#[derive(Debug, Deserialize)]
/// Token usage breakdown from Anthropic response.
struct Usage {
    /// Input tokens billed.
    #[serde(rename = "input_tokens")]
    input: u64,
    /// Output tokens returned.
    #[serde(rename = "output_tokens")]
    output: u64,
    /// Cached tokens read (if any).
    #[serde(default, rename = "cache_read_input_tokens")]
    cache_read_input: Option<u64>,
    /// Tokens written into cache (if any).
    #[serde(default, rename = "cache_creation_input_tokens")]
    cache_creation_input: Option<u64>,
}
