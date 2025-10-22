//! Model routing and tier selection.
//!
//! This module handles intelligent routing of tasks to appropriate model tiers
//! based on difficulty ratings (1-10).

/// Model registry for difficulty-based routing
pub mod model_registry;
/// Model definitions and enumerations
pub mod models;
/// Provider registry for managing provider instances
pub mod provider_registry;
/// Tier management and availability checking
pub mod tiers;

use crate::{Result, Task};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use model_registry::ModelRegistry;
pub use models::{Model, TierCategory};
pub use provider_registry::ProviderRegistry;
pub use tiers::{AvailabilityChecker, StrategyRouter};

/// Routing decision with rationale and cost estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected model for this task
    pub model: Model,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u64,
    /// Explanation of why this model was chosen
    pub reasoning: String,
}

impl Default for RoutingDecision {
    fn default() -> Self {
        Self::new(Model::default(), "Default routing decision".to_owned())
    }
}

impl RoutingDecision {
    /// Create a new routing decision.
    #[must_use]
    pub fn new(model: Model, reasoning: String) -> Self {
        let estimated_cost = Self::estimate_cost(model);
        let estimated_latency_ms = Self::estimate_latency(model);

        Self {
            model,
            estimated_cost,
            estimated_latency_ms,
            reasoning,
        }
    }

    /// Estimate cost for a model based on rough token usage.
    const fn estimate_cost(model: Model) -> f64 {
        // Assume average of 10k tokens per request
        model.cost_per_million_tokens() * 0.01
    }

    /// Estimate latency for a model.
    const fn estimate_latency(model: Model) -> u64 {
        match model.tier_category() {
            TierCategory::Local => 100,
            TierCategory::Groq => 500,
            TierCategory::Premium => 2000,
        }
    }
}

/// Trait for routing strategies
#[async_trait]
pub trait ModelRouter: Send + Sync {
    /// Route a task to appropriate model tier
    async fn route(&self, task: &Task) -> Result<RoutingDecision>;

    /// Check if a model is available and has quota
    async fn is_available(&self, model: &Model) -> bool;
}
