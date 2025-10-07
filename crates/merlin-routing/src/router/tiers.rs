use std::cmp::Reverse;
use std::sync::Arc;
use crate::{ModelRouter, ModelTier, Result, RoutingDecision, RoutingError, Task};
use super::strategy::RoutingStrategy;
use async_trait::async_trait;

/// Availability checker for model tiers
pub struct AvailabilityChecker {
    // In a real implementation, this would track quotas, rate limits, etc.
}

impl AvailabilityChecker {
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
    
    #[must_use] 
    pub fn check(&self, _tier: &ModelTier) -> bool {
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
    local_enabled: bool,
    groq_enabled: bool,
    premium_enabled: bool,
}

impl StrategyRouter {
    #[must_use]
    pub fn new(strategies: Vec<Arc<dyn RoutingStrategy>>) -> Self {
        let mut sorted_strategies = strategies;
        sorted_strategies.sort_by_key(|strategy| Reverse(strategy.priority()));
        
        Self {
            strategies: sorted_strategies,
            availability_checker: Arc::new(AvailabilityChecker::new()),
            local_enabled: true,
            groq_enabled: true,
            premium_enabled: true,
        }
    }

    #[must_use]
    pub fn with_tier_config(mut self, local: bool, groq: bool, premium: bool) -> Self {
        self.local_enabled = local;
        self.groq_enabled = groq;
        self.premium_enabled = premium;
        self
    }
    
    #[must_use]
    pub fn with_default_strategies() -> Self {
        use super::strategies::{QualityCriticalStrategy, LongContextStrategy, CostOptimizationStrategy, ComplexityBasedStrategy};
        
        let strategies: Vec<Arc<dyn RoutingStrategy>> = vec![
            Arc::new(QualityCriticalStrategy::new()),
            Arc::new(LongContextStrategy::default()),
            Arc::new(CostOptimizationStrategy::default()),
            Arc::new(ComplexityBasedStrategy::new()),
        ];
        
        Self::new(strategies)
    }
    
    fn estimate_cost(tier: &ModelTier, task: &Task) -> f64 {
        let tokens = task.context_needs.estimated_tokens as f64;
        
        match tier {
            ModelTier::Local { .. } | ModelTier::Groq { .. } => 0.0,
            ModelTier::Premium { model_name, .. } => {
                if model_name.contains("sonnet") {
                    tokens * 0.000_015
                } else if model_name.contains("haiku") {
                    tokens * 0.000_001
                } else if model_name.contains("deepseek") {
                    tokens * 0.000_000_2
                } else {
                    tokens * 0.00001
                }
            }
        }
    }
    
    fn estimate_latency(tier: &ModelTier) -> u64 {
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
        // Try each strategy in priority order
        for strategy in &self.strategies {
            if !strategy.applies_to(task) {
                continue;
            }
            
            let tier = strategy.select_tier(task).await?;
            
            // Check if tier is enabled in config
            let tier_enabled = match &tier {
                ModelTier::Local { .. } => self.local_enabled,
                ModelTier::Groq { .. } => self.groq_enabled,
                ModelTier::Premium { .. } => self.premium_enabled,
            };
            
            if tier_enabled && self.is_available(&tier).await {
                return Ok(RoutingDecision {
                    tier: tier.clone(),
                    estimated_cost: Self::estimate_cost(&tier, task),
                    estimated_latency_ms: Self::estimate_latency(&tier),
                    reasoning: format!("Selected by {} strategy", strategy.name()),
                });
            }
        }
        
        // Fallback: Try any enabled tier
        if self.groq_enabled {
            return Ok(RoutingDecision {
                tier: ModelTier::Groq {
                    model_name: "llama-3.1-70b-versatile".to_owned(),
                },
                estimated_cost: 0.0,
                estimated_latency_ms: 500,
                reasoning: "Fallback to Groq (no other tiers available)".to_owned(),
            });
        }
        
        if self.local_enabled {
            return Ok(RoutingDecision {
                tier: ModelTier::Local {
                    model_name: "qwen2.5-coder:7b".to_owned(),
                },
                estimated_cost: 0.0,
                estimated_latency_ms: 100,
                reasoning: "Fallback to Local (no other tiers available)".to_owned(),
            });
        }
        
        Err(RoutingError::NoAvailableTier)
    }
    
    async fn is_available(&self, tier: &ModelTier) -> bool {
        self.availability_checker.check(tier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Complexity, ContextRequirements, Priority};
    use crate::Result;

    #[tokio::test]
    /// # Errors
    /// Returns an error if routing fails unexpectedly in the test harness.
    ///
    /// # Panics
    /// Panics if premium tier is not selected for critical tasks.
    async fn test_strategy_router_priority() -> Result<()> {
        let router = StrategyRouter::with_default_strategies();
        
        let critical_task = Task::new("Critical task".to_owned())
            .with_priority(Priority::Critical)
            .with_complexity(Complexity::Simple);
        
        let decision = router.route(&critical_task).await?;
        
        if let ModelTier::Premium { provider, .. } = decision.tier {
            assert_eq!(provider, "anthropic");
        } else { panic!("Critical task should use premium tier"); }
        
        assert!(decision.reasoning.contains("QualityCritical"));
        Ok(())
    }
    
    #[tokio::test]
    /// # Errors
    /// Returns an error if routing fails unexpectedly in the test harness.
    ///
    /// # Panics
    /// Panics if long context strategy does not select a premium tier.
    async fn test_long_context_strategy() -> Result<()> {
        let router = StrategyRouter::with_default_strategies();
        
        let long_context_task = Task::new("Long context task".to_owned())
            .with_context(ContextRequirements::new().with_estimated_tokens(50000));
        
        let decision = router.route(&long_context_task).await?;
        assert!(matches!(decision.tier, ModelTier::Premium { .. }));
        assert!(decision.reasoning.contains("LongContext"));
        Ok(())
    }
    
    #[tokio::test]
    /// # Errors
    /// Returns an error if routing fails unexpectedly in the test harness.
    ///
    /// # Panics
    /// Panics if low-cost tier is not selected for cheap tasks.
    async fn test_cost_optimization() -> Result<()> {
        let router = StrategyRouter::with_default_strategies();
        
        let cheap_task = Task::new("Cheap task".to_owned())
            .with_priority(Priority::Low)
            .with_context(ContextRequirements::new().with_estimated_tokens(2000));
        
        let decision = router.route(&cheap_task).await?;
        // Floating-point comparison: use epsilon to avoid pedantic float-cmp lint
        assert!(decision.estimated_cost.abs() < f64::EPSILON);
        Ok(())
    }
    
    #[tokio::test]
    /// # Errors
    /// Returns an error if routing fails unexpectedly in the test harness.
    ///
    /// # Panics
    /// Panics if router reasoning does not mention expected strategies for medium tasks.
    async fn test_complexity_fallback() -> Result<()> {
        let router = StrategyRouter::with_default_strategies();
        
        let medium_task = Task::new("Medium task".to_owned())
            .with_complexity(Complexity::Medium)
            .with_priority(Priority::Medium);
        
        let decision = router.route(&medium_task).await?;
        assert!(decision.reasoning.contains("Complexity") || decision.reasoning.contains("Cost"));
        Ok(())
    }
}
