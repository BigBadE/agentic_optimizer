use super::super::strategy::RoutingStrategy;
use crate::{Complexity, ModelTier, Result, Task};
use async_trait::async_trait;

/// Routes tasks based on complexity level
#[derive(Default)]
pub struct ComplexityBasedStrategy;

impl ComplexityBasedStrategy {}

#[async_trait]
impl RoutingStrategy for ComplexityBasedStrategy {
    fn applies_to(&self, _task: &Task) -> bool {
        true
    }

    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        // Complexity-based routing with multi-tier model selection:
        // Trivial: Local qwen2.5-coder:7b
        // Simple: Groq llama-3.1-8b-instant
        // Medium: Groq qwen2.5-32b-coder-preview
        // Complex: Premium claude-3-5-sonnet

        Ok(match task.complexity {
            Complexity::Trivial => ModelTier::Local {
                model_name: "qwen2.5-coder:7b".to_owned(),
            },
            Complexity::Simple => ModelTier::Groq {
                model_name: "llama-3.1-8b-instant".to_owned(),
            },
            Complexity::Medium => ModelTier::Groq {
                model_name: "qwen2.5-32b-coder-preview".to_owned(),
            },
            Complexity::Complex => ModelTier::Premium {
                provider: "openrouter".to_owned(),
                model_name: "anthropic/claude-3-5-sonnet-20241022".to_owned(),
            },
        })
    }

    fn priority(&self) -> u8 {
        50
    }

    fn name(&self) -> &'static str {
        "ComplexityBased"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_complexity_routing() {
        let strategy = ComplexityBasedStrategy;

        let simple_task = Task::new("Simple task".to_owned()).with_complexity(Complexity::Simple);
        let tier_simple = match strategy.select_tier(&simple_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for simple task: {error}"),
        };
        assert!(matches!(tier_simple, ModelTier::Groq { .. }));

        let medium_task = Task::new("Medium task".to_owned()).with_complexity(Complexity::Medium);
        let tier_medium = match strategy.select_tier(&medium_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for medium task: {error}"),
        };
        assert!(matches!(tier_medium, ModelTier::Groq { .. }));

        let complex_task =
            Task::new("Complex task".to_owned()).with_complexity(Complexity::Complex);
        let tier_complex = match strategy.select_tier(&complex_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for complex task: {error}"),
        };
        assert!(matches!(tier_complex, ModelTier::Premium { .. }));
    }
}
