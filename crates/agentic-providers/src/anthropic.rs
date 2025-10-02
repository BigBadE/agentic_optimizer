use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::time::Instant;

use agentic_core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-20250514";
const API_VERSION: &str = "2023-06-01";

pub struct AnthropicProvider {
    client: Client,
    api_key: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(Error::MissingApiKey("ANTHROPIC_API_KEY".to_string()));
        }

        Ok(Self {
            client: Client::new(),
            api_key,
        })
    }

    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| Error::MissingApiKey("ANTHROPIC_API_KEY".to_string()))?;
        Self::new(api_key)
    }
}

#[async_trait]
impl ModelProvider for AnthropicProvider {
    fn name(&self) -> &str {
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
            "max_tokens": 4096,
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
            .map(|c| {
                let ContentBlock::Text { text } = c;
                text.clone()
            })
            .ok_or_else(|| Error::InvalidResponse("No text content in response".to_string()))?;

        let tokens_used = TokenUsage {
            input: api_response.usage.input_tokens,
            output: api_response.usage.output_tokens,
            cache_read: api_response.usage.cache_read_input_tokens.unwrap_or(0),
            cache_write: api_response.usage.cache_creation_input_tokens.unwrap_or(0),
        };

        Ok(Response {
            text,
            confidence: 0.9,
            tokens_used,
            provider: self.name().to_string(),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn estimate_cost(&self, context: &Context) -> f64 {
        let tokens = context.token_estimate() as f64;
        tokens * 3.0 / 1_000_000.0
    }
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlock {
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
}
