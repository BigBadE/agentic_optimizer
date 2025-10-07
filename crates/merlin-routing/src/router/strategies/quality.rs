use super::super::strategy::RoutingStrategy;
use crate::{ModelTier, Priority, Result, Task};
use async_trait::async_trait;

/// Routes quality-critical tasks to premium models
pub struct QualityCriticalStrategy;

impl QualityCriticalStrategy {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for QualityCriticalStrategy {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RoutingStrategy for QualityCriticalStrategy {
    fn applies_to(&self, task: &Task) -> bool {
        task.priority == Priority::Critical || task.priority == Priority::High
    }

    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        if task.priority == Priority::Critical {
            Ok(ModelTier::Premium {
                provider: "anthropic".to_owned(),
                model_name: "claude-3.5-sonnet".to_owned(),
            })
        } else {
            Ok(ModelTier::Premium {
                provider: "openrouter".to_owned(),
                model_name: "anthropic/claude-3-haiku".to_owned(),
            })
        }
    }

    fn priority(&self) -> u8 {
        100
    }

    fn name(&self) -> &'static str {
        "QualityCritical"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    /// # Panics
    /// Panics if a premium tier is not selected for critical tasks.
    async fn test_quality_critical_routing() {
        let strategy = QualityCriticalStrategy::new();

        let critical_task = Task::new("Critical task".to_owned()).with_priority(Priority::Critical);

        assert!(strategy.applies_to(&critical_task));
        let tier = match strategy.select_tier(&critical_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for critical task: {error}"),
        };

        if let ModelTier::Premium {
            provider,
            model_name,
        } = tier
        {
            assert_eq!(provider, "anthropic");
            assert!(model_name.contains("sonnet"));
        } else {
            panic!("Expected Premium tier");
        }
    }

    #[tokio::test]
    /// # Panics
    /// Panics if a premium tier is not selected for high priority tasks.
    async fn test_high_priority_routing() {
        let strategy = QualityCriticalStrategy::new();

        let high_task = Task::new("High priority task".to_owned()).with_priority(Priority::High);

        assert!(strategy.applies_to(&high_task));
        let tier = match strategy.select_tier(&high_task).await {
            Ok(tier) => tier,
            Err(error) => panic!("failed to select tier for high priority task: {error}"),
        };
        assert!(matches!(tier, ModelTier::Premium { .. }));
    }

    #[tokio::test]
    /// # Panics
    /// Panics if applicability check fails unexpectedly.
    async fn test_low_priority_not_applicable() {
        let strategy = QualityCriticalStrategy::new();

        let low_task = Task::new("Low priority task".to_owned()).with_priority(Priority::Low);

        assert!(!strategy.applies_to(&low_task));
    }
}
