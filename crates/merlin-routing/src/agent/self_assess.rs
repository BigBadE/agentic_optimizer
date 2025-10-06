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

    fn build_assessment_prompt(&self, task: &Task, _context: &ExecutionContext) -> String {
        format!(
            r#"Task: "{}"

You must respond with ONLY valid JSON. No explanations, no markdown, just JSON.

For simple requests like greetings, respond:
{{"action": "COMPLETE", "reasoning": "Simple greeting", "confidence": 0.95, "details": {{"result": "Hi! How can I help you today?"}}}}

For complex tasks, respond:
{{"action": "DECOMPOSE", "reasoning": "Needs multiple steps", "confidence": 0.9, "details": {{"subtasks": [{{"description": "Step 1", "complexity": "Simple"}}], "execution_mode": "Sequential"}}}}

JSON:"#,
            task.description
        )
    }

    /// Parse an assessment response into a decision (public for executor)
    pub fn parse_assessment_response(&self, response_text: &str, task: &Task) -> Result<TaskDecision> {
        self.parse_decision(response_text, task)
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

        // Try to parse JSON, but don't use fallback - let caller handle errors
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
