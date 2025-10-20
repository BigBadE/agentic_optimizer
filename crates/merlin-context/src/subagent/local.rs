//! Local context agent using Ollama.

use serde_json::{from_str, from_value};
use std::env;
use std::time::Instant;

use super::agent::ContextAgent;
use super::prompts;
use crate::models::{ModelConfig, TaskComplexity};
use crate::query::{ContextPlan, QueryIntent};
use merlin_core::{CoreResult as Result, Error};
use ollama_rs::{Ollama, generation::chat::ChatMessage};
use serde_json::Value;

/// Local context agent that uses Ollama for planning
pub struct LocalContextAgent {
    /// Ollama client
    ollama: Ollama,
}

impl LocalContextAgent {
    /// Creates a new agent with custom configuration.
    pub fn with_config(host: String, port: u16) -> Self {
        Self {
            ollama: Ollama::new(host, port),
        }
    }

    /// Calls Ollama API using the ollama-rs crate.
    /// # Errors
    /// Returns an error when the Ollama request fails.
    async fn call_ollama_static(
        ollama: &Ollama,
        system: &str,
        user: &str,
        model: &str,
    ) -> Result<String> {
        use ollama_rs::generation::chat::request::ChatMessageRequest;

        let messages = vec![
            ChatMessage::system(system.to_owned()),
            ChatMessage::user(user.to_owned()),
        ];

        let request = ChatMessageRequest::new(model.to_owned(), messages);

        let response = ollama
            .send_chat_messages(request)
            .await
            .map_err(|err| Error::Other(format!("Ollama request failed: {err}")))?;

        // Get the message content from the response
        // response.message is a ChatMessage struct with a content field
        Ok(response.message.content)
    }

    /// Parses the JSON response from the agent.
    /// # Errors
    /// Returns an error if the response cannot be parsed into a `ContextPlan`.
    fn parse_plan(response: &str) -> Result<ContextPlan> {
        // Try to extract JSON from markdown code blocks if present
        let json_str = response.find("```json").map_or_else(
            || {
                response
                    .find('{')
                    .map_or_else(|| response.trim(), |start| &response[start..])
            },
            |start| {
                let after_start = &response[start + 7..];
                after_start
                    .find("```")
                    .map_or_else(|| response.trim(), |end| after_start[..end].trim())
            },
        );

        // First attempt strict parsing
        match from_str::<ContextPlan>(json_str) {
            Ok(plan) => Ok(plan),
            Err(first_err) => {
                // Fallback: be lenient and normalize common schema mistakes
                let mut value: Value = from_str(json_str).map_err(|err| {
                    Error::Other(format!(
                        "Failed to parse context plan: {err}\nResponse: {json_str}"
                    ))
                })?;

                // Normalize: strategy.Focused with `patterns` -> `symbols`
                if let Some(strategy) = value.get_mut("strategy")
                    && let Some(focused) = strategy.get_mut("Focused")
                    && let Some(patterns) = focused.get_mut("patterns")
                {
                    let symbols = patterns.clone();
                    if let Some(obj) = focused.as_object_mut() {
                        obj.insert("symbols".to_owned(), symbols);
                        obj.remove("patterns");
                    }
                }

                // Try deserializing again after normalization
                from_value::<ContextPlan>(value).map_err(|second_err| {
                    Error::Other(format!(
                        "Failed to parse context plan after normalization. First error: {first_err}. Second error: {second_err}\nResponse: {json_str}"
                    ))
                })
            }
        }
    }

    /// Generates a context plan using the agent.
    ///
    /// # Errors
    /// Returns an error if the Ollama API call fails or the response cannot be parsed.
    pub async fn generate_plan(
        &self,
        intent: &QueryIntent,
        query_text: &str,
        file_tree: &str,
    ) -> Result<ContextPlan> {
        let system = prompts::system_prompt();
        let user = prompts::user_prompt(query_text, intent, file_tree);

        let config = ModelConfig::from_env();
        let model = config.select_for_task(TaskComplexity::Medium);
        tracing::info!("Calling Ollama API (model: {model})...");

        let start = Instant::now();
        let response = Self::call_ollama_static(&self.ollama, &system, &user, model).await?;
        let elapsed = start.elapsed();

        let response_chars = response.len();
        let tokens_estimate = response_chars / 4;
        let tokens_per_sec = if elapsed.as_secs_f64() > 0.0 {
            tokens_estimate as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        tracing::info!(
            "Received response: {} chars (~{} tokens) in {:.2}s ({:.1} tok/s)",
            response_chars,
            tokens_estimate,
            elapsed.as_secs_f64(),
            tokens_per_sec
        );

        tracing::info!("Parsing context plan...");
        let plan = Self::parse_plan(&response)?;

        Ok(plan)
    }

    /// Checks if Ollama is available.
    ///
    /// # Errors
    /// Returns an error if the Ollama API cannot be queried.
    pub async fn is_available(&self) -> Result<bool> {
        let result = self.ollama.list_local_models().await;
        Ok(result.is_ok())
    }
}

impl ContextAgent for LocalContextAgent {
    fn generate_plan_sync(&self, _intent: &QueryIntent, _query_text: &str) -> Result<ContextPlan> {
        // This is a sync wrapper - should not be called from async context
        // For now, return an error suggesting to use the async version
        Err(Error::Other(
            "generate_plan_sync called from async context. This is a bug.".into(),
        ))
    }

    fn is_available_sync(&self) -> Result<bool> {
        // This is a sync wrapper - should not be called from async context
        Err(Error::Other(
            "is_available_sync called from async context. This is a bug.".into(),
        ))
    }

    fn name(&self) -> &'static str {
        "LocalContextAgent (Ollama)"
    }
}

impl Default for LocalContextAgent {
    /// Creates a new local context agent.
    fn default() -> Self {
        let host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_owned());

        Self {
            ollama: Ollama::new(host, 11434),
        }
    }
}
