//! Model routing and tier selection.
//!
//! This module handles intelligent routing of tasks to appropriate model tiers
//! based on complexity, cost, quality requirements, and context size.

/// Concrete routing strategy implementations
pub mod strategies;
/// Base routing strategy trait
pub mod strategy;
/// Tier management and availability checking
pub mod tiers;

use crate::{Result, Task};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

pub use strategies::{
    ComplexityBasedStrategy, CostOptimizationStrategy, LongContextStrategy, QualityCriticalStrategy,
};
pub use strategy::RoutingStrategy;
pub use tiers::{AvailabilityChecker, StrategyRouter};

/// Model tier selection for routing tasks to appropriate models.
///
/// Tiers are ordered from cheapest/fastest (Local) to most expensive/capable (Premium).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    /// Local model running on user's machine (e.g., Ollama)
    Local {
        /// Name of the local model
        model_name: String,
    },
    /// Fast cloud model via Groq
    Groq {
        /// Name of the Groq model
        model_name: String,
    },
    /// Premium cloud model (e.g., Claude, GPT-4)
    Premium {
        /// Provider name (e.g., "anthropic", "openrouter")
        provider: String,
        /// Name of the premium model
        model_name: String,
    },
}

impl ModelTier {
    /// Get next higher tier for escalation
    pub fn escalate(&self) -> Option<Self> {
        match self {
            Self::Local { .. } => Some(Self::Groq {
                model_name: "llama-3.1-70b-versatile".to_owned(),
            }),
            Self::Groq { .. } => Some(Self::Premium {
                provider: "openrouter".to_owned(),
                model_name: "deepseek/deepseek-coder".to_owned(),
            }),
            Self::Premium { model_name, .. } if model_name.contains("deepseek") => {
                Some(Self::Premium {
                    provider: "openrouter".to_owned(),
                    model_name: "anthropic/claude-3-haiku".to_owned(),
                })
            }
            Self::Premium { model_name, .. } if model_name.contains("haiku") => {
                Some(Self::Premium {
                    provider: "anthropic".to_owned(),
                    model_name: "claude-3.5-sonnet".to_owned(),
                })
            }
            Self::Premium { .. } => None,
        }
    }
}

impl Display for ModelTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Local { model_name } => write!(f, "Local({model_name})"),
            Self::Groq { model_name } => write!(f, "Groq({model_name})"),
            Self::Premium {
                provider,
                model_name,
            } => write!(f, "{provider}/{model_name}"),
        }
    }
}

/// Routing decision with rationale and cost estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    /// Selected model tier for this task
    pub tier: ModelTier,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Estimated latency in milliseconds
    pub estimated_latency_ms: u64,
    /// Explanation of why this tier was chosen
    pub reasoning: String,
}

/// Trait for routing strategies
#[async_trait]
pub trait ModelRouter: Send + Sync {
    /// Route a task to appropriate model tier
    async fn route(&self, task: &Task) -> Result<RoutingDecision>;

    /// Check if a tier is available and has quota
    async fn is_available(&self, tier: &ModelTier) -> bool;
}
