//! Local context agent using Ollama.

use std::time::Instant;
use ollama_rs::{Ollama, generation::chat::ChatMessage};
use agentic_core::Result;
use crate::query::{QueryIntent, ContextPlan};
use crate::models::{ModelConfig, TaskComplexity};
use super::agent::ContextAgent;
use super::prompts;
use serde_json::Value;

/// Local context agent that uses Ollama for planning
pub struct LocalContextAgent {
    /// Ollama client
    ollama: Ollama,
}

impl LocalContextAgent {
    /// Create a new local context agent
    #[must_use]
    pub fn new() -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        
        Self {
            ollama: Ollama::new(host, 11434),
        }
    }

    /// Create with custom configuration
    #[must_use]
    pub fn with_config(host: String, port: u16) -> Self {
        Self {
            ollama: Ollama::new(host, port),
        }
    }

    /// Call Ollama API using the ollama-rs crate (static version for use with Handle)
    async fn call_ollama_static(ollama: &Ollama, system: &str, user: &str, model: &str) -> Result<String> {
        use ollama_rs::generation::chat::request::ChatMessageRequest;
        
        let messages = vec![
            ChatMessage::system(system.to_string()),
            ChatMessage::user(user.to_string()),
        ];

        let request = ChatMessageRequest::new(model.to_string(), messages);

        let response = ollama
            .send_chat_messages(request)
            .await
            .map_err(|e| agentic_core::Error::Other(format!("Ollama request failed: {e}")))?;

        // Get the message content from the response
        // response.message is a ChatMessage struct with a content field
        Ok(response.message.content)
    }

    /// Parse the JSON response from the agent
    fn parse_plan(&self, response: &str) -> Result<ContextPlan> {
        // Try to extract JSON from markdown code blocks if present
        let json_str = if let Some(start) = response.find("```json") {
            let after_start = &response[start + 7..];
            if let Some(end) = after_start.find("```") {
                after_start[..end].trim()
            } else {
                response.trim()
            }
        } else if let Some(start) = response.find('{') {
            // Find the JSON object
            &response[start..]
        } else {
            response.trim()
        };

        // First attempt strict parsing
        match serde_json::from_str::<ContextPlan>(json_str) {
            Ok(plan) => Ok(plan),
            Err(first_err) => {
                // Fallback: be lenient and normalize common schema mistakes
                let mut value: Value = serde_json::from_str(json_str).map_err(|e| {
                    agentic_core::Error::Other(format!(
                        "Failed to parse context plan: {e}\nResponse: {json_str}"
                    ))
                })?;

                // Normalize: strategy.Focused with `patterns` -> `symbols`
                if let Some(strategy) = value.get_mut("strategy") {
                    if let Some(focused) = strategy.get_mut("Focused") {
                        if let Some(patterns) = focused.get_mut("patterns") {
                            let symbols = patterns.clone();
                            if let Some(obj) = focused.as_object_mut() {
                                obj.insert("symbols".to_string(), symbols);
                                obj.remove("patterns");
                            }
                        }
                    }
                }

                // Try deserializing again after normalization
                serde_json::from_value::<ContextPlan>(value).map_err(|e| {
                    agentic_core::Error::Other(format!(
                        "Failed to parse context plan after normalization. First error: {first_err}. Second error: {e}\nResponse: {json_str}"
                    ))
                })
            }
        }
    }
}

impl Default for LocalContextAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalContextAgent {
    /// Generate a context plan (async version)
    pub async fn generate_plan(&self, intent: &QueryIntent, query_text: &str, file_tree: &str) -> Result<ContextPlan> {
        let system = prompts::system_prompt();
        let user = prompts::user_prompt(query_text, intent, file_tree);

        let config = ModelConfig::from_env();
        let model = config.select_for_task(TaskComplexity::Medium);
        eprintln!("  Calling Ollama API (model: {})...", model);
        
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
        
        eprintln!("  Received response: {} chars (~{} tokens) in {:.2}s ({:.1} tok/s)", 
            response_chars, tokens_estimate, elapsed.as_secs_f64(), tokens_per_sec);
        
        eprintln!("  Parsing context plan...");
        let plan = self.parse_plan(&response)?;
        
        Ok(plan)
    }

    /// Check if Ollama is available (async version)
    pub async fn is_available(&self) -> Result<bool> {
        let result = self.ollama.list_local_models().await;
        Ok(result.is_ok())
    }
}

impl ContextAgent for LocalContextAgent {
    fn generate_plan_sync(&self, _intent: &QueryIntent, _query_text: &str) -> Result<ContextPlan> {
        // This is a sync wrapper - should not be called from async context
        // For now, return an error suggesting to use the async version
        Err(agentic_core::Error::Other(
            "generate_plan_sync called from async context. This is a bug.".into()
        ))
    }

    fn is_available_sync(&self) -> Result<bool> {
        // This is a sync wrapper - should not be called from async context
        Err(agentic_core::Error::Other(
            "is_available_sync called from async context. This is a bug.".into()
        ))
    }

    fn name(&self) -> &str {
        "LocalContextAgent (Ollama)"
    }
}
