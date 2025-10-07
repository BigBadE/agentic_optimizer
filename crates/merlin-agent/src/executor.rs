use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result as AnyResult;
use merlin_context::ContextBuilder;
use merlin_core::{Context, ModelProvider, Query};
use merlin_languages::LanguageProvider;
use merlin_tools::ToolInput;
use serde_json::{from_str, Value};
use tracing::{info, warn};

use crate::{AgentConfig, AgentRequest, AgentResponse, ExecutionMetadata, ExecutionResult, ToolRegistry};

pub struct AgentExecutor {
    provider: Arc<dyn ModelProvider>,
    config: AgentConfig,
    context_builder: Option<ContextBuilder>,
    tool_registry: ToolRegistry,
}

impl AgentExecutor {
    pub fn new(provider: Arc<dyn ModelProvider>, config: AgentConfig) -> Self {
        Self {
            provider,
            config,
            context_builder: None,
            tool_registry: ToolRegistry::new(),
        }
    }

    #[must_use]
    pub fn with_language_backend(mut self, backend: Box<dyn LanguageProvider>) -> Self {
        let builder = if let Some(mut builder) = self.context_builder.take() {
            builder = builder.with_language_backend(backend);
            builder
        } else {
            ContextBuilder::new(env::current_dir().unwrap_or_default())
                .with_language_backend(backend)
        };
        self.context_builder = Some(builder);
        self
    }

    #[must_use] 
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    /// Execute an agent request
    ///
    /// # Errors
    /// Returns an error if context building or provider generation fails
    pub async fn execute(&mut self, request: AgentRequest) -> AnyResult<ExecutionResult> {
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
        let mut response = self.provider.generate(&query, &context).await?;
        let provider_call_time = provider_start.elapsed().as_millis() as u64;

        // Check if response contains a tool call and execute it
        if let Some(tool_result) = self.try_execute_tool_call(&response.text, &request.workspace_root).await {
            info!("Tool call detected and executed");
            response.text = tool_result;
        }

        let total_time = total_start.elapsed().as_millis() as u64;

        let agent_response = AgentResponse {
            content: response.text,
            confidence: response.confidence,
            provider_used: response.provider,
            tokens_used: response.tokens_used,
            latency_ms: response.latency_ms,
            context_files_used: context.files.iter().map(|file| file.path.clone()).collect(),
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

    /// # Errors
    /// Returns an error if building the context fails.
    async fn build_context(&mut self, request: &AgentRequest) -> AnyResult<Context> {
        if self.context_builder.is_none() {
            self.context_builder = Some(
                ContextBuilder::new(request.workspace_root.clone())
                    .with_max_files(self.config.top_k_context_files),
            );
        }

        let Some(builder) = self.context_builder.as_mut() else {
            return Err(anyhow::anyhow!("context_builder not initialized"));
        };

        let query = Query::new(&request.query).with_files(request.context_files.clone());

        let mut context = builder.build_context(&query).await?;

        context.system_prompt = self.build_system_prompt();

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

    fn build_system_prompt(&self) -> String {
        let tools = self.tool_registry.list_tools();
        
        let mut prompt = self.config.system_prompt.clone();
        
        if !tools.is_empty() {
            prompt.push_str("\n\n# Available Tools\n\n");
            prompt.push_str("You have access to the following tools to help complete tasks:\n\n");
            
            for (name, description) in tools {
                prompt.push_str("## ");
                prompt.push_str(name);
                prompt.push('\n');
                prompt.push_str(description);
                prompt.push_str("\n\n");
            }
            
            prompt.push_str("To use a tool, respond with a JSON object in the following format:\n");
            prompt.push_str("```json\n");
            prompt.push_str("{\n");
            prompt.push_str("  \"tool\": \"tool_name\",\n");
            prompt.push_str("  \"params\": {\n");
            prompt.push_str("    \"param1\": \"value1\",\n");
            prompt.push_str("    \"param2\": \"value2\"\n");
            prompt.push_str("  }\n");
            prompt.push_str("}\n");
            prompt.push_str("```\n\n");
            prompt.push_str("IMPORTANT: For file_path parameters, ALWAYS use the full relative path from the workspace root.\n");
            prompt.push_str("Examples:\n");
            prompt.push_str("- CORRECT: \"crates/agentic-tools/src/lib.rs\"\n");
            prompt.push_str("- CORRECT: \"benchmarks/testing.md\"\n");
            prompt.push_str("- WRONG: \"lib.rs\" (ambiguous - which lib.rs?)\n");
            prompt.push_str("- WRONG: \"testing.md\" (ambiguous - which directory?)\n");
        }
        
        prompt
    }

    async fn try_execute_tool_call(&self, response_text: &str, workspace_root: &Path) -> Option<String> {
        let mut tool_call = Self::extract_tool_call(response_text)?;
        
        info!("Detected tool call: {} with params: {:?}", tool_call.tool, tool_call.input.params);
        
        // Resolve all file paths relative to workspace root
        if let Some(params_obj) = tool_call.input.params.as_object_mut()
            && let Some(file_path) = params_obj.get("file_path").and_then(|value| value.as_str()) {
            let path = Path::new(file_path);
            // Always resolve relative to workspace root (even if path looks absolute)
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                workspace_root.join(path)
            };
            let absolute_path_str = absolute_path.to_string_lossy().to_string();
            info!("Resolved path '{}' to '{}'", file_path, absolute_path.display());
            params_obj.insert("file_path".to_owned(), serde_json::json!(absolute_path_str));
        }
        
        match self.tool_registry.execute(&tool_call.tool, tool_call.input.clone()).await {
            Ok(output) => {
                let result = if output.success {
                    format!("Tool '{}' executed successfully:\n{}\n\nData: {:?}", 
                        tool_call.tool, output.message, output.data)
                } else {
                    format!("Tool '{}' failed:\n{}\n\nInput: {:?}", 
                        tool_call.tool, output.message, tool_call.input.params)
                };
                Some(result)
            }
            Err(error) => {
                warn!("Tool execution failed: {} with input: {:?}", error, tool_call.input.params);
                Some(format!("Tool execution failed: {error}\n\nInput: {:?}", tool_call.input.params))
            }
        }
    }

    fn extract_tool_call(text: &str) -> Option<ToolCall> {
        let json_str = if let Some(start) = text.find("```json") {
            let after_start = text.get(start + 7..)?;
            let end = after_start.find("```")?;
            after_start.get(..end)?.trim()
        } else if let Some(start) = text.find('{') {
            let end = text.rfind('}')?;
            text.get(start..=end)?
        } else {
            return None;
        };

        let value: Value = from_str(json_str).ok()?;
        
        let tool = value.get("tool")?.as_str()?.to_owned();
        let params = value.get("params")?.clone();
        
        Some(ToolCall {
            tool,
            input: ToolInput { params },
        })
    }
}

struct ToolCall {
    tool: String,
    input: ToolInput,
}

