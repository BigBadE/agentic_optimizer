use async_trait::async_trait;
use crate::{ModelTier, Priority, Result, Task};
use super::super::strategy::RoutingStrategy;

/// Routes tasks to minimize cost (prefer local/free tiers)
pub struct CostOptimizationStrategy {
    max_tokens_for_local: usize,
}

impl CostOptimizationStrategy {
    #[must_use] 
    pub fn new(max_tokens_for_local: usize) -> Self {
        Self {
            max_tokens_for_local,
        }
    }
}

impl Default for CostOptimizationStrategy {
    fn default() -> Self {
        Self::new(4000)
    }
}

#[async_trait]
impl RoutingStrategy for CostOptimizationStrategy {
    fn applies_to(&self, task: &Task) -> bool {
        task.priority != Priority::Critical
    }
    
    async fn select_tier(&self, task: &Task) -> Result<ModelTier> {
        if task.context_needs.estimated_tokens <= self.max_tokens_for_local {
            Ok(ModelTier::Local {
                model_name: "qwen2.5-coder:7b".to_string(),
            })
        } else if task.context_needs.estimated_tokens <= 8000 {
            Ok(ModelTier::Groq {
                model_name: "llama-3.1-70b-versatile".to_string(),
            })
        } else {
            Ok(ModelTier::Premium {
                provider: "openrouter".to_string(),
                model_name: "deepseek/deepseek-coder".to_string(),
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
        let strategy = CostOptimizationStrategy::new(4000);
        
        let small_task = Task::new("Small task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(2000));
        let tier = strategy.select_tier(&small_task).await.unwrap();
        assert!(matches!(tier, ModelTier::Local { .. }));
        
        let medium_task = Task::new("Medium task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(6000));
        let tier = strategy.select_tier(&medium_task).await.unwrap();
        assert!(matches!(tier, ModelTier::Groq { .. }));
        
        let large_task = Task::new("Large task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(10000));
        let tier = strategy.select_tier(&large_task).await.unwrap();
        assert!(matches!(tier, ModelTier::Premium { .. }));
    }
    
    #[tokio::test]
    async fn test_critical_tasks_not_applicable() {
        let strategy = CostOptimizationStrategy::new(4000);
        
        let critical_task = Task::new("Critical task".to_string())
            .with_priority(Priority::Critical);
        
        assert!(!strategy.applies_to(&critical_task));
    }
}
