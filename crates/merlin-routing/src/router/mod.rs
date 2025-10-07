pub mod strategies;
pub mod strategy;
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

/// Model tier selection
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    Local {
        model_name: String,
    },
    Groq {
        model_name: String,
    },
    Premium {
        provider: String,
        model_name: String,
    },
}

impl ModelTier {
    /// Get next higher tier for escalation
    #[must_use]
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

/// Routing decision with rationale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub tier: ModelTier,
    pub estimated_cost: f64,
    pub estimated_latency_ms: u64,
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
