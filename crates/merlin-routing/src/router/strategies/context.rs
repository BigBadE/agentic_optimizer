use async_trait::async_trait;
use crate::{ModelTier, Result, Task};
use super::super::strategy::RoutingStrategy;

/// Routes tasks with long context to appropriate models
pub struct LongContextStrategy {
    long_context_threshold: usize,
}

impl LongContextStrategy {
    #[must_use] 
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
        if task.context_needs.estimated_tokens > 100000 {
            Ok(ModelTier::Premium {
                provider: "anthropic".to_string(),
                model_name: "claude-3.5-sonnet".to_string(),
            })
        } else if task.context_needs.estimated_tokens > 32000 {
            Ok(ModelTier::Premium {
                provider: "openrouter".to_string(),
                model_name: "anthropic/claude-3-haiku".to_string(),
            })
        } else {
            Ok(ModelTier::Groq {
                model_name: "llama-3.1-70b-versatile".to_string(),
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
        
        let huge_context = Task::new("Huge context task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(150000));
        
        assert!(strategy.applies_to(&huge_context));
        let tier = strategy.select_tier(&huge_context).await.unwrap();
        
        if let ModelTier::Premium { model_name, .. } = tier {
            assert!(model_name.contains("sonnet"));
        } else {
            panic!("Expected Premium tier for huge context");
        }
    }
    
    #[tokio::test]
    async fn test_medium_long_context() {
        let strategy = LongContextStrategy::new(16000);
        
        let medium_context = Task::new("Medium context task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(50000));
        
        assert!(strategy.applies_to(&medium_context));
        let tier = strategy.select_tier(&medium_context).await.unwrap();
        assert!(matches!(tier, ModelTier::Premium { .. }));
    }
    
    #[tokio::test]
    async fn test_requires_full_context() {
        let strategy = LongContextStrategy::new(16000);
        
        let full_context = Task::new("Full context task".to_string())
            .with_context(
                ContextRequirements::new()
                    .with_estimated_tokens(10000)
                    .with_full_context(true)
            );
        
        assert!(strategy.applies_to(&full_context));
    }
}
