use super::super::strategy::RoutingStrategy;
use crate::{ModelTier, Result, Task};
use async_trait::async_trait;

/// Routes tasks with long context to appropriate models
pub struct LongContextStrategy {
    long_context_threshold: usize,
}

impl LongContextStrategy {
    /// Create a new long context strategy
    pub fn new(long_context_threshold: usize) -> Self {
        Self {
            long_context_threshold,
        }
    }
}

impl Default for LongContextStrategy {
    fn default() -> Self {
        Self::new(16000)
    }
}

#[async_trait]
impl RoutingStrategy for LongContextStrategy {
    fn applies_to(&self, task: &Task) -> bool {
        task.context_needs.estimated_tokens > self.long_context_threshold
            || task.context_needs.requires_full_context
    }

    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        // Long-context routing - select models based on context size
        // 16k-32k: Groq qwen2.5-32b (32k window)
        // 32k-100k: Premium haiku (200k window)
        // 100k-200k: Premium glm-4.6 (long context specialist)
        // 200k+: Premium sonnet (200k window, best reasoning)

        if task.context_needs.estimated_tokens > 200_000 {
            Ok(ModelTier::Premium {
                provider: "anthropic".to_owned(),
                model_name: "claude-3-5-sonnet-20241022".to_owned(),
            })
        } else if task.context_needs.estimated_tokens > 100_000 {
            Ok(ModelTier::Premium {
                provider: "openrouter".to_owned(),
                model_name: "01-ai/yi-lightning".to_owned(),
            })
        } else if task.context_needs.estimated_tokens > 32000 {
            Ok(ModelTier::Premium {
                provider: "anthropic".to_owned(),
                model_name: "claude-3-5-haiku-20241022".to_owned(),
            })
        } else {
            // 16k-32k range
            Ok(ModelTier::Groq {
                model_name: "qwen2.5-32b-coder-preview".to_owned(),
            })
        }
    }

    fn priority(&self) -> u8 {
        90
    }

    fn name(&self) -> &'static str {
        "LongContext"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContextRequirements;

    #[tokio::test]
    async fn test_long_context_routing() {
        let strategy = LongContextStrategy::new(16000);

        let huge_context = Task::new("Huge context task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(150_000));

        assert!(strategy.applies_to(&huge_context));
        let tier = match strategy.select_tier(&huge_context).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for huge context: {error}"),
        };

        // 150k tokens should route to yi-lightning (100k-200k range)
        if let ModelTier::Premium { model_name, .. } = tier {
            assert!(model_name.contains("yi-lightning"));
        } else {
            panic!("Expected Premium tier for huge context");
        }
    }

    #[tokio::test]
    async fn test_medium_long_context() {
        let strategy = LongContextStrategy::new(16000);

        let medium_context = Task::new("Medium context task".to_owned())
            .with_context(ContextRequirements::default().with_estimated_tokens(50000));

        assert!(strategy.applies_to(&medium_context));
        let tier = match strategy.select_tier(&medium_context).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for medium context: {error}"),
        };
        assert!(matches!(tier, ModelTier::Premium { .. }));
    }

    #[tokio::test]
    async fn test_requires_full_context() {
        let strategy = LongContextStrategy::new(16000);

        let full_context = Task::new("Full context task".to_owned()).with_context(
            ContextRequirements::default()
                .with_estimated_tokens(10000)
                .with_full_context(true),
        );

        assert!(strategy.applies_to(&full_context));
    }
}
