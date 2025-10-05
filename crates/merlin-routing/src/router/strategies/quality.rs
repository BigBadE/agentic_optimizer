use async_trait::async_trait;
use crate::{ModelTier, Priority, Result, Task};
use super::super::strategy::RoutingStrategy;

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
                provider: "anthropic".to_string(),
                model_name: "claude-3.5-sonnet".to_string(),
            })
        } else {
            Ok(ModelTier::Premium {
                provider: "openrouter".to_string(),
                model_name: "anthropic/claude-3-haiku".to_string(),
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
    async fn test_quality_critical_routing() {
        let strategy = QualityCriticalStrategy::new();
        
        let critical_task = Task::new("Critical task".to_string())
            .with_priority(Priority::Critical);
        
        assert!(strategy.applies_to(&critical_task));
        let tier = strategy.select_tier(&critical_task).await.unwrap();
        
        if let ModelTier::Premium { provider, model_name } = tier {
            assert_eq!(provider, "anthropic");
            assert!(model_name.contains("sonnet"));
        } else {
            panic!("Expected Premium tier");
        }
    }
    
    #[tokio::test]
    async fn test_high_priority_routing() {
        let strategy = QualityCriticalStrategy::new();
        
        let high_task = Task::new("High priority task".to_string())
            .with_priority(Priority::High);
        
        assert!(strategy.applies_to(&high_task));
        let tier = strategy.select_tier(&high_task).await.unwrap();
        assert!(matches!(tier, ModelTier::Premium { .. }));
    }
    
    #[tokio::test]
    async fn test_low_priority_not_applicable() {
        let strategy = QualityCriticalStrategy::new();
        
        let low_task = Task::new("Low priority task".to_string())
            .with_priority(Priority::Low);
        
        assert!(!strategy.applies_to(&low_task));
    }
}
