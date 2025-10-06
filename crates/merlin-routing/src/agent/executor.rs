use serde_json::Value;
use std::sync::Arc;
use std::time::Instant;

use crate::{
    ModelRouter, Result, RoutingError, Task, TaskId, TaskResult, ToolRegistry, UiChannel,
    UiEvent, Validator, streaming::StepType,
};
use merlin_core::{Context, ModelProvider, Query, Response};

use super::step::StepTracker;

/// Agent executor that streams task execution with tool calling
pub struct AgentExecutor {
    router: Arc<dyn ModelRouter>,
    validator: Arc<dyn Validator>,
    tool_registry: Arc<ToolRegistry>,
    step_tracker: StepTracker,
}

impl AgentExecutor {
    pub fn new(
        router: Arc<dyn ModelRouter>,
        validator: Arc<dyn Validator>,
        tool_registry: Arc<ToolRegistry>,
    ) -> Self {
        Self {
            router,
            validator,
            tool_registry,
            step_tracker: StepTracker::new(),
        }
    }
    
    /// Execute a task with streaming updates
    pub async fn execute_streaming(
        &mut self,
        task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        let start = Instant::now();
        let task_id = task.id;
        
        // Step 1: Route the task
        let decision = self.router.route(&task).await?;
        
        // Step 2: Create provider
        let provider = self.create_provider(&decision.tier)?;
        
        // Step 3: Build context
        let context = self.build_context(&task).await?;
        
        // Step 4: Create query with tool descriptions
        let query = self.create_query_with_tools(&task);
        
        // Step 5: Execute with streaming
        let response = self.execute_with_streaming(
            task_id,
            &provider,
            &query,
            &context,
            &ui_channel,
        ).await?;
        
        // Step 6: Validate
        let validation = if true { // TODO: Check config
            self.validator.validate(&response, &task).await?
        } else {
            crate::ValidationResult::default()
        };
        
        let duration_ms = start.elapsed().as_millis() as u64;
        
        Ok(TaskResult {
            task_id,
            response,
            tier_used: decision.tier.to_string(),
            validation,
            duration_ms,
        })
    }
    
    /// Execute with streaming and tool calling support
    async fn execute_with_streaming(
        &mut self,
        task_id: TaskId,
        provider: &Arc<dyn ModelProvider>,
        query: &Query,
        context: &Context,
        ui_channel: &UiChannel,
    ) -> Result<Response> {
        // Execute the query directly without extra steps
        let mut response = provider.generate(query, context).await
            .map_err(|e| RoutingError::Other(format!("Provider error: {}", e)))?;
        
        // Check if response contains tool calls (simulated for now)
        // In a real implementation, this would parse the LLM response for tool calls
        let tool_calls = self.extract_tool_calls(&response);
        
        if !tool_calls.is_empty() {
            // Execute tool calls
            for (tool_name, args) in tool_calls {
                // Send tool call started event
                ui_channel.send(UiEvent::ToolCallStarted {
                    task_id,
                    tool: tool_name.clone(),
                    args: args.clone(),
                });
                
                let tool_step = self.step_tracker.create_step(
                    task_id,
                    StepType::ToolCall {
                        tool: tool_name.clone(),
                        args: args.clone(),
                    },
                    format!("Calling tool: {}", tool_name),
                );
                
                ui_channel.send(UiEvent::TaskStepStarted {
                    task_id,
                    step_id: format!("{:?}", tool_step.id),
                    step_type: "ToolCall".to_string(),
                    content: tool_step.content.clone(),
                });
                
                // Execute the tool
                let result = self.execute_tool(&tool_name, args).await?;
                
                // Send tool call completed event
                ui_channel.send(UiEvent::ToolCallCompleted {
                    task_id,
                    tool: tool_name.clone(),
                    result: result.clone(),
                });
                
                let result_step = self.step_tracker.create_step(
                    task_id,
                    StepType::ToolResult {
                        tool: tool_name.clone(),
                        result: result.clone(),
                    },
                    format!("Tool result: {}", result),
                );
                
                ui_channel.send(UiEvent::TaskStepCompleted {
                    task_id,
                    step_id: format!("{:?}", result_step.id),
                });
                
                // Add tool result to response (in real implementation, would re-query LLM with results)
                response.text.push_str(&format!("\n\nTool '{}' result: {}", tool_name, result));
            }
        }
        
        // Send output directly as text (no wrapper step)
        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: response.text.clone(),
        });
        
        Ok(response)
    }
    
    /// Execute a tool by name
    async fn execute_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        let tool = self.tool_registry.get_tool(tool_name)
            .ok_or_else(|| RoutingError::Other(format!("Tool not found: {}", tool_name)))?;
        
        tool.execute(args).await
    }
    
    /// Extract tool calls from LLM response (simplified for Phase 2)
    /// In a real implementation, this would parse function calling format
    fn extract_tool_calls(&self, _response: &Response) -> Vec<(String, Value)> {
        // For Phase 2, we'll simulate tool calling by looking for markers in the text
        // Real implementation would use proper function calling API
        let tool_calls = Vec::new();
        
        // Example: Look for patterns like "TOOL:read_file:path/to/file"
        // This is a placeholder - real implementation would use LLM's function calling
        
        tool_calls
    }
    
    /// Create query with tool descriptions
    fn create_query_with_tools(&self, task: &Task) -> Query {
        let mut prompt = task.description.clone();
        
        // Add tool descriptions to the prompt
        let tools = self.tool_registry.list_tools();
        if !tools.is_empty() {
            prompt.push_str("\n\nAvailable tools:\n");
            for tool in tools {
                prompt.push_str(&format!(
                    "- {}: {}\n",
                    tool.name(),
                    tool.description()
                ));
            }
        }
        
        Query::new(prompt)
    }
    
    /// Create provider based on tier
    fn create_provider(&self, tier: &crate::ModelTier) -> Result<Arc<dyn ModelProvider>> {
        match tier {
            crate::ModelTier::Local { model_name } => {
                Ok(Arc::new(merlin_local::LocalModelProvider::new(model_name.clone())))
            }
            crate::ModelTier::Groq { model_name } => {
                let provider = merlin_providers::GroqProvider::new()
                    .map_err(|e| RoutingError::Other(e.to_string()))?
                    .with_model(model_name.clone());
                Ok(Arc::new(provider))
            }
            crate::ModelTier::Premium { provider: provider_name, model_name } => {
                match provider_name.as_str() {
                    "openrouter" => {
                        let api_key = std::env::var("OPENROUTER_API_KEY")
                            .map_err(|_| RoutingError::Other("OPENROUTER_API_KEY not set".to_string()))?;
                        let provider = merlin_providers::OpenRouterProvider::new(api_key)?
                            .with_model(model_name.clone());
                        Ok(Arc::new(provider))
                    }
                    "anthropic" => {
                        let api_key = std::env::var("ANTHROPIC_API_KEY")
                            .map_err(|_| RoutingError::Other("ANTHROPIC_API_KEY not set".to_string()))?;
                        let provider = merlin_providers::AnthropicProvider::new(api_key)?;
                        Ok(Arc::new(provider))
                    }
                    _ => Err(RoutingError::Other(format!("Unknown provider: {}", provider_name)))
                }
            }
        }
    }
    
    /// Build context from task requirements
    async fn build_context(&self, _task: &Task) -> Result<Context> {
        let context = Context::new("You are a helpful coding assistant with access to tools.");
        
        // In a real implementation, would read required files and add to context
        // For Phase 2, we'll use basic context
        
        Ok(context)
    }
    
    /// Check if a request is simple enough to skip assessment
    fn is_simple_request(&self, description: &str) -> bool {
        let desc_lower = description.to_lowercase();
        let word_count = description.split_whitespace().count();
        
        // Simple greetings
        let is_greeting = desc_lower == "hi" 
            || desc_lower == "hello" 
            || desc_lower == "hey"
            || desc_lower.starts_with("say hi")
            || desc_lower.starts_with("say hello");
        
        // Very short requests
        let is_very_short = word_count <= 3;
        
        is_greeting || is_very_short
    }
    
    /// Execute a task with self-determination (Phase 1)
    /// The task assesses itself and decides whether to complete, decompose, or gather context
    pub async fn execute_self_determining(
        &mut self,
        mut task: Task,
        ui_channel: UiChannel,
    ) -> Result<TaskResult> {
        let start = Instant::now();
        let task_id = task.id;
        
        // Check if this is a simple request that doesn't need assessment
        let is_simple = self.is_simple_request(&task.description);
        
        if is_simple {
            // Skip assessment for simple requests, execute directly
            task.state = crate::TaskState::Executing;
            return self.execute_streaming(task, ui_channel).await;
        }
        
        // Update task state
        task.state = crate::TaskState::Assessing;
        
        // Start "Analysis" step (will be collapsed by default)
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "analysis".to_string(),
            step_type: "Thinking".to_string(),
            content: "Analyzing...".to_string(),
        });
        
        // Create assessor with the router's provider
        let decision_result = self.router.route(&task).await?;
        let provider = self.create_provider(&decision_result.tier)?;
        let assessor = crate::SelfAssessor::new(provider.clone());
        
        // Build and execute assessment query
        let query = merlin_core::Query {
            text: format!(
                "Analyze this task and decide if you can complete it immediately or if it needs decomposition:\n\n\"{}\"",
                task.description
            ),
            conversation_id: None,
            files_context: Vec::new(),
        };
        
        let context = merlin_core::Context::new("You are a task assessment system.");
        let assessment_response = provider.generate(&query, &context).await
            .map_err(|e| RoutingError::Other(format!("Assessment failed: {e}")))?;
        
        // Parse the decision FIRST (before sending to UI)
        let decision = match assessor.parse_assessment_response(&assessment_response.text, &task) {
            Ok(d) => d,
            Err(_e) => {
                // If parsing fails, fall back to streaming execution without showing error
                ui_channel.send(UiEvent::TaskStepCompleted {
                    task_id,
                    step_id: "analysis".to_string(),
                });
                
                task.state = crate::TaskState::Executing;
                return self.execute_streaming(task, ui_channel).await;
            }
        };
        
        // Send the raw assessment output to UI (will be under "Analysis" step)
        ui_channel.send(UiEvent::TaskOutput {
            task_id,
            output: assessment_response.text.clone(),
        });
        
        // Store decision in history
        task.decision_history.push(decision.clone());
        
        // Complete the analysis step
        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "analysis".to_string(),
        });
        
        // Execute based on decision
        match decision.action {
            crate::TaskAction::Complete { result } => {
                // Task can be completed immediately - add output as text
                ui_channel.send(UiEvent::TaskOutput {
                    task_id,
                    output: result.clone(),
                });
                
                task.state = crate::TaskState::Completed;
                
                let response = Response {
                    text: result,
                    confidence: decision.confidence as f64,
                    tokens_used: merlin_core::TokenUsage::default(),
                    provider: decision_result.tier.to_string(),
                    latency_ms: start.elapsed().as_millis() as u64,
                };
                
                Ok(TaskResult {
                    task_id,
                    response,
                    tier_used: decision_result.tier.to_string(),
                    validation: crate::ValidationResult::default(),
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            
            crate::TaskAction::Decompose { subtasks, execution_mode: _ } => {
                // Task needs to be broken down - for Phase 1 fall back to standard execution
                ui_channel.send(UiEvent::TaskStepStarted {
                    task_id,
                    step_id: "decompose".to_string(),
                    step_type: "Output".to_string(),
                    content: format!(
                        "Decision: DECOMPOSE into {} subtasks ({}). Falling back to regular execution.",
                        subtasks.len(),
                        decision.reasoning
                    ),
                });

                task.state = crate::TaskState::Executing;
                self.execute_streaming(task, ui_channel).await
            }
            
            crate::TaskAction::GatherContext { needs } => {
                // Task needs more information - for Phase 1, fall back to regular execution
                ui_channel.send(UiEvent::TaskStepStarted {
                    task_id,
                    step_id: "gather".to_string(),
                    step_type: "Output".to_string(),
                    content: format!("Decision: GATHER context - needs: {:?} ({})", needs, decision.reasoning),
                });
                
                // Fall back to regular streaming execution
                task.state = crate::TaskState::Executing;
                self.execute_streaming(task, ui_channel).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        StrategyRouter, ToolRegistry,
        ValidationPipeline, ReadFileTool, WriteFileTool, ListFilesTool, RunCommandTool,
    };
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_agent_executor_creation() {
        let router = Arc::new(StrategyRouter::with_default_strategies());
        let validator = Arc::new(ValidationPipeline::with_default_stages());
        let tool_registry = Arc::new(ToolRegistry::new());
        
        let executor = AgentExecutor::new(router, validator, tool_registry);
        
        // Just verify it was created successfully
        assert!(executor.step_tracker.get_steps(&TaskId::new()).is_none());
    }

    #[tokio::test]
    async fn test_tool_registry_integration() {
        let workspace = PathBuf::from(".");
        let tool_registry = Arc::new(
            ToolRegistry::new()
                .with_tool(Arc::new(ReadFileTool::new(workspace.clone())))
                .with_tool(Arc::new(WriteFileTool::new(workspace.clone())))
                .with_tool(Arc::new(ListFilesTool::new(workspace.clone())))
                .with_tool(Arc::new(RunCommandTool::new(workspace)))
        );
        
        assert!(tool_registry.get_tool("read_file").is_some());
        assert!(tool_registry.get_tool("write_file").is_some());
        assert!(tool_registry.get_tool("list_files").is_some());
        assert!(tool_registry.get_tool("run_command").is_some());
        assert!(tool_registry.get_tool("nonexistent").is_none());
    }
}
