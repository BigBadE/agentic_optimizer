use merlin_core::{
    TaskId,
    streaming::{ExecutionStep, ExecutionStepType},
};
use std::collections::HashMap;

/// Tracks execution steps for tasks
#[derive(Default, Clone)]
pub struct StepTracker {
    steps: HashMap<TaskId, Vec<ExecutionStep>>,
}

impl StepTracker {
    /// Add a step to the tracker
    pub fn add_step(&mut self, step: ExecutionStep) {
        self.steps.entry(step.task_id).or_default().push(step);
    }

    /// Get steps for a task
    pub fn get_steps(&self, task_id: &TaskId) -> Option<&Vec<ExecutionStep>> {
        self.steps.get(task_id)
    }

    /// Create and track a new step
    pub fn create_step(
        &mut self,
        task_id: TaskId,
        step_type: ExecutionStepType,
        content: String,
    ) -> ExecutionStep {
        let step = ExecutionStep::new(task_id, step_type, content);
        self.add_step(step.clone());
        step
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_tracker_default() {
        let tracker = StepTracker::default();
        assert!(tracker.steps.is_empty());
    }

    #[test]
    fn test_add_step() {
        let mut tracker = StepTracker::default();
        let task_id = TaskId::default();
        let step = ExecutionStep::new(task_id, ExecutionStepType::Thinking, "test".to_owned());

        tracker.add_step(step);

        let steps = tracker.get_steps(&task_id).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].task_id, task_id);
    }

    #[test]
    fn test_get_steps_none() {
        let tracker = StepTracker::default();
        let task_id = TaskId::default();
        assert!(tracker.get_steps(&task_id).is_none());
    }

    #[test]
    fn test_create_step() {
        let mut tracker = StepTracker::default();
        let task_id = TaskId::default();

        let step = tracker.create_step(
            task_id,
            ExecutionStepType::Thinking,
            "thinking content".to_owned(),
        );

        assert_eq!(step.task_id, task_id);
        assert_eq!(step.content, "thinking content");

        let steps = tracker.get_steps(&task_id).unwrap();
        assert_eq!(steps.len(), 1);
    }

    #[test]
    fn test_multiple_steps_for_task() {
        use merlin_deps::serde_json::json;

        let mut tracker = StepTracker::default();
        let task_id = TaskId::default();

        tracker.create_step(task_id, ExecutionStepType::Thinking, "step 1".to_owned());
        tracker.create_step(
            task_id,
            ExecutionStepType::ToolCall {
                tool: "bash".to_owned(),
                args: json!({"command": "echo test"}),
            },
            "step 2".to_owned(),
        );
        tracker.create_step(task_id, ExecutionStepType::Thinking, "step 3".to_owned());

        let steps = tracker.get_steps(&task_id).unwrap();
        assert_eq!(steps.len(), 3);
    }

    #[test]
    fn test_multiple_tasks() {
        let mut tracker = StepTracker::default();
        let task1 = TaskId::default();
        let task2 = TaskId::default();

        tracker.create_step(task1, ExecutionStepType::Thinking, "task1 step".to_owned());
        tracker.create_step(task2, ExecutionStepType::Thinking, "task2 step".to_owned());

        assert_eq!(tracker.get_steps(&task1).unwrap().len(), 1);
        assert_eq!(tracker.get_steps(&task2).unwrap().len(), 1);
    }

    #[test]
    fn test_clone_tracker() {
        let mut tracker = StepTracker::default();
        let task_id = TaskId::default();
        tracker.create_step(task_id, ExecutionStepType::Thinking, "test".to_owned());

        let cloned = tracker.clone();
        assert_eq!(cloned.get_steps(&task_id).unwrap().len(), 1);
    }
}
