//! Local context agent using Ollama.

use ollama_rs::{Ollama, generation::chat::ChatMessage};
use agentic_core::Result;
use crate::query::{QueryIntent, ContextPlan};
use super::agent::ContextAgent;
use super::prompts;

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
    async fn call_ollama_static(ollama: &Ollama, system: &str, user: &str) -> Result<String> {
        use ollama_rs::generation::chat::request::ChatMessageRequest;
        
        let messages = vec![
            ChatMessage::system(system.to_string()),
            ChatMessage::user(user.to_string()),
        ];

        // Get model from environment or use default
        let model = std::env::var("OLLAMA_MODEL")
            .unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());

        let request = ChatMessageRequest::new(model, messages);

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

        serde_json::from_str(json_str)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to parse context plan: {e}\nResponse: {json_str}")))
    }
}

impl Default for LocalContextAgent {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalContextAgent {
    /// Generate a context plan (async version)
    pub async fn generate_plan(&self, intent: &QueryIntent, query_text: &str) -> Result<ContextPlan> {
        let system = prompts::system_prompt();
        let user = prompts::user_prompt(query_text, intent);

        let response = Self::call_ollama_static(&self.ollama, &system, &user).await?;
        
        tracing::debug!("Ollama response: {}", response);
        
        self.parse_plan(&response)
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
