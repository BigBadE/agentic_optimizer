use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;
use crate::{Result, Task, ValidationResult, ValidationStageType as StageType};
use super::Validator;

/// Individual validation stage trait
#[async_trait]
pub trait ValidationStage: Send + Sync {
    async fn validate(&self, response: &agentic_core::Response, task: &Task) -> Result<StageResult>;
    async fn quick_check(&self, response: &agentic_core::Response) -> Result<bool>;
    fn name(&self) -> &'static str;
    fn stage_type(&self) -> StageType;
}

#[derive(Debug, Clone)]
pub struct StageResult {
    pub stage: StageType,
    pub passed: bool,
    pub duration_ms: u64,
    pub details: String,
    pub score: f64,
}

/// Multi-stage validation pipeline
pub struct ValidationPipeline {
    stages: Vec<Arc<dyn ValidationStage>>,
    early_exit: bool,
}

impl ValidationPipeline {
    pub fn new(stages: Vec<Arc<dyn ValidationStage>>) -> Self {
        Self {
            stages,
            early_exit: true,
        }
    }
    
    pub fn with_early_exit(mut self, early_exit: bool) -> Self {
        self.early_exit = early_exit;
        self
    }
    
    pub fn with_default_stages() -> Self {
        use super::stages::*;
        
        let stages: Vec<Arc<dyn ValidationStage>> = vec![
            Arc::new(SyntaxValidationStage::new()),
            Arc::new(BuildValidationStage::new()),
            Arc::new(TestValidationStage::new()),
            Arc::new(LintValidationStage::new()),
        ];
        
        Self::new(stages)
    }
}

#[async_trait]
impl Validator for ValidationPipeline {
    async fn validate(&self, response: &agentic_core::Response, task: &Task) -> Result<ValidationResult> {
        let mut result = ValidationResult {
            passed: true,
            score: 1.0,
            errors: Vec::new(),
            warnings: Vec::new(),
            stages: Vec::new(),
        };
        
        for stage in &self.stages {
            let start = Instant::now();
            let stage_result = stage.validate(response, task).await?;
            
            result.stages.push(crate::StageResult {
                stage: stage_result.stage,
                passed: stage_result.passed,
                duration_ms: start.elapsed().as_millis() as u64,
                details: stage_result.details.clone(),
                score: stage_result.score,
            });
            
            result.score *= stage_result.score;
            result.passed &= stage_result.passed;
            
            if !stage_result.passed {
                result.errors.push(crate::ValidationError {
                    stage: stage_result.stage,
                    message: stage_result.details,
                    severity: crate::Severity::Error,
                });
                
                if self.early_exit {
                    break;
                }
            }
        }
        
        Ok(result)
    }
    
    async fn quick_validate(&self, response: &agentic_core::Response) -> Result<bool> {
        if let Some(syntax_stage) = self.stages.first() {
            syntax_stage.quick_check(response).await
        } else {
            Ok(true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Task, ValidationStageType as StageType};

    struct MockStage {
        name: &'static str,
        should_pass: bool,
    }

    #[async_trait]
    impl ValidationStage for MockStage {
        async fn validate(&self, _response: &agentic_core::Response, _task: &Task) -> Result<StageResult> {
            Ok(StageResult {
                stage: StageType::Syntax,
                passed: self.should_pass,
                duration_ms: 10,
                details: format!("{} result", self.name),
                score: if self.should_pass { 1.0 } else { 0.0 },
            })
        }
        
        async fn quick_check(&self, _response: &agentic_core::Response) -> Result<bool> {
            Ok(self.should_pass)
        }
        
        fn name(&self) -> &'static str {
            self.name
        }
        
        fn stage_type(&self) -> StageType {
            StageType::Syntax
        }
    }

    #[tokio::test]
    async fn test_pipeline_all_pass() {
        let stages: Vec<Arc<dyn ValidationStage>> = vec![
            Arc::new(MockStage { name: "Stage1", should_pass: true }),
            Arc::new(MockStage { name: "Stage2", should_pass: true }),
        ];
        
        let pipeline = ValidationPipeline::new(stages);
        let task = Task::new("Test".to_string());
        let response = agentic_core::Response {
            text: "test".to_string(),
            confidence: 1.0,
            tokens_used: agentic_core::TokenUsage::default(),
            provider: "test".to_string(),
            latency_ms: 0,
        };
        
        let result = pipeline.validate(&response, &task).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.stages.len(), 2);
    }
    
    #[tokio::test]
    async fn test_pipeline_early_exit() {
        let stages: Vec<Arc<dyn ValidationStage>> = vec![
            Arc::new(MockStage { name: "Stage1", should_pass: false }),
            Arc::new(MockStage { name: "Stage2", should_pass: true }),
        ];
        
        let pipeline = ValidationPipeline::new(stages).with_early_exit(true);
        let task = Task::new("Test".to_string());
        let response = agentic_core::Response {
            text: "test".to_string(),
            confidence: 1.0,
            tokens_used: agentic_core::TokenUsage::default(),
            provider: "test".to_string(),
            latency_ms: 0,
        };
        
        let result = pipeline.validate(&response, &task).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.stages.len(), 1);
    }
}
