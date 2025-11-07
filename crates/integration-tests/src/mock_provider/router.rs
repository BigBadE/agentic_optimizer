//! Mock router for testing.

use async_trait::async_trait;
use merlin_core::Result;
use merlin_deps::tracing;
use merlin_routing::{Model, ModelRouter, RoutingDecision, Task as RoutingTask};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Mock router that can simulate multi-tier routing and escalation
pub struct MockRouter {
    /// Current tier being used (increments on each route call for escalation testing)
    tier: AtomicUsize,
    /// Available tiers (provider names in order of escalation)
    tiers: Vec<&'static str>,
}

impl MockRouter {
    /// Create a new mock router with default single tier
    #[must_use]
    pub fn new() -> Self {
        Self {
            tier: AtomicUsize::new(0),
            tiers: vec!["test-mock"],
        }
    }
}

impl Default for MockRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ModelRouter for MockRouter {
    async fn route(&self, _task: &RoutingTask) -> Result<RoutingDecision> {
        let tier_idx = self.tier.fetch_add(1, Ordering::SeqCst);

        let tier_name = self
            .tiers
            .get(tier_idx.min(self.tiers.len() - 1))
            .unwrap_or(&"test-mock");

        tracing::debug!("MockRouter routing to tier {} ({})", tier_idx, tier_name);

        Ok(RoutingDecision {
            model: Model::Qwen25Coder32B,
            estimated_cost: 0.0,
            estimated_latency_ms: 100,
            reasoning: format!("Test routing to tier {tier_idx}"),
        })
    }

    async fn is_available(&self, _model: &Model) -> bool {
        true
    }
}
