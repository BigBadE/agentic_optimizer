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
        Ok(match task.complexity {
            Complexity::Trivial | Complexity::Simple => ModelTier::Local {
                model_name: "qwen2.5-coder:7b".to_owned(),
            },
            Complexity::Medium => ModelTier::Groq {
                model_name: "llama-3.1-70b-versatile".to_owned(),
            },
            Complexity::Complex => ModelTier::Premium {
                provider: "openrouter".to_owned(),
                model_name: "deepseek/deepseek-coder".to_owned(),
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
    /// # Panics
    /// Panics if selected tiers do not match expected routing by complexity.
    async fn test_complexity_routing() {
        let strategy = ComplexityBasedStrategy;

        let simple_task = Task::new("Simple task".to_owned()).with_complexity(Complexity::Simple);
        let tier_simple = match strategy.select_tier(&simple_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for simple task: {error}"),
        };
        assert!(matches!(tier_simple, ModelTier::Local { .. }));

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
