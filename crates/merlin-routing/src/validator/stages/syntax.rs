use async_trait::async_trait;
use crate::{Result, Task, ValidationStageType as StageType};
use super::super::pipeline::{StageResult, ValidationStage};

/// Syntax validation using heuristics and pattern matching
pub struct SyntaxValidationStage {
    min_score_threshold: f64,
}

impl SyntaxValidationStage {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            min_score_threshold: 0.8,
        }
    }
    
    #[must_use]
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.min_score_threshold = threshold;
        self
    }
    
    fn check_syntax_errors(&self, text: &str) -> (bool, f64, String) {
        let mut score = 1.0;
        let mut issues = Vec::new();
        
        if text.contains("syntax error") || text.contains("SyntaxError") {
            score *= 0.0;
            issues.push("Explicit syntax error found");
        }
        
        if text.contains("parse error") || text.contains("ParseError") {
            score *= 0.2;
            issues.push("Parse error detected");
        }
        
        if text.contains("unexpected token") {
            score *= 0.3;
            issues.push("Unexpected token found");
        }
        
        let open_braces = text.matches('{').count();
        let close_braces = text.matches('}').count();
        if open_braces != close_braces {
            score *= 0.5;
            issues.push("Mismatched braces");
        }
        
        let open_parens = text.matches('(').count();
        let close_parens = text.matches(')').count();
        if open_parens != close_parens {
            score *= 0.5;
            issues.push("Mismatched parentheses");
        }
        
        let open_brackets = text.matches('[').count();
        let close_brackets = text.matches(']').count();
        if open_brackets != close_brackets {
            score *= 0.5;
            issues.push("Mismatched brackets");
        }
        
        let passed = score >= self.min_score_threshold;
        let details = if issues.is_empty() {
            "Syntax check passed".to_owned()
        } else {
            format!("Issues: {}", issues.join(", "))
        };
        
        (passed, score, details)
    }
}

impl Default for SyntaxValidationStage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ValidationStage for SyntaxValidationStage {
    async fn validate(&self, response: &merlin_core::Response, _task: &Task) -> Result<StageResult> {
        let (passed, score, details) = self.check_syntax_errors(&response.text);
        
        Ok(StageResult {
            stage: StageType::Syntax,
            passed,
            duration_ms: 0,
            details,
            score,
        })
    }
    
    async fn quick_check(&self, response: &merlin_core::Response) -> Result<bool> {
        let has_errors = response.text.contains("syntax error")
            || response.text.contains("parse error");
        Ok(!has_errors)
    }
    
    fn name(&self) -> &'static str {
        "Syntax"
    }
    
    fn stage_type(&self) -> StageType {
        StageType::Syntax
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_syntax_validation_pass() {
        let stage = SyntaxValidationStage::new();
        let response = merlin_core::Response {
            text: "fn main() { println!(\"Hello\"); }".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());
        
        let result = stage.validate(&response, &task).await.unwrap();
        assert!(result.passed);
        assert_eq!(result.score, 1.0);
    }
    
    #[tokio::test]
    async fn test_syntax_validation_fail() {
        let stage = SyntaxValidationStage::new();
        let response = merlin_core::Response {
            text: "syntax error: unexpected token".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());
        
        let result = stage.validate(&response, &task).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.score, 0.0);
    }
    
    #[tokio::test]
    async fn test_mismatched_braces() {
        let stage = SyntaxValidationStage::new();
        let response = merlin_core::Response {
            text: "fn main() { println!(\"Hello\");".to_owned(),
            confidence: 1.0,
            tokens_used: merlin_core::TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());
        
        let result = stage.validate(&response, &task).await.unwrap();
        assert!(!result.passed);
        assert!(result.score < 0.8);
    }
}

