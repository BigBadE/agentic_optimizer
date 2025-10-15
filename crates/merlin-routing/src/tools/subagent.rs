//! Subagent tool for delegating tasks to weaker/faster models.
//!
//! This tool allows a primary agent to spawn sub-tasks that are handled by
//! cheaper, faster models for simple operations like data extraction, formatting,
//! or basic analysis.

use async_trait::async_trait;
use merlin_core::{Context, ModelProvider as _, Query};
use merlin_local::LocalModelProvider;
use merlin_providers::{AnthropicProvider, GroqProvider};
use serde_json::{Value, json};
use std::env;

use crate::{Result, RoutingConfig, RoutingError, Tool};
use std::sync::Arc;

/// Tool that delegates simple tasks to weaker/faster models
pub struct SubagentTool {
    config: Arc<RoutingConfig>,
}

impl SubagentTool {
    /// Create a new subagent tool
    pub fn new(config: Arc<RoutingConfig>) -> Self {
        Self { config }
    }
}

impl Default for SubagentTool {
    fn default() -> Self {
        Self::new(Arc::new(RoutingConfig::default()))
    }
}

#[async_trait]
impl Tool for SubagentTool {
    fn name(&self) -> &'static str {
        "subagent"
    }

    fn description(&self) -> &'static str {
        "Delegate a simple task to a faster, cheaper model. Use this for:\n\
         - Data extraction from text\n\
         - Format conversion (JSON to CSV, etc.)\n\
         - Simple analysis or summarization\n\
         - Repetitive tasks that don't require deep reasoning\n\
         - Quick fact checking or lookups\n\
         The subagent has no access to tools or files, only the context you provide."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Clear description of what the subagent should do"
                },
                "context": {
                    "type": "string",
                    "description": "All context/data the subagent needs (it has no file access)"
                },
                "model_tier": {
                    "type": "string",
                    "enum": ["local", "groq", "premium"],
                    "description": "Which tier to use (default: local for speed)",
                    "default": "local"
                }
            },
            "required": ["task", "context"]
        })
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Tool execute method with multiple provider branches"
    )]
    async fn execute(&self, args: Value) -> Result<Value> {
        let task = args
            .get("task")
            .and_then(Value::as_str)
            .ok_or_else(|| RoutingError::Other("Missing 'task' parameter".to_owned()))?;

        let context_str = args
            .get("context")
            .and_then(Value::as_str)
            .ok_or_else(|| RoutingError::Other("Missing 'context' parameter".to_owned()))?;

        let model_tier_str = args
            .get("model_tier")
            .and_then(Value::as_str)
            .unwrap_or("local");

        // Build context with the provided data
        let system_prompt = format!(
            "You are a helpful AI assistant. Complete the following task concisely and accurately.\n\n\
             Context/Data:\n{context_str}\n\n\
             Task: {task}\n\n\
             Provide only the requested output without explanations unless asked."
        );

        let context = Context::new(system_prompt);
        let query = Query::new(task.to_owned());

        // Create provider based on tier and execute
        let (response, model_name) = match model_tier_str {
            "local" => {
                let provider = LocalModelProvider::new("qwen2.5-coder:7b".to_owned());
                let resp = provider.generate(&query, &context).await.map_err(|err| {
                    RoutingError::Other(format!("Local model execution failed: {err}"))
                })?;
                (resp, "qwen2.5-coder:7b (local)")
            }
            "groq" => {
                let api_key = self
                    .config
                    .get_api_key("groq")
                    .or_else(|| env::var("GROQ_API_KEY").ok())
                    .ok_or_else(|| {
                        RoutingError::Other(
                            "GROQ_API_KEY not found in config or environment".to_owned(),
                        )
                    })?;
                let provider = GroqProvider::new()
                    .map_err(|err| {
                        RoutingError::Other(format!("Groq provider init failed: {err}"))
                    })?
                    .with_api_key(api_key)
                    .with_model("llama-3.3-70b-versatile".to_owned());
                let resp = provider
                    .generate(&query, &context)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Groq execution failed: {err}")))?;
                (resp, "llama-3.3-70b-versatile (groq)")
            }
            "premium" => {
                let api_key = self
                    .config
                    .get_api_key("anthropic")
                    .or_else(|| env::var("ANTHROPIC_API_KEY").ok())
                    .ok_or_else(|| {
                        RoutingError::Other(
                            "ANTHROPIC_API_KEY not found in config or environment".to_owned(),
                        )
                    })?;
                let provider = AnthropicProvider::new(api_key).map_err(|err| {
                    RoutingError::Other(format!("Anthropic provider init failed: {err}"))
                })?;
                let resp = provider.generate(&query, &context).await.map_err(|err| {
                    RoutingError::Other(format!("Anthropic execution failed: {err}"))
                })?;
                (resp, "claude-3-5-haiku-20241022 (anthropic)")
            }
            _ => {
                return Err(RoutingError::Other(format!(
                    "Invalid model_tier: {model_tier_str}"
                )));
            }
        };

        // Return the response
        Ok(json!({
            "result": response.text,
            "model": model_name,
            "tokens_used": response.tokens_used,
            "confidence": response.confidence,
            "latency_ms": response.latency_ms
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subagent_tool_creation() {
        let subagent = SubagentTool::default();
        assert_eq!(subagent.name(), "subagent");
    }

    #[tokio::test]
    async fn test_subagent_tool_schema() {
        let subagent = SubagentTool::default();
        let schema = subagent.parameters_schema();

        assert!(schema.get("properties").is_some());
        assert!(schema.get("required").is_some());

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&json!("context")));
    }

    #[tokio::test]
    async fn test_subagent_missing_task() {
        let subagent = SubagentTool::default();

        let result = subagent.execute(json!({ "context": "some data" })).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'task'"));
    }

    #[tokio::test]
    async fn test_subagent_missing_context() {
        let subagent = SubagentTool::default();

        let result = subagent.execute(json!({ "task": "do something" })).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing 'context'")
        );
    }

    #[tokio::test]
    async fn test_subagent_invalid_model_tier() {
        let subagent = SubagentTool::default();

        let result = subagent
            .execute(json!({
                "task": "test",
                "context": "data",
                "model_tier": "invalid"
            }))
            .await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid model_tier")
        );
    }
}
