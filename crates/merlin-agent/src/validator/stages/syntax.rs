use async_trait::async_trait;

use merlin_core::Response;

use super::super::pipeline::{StageResult, ValidationStage};
use merlin_core::{Result, Task, ValidationStageType as StageType};

/// Syntax validation using heuristics and pattern matching
pub struct SyntaxValidationStage {
    /// Minimum score required to consider the syntax check as passed
    min_score_threshold: f64,
}

impl SyntaxValidationStage {
    /// Set minimum score threshold
    #[must_use]
    pub fn with_threshold(mut self, threshold: f64) -> Self {
        self.min_score_threshold = threshold;
        self
    }

    /// Heuristically checks for syntax issues and returns (passed, score, details)
    fn check_syntax_errors(&self, text: &str) -> (bool, f64, String) {
        let mut score = 1.0;
        let mut issues = Vec::default();

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
        Self {
            min_score_threshold: 0.8,
        }
    }
}

#[async_trait]
impl ValidationStage for SyntaxValidationStage {
    async fn validate(&self, response: &Response, _task: &Task) -> Result<StageResult> {
        let (passed, score, details) = self.check_syntax_errors(&response.text);

        Ok(StageResult {
            stage: StageType::Syntax,
            passed,
            duration_ms: 0,
            details,
            score,
        })
    }

    async fn quick_check(&self, response: &Response) -> Result<bool> {
        let has_errors =
            response.text.contains("syntax error") || response.text.contains("parse error");
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
    use merlin_core::{Result, TokenUsage};

    #[tokio::test]
    async fn test_syntax_validation_pass() -> Result<()> {
        let stage = SyntaxValidationStage::default();
        let response = Response {
            text: "fn main() { println!(\"Hello\"); }".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());

        let result = stage.validate(&response, &task).await?;
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
        Ok(())
    }

    #[tokio::test]
    async fn test_syntax_validation_fail() -> Result<()> {
        let stage = SyntaxValidationStage::default();
        let response = Response {
            text: "syntax error: unexpected token".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());

        let result = stage.validate(&response, &task).await?;
        assert!(!result.passed);
        assert!(result.score.abs() < f64::EPSILON);
        Ok(())
    }

    #[tokio::test]
    async fn test_mismatched_braces() -> Result<()> {
        let stage = SyntaxValidationStage::default();
        let response = Response {
            text: "fn main() { println!(\"Hello\");".to_owned(),
            confidence: 1.0,
            tokens_used: TokenUsage::default(),
            provider: "test".to_owned(),
            latency_ms: 0,
        };
        let task = Task::new("Test".to_owned());

        let result = stage.validate(&response, &task).await?;
        assert!(!result.passed);
        assert!(result.score < 0.8);
        Ok(())
    }
}
