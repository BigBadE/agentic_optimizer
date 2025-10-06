use crate::{TaskId, streaming::{StepType, TaskStep}};
use std::collections::HashMap;

/// Tracks execution steps for tasks
pub struct StepTracker {
    steps: HashMap<TaskId, Vec<TaskStep>>,
}

impl StepTracker {
    pub fn new() -> Self {
        Self {
            steps: HashMap::new(),
        }
    }
    
    pub fn add_step(&mut self, step: TaskStep) {
        self.steps
            .entry(step.task_id)
            .or_insert_with(Vec::new)
            .push(step);
    }
    
    pub fn get_steps(&self, task_id: &TaskId) -> Option<&Vec<TaskStep>> {
        self.steps.get(task_id)
    }
    
    pub fn create_step(&mut self, task_id: TaskId, step_type: StepType, content: String) -> TaskStep {
        let step = TaskStep::new(task_id, step_type, content);
        self.add_step(step.clone());
        step
    }
}

impl Default for StepTracker {
    fn default() -> Self {
        Self::new()
    }
}
