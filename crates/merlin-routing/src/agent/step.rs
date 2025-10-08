use crate::{
    TaskId,
    streaming::{StepType, TaskStep},
};
use std::collections::HashMap;

/// Tracks execution steps for tasks
#[derive(Default)]
pub struct StepTracker {
    steps: HashMap<TaskId, Vec<TaskStep>>,
}

impl StepTracker {
    /// Add a step to the tracker
    pub fn add_step(&mut self, step: TaskStep) {
        self.steps.entry(step.task_id).or_default().push(step);
    }

    /// Get steps for a task
    pub fn get_steps(&self, task_id: &TaskId) -> Option<&Vec<TaskStep>> {
        self.steps.get(task_id)
    }

    /// Create and track a new step
    pub fn create_step(
        &mut self,
        task_id: TaskId,
        step_type: StepType,
        content: String,
    ) -> TaskStep {
        let step = TaskStep::new(task_id, step_type, content);
        self.add_step(step.clone());
        step
    }
}
