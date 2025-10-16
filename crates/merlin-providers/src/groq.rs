use async_trait::async_trait;
use merlin_core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Instant;

/// Groq API endpoint URL.
const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
/// Default model for Groq.
const DEFAULT_MODEL: &str = "llama-3.1-70b-versatile";
/// Env var key for Groq API key.
const ENV_GROQ_API_KEY: &str = "GROQ_API_KEY";

/// Groq API provider (free tier with rate limits).
pub struct GroqProvider {
    /// HTTP client for API requests.
    client: Client,
    /// Groq API key.
    api_key: String,
    /// Model name to use.
    model: String,
}

impl GroqProvider {
    /// Creates a new `GroqProvider` from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if the `GROQ_API_KEY` environment variable is not set.
    pub fn new() -> Result<Self> {
        let api_key = env::var(ENV_GROQ_API_KEY)
            .map_err(|_| Error::Other(format!("{ENV_GROQ_API_KEY} not set")))?;

        Ok(Self {
            client: Client::default(),
            api_key,
            model: DEFAULT_MODEL.to_owned(),
        })
    }

    /// Creates a new `GroqProvider` with the given API key.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided API key is empty.
    pub fn with_api_key_direct(api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(Error::MissingApiKey(ENV_GROQ_API_KEY.to_owned()));
        }

        Ok(Self {
            client: Client::default(),
            api_key,
            model: DEFAULT_MODEL.to_owned(),
        })
    }

    /// Sets the model to use for generation.
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Sets the API key.
    #[must_use]
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = api_key;
        self
    }
}

/// Request payload sent to the Groq chat completion API.
#[derive(Debug, Serialize)]
struct GroqRequest {
    /// Model identifier provided by the Groq service.
    model: String,
    /// Messages that form the conversation context for the request.
    messages: Vec<GroqMessage>,
    /// Sampling temperature controlling response randomness.
    temperature: f32,
    /// Maximum number of tokens allowed in the completion.
    max_tokens: usize,
}

/// Message delivered to the Groq API.
#[derive(Debug, Serialize)]
struct GroqMessage {
    /// Role of the message author (for example `system` or `user`).
    role: String,
    /// Textual content of the message.
    content: String,
}

/// Response payload returned by Groq.
#[derive(Debug, Deserialize)]
struct GroqResponse {
    /// List of candidate completions.
    choices: Vec<GroqChoice>,
    /// Token accounting information for the request.
    usage: GroqUsage,
}

/// A single completion choice returned by Groq.
#[derive(Debug, Deserialize)]
struct GroqChoice {
    /// Message generated for the choice.
    message: GroqResponseMessage,
}

/// Response message containing the generated text.
#[derive(Debug, Deserialize)]
struct GroqResponseMessage {
    /// Generated text content.
    content: String,
}

/// Token usage metrics for a Groq response.
#[derive(Debug, Deserialize)]
struct GroqUsage {
    /// Number of tokens in the prompt portion of the request.
    prompt_tokens: usize,
    /// Number of tokens produced in the completion.
    completion_tokens: usize,
}

#[async_trait]
impl ModelProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "Groq"
    }

    async fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();

        // Use provided system prompt or default
        let system_content = if context.system_prompt.is_empty() {
            "You are an expert coding assistant. Provide clear, concise, and correct code solutions.".to_owned()
        } else {
            context.system_prompt.clone()
        };

        let mut messages = vec![GroqMessage {
            role: "system".to_owned(),
            content: system_content,
        }];

        let mut user_content = query.text.clone();

        if !context.files.is_empty() {
            user_content.push_str("\n\nContext files:\n");
            for file_ctx in &context.files {
                user_content.push_str("\n--- ");
                user_content.push_str(&file_ctx.path.display().to_string());
                user_content.push_str(" ---\n");
                user_content.push_str(&file_ctx.content);
                user_content.push('\n');
            }
        }

        messages.push(GroqMessage {
            role: "user".to_owned(),
            content: user_content,
        });

        let request = GroqRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: 8000,
        };

        let response = self
            .client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|err| Error::Other(format!("Groq API request failed: {err}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_owned());
            return Err(Error::Other(format!(
                "Groq API error {status}: {error_text}"
            )));
        }

        let groq_response: GroqResponse = response
            .json()
            .await
            .map_err(|err| Error::Other(format!("Failed to parse Groq response: {err}")))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        let text = groq_response
            .choices
            .first()
            .map(|choice| choice.message.content.clone())
            .ok_or_else(|| Error::Other("No response from Groq".to_owned()))?;

        let tokens_used = TokenUsage {
            input: groq_response.usage.prompt_tokens as u64,
            output: groq_response.usage.completion_tokens as u64,
            cache_read: 0,
            cache_write: 0,
        };

        Ok(Response {
            text,
            confidence: 0.9,
            tokens_used,
            provider: format!("Groq/{}", self.model),
            latency_ms,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groq_provider_with_api_key() {
        let provider = GroqProvider {
            client: Client::default(),
            api_key: "test_key".to_owned(),
            model: DEFAULT_MODEL.to_owned(),
        };

        assert_eq!(provider.name(), "Groq");
        assert_eq!(provider.model, DEFAULT_MODEL);
    }

    #[test]
    fn cost_estimation() {
        let provider = GroqProvider {
            client: Client::default(),
            api_key: "test_key".to_owned(),
            model: DEFAULT_MODEL.to_owned(),
        };

        let context = Context::new("");
        let cost = provider.estimate_cost(&context);
        assert!(cost.abs() < f64::EPSILON);
    }
}
