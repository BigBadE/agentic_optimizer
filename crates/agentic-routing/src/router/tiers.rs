use std::sync::Arc;
use crate::{ModelRouter, ModelTier, Result, RoutingDecision, RoutingError, Task};
use super::strategy::RoutingStrategy;
use async_trait::async_trait;

/// Availability checker for model tiers
pub struct AvailabilityChecker {
    // In a real implementation, this would track quotas, rate limits, etc.
}

impl AvailabilityChecker {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn check(&self, _tier: &ModelTier) -> bool {
        // For now, assume all tiers are available
        // In production, this would check:
        // - API key presence
        // - Rate limit status
        // - Quota remaining
        // - Service health
        true
    }
}

impl Default for AvailabilityChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Strategy-based router implementation
pub struct StrategyRouter {
    strategies: Vec<Arc<dyn RoutingStrategy>>,
    availability_checker: Arc<AvailabilityChecker>,
}

impl StrategyRouter {
    pub fn new(strategies: Vec<Arc<dyn RoutingStrategy>>) -> Self {
        let mut sorted_strategies = strategies;
        sorted_strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        
        Self {
            strategies: sorted_strategies,
            availability_checker: Arc::new(AvailabilityChecker::new()),
        }
    }
    
    pub fn with_default_strategies() -> Self {
        use super::strategies::*;
        
        let strategies: Vec<Arc<dyn RoutingStrategy>> = vec![
            Arc::new(QualityCriticalStrategy::new()),
            Arc::new(LongContextStrategy::default()),
            Arc::new(CostOptimizationStrategy::default()),
            Arc::new(ComplexityBasedStrategy::new()),
        ];
        
        Self::new(strategies)
    }
    
    fn estimate_cost(&self, tier: &ModelTier, task: &Task) -> f64 {
        let tokens = task.context_needs.estimated_tokens as f64;
        
        match tier {
            ModelTier::Local { .. } => 0.0,
            ModelTier::Groq { .. } => 0.0,
            ModelTier::Premium { model_name, .. } => {
                if model_name.contains("sonnet") {
                    tokens * 0.000015
                } else if model_name.contains("haiku") {
                    tokens * 0.000001
                } else if model_name.contains("deepseek") {
                    tokens * 0.0000002
                } else {
                    tokens * 0.00001
                }
            }
        }
    }
    
    fn estimate_latency(&self, tier: &ModelTier) -> u64 {
        match tier {
            ModelTier::Local { .. } => 100,
            ModelTier::Groq { .. } => 500,
            ModelTier::Premium { .. } => 2000,
        }
    }
}

#[async_trait]
impl ModelRouter for StrategyRouter {
    async fn route(&self, task: &Task) -> Result<RoutingDecision> {
        for strategy in &self.strategies {
            if !strategy.applies_to(task) {
                continue;
            }
            
            let tier = strategy.select_tier(task).await?;
            
            if self.is_available(&tier).await {
                return Ok(RoutingDecision {
                    tier: tier.clone(),
                    estimated_cost: self.estimate_cost(&tier, task),
                    estimated_latency_ms: self.estimate_latency(&tier),
                    reasoning: format!("Selected by {} strategy", strategy.name()),
                });
            }
        }
        
        Err(RoutingError::NoAvailableTier)
    }
    
    async fn is_available(&self, tier: &ModelTier) -> bool {
        self.availability_checker.check(tier).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Complexity, ContextRequirements, Priority};

    #[tokio::test]
    async fn test_strategy_router_priority() {
        let router = StrategyRouter::with_default_strategies();
        
        let critical_task = Task::new("Critical task".to_string())
            .with_priority(Priority::Critical)
            .with_complexity(Complexity::Simple);
        
        let decision = router.route(&critical_task).await.unwrap();
        
        if let ModelTier::Premium { provider, .. } = decision.tier {
            assert_eq!(provider, "anthropic");
        } else {
            panic!("Critical task should use premium tier");
        }
        
        assert!(decision.reasoning.contains("QualityCritical"));
    }
    
    #[tokio::test]
    async fn test_long_context_strategy() {
        let router = StrategyRouter::with_default_strategies();
        
        let long_context_task = Task::new("Long context task".to_string())
            .with_context(ContextRequirements::new().with_estimated_tokens(50000));
        
        let decision = router.route(&long_context_task).await.unwrap();
        assert!(matches!(decision.tier, ModelTier::Premium { .. }));
        assert!(decision.reasoning.contains("LongContext"));
    }
    
    #[tokio::test]
    async fn test_cost_optimization() {
        let router = StrategyRouter::with_default_strategies();
        
        let cheap_task = Task::new("Cheap task".to_string())
            .with_priority(Priority::Low)
            .with_context(ContextRequirements::new().with_estimated_tokens(2000));
        
        let decision = router.route(&cheap_task).await.unwrap();
        assert_eq!(decision.estimated_cost, 0.0);
    }
    
    #[tokio::test]
    async fn test_complexity_fallback() {
        let router = StrategyRouter::with_default_strategies();
        
        let medium_task = Task::new("Medium task".to_string())
            .with_complexity(Complexity::Medium)
            .with_priority(Priority::Medium);
        
        let decision = router.route(&medium_task).await.unwrap();
        assert!(decision.reasoning.contains("Complexity") || decision.reasoning.contains("Cost"));
    }
}
