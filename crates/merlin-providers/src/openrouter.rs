use std::env;
use std::time::Instant;

use async_trait::async_trait;
use merlin_deps::reqwest::Client;
use merlin_deps::serde_json::{Value, json};
use serde::Deserialize;

use merlin_core::{Context, CoreResult, Error, ModelProvider, Query, Response, Result, TokenUsage};

/// `OpenRouter` API endpoint URL.
const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";
/// Default model for `OpenRouter`.
const DEFAULT_MODEL: &str = "anthropic/claude-sonnet-4-20250514";
/// Env var key for `OpenRouter` API key.
const ENV_OPENROUTER_API_KEY: &str = "OPENROUTER_API_KEY";

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
    pub fn new(api_key: String) -> CoreResult<Self> {
        if api_key.is_empty() {
            return Err(Error::MissingApiKey(ENV_OPENROUTER_API_KEY.to_owned()));
        }

        Ok(Self {
            client: Client::default(),
            api_key,
            model: DEFAULT_MODEL.to_owned(),
        })
    }

    /// Creates a new `OpenRouterProvider` from environment variables.
    ///
    /// # Errors
    /// Returns an error if the env var is missing.
    pub fn from_env() -> CoreResult<Self> {
        let api_key = env::var(ENV_OPENROUTER_API_KEY)
            .map_err(|_| Error::MissingApiKey(ENV_OPENROUTER_API_KEY.to_owned()))?;
        Self::new(api_key)
    }

    /// Creates a new `OpenRouterProvider` from config or environment.
    ///
    /// # Errors
    /// Returns an error if the API key is not provided.
    pub fn from_config_or_env(config_key: Option<String>) -> CoreResult<Self> {
        let api_key = config_key
            .or_else(|| env::var(ENV_OPENROUTER_API_KEY).ok())
            .ok_or_else(|| {
                Error::MissingApiKey(format!(
                    "{ENV_OPENROUTER_API_KEY} or config.toml openrouter_key"
                ))
            })?;
        Self::new(api_key)
    }

    /// Sets the model to use for generation.
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Builds messages from context and query for the `OpenRouter` API.
    fn build_messages(context: &Context, query: &Query) -> Vec<Value> {
        let mut messages = vec![json!({
            "role": "system",
            "content": context.system_prompt
        })];

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

        messages
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

        let messages = Self::build_messages(context, query);

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
            .header(
                "HTTP-Referer",
                "https://github.com/BigBadE/agentic_optimizer",
            )
            .header("X-Title", "Agentic Optimizer")
            .json(&request_body)
            .send()
            .await
            .map_err(|err| Error::Provider(format!("Request failed: {err}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Error::Provider(format!(
                "OpenRouter API request failed with status {status}: {error_text}"
            ))
            .into());
        }

        let api_response: OpenRouterResponse = response
            .json()
            .await
            .map_err(|err| Error::Provider(format!("Failed to parse response: {err}")))?;

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
                input: usage.prompt_tokens - cache_read,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that creating a provider with an empty API key returns an error.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_new_with_empty_api_key() {
        let result = OpenRouterProvider::new(String::new());
        assert!(result.is_err(), "Empty API key should return an error");

        if let Err(err) = result {
            assert!(
                matches!(err, Error::MissingApiKey(_)),
                "Should be a MissingApiKey error"
            );
        }
    }

    /// Tests that creating a provider with a valid API key succeeds.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_new_with_valid_api_key() {
        let result = OpenRouterProvider::new("valid_key".to_owned());
        assert!(result.is_ok(), "Valid API key should succeed");

        if let Ok(provider) = result {
            assert_eq!(provider.api_key, "valid_key");
            assert_eq!(provider.model, DEFAULT_MODEL);
        }
    }

    /// Tests that `with_model` correctly sets the model.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_with_model() {
        let result = OpenRouterProvider::new("test_key".to_owned());
        assert!(result.is_ok());
        if let Ok(provider) = result {
            let provider = provider.with_model("custom-model".to_owned());
            assert_eq!(provider.model, "custom-model");
        }
    }

    /// Tests provider name returns correct identifier.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_provider_name() {
        let result = OpenRouterProvider::new("test_key".to_owned());
        assert!(result.is_ok());
        if let Ok(provider) = result {
            assert_eq!(provider.name(), "openrouter");
        }
    }

    /// Tests cost estimation for non-empty context.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cost_estimation() {
        let result = OpenRouterProvider::new("test_key".to_owned());
        assert!(result.is_ok());
        if let Ok(provider) = result {
            let context = Context::new("test query");
            let cost = provider.estimate_cost(&context);

            // Cost should be positive for non-empty context
            assert!(cost > 0.0, "Cost should be positive for non-empty context");
        }
    }

    /// Tests that cost scales with context size.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_cost_estimation_scaling() {
        let result = OpenRouterProvider::new("test_key".to_owned());
        assert!(result.is_ok());
        if let Ok(provider) = result {
            let small_context = Context::new("small");
            let large_context = Context::new("large ".repeat(100));

            let small_cost = provider.estimate_cost(&small_context);
            let large_cost = provider.estimate_cost(&large_context);

            // Larger context should cost more
            assert!(
                large_cost > small_cost,
                "Larger context should have higher cost"
            );
        }
    }

    /// Tests message building with context and query.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_build_messages_with_context() {
        let context = Context::new("test query");
        let query = Query::new("user question");

        let messages = OpenRouterProvider::build_messages(&context, &query);

        // Should have at least 2 messages (system + user)
        assert!(messages.len() >= 2, "Should have at least 2 messages");

        // First message should be system
        assert_eq!(
            messages[0]["role"].as_str(),
            Some("system"),
            "First message should be system role"
        );

        // Last message should be user with query text
        assert!(!messages.is_empty(), "Messages should not be empty");
        let last = &messages[messages.len() - 1];
        assert_eq!(
            last["role"].as_str(),
            Some("user"),
            "Last message should be user role"
        );
        assert_eq!(
            last["content"].as_str(),
            Some("user question"),
            "Last message should contain query text"
        );
    }

    /// Tests that methods can be chained.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_model_chaining() {
        let result = OpenRouterProvider::new("test_key".to_owned());
        assert!(result.is_ok());
        if let Ok(base_provider) = result {
            let provider = base_provider.with_model("custom-model".to_owned());
            assert_eq!(provider.model, "custom-model");
            assert_eq!(provider.api_key, "test_key");
        }
    }
}
