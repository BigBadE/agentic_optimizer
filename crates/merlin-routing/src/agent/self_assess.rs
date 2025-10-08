use crate::{
    Complexity, ExecutionContext, ExecutionMode, Result, RoutingError, SubtaskSpec, Task,
    TaskAction, TaskDecision,
};
use merlin_core::{Context, ModelProvider, Query};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::sync::Arc;

/// Self-assessment engine for tasks
pub struct SelfAssessor {
    provider: Arc<dyn ModelProvider>,
}

impl SelfAssessor {
    /// Create a new self-assessor with the given provider
    pub fn new(provider: Arc<dyn ModelProvider>) -> Self {
        Self { provider }
    }

    /// Assess a task and decide what action to take
    ///
    /// # Errors
    ///
    /// Returns an error if the assessment generation fails or the response cannot be parsed
    pub async fn assess_task(
        &self,
        task: &Task,
        exec_context: &ExecutionContext,
    ) -> Result<TaskDecision> {
        let prompt = Self::build_assessment_prompt(task, exec_context);

        let query = Query {
            text: prompt,
            conversation_id: None,
            files_context: Vec::default(),
        };

        let context = Context::new(
            "You are a task assessment system. Analyze tasks and decide how to execute them.",
        );

        let response = self
            .provider
            .generate(&query, &context)
            .await
            .map_err(|err| RoutingError::Other(format!("Assessment failed: {err}")))?;

        Self::parse_decision(&response.text, task)
    }

    fn build_assessment_prompt(task: &Task, _context: &ExecutionContext) -> String {
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
    ///
    /// # Errors
    /// Returns an error if the response cannot be parsed as valid JSON or contains an unknown action
    pub fn parse_assessment_response(
        &self,
        response_text: &str,
        task: &Task,
    ) -> Result<TaskDecision> {
        Self::parse_decision(response_text, task)
    }

    /// Parse raw model response text into a `TaskDecision`.
    ///
    /// # Errors
    /// Returns an error if the text cannot be parsed into JSON or contains an unknown action.
    fn parse_decision(response_text: &str, task: &Task) -> Result<TaskDecision> {
        // Try to extract JSON from the response
        let json_str = match (response_text.find('{'), response_text.rfind('}')) {
            (Some(start), Some(end)) if start <= end => &response_text[start..=end],
            _ => response_text,
        };

        // Try to parse JSON, but don't use fallback - let caller handle errors
        let parsed: AssessmentResponse = from_str(json_str).map_err(|err| {
            RoutingError::Other(format!(
                "Failed to parse assessment response: {err}\nResponse: {response_text}"
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
                let subtasks = parsed.details.subtasks.unwrap_or_else(|| {
                    vec![SubtaskSpec {
                        description: task.description.clone(),
                        complexity: Complexity::Medium,
                    }]
                });

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
                )));
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
    use merlin_core::ModelProvider;
    use merlin_local::LocalModelProvider;

    #[test]
    /// # Panics
    /// Panics if the assessment prompt does not contain required markers.
    fn test_assessment_prompt_generation() {
        let provider: Arc<dyn ModelProvider> =
            Arc::new(LocalModelProvider::new("qwen2.5-coder:7b".to_string()));
        let _assessor = SelfAssessor::new(provider);

        let task = Task::new("say hi".to_owned());
        let context = ExecutionContext::new("say hi".to_owned());

        let prompt = SelfAssessor::build_assessment_prompt(&task, &context);

        assert!(prompt.contains("say hi"));
        assert!(prompt.contains("COMPLETE"));
        assert!(prompt.contains("DECOMPOSE"));
        // GATHER action exists in the parser but isn't shown in the prompt template
        assert!(prompt.contains("JSON"));
    }
}
