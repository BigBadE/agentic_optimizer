use super::super::strategy::RoutingStrategy;
use crate::{ModelTier, Priority, Result, Task};
use async_trait::async_trait;

/// Routes tasks to minimize cost (prefer local/free tiers)
pub struct CostOptimizationStrategy {
    max_tokens_for_local: usize,
}

impl CostOptimizationStrategy {
    /// Create a new cost optimization strategy
    pub fn new(max_tokens_for_local: usize) -> Self {
        Self {
            max_tokens_for_local,
        }
    }
}

impl Default for CostOptimizationStrategy {
    fn default() -> Self {
        // Set to 0 tokens - never use local tier for cost optimization
        // This ensures all messages, including initial requests, go to at least Groq tier
        // Local tier can still be explicitly selected via ComplexityBasedStrategy for Trivial tasks
        Self::new(0)
    }
}

#[async_trait]
impl RoutingStrategy for CostOptimizationStrategy {
    fn applies_to(&self, task: &Task) -> bool {
        task.priority != Priority::Critical
    }

    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        // Cost-optimized routing:
        // Never use Local tier (max_tokens_for_local = 0 by default)
        // Groq (llama-3.1-8b, qwen-32b, llama-70b) - small to medium contexts
        // Premium (haiku, yi-lightning, sonnet) - larger contexts

        if self.max_tokens_for_local > 0
            && task.context_needs.estimated_tokens <= self.max_tokens_for_local
        {
            // Use local model only for very small contexts (disabled by default)
            Ok(ModelTier::Local {
                model_name: "qwen2.5-coder:7b".to_owned(),
            })
        } else if task.context_needs.estimated_tokens <= 8000 {
            // Use Groq's llama-3.1-8b for small-medium contexts
            Ok(ModelTier::Groq {
                model_name: "llama-3.1-8b-instant".to_owned(),
            })
        } else if task.context_needs.estimated_tokens <= 16000 {
            // Use Groq's qwen-32b for medium contexts
            Ok(ModelTier::Groq {
                model_name: "qwen2.5-32b-coder-preview".to_owned(),
            })
        } else if task.context_needs.estimated_tokens <= 50000 {
            // Use premium haiku for medium-large contexts
            Ok(ModelTier::Premium {
                provider: "anthropic".to_owned(),
                model_name: "claude-3-5-haiku-20241022".to_owned(),
            })
        } else if task.context_needs.estimated_tokens <= 100_000 {
            // Use OpenRouter glm-4.6 for large contexts
            Ok(ModelTier::Premium {
                provider: "openrouter".to_owned(),
                model_name: "01-ai/yi-lightning".to_owned(),
            })
        } else {
            // Use best model for very large contexts
            Ok(ModelTier::Premium {
                provider: "anthropic".to_owned(),
                model_name: "claude-3-5-sonnet-20241022".to_owned(),
            })
        }
    }

    fn priority(&self) -> u8 {
        70
    }

    fn name(&self) -> &'static str {
        "CostOptimization"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextRequirements;

    #[tokio::test]
    async fn test_cost_optimization() {
        // Test with default (0 tokens) - should use Groq for small tasks
        let strategy = CostOptimizationStrategy::default();

        let small_task = Task::new("Small task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(2000));
        let tier_small = match strategy.select_tier(&small_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for small task: {error}"),
        };
        // With default config (max_tokens_for_local=0), small tasks use Groq
        assert!(matches!(tier_small, ModelTier::Groq { .. }));

        let medium_task = Task::new("Medium task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(6000));
        let tier_medium = match strategy.select_tier(&medium_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for medium task: {error}"),
        };
        assert!(matches!(tier_medium, ModelTier::Groq { .. }));

        // 10k tokens -> Groq qwen-32b (8k-16k range)
        let medium_large_task = Task::new("Medium-large task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(10000));
        let tier_medium_large = match strategy.select_tier(&medium_large_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for medium-large task: {error}"),
        };
        assert!(matches!(tier_medium_large, ModelTier::Groq { .. }));

        // 60k tokens -> Premium haiku
        let large_task = Task::new("Large task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(60000));
        let tier_large = match strategy.select_tier(&large_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for large task: {error}"),
        };
        assert!(matches!(tier_large, ModelTier::Premium { .. }));
    }

    #[tokio::test]
    async fn test_local_tier_when_explicitly_configured() {
        // Test that Local tier can still be used when explicitly configured
        let strategy = CostOptimizationStrategy::new(4000);

        let small_task = Task::new("Small task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(2000));
        let tier_small = match strategy.select_tier(&small_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for small task: {error}"),
        };
        // With max_tokens_for_local=4000, tasks under that use Local
        assert!(matches!(tier_small, ModelTier::Local { .. }));
    }

    #[tokio::test]
    async fn test_zero_tokens_uses_groq_with_default() {
        // Test that tasks with 0 tokens (initial requests) use Groq, not Local
        let strategy = CostOptimizationStrategy::default();

        let zero_token_task = Task::new("Initial request".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(0));
        let tier = match strategy.select_tier(&zero_token_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for zero token task: {error}"),
        };
        // With default config (max_tokens_for_local=0), zero token tasks use Groq
        assert!(matches!(tier, ModelTier::Groq { .. }));
    }

    #[tokio::test]
    async fn test_critical_tasks_not_applicable() {
        let strategy = CostOptimizationStrategy::new(4000);

        let critical_task = Task::new("Critical task".to_owned()).with_priority(Priority::Critical);

        assert!(!strategy.applies_to(&critical_task));
    }
}
