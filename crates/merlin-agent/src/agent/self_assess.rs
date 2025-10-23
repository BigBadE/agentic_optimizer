use merlin_core::{Context, ModelProvider, Query, prompts::load_prompt};
use merlin_core::{
    ExecutionContext, ExecutionMode, Result, RoutingError, SubtaskSpec, Task, TaskAction,
    TaskDecision,
};
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

    /// Build assessment prompt for a task
    ///
    /// # Panics
    /// Panics if the `task_assessment` prompt cannot be loaded (should never happen as prompts are embedded)
    fn build_assessment_prompt(task: &Task, _context: &ExecutionContext) -> String {
        let template = load_prompt("task_assessment")
            .unwrap_or_else(|err| panic!("Failed to load task_assessment prompt: {err}"));
        template.replace("{task_description}", &task.description)
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
                        difficulty: 5,
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

    #[test]
    fn test_parse_complete_action() {
        let task = Task::new("test task".to_owned());
        let response = r#"{
            "action": "COMPLETE",
            "reasoning": "Task is simple",
            "confidence": 0.9,
            "details": {
                "result": "Task completed successfully"
            }
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::Complete { result } => {
                assert_eq!(result, "Task completed successfully");
            }
            _ => panic!("Expected Complete action"),
        }
        assert_eq!(decision.reasoning, "Task is simple");
        assert!((decision.confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_parse_decompose_action() {
        let task = Task::new("complex task".to_owned());
        let response = r#"{
            "action": "DECOMPOSE",
            "reasoning": "Task is complex",
            "confidence": 0.8,
            "details": {
                "subtasks": [
                    {"description": "subtask 1", "difficulty": 3},
                    {"description": "subtask 2", "difficulty": 5}
                ],
                "execution_mode": "Parallel"
            }
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::Decompose {
                subtasks,
                execution_mode,
            } => {
                assert_eq!(subtasks.len(), 2);
                assert_eq!(subtasks[0].description, "subtask 1");
                assert_eq!(subtasks[0].difficulty, 3);
                assert!(matches!(execution_mode, ExecutionMode::Parallel));
            }
            _ => panic!("Expected Decompose action"),
        }
    }

    #[test]
    fn test_parse_decompose_default_sequential() {
        let task = Task::new("task".to_owned());
        let response = r#"{
            "action": "DECOMPOSE",
            "reasoning": "Needs decomposition",
            "confidence": 0.7,
            "details": {
                "subtasks": [
                    {"description": "subtask 1", "difficulty": 2}
                ]
            }
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::Decompose { execution_mode, .. } => {
                assert!(matches!(execution_mode, ExecutionMode::Sequential));
            }
            _ => panic!("Expected Decompose action"),
        }
    }

    #[test]
    fn test_parse_gather_action() {
        let task = Task::new("research task".to_owned());
        let response = r#"{
            "action": "GATHER",
            "reasoning": "Need more context",
            "confidence": 0.6,
            "details": {
                "needs": ["file1.rs", "file2.rs"]
            }
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::GatherContext { needs } => {
                assert_eq!(needs.len(), 2);
                assert_eq!(needs[0], "file1.rs");
            }
            _ => panic!("Expected GatherContext action"),
        }
    }

    #[test]
    fn test_parse_json_with_surrounding_text() {
        let task = Task::new("task".to_owned());
        let response = r#"
            Here is my assessment:
            {
                "action": "COMPLETE",
                "reasoning": "Simple task",
                "confidence": 0.95,
                "details": {
                    "result": "Done"
                }
            }
            This is the result.
        "#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();
        assert!(matches!(decision.action, TaskAction::Complete { .. }));
    }

    #[test]
    fn test_parse_unknown_action_error() {
        let task = Task::new("task".to_owned());
        let response = r#"{
            "action": "UNKNOWN_ACTION",
            "reasoning": "Test",
            "confidence": 0.5,
            "details": {}
        }"#;

        let result = SelfAssessor::parse_decision(response, &task);
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Unknown action"));
    }

    #[test]
    fn test_parse_invalid_json_error() {
        let task = Task::new("task".to_owned());
        let response = "This is not valid JSON";

        let result = SelfAssessor::parse_decision(response, &task);
        result.unwrap_err();
    }

    #[test]
    fn test_parse_complete_without_result() {
        let task = Task::new("test task".to_owned());
        let response = r#"{
            "action": "COMPLETE",
            "reasoning": "Done",
            "confidence": 1.0,
            "details": {}
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::Complete { result } => {
                assert!(result.contains("test task"));
            }
            _ => panic!("Expected Complete action"),
        }
    }

    #[test]
    fn test_parse_decompose_without_subtasks() {
        let task = Task::new("task".to_owned());
        let response = r#"{
            "action": "DECOMPOSE",
            "reasoning": "Decompose needed",
            "confidence": 0.8,
            "details": {}
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::Decompose { subtasks, .. } => {
                assert_eq!(subtasks.len(), 1);
                assert_eq!(subtasks[0].description, "task");
            }
            _ => panic!("Expected Decompose action"),
        }
    }

    #[test]
    fn test_parse_gather_without_needs() {
        let task = Task::new("task".to_owned());
        let response = r#"{
            "action": "GATHER",
            "reasoning": "Need context",
            "confidence": 0.5,
            "details": {}
        }"#;

        let decision = SelfAssessor::parse_decision(response, &task).unwrap();

        match decision.action {
            TaskAction::GatherContext { needs } => {
                assert!(needs.is_empty());
            }
            _ => panic!("Expected GatherContext action"),
        }
    }

    #[test]
    fn test_parse_assessment_response_public_method() {
        let provider: Arc<dyn ModelProvider> =
            Arc::new(LocalModelProvider::new("qwen2.5-coder:7b".to_string()));
        let assessor = SelfAssessor::new(provider);
        let task = Task::new("test".to_owned());

        let response = r#"{
            "action": "COMPLETE",
            "reasoning": "Done",
            "confidence": 0.9,
            "details": {"result": "Success"}
        }"#;

        let result = assessor.parse_assessment_response(response, &task);
        result.unwrap();
    }
}
