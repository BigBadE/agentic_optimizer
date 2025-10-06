use crate::{
    Complexity, ExecutionContext, ExecutionMode, Result, RoutingError, SubtaskSpec, Task,
    TaskAction, TaskDecision,
};
use merlin_core::{Context, ModelProvider, Query};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Self-assessment engine for tasks
pub struct SelfAssessor {
    provider: Arc<dyn ModelProvider>,
}

impl SelfAssessor {
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self { provider }
    }

    /// Assess a task and decide what action to take
    pub async fn assess_task(
        &self,
        task: &Task,
        exec_context: &ExecutionContext,
    ) -> Result<TaskDecision> {
        let prompt = self.build_assessment_prompt(task, exec_context);

        let query = Query {
            text: prompt,
            conversation_id: None,
            files_context: Vec::new(),
        };

        let context = Context::new("You are a task assessment system. Analyze tasks and decide how to execute them.");

        let response = self
            .provider
            .generate(&query, &context)
            .await
            .map_err(|e| RoutingError::Other(format!("Assessment failed: {e}")))?;

        self.parse_decision(&response.text, task)
    }

    fn build_assessment_prompt(&self, task: &Task, context: &ExecutionContext) -> String {
        let context_summary = if context.files_read.is_empty() {
            "No files read yet".to_string()
        } else {
            format!("Files read: {}", context.files_read.len())
        };

        format!(
            r#"You are assessing whether you can complete this task or if it needs to be broken down.

Task: "{}"
Complexity estimate: {:?}
Context: {}

Analyze this task and decide ONE of the following:

1. COMPLETE - You can solve this immediately (use for simple greetings, basic questions, straightforward requests)
   Example: "say hi", "hello", "what time is it"

2. DECOMPOSE - This needs to be broken into subtasks (use for complex work requiring multiple steps)
   Example: "refactor the module", "implement feature X with tests"

3. GATHER - You need more information first (use when you need to read files, understand context)
   Example: "fix the bug" (need to see code first)

Guidelines:
- If the request is 5 words or less and conversational, choose COMPLETE
- If it's a greeting or simple question, choose COMPLETE
- If it requires code changes across multiple files, choose DECOMPOSE
- If you need to understand existing code first, choose GATHER

Respond ONLY with valid JSON in this exact format:
{{
  "action": "COMPLETE" | "DECOMPOSE" | "GATHER",
  "reasoning": "brief explanation of your choice",
  "confidence": 0.0-1.0,
  "details": {{
    // For COMPLETE:
    "result": "your response text"
    
    // For DECOMPOSE:
    "subtasks": [
      {{"description": "task 1", "complexity": "Simple"}},
      {{"description": "task 2", "complexity": "Medium"}}
    ],
    "execution_mode": "Sequential" | "Parallel"
    
    // For GATHER:
    "needs": ["file path 1", "file path 2"]
  }}
}}

JSON Response:"#,
            task.description, task.complexity, context_summary
        )
    }

    fn parse_decision(&self, response_text: &str, task: &Task) -> Result<TaskDecision> {
        // Try to extract JSON from the response
        let json_str = if let Some(start) = response_text.find('{') {
            if let Some(end) = response_text.rfind('}') {
                &response_text[start..=end]
            } else {
                response_text
            }
        } else {
            response_text
        };

        let parsed: AssessmentResponse = serde_json::from_str(json_str).map_err(|e| {
            RoutingError::Other(format!(
                "Failed to parse assessment response: {e}\nResponse: {response_text}"
            ))
        })?;

        let action = match parsed.action.as_str() {
            "COMPLETE" => {
                let result = parsed
                    .details
                    .result
                    .unwrap_or_else(|| format!("Completed: {}", task.description));
                TaskAction::Complete { result }
            }
            "DECOMPOSE" => {
                let subtasks = parsed
                    .details
                    .subtasks
                    .unwrap_or_else(|| vec![SubtaskSpec {
                        description: task.description.clone(),
                        complexity: Complexity::Medium,
                    }]);

                let execution_mode = match parsed
                    .details
                    .execution_mode
                    .as_deref()
                    .unwrap_or("Sequential")
                {
                    "Parallel" => ExecutionMode::Parallel,
                    _ => ExecutionMode::Sequential,
                };

                TaskAction::Decompose {
                    subtasks,
                    execution_mode,
                }
            }
            "GATHER" => {
                let needs = parsed.details.needs.unwrap_or_default();
                TaskAction::GatherContext { needs }
            }
            _ => {
                return Err(RoutingError::Other(format!(
                    "Unknown action: {}",
                    parsed.action
                )))
            }
        };

        Ok(TaskDecision {
            action,
            reasoning: parsed.reasoning,
            confidence: parsed.confidence,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct AssessmentResponse {
    action: String,
    reasoning: String,
    confidence: f32,
    details: AssessmentDetails,
}

#[derive(Debug, Deserialize, Serialize)]
struct AssessmentDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtasks: Option<Vec<SubtaskSpec>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    execution_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    needs: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assessment_prompt_generation() {
        let provider = Box::new(merlin_local::OllamaProvider::new("qwen2.5-coder:7b").unwrap());
        let assessor = SelfAssessor::new(provider);

        let task = Task::new("say hi".to_string());
        let context = ExecutionContext::new("say hi".to_string());

        let prompt = assessor.build_assessment_prompt(&task, &context);

        assert!(prompt.contains("say hi"));
        assert!(prompt.contains("COMPLETE"));
        assert!(prompt.contains("DECOMPOSE"));
        assert!(prompt.contains("GATHER"));
    }
}
